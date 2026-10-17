use dioxus::logger::tracing::error;
use dioxus::prelude::dioxus_fullstack::{
    use_websocket, WebSocketOptions, Websocket, WebsocketError,
};
use dioxus::prelude::*;
use futures::future::join_all;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::sync::{LazyLock, Mutex};

#[cfg(feature = "server")]
use tokio::sync::broadcast;

#[cfg(feature = "server")]
use tokio::sync::broadcast::error::RecvError;

#[cfg(feature = "server")]
use dioxus::prelude::dioxus_fullstack::TypedWebsocket;

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct RecordingMetadata {
    id: String,
    en: String, // common name
}

#[derive(Clone, Debug, PartialEq)]
struct Recording {
    metadata: RecordingMetadata,
    url: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ImageLinks {
    en: String,
    url: String,
    square_url: String,
    medium_url: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Bird {
    recording: Recording,
    image_links: ImageLinks,
}

/*
 * multiplayer plan:
 * 1. user can create a room, they type in their name and get back a room name
 * 2. user can try to connect to a room. they type in the room number and name, numbers appended to
 *    name to resolve conflict
 * 3. users request a random seed from the server for their room. on re-draw, server sends the
 *    room's seed to players waiting to join, as well as the round number
 * 4. on join, player generates a layout with (room_seed + round_number) as their rng seed. in
 *    particular, this keeps computation lighter on the server
 * 5. once a user is ready to guess, they send request to the server, which is fulfilled once every
 *    user is ready. then correct guesses are displayed
 * 6. once both users click continue, new round begins (and they receive new round number from the
 *    server)
 *
 *
 *
 *    room holds broadcast channel? so that each connected player can get updates on room state
 *    changes? room owner is authoritative?
 *    maybe:
 *    1. room holds state and channel
 *    2. clients try to apply state updates. when successful, send to channel if necessary, before
 *       releasing lock on room
 *
 *    client and server keep copies of room state, try to keep operations idempotent. at some
 *    interval (like between rounds) state can get re-synced
 */
struct Guessing {
    // what sounds are in each sound box
    sound_positions: Vec<usize>,

    // what sounds the user guessed for each sound box
    sound_guesses: Vec<Option<usize>>,

    // what sound box the user has selected, for guessing
    selected_sound: Option<usize>,

    // what sounds the user submitted for each sound box
    submitted: Option<Vec<usize>>,

    // number of rounds where the user guessed [i] sounds correctly
    correct_guesses_per_round: Vec<u32>,
}

fn main() {
    dioxus::launch(App);
}

static CSS: Asset = asset!("/assets/main.css");
static BIRDS_URL_DIR: &str = "/birds/";

#[cfg(feature = "server")]
static SOUND_RATINGS_DB: &str =
    "/Users/diegoesparza/CS_Ventures/current_projects/bird-call-matching/sound-ratings.db";

#[server]
async fn upvote_sound(id: String) -> Result<(), ServerFnError> {
    use rusqlite::Connection;

    let conn = Connection::open(SOUND_RATINGS_DB).map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "INSERT INTO sound_ratings (id, upvotes, downvotes) VALUES (?1, 1, 0)
         ON CONFLICT(id) DO UPDATE SET upvotes = upvotes + 1",
        (id,),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
async fn downvote_sound(id: String) -> Result<(), ServerFnError> {
    use rusqlite::Connection;

    let conn = Connection::open(SOUND_RATINGS_DB).map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "INSERT INTO sound_ratings (id, upvotes, downvotes) VALUES (?1, 0, 1)
         ON CONFLICT(id) DO UPDATE SET downvotes = downvotes + 1",
        (id,),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
async fn get_ratings(id: String) -> Result<(i64, i64), ServerFnError> {
    use rusqlite::{Connection, Error as RusqliteError};

    let conn = Connection::open(SOUND_RATINGS_DB).map_err(|e| ServerFnError::new(e.to_string()))?;

    let result = conn.query_row(
        "SELECT upvotes, downvotes FROM sound_ratings WHERE id = ?1",
        (id,),
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok(ratings) => Ok(ratings),
        Err(RusqliteError::QueryReturnedNoRows) => Ok((0, 0)),
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}

#[cfg(feature = "server")]
struct Rooms {
    active_rooms: HashMap<usize, RoomServerState>,
    available_room_ids: Vec<usize>,
}

#[cfg(feature = "server")]
impl Rooms {
    fn new() -> Rooms {
        let active_rooms = HashMap::new();

        let max_rooms = 65000;
        let mut available_room_ids = Vec::with_capacity(max_rooms);
        for i in 0..max_rooms {
            available_room_ids.push(i);
        }
        Rooms {
            active_rooms,
            available_room_ids,
        }
    }

    fn create_room(&mut self, initial_player: Player) -> Result<usize, String> {
        let room_number = match self.available_room_ids.pop() {
            Some(number) => Ok(number),
            None => Err("No available capacity for new rooms"),
        }?;

        self.active_rooms.insert(
            room_number,
            RoomServerState {
                channel: broadcast::channel(100).0,
                room_shared_state: RoomSharedState::new(room_number, initial_player),
            },
        );
        Ok(room_number)
    }

    fn map_room<A>(
        &mut self,
        room_number: usize,
        f: impl FnOnce(&mut RoomServerState) -> A,
    ) -> Result<A, String> {
        match self.active_rooms.get_mut(&room_number) {
            Some(room_handle) => Ok(f(room_handle)),
            None => Err("Invalid room number".to_string()),
        }
    }

    fn bind_room<A>(
        &mut self,
        room_number: usize,
        f: impl FnOnce(&mut RoomServerState) -> Result<A, String>,
    ) -> Result<A, String> {
        match self.map_room(room_number, f) {
            Err(err) => Err(err),
            Ok(Err(err)) => Err(err),
            Ok(Ok(res)) => Ok(res),
        }
    }

    // Returns Ok(String) with the actual name for the player, possibly modified to avoid
    // collisions, and the list of all players
    fn player_join(
        &mut self,
        room_number: usize,
        player_name: String,
    ) -> Result<(String, Vec<String>), String> {
        let (player_name, current_players) = self.map_room(room_number, |room_handle| {
            let mut unique_player_name = player_name.clone();
            let mut unique_suffix = 1;
            while room_handle
                .room_shared_state
                .players
                .contains_key(&unique_player_name)
            {
                unique_suffix += 1;
                unique_player_name = format!("{player_name}_{unique_suffix}");
            }

            let new_player = Player {
                name: unique_player_name.clone(),
                correct_guesses_per_round: vec![0, 0, 0, 0],
            };
            let _ = room_handle
                .room_shared_state
                .players
                .insert(new_player.name.clone(), new_player);
            let current_players = room_handle
                .room_shared_state
                .players
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            (unique_player_name, current_players)
        })?;

        Ok((player_name, current_players))
    }

    fn subscribe(&mut self, room_number: usize) -> Result<broadcast::Receiver<Broadcast>, String> {
        self.map_room(room_number, |room_handle| room_handle.channel.subscribe())
    }

    fn send(&mut self, room_number: usize, broadcast: Broadcast) -> Result<(), String> {
        self.bind_room(room_number, |room_handle| {
            match room_handle.channel.send(broadcast) {
                Ok(_) => Ok(()),
                Err(err) => Err(format!("{err}")),
            }
        })
    }
}

#[cfg(feature = "server")]
static ROOMS: LazyLock<Mutex<Rooms>> = LazyLock::new(|| Mutex::new(Rooms::new()));

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct Player {
    name: String,
    correct_guesses_per_round: Vec<u32>,
}

#[cfg(feature = "server")]
struct RoomServerState {
    channel: broadcast::Sender<Broadcast>,
    room_shared_state: RoomSharedState,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
enum RoomStage {
    Lobby,
    Round(usize),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct RoomSharedState {
    room_number: usize,
    players: HashMap<String, Player>,
    stage: RoomStage,
}

impl RoomSharedState {
    fn new(room_number: usize, initial_player: Player) -> RoomSharedState {
        RoomSharedState {
            room_number,
            players: HashMap::from([(initial_player.name.clone(), initial_player)]),
            stage: RoomStage::Lobby,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
enum ClientMessage {
    JoinRoom {
        room_number: usize,
        player_name: String,
    },
    CreateRoom {
        player_name: String,
    },
    StartGame {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
enum ServerMessage {
    YouJoined {
        room_number: usize,
        your_name: String,
        current_players: Vec<String>,
    },
    OtherPlayerJoined {
        name: String,
    },
}

#[derive(Clone)]
enum Broadcast {
    PlayerJoined { name: String },
}

// returns room number, player name and current_players if joined/created successfully
#[cfg(feature = "server")]
fn handle_first_msg(client_msg: ClientMessage) -> Result<(usize, String, Vec<String>), String> {
    match client_msg {
        ClientMessage::JoinRoom {
            room_number,
            player_name,
        } => {
            let (new_player_name, current_players) = ROOMS
                .lock()
                .unwrap()
                .player_join(room_number, player_name)?;
            match ROOMS.lock().unwrap().send(
                room_number,
                Broadcast::PlayerJoined {
                    name: new_player_name.clone(),
                },
            ) {
                Ok(()) => (),
                Err(err) => {
                    // We should still return okay because we updated server state. other clients can
                    // sync back up. Or, we could rely on broadcast message to confirm ourselves
                    // joining.
                    println!("Failed to broadcast player joining {err}")
                }
            };
            Ok((room_number, new_player_name, current_players))
        }
        ClientMessage::CreateRoom { player_name } => {
            let player = Player {
                name: player_name.clone(),
                correct_guesses_per_round: vec![0, 0, 0, 0],
            };
            let room_number = ROOMS.lock().unwrap().create_room(player)?;
            Ok((room_number, player_name.clone(), vec![player_name]))
        }
        ClientMessage::StartGame {} => {
            Err("Unexpected request to start game, player not in room yet.".to_string())
        }
    }
}

#[cfg(feature = "server")]
fn handle_msg(
    client_msg: Result<ClientMessage, WebsocketError>,
    socket: &mut TypedWebsocket<ClientMessage, ServerMessage>,
) -> Result<(), String> {
    match client_msg {
        Ok(ClientMessage::JoinRoom {
            room_number,
            player_name,
        }) => Err(format!(
            "Received JoinRoom message from player {player_name}, for room {room_number}, but they're already in a room"
        )),
        Ok(ClientMessage::CreateRoom{player_name}) => Err(format!(
            "Received CreateRoom message from player {player_name} but they're already in a room"
        )),
        Ok(ClientMessage::StartGame {}) => {
            // TODO
            Ok(())
        }
        Err(websocket_error) => Err(format!("{websocket_error}")),
    }
}

#[cfg(feature = "server")]
async fn handle_broadcast(
    broadcast: Result<Broadcast, RecvError>,
    socket: &mut TypedWebsocket<ClientMessage, ServerMessage>,
) {
    match broadcast {
        Ok(Broadcast::PlayerJoined { name }) => {
            let _ = socket.send(ServerMessage::OtherPlayerJoined { name }).await;
        }
        Err(_) => (),
    }
}

#[get("/api/game_ws")]
async fn game_socket(options: WebSocketOptions) -> Result<Websocket<ClientMessage, ServerMessage>> {
    Ok(options.on_upgrade(
        |mut socket: TypedWebsocket<ClientMessage, ServerMessage>| async move {
            let first_msg = socket.recv().await;
            let room_info = match first_msg {
                Ok(first_msg) => handle_first_msg(first_msg),
                Err(err) => Err(format!("client left? {err}")),
            };

            match room_info {
                Err(err) => {
                    println!("client didn't join room: {err}")
                }
                Ok((room_number, player_name, current_players)) => {
                    let _ = socket
                        .send(ServerMessage::YouJoined {
                            room_number,
                            your_name: player_name,
                            current_players,
                        })
                        .await;

                    let receiver = ROOMS.lock().unwrap().subscribe(room_number);
                    match receiver {
                        Err(err) => {
                            println!("couldn't subscribe to room updates. {err}")
                        }
                        Ok(mut receiver) => loop {
                            tokio::select! {
                                client_msg = socket.recv() => {
                                    match
                                    handle_msg(client_msg, &mut socket) {
                                        Ok(()) => (),
                                        Err(_) => { println!("err"); break}
                                    }
                                },

                                broadcast = receiver.recv() => {
                                    handle_broadcast(broadcast, &mut socket).await
                                }
                            }
                        },
                    }
                }
            }
        },
    ))
}

fn origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

async fn choose_birds(num_birds: usize) -> Result<Vec<Bird>, reqwest::Error> {
    // read all-birds to get directory names
    let all_birds_text = reqwest::get(origin() + "/all-birds.txt")
        .await?
        .text()
        .await;
    let all_birds: Vec<String> = all_birds_text?
        .lines()
        .map(|line| line.trim().to_owned())
        .collect();

    // select random bird directories
    let mut rng = thread_rng();
    let random_birds: Vec<String> = all_birds
        .choose_multiple(&mut rng, num_birds)
        .cloned()
        .collect();

    // select a random sound directory from each bird
    let bird_images_and_sounds = random_birds.into_iter().map(async |bird| {
        let mut rng = thread_rng();
        let sounds_text = reqwest::get(origin() + BIRDS_URL_DIR + &bird + "/all-sounds.txt")
            .await?
            .text()
            .await?;
        let sounds: Vec<&str> = sounds_text.split_whitespace().collect();
        let random_sound = sounds.choose(&mut rng).unwrap();

        let image_links = reqwest::get(origin() + BIRDS_URL_DIR + &bird + "/images/metadata.json")
            .await?
            .json::<ImageLinks>()
            .await?;

        Ok::<(ImageLinks, String), reqwest::Error>((
            image_links,
            String::from(BIRDS_URL_DIR) + &bird + "/" + random_sound,
        ))
    });
    let bird_images_and_sounds: Vec<(ImageLinks, String)> = join_all(bird_images_and_sounds)
        .await
        .into_iter()
        .collect::<Result<_, reqwest::Error>>()?;

    // get metadatas from each sound dir
    let images_and_sound_metadatas_and_dirs =
        bird_images_and_sounds
            .into_iter()
            .map(async |(image_links, sound_dir)| {
                let metadata = reqwest::get(origin() + &sound_dir + "/metadata.json")
                    .await?
                    .json::<RecordingMetadata>()
                    .await?;
                Ok::<(ImageLinks, RecordingMetadata, String), reqwest::Error>((
                    image_links,
                    metadata,
                    sound_dir,
                ))
            });
    let images_and_sound_metadatas_and_dirs: Vec<(ImageLinks, RecordingMetadata, String)> =
        join_all(images_and_sound_metadatas_and_dirs)
            .await
            .into_iter()
            .collect::<Result<_, reqwest::Error>>()?;

    // convert to [Recording]s and return
    let birds = images_and_sound_metadatas_and_dirs
        .into_iter()
        .map(|(image_links, metadata, dir)| {
            let recording = Recording {
                metadata,
                url: dir + "/sound.wav",
            };
            Bird {
                recording,
                image_links,
            }
        })
        .collect();
    Ok(birds)
}

#[component]
fn App() -> Element {
    let mut birds_resource = use_resource(|| async { choose_birds(5).await });

    let guessing = use_signal(|| {
        let mut rng = thread_rng();
        let mut sound_positions = vec![0, 1, 2, 3, 4];
        sound_positions.shuffle(&mut rng);
        Guessing {
            sound_positions,
            sound_guesses: vec![None, None, None],
            selected_sound: None,
            submitted: None,
            correct_guesses_per_round: vec![0, 0, 0, 0],
        }
    });

    rsx! {
        document::Stylesheet { href: CSS }
        main { class: "app",
            Title {}
            match &*birds_resource.read() {
                Some(Ok(birds)) => rsx! {
                    Multiplayer {}
                    Sounds {
                        recordings: birds.iter().map(|bird| bird.recording.clone()).collect(),
                        guessing,
                    }
                    Images {
                        image_links: birds.iter().map(|bird| bird.image_links.clone()).collect(),
                        guessing,
                    }
                    Redraw { birds_resource, guessing }
                    Score { guessing }
                },
                Some(Err(e)) => rsx! {
                    div { class: "status error", "Couldn't load birds: {e}" }
                    button { class: "btn primary", onclick: move |_| birds_resource.restart(), "Try again" }
                },
                None => rsx! {
                    div { class: "status", "Loading birds…" }
                },
            }
        }
    }
}

#[component]
fn Title() -> Element {
    rsx! {
        header { class: "masthead",
            h1 { "Go Birdy" }
            p { class: "lede", "Match each call to the bird that made it." }
        }
    }
}

#[component]
fn Multiplayer() -> Element {
    let mut create_room_name_text = use_signal(String::new);
    let mut join_room_name_text = use_signal(String::new);
    let mut join_room_number_text = use_signal(String::new);

    let mut socket = use_websocket(|| game_socket(WebSocketOptions::new()));
    let mut joined_room = use_signal(|| None);

    use_future(move || async move {
        while let Ok(server_message) = socket.recv().await {
            match server_message {
                ServerMessage::YouJoined {
                    room_number,
                    your_name,
                    current_players,
                } => joined_room.set(Some((room_number, your_name, current_players))),
                ServerMessage::OtherPlayerJoined { name } => joined_room.with_mut(|joined_room| {
                    if let Some((_room_number, _your_name, current_players)) = joined_room {
                        current_players.push(name)
                    }
                }),
            }
        }
    });
    let create_room = async move |player_name: String| match socket
        .send(ClientMessage::CreateRoom {
            player_name: player_name.clone(),
        })
        .await
    {
        Ok(()) => (),
        Err(weberr) => error!("{}", weberr),
    };
    let join_room = async move |room_number: usize, player_name: String| match socket
        .send(ClientMessage::JoinRoom {
            room_number,
            player_name: player_name.clone(),
        })
        .await
    {
        Ok(()) => (),
        Err(weberr) => error!("{}", weberr),
    };

    // TODO: on invalid room number, show error
    rsx! {
        div {
            match &*joined_room.read() {
                Some((room_number, player_name, current_players)) => {
                    rsx! {
                        p { "Joined {room_number} as {player_name}" }
                        for player in current_players {
                            div {
                                key: "{player}",
                                p { "Player: {player}" },
                            }
                        }
                    }
                }
                None => {
                    rsx! {
                        form {
                            onsubmit: move |evt| {
                                evt.prevent_default();
                                let name = create_room_name_text();
                                create_room_name_text.set(String::new());
                                async move { create_room(name).await }
                            },
                            input {
                                r#type: "text",
                                placeholder: "Player name",
                                value: "{create_room_name_text}",
                                oninput: move |evt| create_room_name_text.set(evt.value()),
                            }
                            button { r#type: "submit", "Create Room" }
                        }

                        form {
                            onsubmit: move |evt| {
                                evt.prevent_default();
                                let room_number = join_room_number_text().trim().parse::<usize>();
                                let name = join_room_name_text();
                                if room_number.is_ok() {
                                    join_room_number_text.set(String::new());
                                    join_room_name_text.set(String::new());
                                }
                                async move {
                                    if let Ok(room_number) = room_number {
                                        join_room(room_number, name).await
                                    }
                                }
                            },
                            input {
                                r#type: "text",
                                placeholder: "Player name",
                                value: "{join_room_name_text}",
                                oninput: move |evt| join_room_name_text.set(evt.value()),
                            }
                            input {
                                r#type: "text",
                                placeholder: "Room number",
                                value: "{join_room_number_text}",
                                oninput: move |evt| join_room_number_text.set(evt.value()),
                            }
                            button { r#type: "submit", "Join Room" }
                        }
                    }
                }
            }
        }

    }
}

#[component]
fn Score(guessing: Signal<Guessing>) -> Element {
    const TIERS: [&str; 4] = ["🪹", "🪺", "🐣", "🐦‍🔥"];
    rsx! {
        section {
            h2 { class: "panel-title", "Score" }
            div { class: "score-grid",
                for (correct, emoji) in TIERS.iter().enumerate() {
                    div { key: "{correct}", class: "score-tile",
                        span { class: "score-emoji", "{emoji}" }
                        span { class: "score-count",
                            "{guessing.read().correct_guesses_per_round[correct]}"
                        }
                        span { class: "score-label", "{correct} correct" }
                    }
                }
            }
        }
    }
}

#[component]
fn Redraw(
    mut birds_resource: Resource<Result<Vec<Bird>, reqwest::Error>>,
    mut guessing: Signal<Guessing>,
) -> Element {
    let submit = |guesses: Vec<usize>| {
        move |_| {
            guessing.write().submitted = Some(guesses.clone());

            let mut correct_guesses = 0;
            for (audio_box, guess) in guesses.iter().enumerate() {
                if *guess == guessing.read().sound_positions[audio_box] {
                    correct_guesses += 1;
                }
            }

            guessing.write().correct_guesses_per_round[correct_guesses] += 1;
            guessing.write().selected_sound = None;
        }
    };

    let redraw = move |_| {
        birds_resource.restart();
        guessing.write().sound_guesses = vec![None, None, None, None, None];
        guessing.write().submitted = None;
    };

    rsx! {
        div { class: "actions",
            match &guessing.read().submitted {
                None => {
                    let guesses: Vec<usize> = guessing
                        .read()
                        .sound_guesses
                        .iter()
                        .filter_map(|guess| *guess)
                        .collect();
                    if guesses.len() == 3 {
                        rsx! {
                            button { class: "btn primary", onclick: submit(guesses), "Submit guesses" }
                        }
                    } else {
                        rsx! {
                            div { class: "hint", "Click a call, then click the bird you think made it." }
                        }
                    }
                }
                Some(_) => rsx! {
                    button { class: "btn primary", onclick: redraw, "New round" }
                },
            }
        }
    }
}

#[component]
fn Sounds(recordings: Vec<Recording>, mut guessing: Signal<Guessing>) -> Element {
    let mut playing = use_signal(|| vec![false, false, false]);
    let audio_element_id = |id: usize| format!("sound{}-id", id);

    let mut do_select = move |id| guessing.write().selected_sound = Some(id);
    let do_listen = move |id| {
        let element_id = audio_element_id(id);
        spawn(async move {
            playing.write()[id] = true;
            let js = format!(
                "const el = document.getElementById('{element_id}');
                 el.play();
                 await new Promise(resolve => {{ el.onended = resolve; }});"
            );
            let _ = document::eval(&js).await;
            playing.write()[id] = false;
        });
    };
    let select = |id| move |_| do_select(id);
    let listen_and_select = |id: usize| {
        move |_| {
            do_select(id);
            do_listen(id)
        }
    };

    let card_class = |id: usize| {
        let playing_class = if playing.read()[id] { " playing" } else { "" };
        let selected_class = if guessing.read().selected_sound == Some(id) {
            " selected"
        } else {
            ""
        };
        format!("sound-card{playing_class}{selected_class}")
    };
    let corresponding_recording = |id| guessing.read().sound_positions[id];
    let recording_for = |id: usize| &recordings[corresponding_recording(id)];

    rsx! {
        section {
            h2 { class: "panel-title", "Calls" }
            div { id: "sounds", class: "card-row",
                for audio_box in 0..3 {
                    div {
                        key: "{audio_box}",
                        class: "{card_class(audio_box)}",
                        onclick: select(audio_box),
                        audio {
                            id: "{audio_element_id(audio_box)}",
                            src: "{recording_for(audio_box).url}",
                        }
                        div { class: "sound-label", "Call {audio_box + 1}" }
                        div { class: "wave",
                            for bar in 0..7 {
                                span { key: "{bar}" }
                            }
                        }
                        button {
                            class: "btn listen",
                            onclick: listen_and_select(audio_box),
                            "▶ Listen"
                        }
                        Votes { recording_id: recording_for(audio_box).metadata.id.clone() }
                        match guessing.read().sound_guesses[audio_box] {
                            None => rsx! {},
                            Some(guess) => rsx! {
                                div { class: "guess", "You guessed {recordings[guess].metadata.en}" }
                            },
                        }
                        match &guessing.read().submitted {
                            None => rsx! {},
                            Some(submitted) => {
                                let correct = submitted[audio_box]
                                    == guessing.read().sound_positions[audio_box];
                                let name = &recording_for(audio_box).metadata.en;
                                if correct {
                                    rsx! {
                                        div { class: "result correct", "✓ {name}" }
                                    }
                                } else {
                                    rsx! {
                                        div { class: "result incorrect", "✗ It was {name}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DirectionalVote(
    recording_id: ReadSignal<String>,
    num_votes: ReadSignal<i64>,
    is_upvote: bool,
    mut user_voted_either_direction: Signal<bool>,
) -> Element {
    let mut user_voted_this_direction = use_signal(|| false);
    use_effect(move || {
        recording_id();
        user_voted_this_direction.set(false);
    });

    let vote_text = |num_votes: &i64| {
        let emoji = if is_upvote { "👍" } else { "👎" };
        let plus_one = if *user_voted_this_direction.read() {
            "+1"
        } else {
            ""
        };
        format!("{emoji}({num_votes}{plus_one})")
    };

    let num_votes = &*num_votes.read();

    rsx! {

        if *user_voted_either_direction.read() {
            div { "{vote_text(num_votes)}" }
        } else {

            button {
                onclick: move |_| {

                    spawn(async move {
                        if upvote_sound(recording_id()).await.is_ok() {
                            user_voted_this_direction.set(true);
                            user_voted_either_direction.set(true);
                        }
                    });
                },
                "{vote_text(num_votes)}"

            }
        }
    }
}

#[component]
fn Votes(recording_id: ReadSignal<String>) -> Element {
    let ratings_resource = use_resource(move || async move { get_ratings(recording_id()).await });

    let mut user_voted_either_direction = use_signal(|| false);
    use_effect(move || {
        recording_id();
        user_voted_either_direction.set(false);
    });

    rsx! {
        match &*ratings_resource.read() {
            Some(Ok((upvotes, downvotes))) => {
                let num_upvotes = use_signal(|| *upvotes);
                let num_downvotes = use_signal(|| *downvotes);
                rsx! {
                    DirectionalVote {
                        recording_id,
                        num_votes: num_upvotes,
                        is_upvote: true,
                        user_voted_either_direction,
                    }
                    DirectionalVote {
                        recording_id,
                        num_votes: num_downvotes,
                        is_upvote: false,
                        user_voted_either_direction,
                    }
                }
            }
            Some(Err(_)) => rsx! {
                div { "Failed to fetch votes" }
            },
            None => rsx! {
                div { "Loading votes..." }
            },
        }
    }
}

#[component]
fn Images(image_links: Vec<ImageLinks>, mut guessing: Signal<Guessing>) -> Element {
    let guess = |image_id| {
        move |_| {
            let mut guessing_value = guessing.write();
            if let Some(sound_id) = guessing_value.selected_sound {
                guessing_value.sound_guesses[sound_id] = Some(image_id);
                guessing_value.selected_sound = None;
            }
        }
    };
    let pictures_class = if guessing.read().selected_sound.is_some() {
        "pictures armed"
    } else {
        "pictures"
    };

    rsx! {
        section {
            h2 { class: "panel-title", "Birds" }
            div { id: "pictures", class: "{pictures_class}",
                for bird_image in 0..5 {
                    div {
                        key: "{bird_image}",
                        class: "photo",
                        onclick: guess(bird_image),
                        img {
                            src: "{image_links[bird_image].medium_url}",
                            alt: "",
                        }
                        div { class: "name", "{image_links[bird_image].en}" }
                    }
                }
            }
        }
    }
}

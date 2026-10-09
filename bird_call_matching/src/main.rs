use dioxus::prelude::*;
use futures::future::join_all;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use tokio::sync::broadcast;

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
 * 1. user can create a room, they type in their name and the room name
 * 2. user can try to connect to a room. they type in their name and the room name
 * 3. users request a random seed from the server for their room. on re-draw, server sends the
 *    room's seed to players waiting to join, as well as the round number
 * 4. on join, player generates a layout with (room_seed + round_number) as their rng seed. in
 *    particular, this keeps computation lighter on the server
 * 5. once a user is ready to guess, they send request to the server, which is fulfilled once every
 *    user is ready. then correct guesses are displayed
 * 6. once both users click continue, new round begins (and they receive new round number from the
 *    server)
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
static ROOMS: LazyLock<Mutex<GameState>> = LazyLock::new(|| HashMap::new());

#[cfg(feature = "server")]
struct Player {
    name: String,
    correct_guesses_per_round: Vec<u32>,
}

#[cfg(feature = "server")]
struct Room {
    players: Vec<Player>,
}

#[cfg(feature = "server")]
#[derive(Serialize, Deserialize, Clone)]
enum ClientMessage {
    CreateRoom {
        room_name: String,
        player_name: String,
    },
    StartGame {
        room_name: String,
    },
}

#[cfg(feature = "server")]
#[derive(Serialize, Deserialize, Clone)]
enum ServerMessage {
    PlayerJoinedPendingRoom { player_name: String },
}

#[server]
async fn game_socket(options: WebSocketOptions) -> Result<Websocket<ClientMessage, ServerMessage>> {
    Ok(options.on_upgrade(|mut socket| async move {
        while let Some(Ok(msg)) = socket.recv().await {
            match msg {
                ClientMove::Guess {
                    player_id,
                    sound_box,
                    bird_guess,
                } => {
                    // update shared game state here (e.g. a broadcast channel, or shared Mutex<State>)
                    // then broadcast to all connected clients:
                    let _ = socket
                        .send(ServerUpdate::PlayerGuessed {
                            player_id,
                            sound_box,
                            bird_guess,
                        })
                        .await;
                }
            }
        }
    }))
}

async fn create_room(room_name: String, username: String) -> Result<(), ServerFnError> {
    let mut rooms = ROOMS.lock().unwrap();
    let initial_player = Player {
        name: usermame,
        correct_guesses_per_round: vec![0, 0, 0, 0],
    };
    let room = Room {
        players: vec![initial_player],
    };
    rooms.insert(room_name, room)
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
        match &*birds_resource.read() {
            Some(Ok(birds)) => rsx! {
                Title {}
                Sounds { recordings: birds.iter().map(|bird| bird.recording.clone()).collect(), guessing }
                Images { image_links: birds.iter().map(|bird| bird.image_links.clone()).collect(), guessing }
                Redraw { birds_resource, guessing  }
                Score {guessing}
            },
            Some(Err(e)) => rsx! {
                div { "Failed to load: {e}" }
                button { onclick: move |_| birds_resource.restart(), "retry" }
            },
            None => rsx! {
                div { "Loading..." }
            },
        }
    }
}

#[component]
fn Title() -> Element {
    rsx! {
        div { id: "title",
            h1 { "Go Birdy" }
        }
    }
}

#[component]
fn Score(guessing: Signal<Guessing>) -> Element {
    rsx! {
        div {
            h2 {"Score"}
            p { "😭🪹 0: {guessing.read().correct_guesses_per_round[0]}" }
            p { "😕🪺 1: {guessing.read().correct_guesses_per_round[1]}" }
            p { "🙂🐣 2: {guessing.read().correct_guesses_per_round[2]}" }
            p { "🤓🐦‍🔥 3: {guessing.read().correct_guesses_per_round[3]}" }
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
    {
        match &guessing.read().submitted {
            None => {
                let guesses : Vec<usize> = guessing.read().sound_guesses.iter().filter_map(|guess| *guess).collect();
                if guesses.len() == 3 {
                    rsx! {
                        button {
                            onclick: submit(guesses),
                            "submit"
                        }
                    }
                } else {
                    rsx! {}
                }


            }
            Some(_) => {
                rsx! {
                    button {
                        onclick: redraw,  "re-draw" }
                    }
                }
            }
        }
    }
}

#[component]
fn Sounds(recordings: Vec<Recording>, mut guessing: Signal<Guessing>) -> Element {
    let widths = 200 - 2;
    let heights = 150 - 2;

    let mut playing = use_signal(|| vec![false, false, false]);
    let audio_element_id = |id: usize| format!("sound{}-id", id);
    let border = |id: usize| {
        if playing.read()[id] {
            "red"
        } else {
            "white"
        }
    };

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

    let background = |id: usize| {
        let selected = guessing
            .read()
            .selected_sound
            .map(|selected| selected == id)
            .unwrap_or(false);
        if selected {
            "cyan"
        } else {
            "gray"
        }
    };
    let corresponding_recording = |id| guessing.read().sound_positions[id];

    rsx! {
        div { id: "sounds", style: "display: flex; flex-wrap: wrap; gap: 8px;",

            for audio_box in (0..3) {
                div {
            key: "{audio_box}",
            audio { id: "{audio_element_id(audio_box)}", src: "{recordings[corresponding_recording(audio_box)].url}" }
            div {
                style: "width: {widths}px; height: {heights}px; border: 1px solid {border(audio_box)}; background: {background(audio_box)};",
                onclick: select(audio_box),
                button {
                    onclick: listen_and_select(audio_box),
                    "listen"
                }
                Votes {
                    key:  "{recordings[corresponding_recording(audio_box)].metadata.id}",
                    recording_id: "{recordings[corresponding_recording(audio_box)].metadata.id}"
                }
                {
                match guessing.read().sound_guesses[audio_box] {
                    None => rsx! {},
                    Some(guess) => rsx! {
                        div { "{recordings[guess].metadata.en}" }
                    }
                }

                    }
                {
                match &guessing.read().submitted {
                    None => rsx! {},
                    Some(submitted_value) => {
                        let color =
                        if submitted_value[audio_box] == guessing.read().sound_positions[audio_box] {
                            "green"
                        } else {
                             "red"
                        };


                        rsx! {
                            div {color:"{color}", "{recordings[corresponding_recording(audio_box)].metadata.en}"}
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
    // TODO: factor out directional vote button, so we don't repeat for up and down votes. probably
    // need some "user_voted: Signal<bool> param so if the user upvotes, the downvote button also
    // disappears and vice-versa

                if *user_voted_either_direction.read() {
                    div {"{vote_text(num_votes)}"}
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


                    }}
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
            Some(Ok((upvotes, downvotes))) =>  {
                let num_upvotes = use_signal(|| *upvotes);
                let num_downvotes = use_signal(|| *downvotes);
                rsx! {
                DirectionalVote{recording_id, num_votes:num_upvotes, is_upvote:true, user_voted_either_direction}
                DirectionalVote{recording_id, num_votes:num_downvotes, is_upvote:false, user_voted_either_direction}
                }


            },
            Some(Err(_)) => rsx! {
                div {"Failed to fetch votes"}
            },
            None => rsx! {
                div {"Loading votes..."}
            },
        }
    }
}

#[component]
fn Images(image_links: Vec<ImageLinks>, mut guessing: Signal<Guessing>) -> Element {
    let border_size = 2;
    let widths = 200 - 2 * border_size;
    let heights = 150 - 2 * border_size;

    let guess = |image_id| {
        move |_| {
            let mut guessing_value = guessing.write();

            match guessing_value.selected_sound {
                None => {}
                Some(sound_id) => {
                    guessing_value.sound_guesses[sound_id] = Some(image_id);
                    guessing_value.selected_sound = None;
                }
            }
        }
    };

    rsx! {
        div {
            id: "pictures",
            style: "display: flex; flex-wrap: wrap; gap: 3px;",
            for bird_image in (0..5) {

            div {
                key: "{bird_image}",
                style: "width: {widths}px; height: {heights}px; border: {border_size}px solid cyan;",
                onclick: guess(bird_image),
                img { src:"{image_links[bird_image].medium_url}",
                style: "width: 80%; height: 80%; object-fit: cover; display: block;"
                },
                "{image_links[bird_image].en}" }
            }
        }
    }
}

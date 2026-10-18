use dioxus::logger::tracing::error;
use dioxus::prelude::dioxus_fullstack::{use_websocket, WebSocketOptions};
use dioxus::prelude::*;

use crate::room::{game_socket, ClientMessage, RoomSharedState, RoomStage, ServerMessage};

/// What this client knows about the room it's in.
#[derive(Clone, PartialEq, Debug)]
struct JoinedRoom {
    player_name: String,
    room: RoomSharedState,
}

#[component]
pub fn Multiplayer() -> Element {
    let mut create_room_name_text = use_signal(String::new);
    let mut join_room_name_text = use_signal(String::new);
    let mut join_room_number_text = use_signal(String::new);

    let mut socket = use_websocket(|| game_socket(WebSocketOptions::new()));
    let mut joined_room = use_signal(|| None::<JoinedRoom>);

    use_future(move || async move {
        while let Ok(server_message) = socket.recv().await {
            match server_message {
                ServerMessage::YouJoined { your_name, room } => joined_room.set(Some(JoinedRoom {
                    player_name: your_name,
                    room,
                })),
                ServerMessage::RoomUpdated(room) => joined_room.with_mut(|joined_room| {
                    if let Some(joined_room) = joined_room {
                        joined_room.room = room;
                    }
                }),
            }
        }
    });
    let send = move |message: ClientMessage| async move {
        if let Err(weberr) = socket.send(message).await {
            error!("{}", weberr);
        }
    };

    // TODO: on invalid room number, show error
    rsx! {
        div {
            match &*joined_room.read() {
                Some(JoinedRoom { player_name, room }) => rsx! {
                    RoomView {
                        player_name: player_name.clone(),
                        room: room.clone(),
                        on_start_game: move |_| send(ClientMessage::StartGame {}),
                        on_ready: move |_| send(ClientMessage::Ready {}),
                    }
                },
                None => {
                    rsx! {
                        form {
                            onsubmit: move |evt| {
                                evt.prevent_default();
                                let player_name = create_room_name_text();
                                create_room_name_text.set(String::new());
                                send(ClientMessage::CreateRoom { player_name })
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
                                let player_name = join_room_name_text();
                                if room_number.is_ok() {
                                    join_room_number_text.set(String::new());
                                    join_room_name_text.set(String::new());
                                }
                                async move {
                                    if let Ok(room_number) = room_number {
                                        send(ClientMessage::JoinRoom { room_number, player_name }).await
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

/// The room once this player is in it: who's here, and what to do next for the current stage.
#[component]
fn RoomView(
    player_name: String,
    room: RoomSharedState,
    on_start_game: EventHandler,
    on_ready: EventHandler,
) -> Element {
    let room_number = room.room_number;
    let is_ready = room
        .players
        .get(&player_name)
        .is_some_and(|player| player.ready);

    rsx! {
        p { "Joined {room_number} as {player_name}" }
        match room.stage {
            RoomStage::Lobby => rsx! {
                for name in room.players.keys() {
                    div { key: "{name}",
                        p { "Player: {name}" }
                    }
                }
                if room.is_host(&player_name) {
                    button { onclick: move |_| on_start_game.call(()), "Start game" }
                } else {
                    p { "Waiting for host..." }
                }
            },
            RoomStage::Round(round) => rsx! {
                p { "Round {round}" }
                for (name, player) in room.players.iter() {
                    div { key: "{name}",
                        p {
                            "Player: {name}"
                            if player.ready {
                                " ✓ ready"
                            }
                        }
                    }
                }
                if !is_ready {
                    button { onclick: move |_| on_ready.call(()), "Ready" }
                }
            },
        }
    }
}

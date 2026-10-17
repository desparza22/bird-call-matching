use dioxus::logger::tracing::error;
use dioxus::prelude::dioxus_fullstack::{use_websocket, WebSocketOptions};
use dioxus::prelude::*;

use crate::room::{game_socket, ClientMessage, JoinedRoom, ServerMessage};

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
                ServerMessage::YouJoined {
                    room_number,
                    your_name,
                    current_players,
                } => joined_room.set(Some(JoinedRoom {
                    room_number,
                    player_name: your_name,
                    current_players,
                })),
                ServerMessage::OtherPlayerJoined { name } => joined_room.with_mut(|joined_room| {
                    if let Some(joined_room) = joined_room {
                        joined_room.current_players.push(name)
                    }
                }),
            }
        }
    });
    let create_room = async move |player_name: String| {
        if let Err(weberr) = socket.send(ClientMessage::CreateRoom { player_name }).await {
            error!("{}", weberr);
        }
    };
    let join_room = async move |room_number: usize, player_name: String| {
        if let Err(weberr) = socket
            .send(ClientMessage::JoinRoom {
                room_number,
                player_name,
            })
            .await
        {
            error!("{}", weberr);
        }
    };

    // TODO: on invalid room number, show error
    rsx! {
        div {
            match &*joined_room.read() {
                Some(JoinedRoom { room_number, player_name, current_players }) => {
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

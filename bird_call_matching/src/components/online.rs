use dioxus::logger::tracing::error;
use dioxus::prelude::dioxus_fullstack::{use_websocket, WebSocketOptions};
use dioxus::prelude::*;

use super::{GameBoard, GameLayout, Leaderboard, LeaderboardEntry};
use crate::board_state::BoardState;
use crate::room::{game_socket, ClientMessage, RoomSharedState, RoomStage, ServerMessage};

/// What this client knows about the room it's in.
#[derive(Clone, PartialEq, Debug)]
struct JoinedRoom {
    player_name: String,
    room: RoomSharedState,
}

/// A shared room. Opens the websocket when mounted (and closes it when unmounted), sends
/// `first_message` to create or join the room, then keeps the board in sync with the server.
/// Submitting guesses marks this player ready; the server starts the next round, birds included,
/// once everyone is.
#[component]
pub fn OnlineGame(
    mut board_state: Signal<BoardState>,
    first_message: ClientMessage,
    on_leave: EventHandler,
) -> Element {
    let mut socket = use_websocket(|| game_socket(WebSocketOptions::new()));
    let mut joined_room = use_signal(|| None::<JoinedRoom>);
    let mut disconnected = use_signal(|| false);

    use_future(move || {
        let first_message = first_message.clone();
        async move {
            socket.connect().await;
            if let Err(err) = socket.send(first_message).await {
                error!("{}", err);
            }

            while let Ok(server_message) = socket.recv().await {
                let room = match server_message {
                    ServerMessage::YouJoined { your_name, room } => {
                        joined_room.set(Some(JoinedRoom {
                            player_name: your_name,
                            room: room.clone(),
                        }));
                        room
                    }
                    ServerMessage::RoomUpdated(room) => {
                        joined_room.with_mut(|joined_room| {
                            if let Some(joined_room) = joined_room {
                                joined_room.room = room.clone();
                            }
                        });
                        room
                    }
                };
                if let RoomStage::Round { number, setup } = room.stage {
                    board_state.write().queue_round(number, setup);
                }
            }
            // The server closes the connection if it rejects the create/join request.
            disconnected.set(true);
        }
    });
    let send = move |message: ClientMessage| async move {
        if let Err(err) = socket.send(message).await {
            error!("{}", err);
        }
    };

    if disconnected() {
        return rsx! {
            GameLayout {
                sidebar: rsx! {
                    section { class: "panel",
                        div { class: "status error", "Lost connection to the room." }
                        button { class: "btn primary", onclick: move |_| on_leave.call(()), "Back to solo play" }
                    }
                },
            }
        };
    }

    let joined_room = joined_room.read();
    let Some(JoinedRoom { player_name, room }) = &*joined_room else {
        return rsx! {
            GameLayout {
                sidebar: rsx! {
                    section { class: "panel",
                        div { class: "status", "Connecting…" }
                    }
                },
            }
        };
    };
    let leaderboard: Vec<LeaderboardEntry> = room
        .players
        .values()
        .map(|player| LeaderboardEntry {
            name: Some(player.name.clone()),
            correct_guesses_per_round: player.correct_guesses_per_round.clone(),
            is_current_player: player.name == *player_name,
        })
        .collect();

    rsx! {
        GameLayout {
            sidebar: rsx! {
                RoomView {
                    player_name: player_name.clone(),
                    room: room.clone(),
                    on_start_game: move |_| send(ClientMessage::StartGame {}),
                }
                Leaderboard { entries: leaderboard }
            },
            if board_state.read().has_round() {
                GameBoard {
                    board_state,
                    on_submit: move |guesses| send(ClientMessage::SubmitGuesses { guesses }),
                }
            } else {
                div { class: "status waiting", "The round starts once the host starts the game." }
            }
        }
    }
}

/// The room's number, stage and players.
#[component]
fn RoomView(player_name: String, room: RoomSharedState, on_start_game: EventHandler) -> Element {
    let round = match room.stage {
        RoomStage::Lobby => None,
        RoomStage::Round { number, .. } => Some(number),
    };

    rsx! {
        section { class: "panel room",
            div { class: "room-meta",
                h2 { class: "panel-title", "Room {room.room_number}" }
                match round {
                    Some(number) => rsx! {
                        span { class: "tag", "Round {number}" }
                    },
                    None => rsx! {
                        span { class: "tag", "Lobby" }
                    },
                }
            }
            ul { class: "players",
                for (name, player) in room.players.iter() {
                    li {
                        key: "{name}",
                        class: if round.is_some() && player.ready { "ready" },
                        span { class: "player-name",
                            "{name}"
                            if *name == player_name {
                                " (you)"
                            }
                        }
                        if round.is_some() {
                            span { class: "player-status",
                                if player.ready {
                                    "✓ ready"
                                } else {
                                    "guessing…"
                                }
                            }
                        }
                    }
                }
            }
            if round.is_none() {
                if room.is_host(&player_name) {
                    button { class: "btn primary", onclick: move |_| on_start_game.call(()), "Start game" }
                } else {
                    p { class: "hint", "Waiting for host..." }
                }
            }
        }
    }
}

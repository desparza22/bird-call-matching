use dioxus::logger::tracing::error;
use dioxus::prelude::dioxus_fullstack::{use_websocket, WebSocketOptions};
use dioxus::prelude::*;

use super::{GameBoard, LeaderboardEntry};
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
            div { class: "status error", "Lost connection to the room." }
            button { class: "btn primary", onclick: move |_| on_leave.call(()), "Back to solo play" }
        };
    }

    rsx! {
        match &*joined_room.read() {
            None => rsx! {
                div { class: "status", "Connecting…" }
            },
            Some(JoinedRoom { player_name, room }) => rsx! {
                RoomView {
                    player_name: player_name.clone(),
                    room: room.clone(),
                    on_start_game: move |_| send(ClientMessage::StartGame {}),
                }
                if board_state.read().has_round() {
                    GameBoard {
                        board_state,
                        on_submit: move |guesses| send(ClientMessage::SubmitGuesses { guesses }),
                        leaderboard: room
                            .players
                            .values()
                            .map(|player| LeaderboardEntry {
                                name: Some(player.name.clone()),
                                correct_guesses_per_round: player.correct_guesses_per_round.clone(),
                                is_current_player: player.name == *player_name,
                            })
                            .collect::<Vec<_>>(),
                    }
                }
            },
        }
    }
}

/// The room's players and stage.
#[component]
fn RoomView(player_name: String, room: RoomSharedState, on_start_game: EventHandler) -> Element {
    let room_number = room.room_number;

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
            RoomStage::Round { number, .. } => rsx! {
                p { "Round {number}" }
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
            },
        }
    }
}

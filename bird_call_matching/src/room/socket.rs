//! The game websocket endpoint and the server-side handling of each connection.

use dioxus::prelude::dioxus_fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;

use super::protocol::{ClientMessage, ServerMessage};

#[cfg(feature = "server")]
use {
    super::protocol::Broadcast,
    super::registry::{rooms, Membership},
    crate::birds::{catalog, RoundSetup, BIRDS_PER_ROUND},
    dioxus::prelude::dioxus_fullstack::TypedWebsocket,
    tokio::sync::broadcast::error::RecvError,
};

#[cfg(feature = "server")]
type GameSocket = TypedWebsocket<ClientMessage, ServerMessage>;

#[cfg(feature = "server")]
fn new_round() -> RoundSetup {
    catalog::get().random_round(BIRDS_PER_ROUND)
}

#[get("/api/game_ws")]
pub async fn game_socket(
    options: WebSocketOptions,
) -> Result<Websocket<ClientMessage, ServerMessage>> {
    Ok(options.on_upgrade(handle_connection))
}

#[cfg(feature = "server")]
async fn handle_connection(mut socket: GameSocket) {
    let membership = match socket.recv().await {
        Ok(first_msg) => handle_first_msg(first_msg),
        Err(err) => Err(format!("client left? {err}")),
    };

    let Membership {
        player_name,
        room,
        mut receiver,
    } = match membership {
        Ok(membership) => membership,
        Err(err) => {
            println!("client didn't join room: {err}");
            return;
        }
    };
    let room_number = room.room_number;

    let _ = socket
        .send(ServerMessage::YouJoined {
            your_name: player_name.clone(),
            room,
        })
        .await;

    loop {
        tokio::select! {
            client_msg = socket.recv() => {
                let client_msg = match client_msg {
                    Ok(client_msg) => client_msg,
                    Err(err) => {
                        println!("{player_name} disconnected: {err}");
                        break;
                    }
                };
                if let Err(err) = handle_msg(client_msg, room_number, &player_name) {
                    println!("rejected message: {err}");
                }
            },

            broadcast = receiver.recv() => {
                handle_broadcast(broadcast, &mut socket).await
            }
        }
    }
}

/// Creates or joins a room, depending on the client's first message.
#[cfg(feature = "server")]
fn handle_first_msg(client_msg: ClientMessage) -> Result<Membership, String> {
    match client_msg {
        ClientMessage::JoinRoom {
            room_number,
            player_name,
        } => rooms().join_room(room_number, player_name),
        ClientMessage::CreateRoom { player_name } => rooms().create_room(player_name),
        ClientMessage::StartGame {} | ClientMessage::SubmitGuesses { .. } => {
            Err("Unexpected message, player not in room yet.".to_string())
        }
    }
}

/// Handles a message from a player who's already in `room_number`.
#[cfg(feature = "server")]
fn handle_msg(
    client_msg: ClientMessage,
    room_number: usize,
    player_name: &str,
) -> Result<(), String> {
    match client_msg {
        ClientMessage::JoinRoom { room_number, .. } => Err(format!(
            "Received JoinRoom message from player {player_name}, for room {room_number}, but they're already in a room"
        )),
        ClientMessage::CreateRoom { .. } => Err(format!(
            "Received CreateRoom message from player {player_name} but they're already in a room"
        )),
        ClientMessage::StartGame {} => {
            rooms().update_room(room_number, |room| room.start_game(player_name, new_round))
        }
        ClientMessage::SubmitGuesses { guesses } => rooms().update_room(room_number, |room| {
            room.submit_guesses(player_name, &guesses, new_round)
        }),
    }
}

#[cfg(feature = "server")]
async fn handle_broadcast(broadcast: Result<Broadcast, RecvError>, socket: &mut GameSocket) {
    match broadcast {
        Ok(Broadcast::RoomChanged(room)) => {
            let _ = socket.send(ServerMessage::RoomUpdated(room)).await;
        }
        Err(_) => (),
    }
}

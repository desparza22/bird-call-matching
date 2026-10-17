//! The game websocket endpoint and the server-side handling of each connection.

use dioxus::prelude::dioxus_fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;

use super::protocol::{ClientMessage, ServerMessage};

#[cfg(feature = "server")]
use {
    super::protocol::{Broadcast, JoinedRoom, Player},
    super::registry::rooms,
    dioxus::prelude::dioxus_fullstack::{TypedWebsocket, WebsocketError},
    tokio::sync::broadcast::error::RecvError,
};

#[cfg(feature = "server")]
type GameSocket = TypedWebsocket<ClientMessage, ServerMessage>;

#[get("/api/game_ws")]
pub async fn game_socket(
    options: WebSocketOptions,
) -> Result<Websocket<ClientMessage, ServerMessage>> {
    Ok(options.on_upgrade(handle_connection))
}

#[cfg(feature = "server")]
async fn handle_connection(mut socket: GameSocket) {
    let room_info = match socket.recv().await {
        Ok(first_msg) => handle_first_msg(first_msg),
        Err(err) => Err(format!("client left? {err}")),
    };

    let JoinedRoom {
        room_number,
        player_name,
        current_players,
    } = match room_info {
        Ok(joined_room) => joined_room,
        Err(err) => {
            println!("client didn't join room: {err}");
            return;
        }
    };

    let _ = socket
        .send(ServerMessage::YouJoined {
            room_number,
            your_name: player_name,
            current_players,
        })
        .await;

    let mut receiver = match rooms().subscribe(room_number) {
        Ok(receiver) => receiver,
        Err(err) => {
            println!("couldn't subscribe to room updates. {err}");
            return;
        }
    };

    loop {
        tokio::select! {
            client_msg = socket.recv() => {
                if handle_msg(client_msg, &mut socket).is_err() {
                    println!("err");
                    break;
                }
            },

            broadcast = receiver.recv() => {
                handle_broadcast(broadcast, &mut socket).await
            }
        }
    }
}

// returns the joined room if joined/created successfully
#[cfg(feature = "server")]
fn handle_first_msg(client_msg: ClientMessage) -> Result<JoinedRoom, String> {
    match client_msg {
        ClientMessage::JoinRoom {
            room_number,
            player_name,
        } => {
            let (new_player_name, current_players) =
                rooms().player_join(room_number, player_name)?;
            let broadcast = Broadcast::PlayerJoined {
                name: new_player_name.clone(),
            };
            if let Err(err) = rooms().send(room_number, broadcast) {
                // We should still return okay because we updated server state. other clients can
                // sync back up. Or, we could rely on broadcast message to confirm ourselves
                // joining.
                println!("Failed to broadcast player joining {err}")
            }
            Ok(JoinedRoom {
                room_number,
                player_name: new_player_name,
                current_players,
            })
        }
        ClientMessage::CreateRoom { player_name } => {
            let room_number = rooms().create_room(Player::new(player_name.clone()))?;
            Ok(JoinedRoom {
                room_number,
                player_name: player_name.clone(),
                current_players: vec![player_name],
            })
        }
        ClientMessage::StartGame {} => {
            Err("Unexpected request to start game, player not in room yet.".to_string())
        }
    }
}

#[cfg(feature = "server")]
fn handle_msg(
    client_msg: Result<ClientMessage, WebsocketError>,
    socket: &mut GameSocket,
) -> Result<(), String> {
    match client_msg {
        Ok(ClientMessage::JoinRoom {
            room_number,
            player_name,
        }) => Err(format!(
            "Received JoinRoom message from player {player_name}, for room {room_number}, but they're already in a room"
        )),
        Ok(ClientMessage::CreateRoom { player_name }) => Err(format!(
            "Received CreateRoom message from player {player_name} but they're already in a room"
        )),
        Ok(ClientMessage::StartGame {}) => {
            // TODO
            Ok(())
        }
        Err(websocket_error) => Err(websocket_error.to_string()),
    }
}

#[cfg(feature = "server")]
async fn handle_broadcast(broadcast: Result<Broadcast, RecvError>, socket: &mut GameSocket) {
    match broadcast {
        Ok(Broadcast::PlayerJoined { name }) => {
            let _ = socket.send(ServerMessage::OtherPlayerJoined { name }).await;
        }
        Err(_) => (),
    }
}

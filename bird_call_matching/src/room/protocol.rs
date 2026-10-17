//! Types shared between the client and the server.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Player {
    pub name: String,
    pub correct_guesses_per_round: Vec<u32>,
}

impl Player {
    pub fn new(name: String) -> Self {
        Self {
            name,
            correct_guesses_per_round: vec![0; 4],
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum RoomStage {
    Lobby,
    Round(usize),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct RoomSharedState {
    pub room_number: usize,
    pub players: HashMap<String, Player>,
    pub stage: RoomStage,
}

impl RoomSharedState {
    pub fn new(room_number: usize, initial_player: Player) -> Self {
        Self {
            room_number,
            players: HashMap::from([(initial_player.name.clone(), initial_player)]),
            stage: RoomStage::Lobby,
        }
    }
}

/// What a player knows about the room they're in.
#[derive(Clone, PartialEq, Debug)]
pub struct JoinedRoom {
    pub room_number: usize,
    pub player_name: String,
    pub current_players: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ClientMessage {
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
pub enum ServerMessage {
    YouJoined {
        room_number: usize,
        your_name: String,
        current_players: Vec<String>,
    },
    OtherPlayerJoined {
        name: String,
    },
}

/// Sent between the server tasks of players in the same room.
#[cfg(feature = "server")]
#[derive(Clone)]
pub enum Broadcast {
    PlayerJoined { name: String },
}

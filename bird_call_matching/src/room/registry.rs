//! The server's in-memory registry of active rooms. Server-only.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};
use tokio::sync::broadcast;

use super::protocol::{Broadcast, Player, RoomSharedState};

const MAX_ROOMS: usize = 65000;

static ROOMS: LazyLock<Mutex<Rooms>> = LazyLock::new(|| Mutex::new(Rooms::new()));

/// Locks and returns the global room registry.
pub fn rooms() -> MutexGuard<'static, Rooms> {
    ROOMS.lock().unwrap()
}

pub struct RoomServerState {
    channel: broadcast::Sender<Broadcast>,
    room_shared_state: RoomSharedState,
}

pub struct Rooms {
    active_rooms: HashMap<usize, RoomServerState>,
    available_room_ids: Vec<usize>,
}

impl Rooms {
    fn new() -> Self {
        Self {
            active_rooms: HashMap::new(),
            available_room_ids: (0..MAX_ROOMS).collect(),
        }
    }

    pub fn create_room(&mut self, initial_player: Player) -> Result<usize, String> {
        let room_number = self
            .available_room_ids
            .pop()
            .ok_or("No available capacity for new rooms")?;

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
        self.active_rooms
            .get_mut(&room_number)
            .map(f)
            .ok_or_else(|| "Invalid room number".to_string())
    }

    fn bind_room<A>(
        &mut self,
        room_number: usize,
        f: impl FnOnce(&mut RoomServerState) -> Result<A, String>,
    ) -> Result<A, String> {
        self.map_room(room_number, f)?
    }

    // Returns Ok(String) with the actual name for the player, possibly modified to avoid
    // collisions, and the list of all players
    pub fn player_join(
        &mut self,
        room_number: usize,
        player_name: String,
    ) -> Result<(String, Vec<String>), String> {
        self.map_room(room_number, |room_handle| {
            let players = &mut room_handle.room_shared_state.players;

            let mut unique_player_name = player_name.clone();
            let mut unique_suffix = 1;
            while players.contains_key(&unique_player_name) {
                unique_suffix += 1;
                unique_player_name = format!("{player_name}_{unique_suffix}");
            }

            players.insert(
                unique_player_name.clone(),
                Player::new(unique_player_name.clone()),
            );
            let current_players = players.keys().cloned().collect();
            (unique_player_name, current_players)
        })
    }

    pub fn subscribe(
        &mut self,
        room_number: usize,
    ) -> Result<broadcast::Receiver<Broadcast>, String> {
        self.map_room(room_number, |room_handle| room_handle.channel.subscribe())
    }

    pub fn send(&mut self, room_number: usize, broadcast: Broadcast) -> Result<(), String> {
        self.bind_room(room_number, |room_handle| {
            room_handle
                .channel
                .send(broadcast)
                .map(|_| ())
                .map_err(|err| err.to_string())
        })
    }
}

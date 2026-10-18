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

impl RoomServerState {
    /// Sends the current state to every subscribed player.
    fn broadcast_state(&self) {
        // `send` only fails when nobody is subscribed, in which case there's nobody to tell.
        let _ = self
            .channel
            .send(Broadcast::RoomChanged(self.room_shared_state.clone()));
    }
}

/// Everything a connection needs after joining or creating a room.
pub struct Membership {
    pub player_name: String,
    pub room: RoomSharedState,
    pub receiver: broadcast::Receiver<Broadcast>,
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

    pub fn create_room(&mut self, host_name: String) -> Result<Membership, String> {
        let room_number = self
            .available_room_ids
            .pop()
            .ok_or("No available capacity for new rooms")?;

        let (channel, receiver) = broadcast::channel(100);
        let room_shared_state = RoomSharedState::new(room_number, Player::new(host_name.clone()));
        let membership = Membership {
            player_name: host_name,
            room: room_shared_state.clone(),
            receiver,
        };
        self.active_rooms.insert(
            room_number,
            RoomServerState {
                channel,
                room_shared_state,
            },
        );
        Ok(membership)
    }

    /// Adds the player to the room and tells everyone already there. The new player is
    /// subscribed while the registry is still locked, so they can't miss an update.
    pub fn join_room(
        &mut self,
        room_number: usize,
        player_name: String,
    ) -> Result<Membership, String> {
        self.map_room(room_number, |room_handle| {
            let player_name = room_handle.room_shared_state.add_player(player_name);
            room_handle.broadcast_state();
            Membership {
                player_name,
                room: room_handle.room_shared_state.clone(),
                receiver: room_handle.channel.subscribe(),
            }
        })
    }

    /// Applies `update` to the room's state and, if it succeeds, sends the new state to everyone
    /// in the room.
    pub fn update_room(
        &mut self,
        room_number: usize,
        update: impl FnOnce(&mut RoomSharedState) -> Result<(), String>,
    ) -> Result<(), String> {
        self.bind_room(room_number, |room_handle| {
            update(&mut room_handle.room_shared_state)?;
            room_handle.broadcast_state();
            Ok(())
        })
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
}

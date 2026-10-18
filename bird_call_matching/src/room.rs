//! Multiplayer rooms: the wire protocol shared by client and server, the server's room
//! registry, and the websocket endpoint that connects them.
//!
//! multiplayer plan:
//! 1. user can create a room, they type in their name and get back a room name
//! 2. user can try to connect to a room. they type in the room number and name, numbers appended to
//!    name to resolve conflict
//! 3. users request a random seed from the server for their room. on re-draw, server sends the
//!    room's seed to players waiting to join, as well as the round number
//! 4. on join, player generates a layout with (room_seed + round_number) as their rng seed. in
//!    particular, this keeps computation lighter on the server
//! 5. once a user is ready to guess, they send request to the server, which is fulfilled once every
//!    user is ready. then correct guesses are displayed
//! 6. once both users click continue, new round begins (and they receive new round number from the
//!    server)
//!
//!    room holds broadcast channel? so that each connected player can get updates on room state
//!    changes? room owner is authoritative?
//!    maybe:
//!    1. room holds state and channel
//!    2. clients try to apply state updates. when successful, send to channel if necessary, before
//!       releasing lock on room
//!
//!    client and server keep copies of room state, try to keep operations idempotent. at some
//!    interval (like between rounds) state can get re-synced

mod protocol;
#[cfg(feature = "server")]
mod registry;
mod socket;

pub use protocol::{ClientMessage, RoomSharedState, RoomStage, ServerMessage};
pub use socket::game_socket;

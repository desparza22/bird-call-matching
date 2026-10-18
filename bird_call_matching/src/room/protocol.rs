//! Types shared between the client and the server.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "server")]
use crate::birds::count_correct;
use crate::birds::RoundSetup;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Player {
    pub name: String,
    pub correct_guesses_per_round: Vec<u32>,
    /// whether the player has submitted their guesses this round; also means they're ready for
    /// the next round
    pub ready: bool,
}

#[cfg(feature = "server")]
impl Player {
    pub fn new(name: String) -> Self {
        Self {
            name,
            correct_guesses_per_round: vec![0; 4],
            ready: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum RoomStage {
    Lobby,
    /// A round in progress. The round's setup travels with the stage, so the message announcing a
    /// new round also tells every player exactly what to play.
    Round {
        number: usize,
        setup: RoundSetup,
    },
}

/// The room state every player keeps a copy of. The server's copy is authoritative: it applies
/// changes, then sends the whole updated state to everyone in the room.
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct RoomSharedState {
    pub room_number: usize,
    /// name of the player who created the room
    pub host: String,
    /// a `BTreeMap` rather than a `HashMap` so players are always listed in the same order
    pub players: BTreeMap<String, Player>,
    pub stage: RoomStage,
}

impl RoomSharedState {
    pub fn is_host(&self, player_name: &str) -> bool {
        self.host == player_name
    }
}

/// State changes. Only the server applies these; clients just receive the result.
#[cfg(feature = "server")]
impl RoomSharedState {
    pub fn new(room_number: usize, host: Player) -> Self {
        Self {
            room_number,
            host: host.name.clone(),
            players: BTreeMap::from([(host.name.clone(), host)]),
            stage: RoomStage::Lobby,
        }
    }

    /// Adds a player, appending a numeric suffix to their name if it's already taken. Returns the
    /// name they ended up with.
    pub fn add_player(&mut self, player_name: String) -> String {
        let mut unique_player_name = player_name.clone();
        let mut unique_suffix = 1;
        while self.players.contains_key(&unique_player_name) {
            unique_suffix += 1;
            unique_player_name = format!("{player_name}_{unique_suffix}");
        }

        self.players.insert(
            unique_player_name.clone(),
            Player::new(unique_player_name.clone()),
        );
        unique_player_name
    }

    /// Moves the room from the lobby to the first round, set up by `new_round()`. Only the host
    /// can do this.
    pub fn start_game(
        &mut self,
        player_name: &str,
        new_round: impl FnOnce() -> RoundSetup,
    ) -> Result<(), String> {
        if !self.is_host(player_name) {
            return Err(format!(
                "{player_name} tried to start the game but isn't the host"
            ));
        }
        if self.stage != RoomStage::Lobby {
            return Err(format!(
                "{player_name} tried to start a game that already started"
            ));
        }
        self.stage = RoomStage::Round {
            number: 1,
            setup: new_round(),
        };
        Ok(())
    }

    /// Scores the player's guesses against this round's setup and marks them ready. Once every
    /// player is ready, moves on to the next round, set up by `new_round()`, and clears everyone's
    /// ready flag. `new_round` is only called if the round actually advances.
    pub fn submit_guesses(
        &mut self,
        player_name: &str,
        guesses: &[usize],
        new_round: impl FnOnce() -> RoundSetup,
    ) -> Result<(), String> {
        let RoomStage::Round { number, setup } = &self.stage else {
            return Err(format!(
                "{player_name} submitted guesses outside of a round"
            ));
        };
        let round = *number;
        let correct = count_correct(guesses, &setup.sound_positions);

        let player = self
            .players
            .get_mut(player_name)
            .ok_or_else(|| format!("{player_name} isn't in room {}", self.room_number))?;
        if player.ready {
            return Err(format!("{player_name} already submitted this round"));
        }
        *player
            .correct_guesses_per_round
            .get_mut(correct)
            .ok_or_else(|| format!("{player_name} submitted too many guesses"))? += 1;
        player.ready = true;

        if self.players.values().all(|player| player.ready) {
            for player in self.players.values_mut() {
                player.ready = false;
            }
            self.stage = RoomStage::Round {
                number: round + 1,
                setup: new_round(),
            };
        }
        Ok(())
    }
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
    /// The player's guess (a bird index) for each sound box. Also marks them ready.
    SubmitGuesses {
        guesses: Vec<usize>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ServerMessage {
    YouJoined {
        your_name: String,
        room: RoomSharedState,
    },
    RoomUpdated(RoomSharedState),
}

/// Sent between the server tasks of players in the same room.
#[cfg(feature = "server")]
#[derive(Clone)]
pub enum Broadcast {
    RoomChanged(RoomSharedState),
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    impl RoomStage {
        fn round_number(&self) -> Option<usize> {
            match self {
                RoomStage::Lobby => None,
                RoomStage::Round { number, .. } => Some(*number),
            }
        }
    }

    fn room_with(names: &[&str]) -> RoomSharedState {
        let mut room = RoomSharedState::new(0, Player::new(names[0].to_string()));
        for name in &names[1..] {
            room.add_player(name.to_string());
        }
        room
    }

    #[test]
    fn only_the_host_can_start_the_game() {
        let mut room = room_with(&["alice", "bob"]);
        assert!(room.start_game("bob", RoundSetup::default).is_err());
        assert_eq!(room.stage, RoomStage::Lobby);

        room.start_game("alice", RoundSetup::default).unwrap();
        assert_eq!(room.stage.round_number(), Some(1));
        assert!(room.start_game("alice", RoundSetup::default).is_err());
    }

    #[test]
    fn round_advances_once_everyone_is_ready() {
        let mut room = room_with(&["alice", "bob"]);
        let no_new_round = || -> RoundSetup { panic!("round advanced too early") };
        assert!(
            room.submit_guesses("alice", &[0, 1, 2], no_new_round)
                .is_err(),
            "can't submit in the lobby"
        );
        room.start_game("alice", RoundSetup::default).unwrap();

        room.submit_guesses("alice", &[0, 1, 2], no_new_round)
            .unwrap();
        assert_eq!(room.stage.round_number(), Some(1));
        assert!(room.players["alice"].ready);
        assert!(
            room.submit_guesses("alice", &[0, 1, 2], no_new_round)
                .is_err(),
            "can't submit twice in one round"
        );

        room.submit_guesses("bob", &[0, 1, 2], RoundSetup::default)
            .unwrap();
        assert_eq!(room.stage.round_number(), Some(2));
        assert!(room.players.values().all(|player| !player.ready));
    }

    #[test]
    fn submitted_guesses_are_scored_against_the_round() {
        let mut room = room_with(&["alice", "bob"]);
        let setup = || RoundSetup {
            birds: Vec::new(),
            sound_positions: vec![2, 0, 1, 4, 3],
        };
        room.start_game("alice", setup).unwrap();

        room.submit_guesses("alice", &[2, 0, 4], setup).unwrap();
        room.submit_guesses("bob", &[2, 0, 1], setup).unwrap();
        assert_eq!(
            room.players["alice"].correct_guesses_per_round,
            [0, 0, 1, 0]
        );
        assert_eq!(room.players["bob"].correct_guesses_per_round, [0, 0, 0, 1]);
    }

    #[test]
    fn duplicate_names_get_a_suffix() {
        let mut room = room_with(&["alice"]);
        assert_eq!(room.add_player("alice".to_string()), "alice_2");
        assert_eq!(room.add_player("alice".to_string()), "alice_3");
    }
}

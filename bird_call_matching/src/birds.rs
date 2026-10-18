//! Bird data types and the server function that sets up a random round.
//!
//! The server keeps a [`catalog`] of every bird in memory; the client only receives the
//! [`RoundSetup`] for one round, then loads its sound files and images directly.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
pub mod catalog;

/// How many birds are drawn for each round.
pub const BIRDS_PER_ROUND: usize = 5;

/// One bird chosen for a round, as sent over the wire. Kept deliberately small.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Bird {
    /// common name
    pub name: String,
    pub image_url: String,
    pub recording_id: String,
    pub sound_url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    pub id: String,
    /// common name of the bird in the recording
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BirdImage {
    /// common name of the bird in the image
    pub name: String,
    pub url: String,
}

impl Bird {
    /// Splits the bird into the recording and image the board shows separately.
    pub fn into_parts(self) -> (Recording, BirdImage) {
        let recording = Recording {
            id: self.recording_id,
            name: self.name.clone(),
            url: self.sound_url,
        };
        let image = BirdImage {
            name: self.name,
            url: self.image_url,
        };
        (recording, image)
    }
}

/// Everything a player needs to play one round. The server makes every random choice, so all
/// players in a room who receive the same `RoundSetup` see exactly the same board.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct RoundSetup {
    pub birds: Vec<Bird>,
    /// `sound_positions[i]` is the index into `birds` of the recording in sound box `i`
    pub sound_positions: Vec<usize>,
}

/// How many of `guesses` (one bird index per sound box) match the recording actually in that
/// box. Shared by solo scoring on the client and room scoring on the server.
pub fn count_correct(guesses: &[usize], sound_positions: &[usize]) -> usize {
    guesses
        .iter()
        .zip(sound_positions)
        .filter(|(guess, position)| guess == position)
        .count()
}

/// Sets up a solo round with `count` random birds.
#[server]
pub async fn random_round(count: usize) -> Result<RoundSetup, ServerFnError> {
    Ok(catalog::get().random_round(count))
}

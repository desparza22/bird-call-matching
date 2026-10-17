//! Bird data types and the server function that picks a random set of birds for a round.
//!
//! The server keeps a [`catalog`] of every bird in memory; the client only receives the handful of
//! [`Bird`]s it needs for one round, then loads their sound files and images directly.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
pub mod catalog;

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

/// Picks `count` distinct random birds, each with one random recording.
#[server]
pub async fn random_birds(count: usize) -> Result<Vec<Bird>, ServerFnError> {
    Ok(catalog::get().random_birds(count))
}

//! The server's in-memory index of every bird, its image, and its recordings. Server-only.
//!
//! Built once from the files under `public/` (`all-birds.txt`, each bird's `all-sounds.txt` and
//! `images/metadata.json`), so choosing birds for a round never touches the filesystem.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;

use super::{Bird, RoundSetup};

/// URL path the `public/birds` directory is served under.
const BIRDS_URL_DIR: &str = "/birds";

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    let public_dir = public_dir();
    Catalog::load(&public_dir).unwrap_or_else(|e| {
        panic!(
            "couldn't load bird catalog from {}: {e}",
            public_dir.display()
        )
    })
});

/// Loads the catalog now, so a missing or malformed data file fails at startup rather than on
/// the first request.
pub fn init() {
    LazyLock::force(&CATALOG);
}

pub fn get() -> &'static Catalog {
    &CATALOG
}

/// The directory the server serves static files from. Mirrors how `dioxus-server` finds it:
/// `$DIOXUS_PUBLIC_PATH` if set, otherwise `public/` next to the server executable.
fn public_dir() -> PathBuf {
    std::env::var_os("DIOXUS_PUBLIC_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .expect("couldn't find the server executable's path")
                .with_file_name("public")
        })
}

#[derive(Deserialize)]
struct ImageMetadata {
    en: String,
    medium_url: String,
}

struct CatalogBird {
    /// directory name under `public/birds/`
    dir: String,
    /// common name
    name: String,
    image_url: String,
    /// never empty; also the directory names under `public/birds/{dir}/`
    recording_ids: Vec<String>,
}

pub struct Catalog {
    birds: Vec<CatalogBird>,
}

impl Catalog {
    fn load(public_dir: &Path) -> io::Result<Self> {
        let birds_dir = public_dir.join("birds");
        let mut birds = Vec::new();

        for dir in fs::read_to_string(public_dir.join("all-birds.txt"))?.lines() {
            let dir = dir.trim();
            if dir.is_empty() {
                continue;
            }
            let bird_dir = birds_dir.join(dir);

            let recording_ids: Vec<String> = fs::read_to_string(bird_dir.join("all-sounds.txt"))?
                .split_whitespace()
                .map(str::to_owned)
                .collect();
            if recording_ids.is_empty() {
                continue;
            }

            // `serde_json::Error` converts into `io::Error`, so `?` works here too.
            let image: ImageMetadata =
                serde_json::from_str(&fs::read_to_string(bird_dir.join("images/metadata.json"))?)?;

            birds.push(CatalogBird {
                dir: dir.to_owned(),
                name: image.en,
                image_url: image.medium_url,
                recording_ids,
            });
        }

        Ok(Self { birds })
    }

    /// Picks `count` distinct random birds, each with one random recording, and shuffles which
    /// recording goes in which sound box.
    pub fn random_round(&self, count: usize) -> RoundSetup {
        let birds = self.random_birds(count);
        let mut sound_positions: Vec<usize> = (0..birds.len()).collect();
        sound_positions.shuffle(&mut thread_rng());
        RoundSetup {
            birds,
            sound_positions,
        }
    }

    fn random_birds(&self, count: usize) -> Vec<Bird> {
        self.birds
            .choose_multiple(&mut thread_rng(), count)
            .map(|bird| {
                let recording_id = bird
                    .recording_ids
                    .choose(&mut thread_rng())
                    .expect("catalog birds always have a recording")
                    .clone();
                Bird {
                    name: bird.name.clone(),
                    image_url: bird.image_url.clone(),
                    sound_url: format!("{BIRDS_URL_DIR}/{}/{recording_id}/sound.wav", bird.dir),
                    recording_id,
                }
            })
            .collect()
    }
}

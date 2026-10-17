//! Bird data types and fetching random birds from the static `/birds/` directory.

use futures::future::join_all;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;

const BIRDS_URL_DIR: &str = "/birds/";

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct RecordingMetadata {
    pub id: String,
    pub en: String, // common name
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    pub metadata: RecordingMetadata,
    pub url: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct ImageLinks {
    pub en: String,
    pub url: String,
    pub square_url: String,
    pub medium_url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bird {
    pub recording: Recording,
    pub image_links: ImageLinks,
}

fn origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

pub async fn choose_birds(num_birds: usize) -> Result<Vec<Bird>, reqwest::Error> {
    // read all-birds to get directory names
    let all_birds = fetch_all_bird_dirs().await?;

    // select random bird directories
    let random_birds: Vec<String> = all_birds
        .choose_multiple(&mut thread_rng(), num_birds)
        .cloned()
        .collect();

    // select a random sound directory from each bird
    let bird_images_and_sounds: Vec<(ImageLinks, String)> =
        join_all(random_birds.into_iter().map(fetch_image_and_sound_dir))
            .await
            .into_iter()
            .collect::<Result<_, _>>()?;

    // get metadatas from each sound dir
    let images_and_sound_metadatas_and_dirs: Vec<(ImageLinks, RecordingMetadata, String)> =
        join_all(
            bird_images_and_sounds
                .into_iter()
                .map(|(image_links, sound_dir)| async move {
                    let metadata = fetch_recording_metadata(&sound_dir).await?;
                    Ok::<_, reqwest::Error>((image_links, metadata, sound_dir))
                }),
        )
        .await
        .into_iter()
        .collect::<Result<_, _>>()?;

    // convert to [Bird]s and return
    let birds = images_and_sound_metadatas_and_dirs
        .into_iter()
        .map(|(image_links, metadata, dir)| Bird {
            recording: Recording {
                metadata,
                url: dir + "/sound.wav",
            },
            image_links,
        })
        .collect();
    Ok(birds)
}

async fn fetch_all_bird_dirs() -> Result<Vec<String>, reqwest::Error> {
    let all_birds_text = reqwest::get(format!("{}/all-birds.txt", origin()))
        .await?
        .text()
        .await?;
    Ok(all_birds_text
        .lines()
        .map(|line| line.trim().to_owned())
        .collect())
}

/// Returns the bird's image links and the URL path of one randomly chosen sound directory.
async fn fetch_image_and_sound_dir(bird: String) -> Result<(ImageLinks, String), reqwest::Error> {
    let bird_dir = format!("{BIRDS_URL_DIR}{bird}");

    let sounds_text = reqwest::get(format!("{}{bird_dir}/all-sounds.txt", origin()))
        .await?
        .text()
        .await?;
    let sounds: Vec<&str> = sounds_text.split_whitespace().collect();
    let random_sound = sounds.choose(&mut thread_rng()).unwrap();

    let image_links = reqwest::get(format!("{}{bird_dir}/images/metadata.json", origin()))
        .await?
        .json::<ImageLinks>()
        .await?;

    Ok((image_links, format!("{bird_dir}/{random_sound}")))
}

async fn fetch_recording_metadata(sound_dir: &str) -> Result<RecordingMetadata, reqwest::Error> {
    reqwest::get(format!("{}{sound_dir}/metadata.json", origin()))
        .await?
        .json::<RecordingMetadata>()
        .await
}

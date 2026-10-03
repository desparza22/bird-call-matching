use dioxus::prelude::*;
use futures::future::join_all;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct RecordingMetadata {
    id: String,
    en: String, // common name
}

#[derive(Clone, Debug, PartialEq)]
struct Recording {
    metadata: RecordingMetadata,
    url: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ImageLinks {
    en: String,
    url: String,
    square_url: String,
    medium_url: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Bird {
    recording: Recording,
    image_links: ImageLinks,
}

struct Guessing {
    sound_positions: Vec<usize>,
    sound_guesses: Vec<Option<usize>>,
    selected_sound: Option<usize>,
    submitted: Option<Vec<usize>>,
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

static CSS: Asset = asset!("/assets/main.css");
static BIRDS_URL_DIR: &str = "/birds/";

#[server]
async fn upvote_sound(id: String) -> Result<(), ServerFnError> {
    use rusqlite::Connection;

    let conn = Connection::open("ratings.db").map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "INSERT INTO sound_ratings (id, upvotes, downvotes) VALUES (?1, 1, 0)
         ON CONFLICT(id) DO UPDATE SET upvotes = upvotes + 1",
        (id,),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
async fn downvote_sound(id: String) -> Result<(), ServerFnError> {
    use rusqlite::Connection;

    let conn = Connection::open("ratings.db").map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "INSERT INTO sound_ratings (id, upvotes, downvotes) VALUES (?1, 0, 1)
         ON CONFLICT(id) DO UPDATE SET downvotes = downvotes + 1",
        (id,),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
async fn get_ratings(id: String) -> Result<(i64, i64), ServerFnError> {
    use rusqlite::Connection;

    let conn = Connection::open("ratings.db").map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.query_row(
        "SELECT upvotes, downvotes FROM sound_ratings WHERE id = ?1",
        (id,),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))
}

fn origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

async fn choose_birds(num_birds: usize) -> Result<Vec<Bird>, reqwest::Error> {
    // read all-birds to get directory names
    let all_birds_text = reqwest::get(origin() + "/all-birds.txt")
        .await?
        .text()
        .await;
    let all_birds: Vec<String> = all_birds_text?
        .lines()
        .map(|line| line.trim().to_owned())
        .collect();

    // select random bird directories
    let mut rng = thread_rng();
    let random_birds: Vec<String> = all_birds
        .choose_multiple(&mut rng, num_birds)
        .cloned()
        .collect();

    // select a random sound directory from each bird
    let bird_images_and_sounds = random_birds.into_iter().map(async |bird| {
        let mut rng = thread_rng();
        let sounds_text = reqwest::get(origin() + BIRDS_URL_DIR + &bird + "/all-sounds.txt")
            .await?
            .text()
            .await?;
        let sounds: Vec<&str> = sounds_text.split_whitespace().collect();
        let random_sound = sounds.choose(&mut rng).unwrap();

        let image_links = reqwest::get(origin() + BIRDS_URL_DIR + &bird + "/images/metadata.json")
            .await?
            .json::<ImageLinks>()
            .await?;

        Ok::<(ImageLinks, String), reqwest::Error>((
            image_links,
            String::from(BIRDS_URL_DIR) + &bird + "/" + random_sound,
        ))
    });
    let bird_images_and_sounds: Vec<(ImageLinks, String)> = join_all(bird_images_and_sounds)
        .await
        .into_iter()
        .collect::<Result<_, reqwest::Error>>()?;

    // get metadatas from each sound dir
    let images_and_sound_metadatas_and_dirs =
        bird_images_and_sounds
            .into_iter()
            .map(async |(image_links, sound_dir)| {
                let metadata = reqwest::get(origin() + &sound_dir + "/metadata.json")
                    .await?
                    .json::<RecordingMetadata>()
                    .await?;
                Ok::<(ImageLinks, RecordingMetadata, String), reqwest::Error>((
                    image_links,
                    metadata,
                    sound_dir,
                ))
            });
    let images_and_sound_metadatas_and_dirs: Vec<(ImageLinks, RecordingMetadata, String)> =
        join_all(images_and_sound_metadatas_and_dirs)
            .await
            .into_iter()
            .collect::<Result<_, reqwest::Error>>()?;

    // convert to [Recording]s and return
    let birds = images_and_sound_metadatas_and_dirs
        .into_iter()
        .map(|(image_links, metadata, dir)| {
            let recording = Recording {
                metadata,
                url: dir + "/sound.wav",
            };
            Bird {
                recording,
                image_links,
            }
        })
        .collect();
    Ok(birds)
}

#[component]
fn App() -> Element {
    let mut birds_resource = use_resource(|| async { choose_birds(5).await });

    let guessing = use_signal(|| {
        let mut rng = thread_rng();
        let mut sound_positions = vec![0, 1, 2, 3, 4];
        sound_positions.shuffle(&mut rng);
        Guessing {
            sound_positions,
            sound_guesses: vec![None, None, None],
            selected_sound: None,
            submitted: None,
        }
    });

    rsx! {
        document::Stylesheet { href: CSS }
        match &*birds_resource.read() {
            Some(Ok(birds)) => rsx! {
                Title {}
                Sounds { recordings: birds.iter().map(|bird| bird.recording.clone()).collect(), guessing }
                Images { image_links: birds.iter().map(|bird| bird.image_links.clone()).collect(), guessing }
                Redraw { birds_resource, guessing }
            },
            Some(Err(e)) => rsx! {
                div { "Failed to load: {e}" }
                button { onclick: move |_| birds_resource.restart(), "retry" }
            },
            None => rsx! {
                div { "Loading..." }
            },
        }
    }
}

#[component]
fn Title() -> Element {
    rsx! {
        div { id: "title",
            h1 { "Go Birdy" }
        }
    }
}

#[component]
fn Redraw(
    mut birds_resource: Resource<Result<Vec<Bird>, reqwest::Error>>,
    mut guessing: Signal<Guessing>,
) -> Element {
    let submit = |guesses: Vec<usize>| {
        move |_| {
            guessing.write().submitted = Some(guesses.clone());
            guessing.write().selected_sound = None;
        }
    };

    let redraw = move |_| {
        birds_resource.restart();
        guessing.write().sound_guesses = vec![None, None, None, None, None];
        guessing.write().submitted = None;
    };

    rsx! {
    {
        match &guessing.read().submitted {
            None => {
                let guesses : Vec<usize> = guessing.read().sound_guesses.iter().filter_map(|guess| *guess).collect();
                if guesses.len() == 3 {
                    rsx! {
                        button {
                            onclick: submit(guesses),
                            "submit"
                        }
                    }
                } else {
                    rsx! {}
                }


            }
            Some(_) => {
                rsx! {
                    button {
                        onclick: redraw,  "re-draw" }
                    }
                }
            }
        }
    }
}

#[component]
fn Sounds(recordings: Vec<Recording>, mut guessing: Signal<Guessing>) -> Element {
    let widths = 200 - 2;
    let heights = 150 - 2;

    let mut playing = use_signal(|| vec![false, false, false]);
    let audio_element_id = |id: usize| format!("sound{}-id", id);
    let border = |id: usize| {
        if playing.read()[id] {
            "red"
        } else {
            "white"
        }
    };

    let play_sound = |id: usize| {
        move |_| {
            let element_id = audio_element_id(id);
            spawn(async move {
                playing.write()[id] = true;
                let js = format!(
                    "const el = document.getElementById('{element_id}');
                 el.play();
                 await new Promise(resolve => {{ el.onended = resolve; }});"
                );
                let _ = document::eval(&js).await;
                playing.write()[id] = false;
            });
        }
    };

    let background = |id: usize| {
        let selected = guessing
            .read()
            .selected_sound
            .map(|selected| selected == id)
            .unwrap_or(false);
        if selected {
            "cyan"
        } else {
            "gray"
        }
    };
    let corresponding_recording = |id| guessing.read().sound_positions[id];

    let select = |id| move |_| guessing.write().selected_sound = Some(id);

    rsx! {
        div { id: "sounds", style: "display: flex; flex-wrap: wrap; gap: 8px;",

            for audio_box in (0..3) {
                div {
            key: "{audio_box}",
            audio { id: "{audio_element_id(audio_box)}", src: "{recordings[corresponding_recording(audio_box)].url}" }
            div {
                style: "width: {widths}px; height: {heights}px; border: 1px solid {border(audio_box)}; background: {background(audio_box)};",
                button {
                    onclick: play_sound(audio_box),
                    "listen"
                }
                button {
                    onclick: select(audio_box),
                    "guess"
                }
                {
                match guessing.read().sound_guesses[audio_box] {
                    None => rsx! {},
                    Some(guess) => rsx! {
                        div { "{recordings[guess].metadata.en}" }
                    }
                }

                    }
                {
                match &guessing.read().submitted {
                    None => rsx! {},
                    Some(submitted_value) => {
                        let color =
                        if submitted_value[audio_box] == guessing.read().sound_positions[audio_box] {
                            "green"
                        } else {
                             "red"
                        };


                        rsx! {
                            div {color:"{color}", "{recordings[corresponding_recording(audio_box)].metadata.en}"}
                    }
                    }
                }
                }
            }
                }
            }

        }
    }
}

#[component]
fn Votes(recording_id: String) -> Element {
    // TODO fetch votes and display
    // add upvotes and downvotes buttons
}

#[component]
fn Images(image_links: Vec<ImageLinks>, mut guessing: Signal<Guessing>) -> Element {
    let border_size = 2;
    let widths = 200 - 2 * border_size;
    let heights = 150 - 2 * border_size;

    let guess = |image_id| {
        move |_| {
            let mut guessing_value = guessing.write();

            match guessing_value.selected_sound {
                None => {}
                Some(sound_id) => {
                    guessing_value.sound_guesses[sound_id] = Some(image_id);
                    guessing_value.selected_sound = None;
                }
            }
        }
    };

    rsx! {
        div {
            id: "pictures",
            style: "display: flex; flex-wrap: wrap; gap: 3px;",
            for bird_image in (0..5) {

            div {
                key: "{bird_image}",
                style: "width: {widths}px; height: {heights}px; border: {border_size}px solid cyan;",
                onclick: guess(bird_image),
                img { src:"{image_links[bird_image].medium_url}",
                style: "width: 80%; height: 80%; object-fit: cover; display: block;"
                },
                "{image_links[bird_image].en}" }
            }
        }
    }
}

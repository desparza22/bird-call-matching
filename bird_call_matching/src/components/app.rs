use dioxus::prelude::*;

use super::{Images, Multiplayer, Redraw, Score, Sounds, Title};
use crate::birds::choose_birds;
use crate::guessing::Guessing;

static CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    let mut birds_resource = use_resource(|| async { choose_birds(5).await });
    let guessing = use_signal(Guessing::new);

    rsx! {
        document::Stylesheet { href: CSS }
        main { class: "app",
            Title {}
            match &*birds_resource.read() {
                Some(Ok(birds)) => rsx! {
                    Multiplayer {}
                    Sounds {
                        recordings: birds.iter().map(|bird| bird.recording.clone()).collect(),
                        guessing,
                    }
                    Images {
                        image_links: birds.iter().map(|bird| bird.image_links.clone()).collect(),
                        guessing,
                    }
                    Redraw { birds_resource, guessing }
                    Score { guessing }
                },
                Some(Err(e)) => rsx! {
                    div { class: "status error", "Couldn't load birds: {e}" }
                    button { class: "btn primary", onclick: move |_| birds_resource.restart(), "Try again" }
                },
                None => rsx! {
                    div { class: "status", "Loading birds…" }
                },
            }
        }
    }
}

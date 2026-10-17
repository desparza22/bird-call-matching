use dioxus::prelude::*;

use super::{Images, Multiplayer, Redraw, Score, Sounds, Title};
use crate::birds::choose_birds;
use crate::board_state::BoardState;

static CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    let mut board_state = use_signal(BoardState::new);

    // Writes the birds into the board before resolving, so the board is already populated on
    // the first render that sees `Some(Ok(()))`.
    let mut birds_resource = use_resource(move || async move {
        let birds = choose_birds(5).await?;
        board_state.write().start_round(birds);
        Ok::<_, reqwest::Error>(())
    });

    rsx! {
        document::Stylesheet { href: CSS }
        main { class: "app",
            Title {}
            match &*birds_resource.read() {
                Some(Ok(())) => rsx! {
                    Multiplayer {}
                    Sounds { board_state }
                    Images { board_state }
                    Redraw { birds_resource, board_state }
                    Score { board_state }
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

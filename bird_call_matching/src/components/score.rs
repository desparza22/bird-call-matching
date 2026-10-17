use dioxus::prelude::*;

use crate::guessing::Guessing;

#[component]
pub fn Score(guessing: Signal<Guessing>) -> Element {
    const TIERS: [&str; 4] = ["🪹", "🪺", "🐣", "🐦‍🔥"];
    rsx! {
        section {
            h2 { class: "panel-title", "Score" }
            div { class: "score-grid",
                for (correct, emoji) in TIERS.iter().enumerate() {
                    div { key: "{correct}", class: "score-tile",
                        span { class: "score-emoji", "{emoji}" }
                        span { class: "score-count",
                            "{guessing.read().correct_guesses_per_round[correct]}"
                        }
                        span { class: "score-label", "{correct} correct" }
                    }
                }
            }
        }
    }
}

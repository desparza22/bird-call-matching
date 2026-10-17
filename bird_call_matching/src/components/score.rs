use dioxus::prelude::*;

use crate::board_state::BoardState;

#[component]
pub fn Score(board_state: Signal<BoardState>) -> Element {
    const TIERS: [&str; 4] = ["🪹", "🪺", "🐣", "🐦‍🔥"];
    rsx! {
        section {
            h2 { class: "panel-title", "Score" }
            div { class: "score-grid",
                for (correct, emoji) in TIERS.iter().enumerate() {
                    div { key: "{correct}", class: "score-tile",
                        span { class: "score-emoji", "{emoji}" }
                        span { class: "score-count",
                            "{board_state.read().correct_guesses_per_round[correct]}"
                        }
                        span { class: "score-label", "{correct} correct" }
                    }
                }
            }
        }
    }
}

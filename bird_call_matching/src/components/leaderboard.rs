use std::cmp::Reverse;

use dioxus::prelude::*;

/// Column headers: the emoji for getting 0, 1, 2 or 3 sounds right in a round.
const TIERS: [&str; 4] = ["🪹", "🪺", "🐣", "🐦‍🔥"];

/// One row of the leaderboard.
#[derive(Clone, PartialEq, Debug)]
pub struct LeaderboardEntry {
    /// `None` in solo play, where the name column is hidden
    pub name: Option<String>,
    /// `correct_guesses_per_round[i]` is how many rounds the player got `i` sounds right
    pub correct_guesses_per_round: Vec<u32>,
    pub is_current_player: bool,
}

impl LeaderboardEntry {
    /// Total correct guesses across every round; what the leaderboard is ranked by.
    fn total_correct(&self) -> u32 {
        self.correct_guesses_per_round
            .iter()
            .zip(0..)
            .map(|(rounds, correct)| rounds * correct)
            .sum()
    }
}

#[component]
pub fn Leaderboard(mut entries: Vec<LeaderboardEntry>) -> Element {
    // `sort_by_key` is stable, so tied players keep their incoming (alphabetical) order.
    entries.sort_by_key(|entry| Reverse(entry.total_correct()));
    let show_names = entries.iter().any(|entry| entry.name.is_some());

    rsx! {
        section { class: "panel",
            h2 { class: "panel-title", "Leaderboard" }
            table { class: "leaderboard",
                thead {
                    tr {
                        if show_names {
                            th { class: "label", "Player" }
                        }
                        for (correct, emoji) in TIERS.iter().enumerate() {
                            th { key: "{correct}", title: "{correct} correct", "{emoji}" }
                        }
                        th { class: "label total", "Total" }
                    }
                }
                tbody {
                    for entry in entries.iter() {
                        tr {
                            key: "{entry.name.as_deref().unwrap_or_default()}",
                            class: if entry.is_current_player { "you" },
                            if let Some(name) = &entry.name {
                                td {
                                    "{name}"
                                    if entry.is_current_player {
                                        " (you)"
                                    }
                                }
                            }
                            for (correct, rounds) in entry.correct_guesses_per_round.iter().enumerate() {
                                td { key: "{correct}", "{rounds}" }
                            }
                            td { class: "total", "{entry.total_correct()}" }
                        }
                    }
                }
            }
        }
    }
}

use dioxus::prelude::*;

use super::{Images, Leaderboard, LeaderboardEntry, Redraw, Sounds};
use crate::board_state::BoardState;

/// The guessing game itself, shared by solo play and rooms. `on_submit` receives the player's
/// guesses after they submit, so the caller can score them and fetch or request the next round.
#[component]
pub fn GameBoard(
    board_state: Signal<BoardState>,
    on_submit: EventHandler<Vec<usize>>,
    leaderboard: Vec<LeaderboardEntry>,
) -> Element {
    rsx! {
        Sounds { board_state }
        Images { board_state }
        Redraw { board_state, on_submit }
        Leaderboard { entries: leaderboard }
    }
}

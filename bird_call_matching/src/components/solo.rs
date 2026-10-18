use dioxus::prelude::*;

use super::{GameBoard, LeaderboardEntry, RoomForms};
use crate::birds::{random_round, BIRDS_PER_ROUND};
use crate::board_state::BoardState;
use crate::room::ClientMessage;

/// Solo play: a room of one that nobody else can join. Each round's birds come from a plain
/// server function call, so no websocket is needed.
#[component]
pub fn SoloGame(
    mut board_state: Signal<BoardState>,
    on_join: EventHandler<ClientMessage>,
) -> Element {
    let mut load_error = use_signal(|| None::<String>);

    // Fetches the birds for the round after the current one and queues them on the board.
    let load_next_round = move || {
        let round = board_state.read().round + 1;
        spawn(async move {
            load_error.set(None);
            match random_round(BIRDS_PER_ROUND).await {
                Ok(setup) => board_state.write().queue_round(round, setup),
                Err(err) => load_error.set(Some(err.to_string())),
            }
        });
    };
    // Load the first round when solo play starts. `use_hook` runs its closure only on the first
    // render, so this doesn't refetch on every re-render.
    use_hook(load_next_round);

    rsx! {
        RoomForms { on_join }
        if let Some(err) = load_error() {
            div { class: "status error", "Couldn't load birds: {err}" }
            button { class: "btn primary", onclick: move |_| load_next_round(), "Try again" }
        }
        if board_state.read().has_round() {
            GameBoard {
                board_state,
                on_submit: move |_| load_next_round(),
                leaderboard: vec![LeaderboardEntry {
                    name: None,
                    correct_guesses_per_round: board_state.read().correct_guesses_per_round.clone(),
                    is_current_player: true,
                }],
            }
        } else if load_error.read().is_none() {
            div { class: "status", "Loading birds…" }
        }
    }
}

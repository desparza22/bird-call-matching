use dioxus::prelude::*;

use crate::board_state::BoardState;

#[component]
pub fn Redraw(mut board_state: Signal<BoardState>, on_submit: EventHandler<Vec<usize>>) -> Element {
    let submit = move |guesses: Vec<usize>| {
        move |_| {
            board_state.write().submit(guesses.clone());
            on_submit.call(guesses.clone());
        }
    };
    let next_round = move |_| board_state.write().advance_round();

    let board = board_state.read();
    rsx! {
        div { class: "actions",
            match (&board.submitted, &board.next_round) {
                (None, _) => {
                    let guesses: Vec<usize> = board
                        .sound_guesses
                        .iter()
                        .filter_map(|guess| *guess)
                        .collect();
                    if guesses.len() == 3 {
                        rsx! {
                            button { class: "btn primary", onclick: submit(guesses), "Submit guesses" }
                        }
                    } else {
                        rsx! {
                            div { class: "hint", "Click a call, then click the bird you think made it." }
                        }
                    }
                }
                (Some(_), Some(_)) => rsx! {
                    button { class: "btn primary", onclick: next_round, "Next round" }
                },
                (Some(_), None) => rsx! {
                    div { class: "hint", "Waiting for the next round…" }
                },
            }
        }
    }
}

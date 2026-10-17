use dioxus::prelude::*;

use crate::board_state::BoardState;

#[component]
pub fn Redraw(
    mut birds_resource: Resource<Result<(), reqwest::Error>>,
    mut board_state: Signal<BoardState>,
) -> Element {
    let submit = |guesses: Vec<usize>| move |_| board_state.write().submit(guesses.clone());

    let redraw = move |_| {
        birds_resource.restart();
        board_state.write().reset_for_new_round();
    };

    rsx! {
        div { class: "actions",
            match &board_state.read().submitted {
                None => {
                    let guesses: Vec<usize> = board_state
                        .read()
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
                Some(_) => rsx! {
                    button { class: "btn primary", onclick: redraw, "New round" }
                },
            }
        }
    }
}

use dioxus::prelude::*;

use crate::birds::Bird;
use crate::guessing::Guessing;

#[component]
pub fn Redraw(
    mut birds_resource: Resource<Result<Vec<Bird>, reqwest::Error>>,
    mut guessing: Signal<Guessing>,
) -> Element {
    let submit = |guesses: Vec<usize>| move |_| guessing.write().submit(guesses.clone());

    let redraw = move |_| {
        birds_resource.restart();
        guessing.write().reset_for_new_round();
    };

    rsx! {
        div { class: "actions",
            match &guessing.read().submitted {
                None => {
                    let guesses: Vec<usize> = guessing
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

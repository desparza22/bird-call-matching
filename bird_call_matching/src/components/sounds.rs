use dioxus::prelude::*;

use super::Votes;
use crate::board_state::BoardState;

#[component]
pub fn Sounds(mut board_state: Signal<BoardState>) -> Element {
    let mut playing = use_signal(|| vec![false, false, false]);
    let audio_element_id = |id: usize| format!("sound{id}-id");

    let mut do_select = move |id| board_state.write().selected_sound = Some(id);
    let do_listen = move |id| {
        let element_id = audio_element_id(id);
        spawn(async move {
            playing.write()[id] = true;
            let js = format!(
                "const el = document.getElementById('{element_id}');
                 el.play();
                 await new Promise(resolve => {{ el.onended = resolve; }});"
            );
            let _ = document::eval(&js).await;
            playing.write()[id] = false;
        });
    };
    let select = |id| move |_| do_select(id);
    let listen_and_select = |id: usize| {
        move |_| {
            do_select(id);
            do_listen(id)
        }
    };

    let board = board_state.read();
    let card_class = |id: usize| {
        let playing_class = if playing.read()[id] { " playing" } else { "" };
        let selected_class = if board.selected_sound == Some(id) {
            " selected"
        } else {
            ""
        };
        format!("sound-card{playing_class}{selected_class}")
    };

    rsx! {
        section {
            h2 { class: "panel-title", "Calls" }
            div { id: "sounds", class: "card-row",
                for audio_box in 0..3 {
                    div {
                        key: "{audio_box}",
                        class: "{card_class(audio_box)}",
                        onclick: select(audio_box),
                        audio {
                            id: "{audio_element_id(audio_box)}",
                            src: "{board.recording_for_box(audio_box).url}",
                        }
                        div { class: "sound-label", "Call {audio_box + 1}" }
                        div { class: "wave",
                            for bar in 0..7 {
                                span { key: "{bar}" }
                            }
                        }
                        button {
                            class: "btn listen",
                            onclick: listen_and_select(audio_box),
                            "▶ Listen"
                        }
                        Votes { recording_id: board.recording_for_box(audio_box).id.clone() }
                        match board.sound_guesses[audio_box] {
                            None => rsx! {},
                            Some(guess) => rsx! {
                                div { class: "guess", "You guessed {board.recordings[guess].name}" }
                            },
                        }
                        match &board.submitted {
                            None => rsx! {},
                            Some(submitted) => {
                                let correct = submitted[audio_box] == board.sound_positions[audio_box];
                                let name = &board.recording_for_box(audio_box).name;
                                if correct {
                                    rsx! {
                                        div { class: "result correct", "✓ {name}" }
                                    }
                                } else {
                                    rsx! {
                                        div { class: "result incorrect", "✗ It was {name}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

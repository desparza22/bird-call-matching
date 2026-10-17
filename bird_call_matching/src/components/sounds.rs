use dioxus::prelude::*;

use super::Votes;
use crate::birds::Recording;
use crate::guessing::Guessing;

#[component]
pub fn Sounds(recordings: Vec<Recording>, mut guessing: Signal<Guessing>) -> Element {
    let mut playing = use_signal(|| vec![false, false, false]);
    let audio_element_id = |id: usize| format!("sound{id}-id");

    let mut do_select = move |id| guessing.write().selected_sound = Some(id);
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

    let card_class = |id: usize| {
        let playing_class = if playing.read()[id] { " playing" } else { "" };
        let selected_class = if guessing.read().selected_sound == Some(id) {
            " selected"
        } else {
            ""
        };
        format!("sound-card{playing_class}{selected_class}")
    };
    let corresponding_recording = |id| guessing.read().sound_positions[id];
    let recording_for = |id: usize| &recordings[corresponding_recording(id)];

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
                            src: "{recording_for(audio_box).url}",
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
                        Votes { recording_id: recording_for(audio_box).metadata.id.clone() }
                        match guessing.read().sound_guesses[audio_box] {
                            None => rsx! {},
                            Some(guess) => rsx! {
                                div { class: "guess", "You guessed {recordings[guess].metadata.en}" }
                            },
                        }
                        match &guessing.read().submitted {
                            None => rsx! {},
                            Some(submitted) => {
                                let correct = submitted[audio_box]
                                    == guessing.read().sound_positions[audio_box];
                                let name = &recording_for(audio_box).metadata.en;
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

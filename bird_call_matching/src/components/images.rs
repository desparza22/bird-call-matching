use dioxus::prelude::*;

use crate::board_state::BoardState;

#[component]
pub fn Images(mut board_state: Signal<BoardState>) -> Element {
    let guess = |image_id| move |_| board_state.write().guess_selected(image_id);

    let board = board_state.read();
    let pictures_class = if board.selected_sound.is_some() {
        "pictures armed"
    } else {
        "pictures"
    };

    rsx! {
        section {
            h2 { class: "panel-title", "Birds" }
            div { id: "pictures", class: "{pictures_class}",
                for (bird_image, links) in board.image_links.iter().enumerate() {
                    div {
                        key: "{bird_image}",
                        class: "photo",
                        onclick: guess(bird_image),
                        img { src: "{links.medium_url}", alt: "" }
                        div { class: "name", "{links.en}" }
                    }
                }
            }
        }
    }
}

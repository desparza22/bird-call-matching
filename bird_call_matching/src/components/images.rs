use dioxus::prelude::*;

use crate::birds::ImageLinks;
use crate::guessing::Guessing;

#[component]
pub fn Images(image_links: Vec<ImageLinks>, mut guessing: Signal<Guessing>) -> Element {
    let guess = |image_id| move |_| guessing.write().guess_selected(image_id);
    let pictures_class = if guessing.read().selected_sound.is_some() {
        "pictures armed"
    } else {
        "pictures"
    };

    rsx! {
        section {
            h2 { class: "panel-title", "Birds" }
            div { id: "pictures", class: "{pictures_class}",
                for bird_image in 0..5 {
                    div {
                        key: "{bird_image}",
                        class: "photo",
                        onclick: guess(bird_image),
                        img {
                            src: "{image_links[bird_image].medium_url}",
                            alt: "",
                        }
                        div { class: "name", "{image_links[bird_image].en}" }
                    }
                }
            }
        }
    }
}

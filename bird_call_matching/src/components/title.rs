use dioxus::prelude::*;

#[component]
pub fn Title() -> Element {
    rsx! {
        header { class: "masthead",
            h1 { "Go Birdy" }
            p { class: "lede", "Match each call to the bird that made it." }
        }
    }
}

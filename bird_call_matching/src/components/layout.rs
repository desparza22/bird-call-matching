use dioxus::prelude::*;

use super::Title;

/// The page frame shared by solo play and rooms: a sidebar with the title, room controls and
/// leaderboard, next to the game board (`children`).
#[component]
pub fn GameLayout(sidebar: Element, children: Element) -> Element {
    rsx! {
        aside { class: "sidebar",
            Title {}
            {sidebar}
        }
        section { class: "board", {children} }
    }
}

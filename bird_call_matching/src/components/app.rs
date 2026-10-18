use dioxus::prelude::*;

use super::{OnlineGame, SoloGame, Title};
use crate::board_state::BoardState;
use crate::room::ClientMessage;

static CSS: Asset = asset!("/assets/main.css");

/// Where the board's rounds come from.
#[derive(Clone, PartialEq, Debug)]
enum Mode {
    /// A private game: rounds are fetched over plain HTTP, and no websocket is opened.
    Solo,
    /// A shared room. Holds the create/join request that opens the connection.
    Online(ClientMessage),
}

#[component]
pub fn App() -> Element {
    let mut board_state = use_signal(BoardState::new);
    let mut mode = use_signal(|| Mode::Solo);

    let mut switch_mode = move |new_mode: Mode| {
        board_state.write().clear_round();
        mode.set(new_mode);
    };

    rsx! {
        document::Stylesheet { href: CSS }
        main { class: "app",
            Title {}
            match mode() {
                Mode::Solo => rsx! {
                    SoloGame {
                        board_state,
                        on_join: move |first_message| switch_mode(Mode::Online(first_message)),
                    }
                },
                Mode::Online(first_message) => rsx! {
                    OnlineGame {
                        board_state,
                        first_message,
                        on_leave: move |_| switch_mode(Mode::Solo),
                    }
                },
            }
        }
    }
}

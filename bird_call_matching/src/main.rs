mod birds;
mod board_state;
mod components;
mod ratings;
mod room;

fn main() {
    #[cfg(feature = "server")]
    birds::catalog::init();

    dioxus::launch(components::App);
}

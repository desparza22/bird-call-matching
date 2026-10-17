mod birds;
mod board_state;
mod components;
mod ratings;
mod room;

fn main() {
    dioxus::launch(components::App);
}

mod birds;
mod components;
mod guessing;
mod ratings;
mod room;

fn main() {
    dioxus::launch(components::App);
}

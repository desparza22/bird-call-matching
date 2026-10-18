use dioxus::prelude::*;

use crate::room::ClientMessage;

/// Forms for creating or joining a room. Submitting one hands the request to `on_join`; the
/// connection itself is opened by [`super::OnlineGame`].
#[component]
pub fn RoomForms(on_join: EventHandler<ClientMessage>) -> Element {
    let mut create_room_name_text = use_signal(String::new);
    let mut join_room_name_text = use_signal(String::new);
    let mut join_room_number_text = use_signal(String::new);

    // TODO: on invalid room number, show error
    rsx! {
        section { class: "panel",
            h2 { class: "panel-title", "Play together" }
            form {
                class: "room-form",
                onsubmit: move |evt| {
                    evt.prevent_default();
                    let player_name = create_room_name_text();
                    create_room_name_text.set(String::new());
                    on_join.call(ClientMessage::CreateRoom { player_name });
                },
                input {
                    r#type: "text",
                    placeholder: "Your name",
                    aria_label: "Your name, to create a room",
                    value: "{create_room_name_text}",
                    oninput: move |evt| create_room_name_text.set(evt.value()),
                }
                button { class: "btn", r#type: "submit", "Create room" }
            }

            form {
                class: "room-form",
                onsubmit: move |evt| {
                    evt.prevent_default();
                    if let Ok(room_number) = join_room_number_text().trim().parse::<usize>() {
                        let player_name = join_room_name_text();
                        join_room_number_text.set(String::new());
                        join_room_name_text.set(String::new());
                        on_join.call(ClientMessage::JoinRoom { room_number, player_name });
                    }
                },
                input {
                    r#type: "text",
                    placeholder: "Your name",
                    aria_label: "Your name, to join a room",
                    value: "{join_room_name_text}",
                    oninput: move |evt| join_room_name_text.set(evt.value()),
                }
                input {
                    class: "room-number",
                    r#type: "text",
                    inputmode: "numeric",
                    placeholder: "Room #",
                    aria_label: "Room number",
                    value: "{join_room_number_text}",
                    oninput: move |evt| join_room_number_text.set(evt.value()),
                }
                button { class: "btn", r#type: "submit", "Join" }
            }
        }
    }
}

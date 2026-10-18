use dioxus::prelude::*;

use crate::ratings::{get_ratings, vote_sound};

#[component]
pub fn Votes(recording_id: ReadSignal<String>) -> Element {
    let ratings_resource = use_resource(move || async move { get_ratings(recording_id()).await });

    let mut user_voted_either_direction = use_signal(|| false);
    use_effect(move || {
        recording_id();
        user_voted_either_direction.set(false);
    });

    let mut num_upvotes = use_signal(|| 0);
    let mut num_downvotes = use_signal(|| 0);

    rsx! {
        div { class: "votes",
        match &*ratings_resource.read() {
            Some(Ok((upvotes, downvotes))) => {
                num_upvotes.set(*upvotes);
                num_downvotes.set(*downvotes);
                rsx! {
                    DirectionalVote {
                        recording_id,
                        num_votes: num_upvotes,
                        is_upvote: true,
                        user_voted_either_direction,
                    }
                    DirectionalVote {
                        recording_id,
                        num_votes: num_downvotes,
                        is_upvote: false,
                        user_voted_either_direction,
                    }
                }
            }
            Some(Err(_)) => rsx! {
                div { class: "vote-count", "Failed to fetch votes" }
            },
            None => rsx! {
                div { class: "vote-count", "Loading votes..." }
            },
        }
        }
    }
}

#[component]
fn DirectionalVote(
    recording_id: ReadSignal<String>,
    num_votes: ReadSignal<i64>,
    is_upvote: bool,
    mut user_voted_either_direction: Signal<bool>,
) -> Element {
    let mut user_voted_this_direction = use_signal(|| false);
    use_effect(move || {
        recording_id();
        user_voted_this_direction.set(false);
    });

    let vote_text = |num_votes: &i64| {
        let emoji = if is_upvote { "👍" } else { "👎" };
        let plus_one = if user_voted_this_direction() {
            "+1"
        } else {
            ""
        };
        format!("{emoji}({num_votes}{plus_one})")
    };

    let num_votes = &*num_votes.read();

    rsx! {
        if user_voted_either_direction() {
            div { class: "vote-count", "{vote_text(num_votes)}" }
        } else {
            button {
                class: "vote-btn",
                onclick: move |_| {
                    spawn(async move {
                        if vote_sound(recording_id(), is_upvote).await.is_ok() {
                            user_voted_this_direction.set(true);
                            user_voted_either_direction.set(true);
                        }
                    });
                },
                "{vote_text(num_votes)}"
            }
        }
    }
}

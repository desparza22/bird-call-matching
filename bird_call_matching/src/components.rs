//! UI components. Each component lives in its own submodule; the ones used elsewhere are
//! re-exported here so callers can write `components::Sounds` instead of
//! `components::sounds::Sounds`.

mod app;
mod game_board;
mod images;
mod layout;
mod leaderboard;
mod online;
mod redraw;
mod room_forms;
mod solo;
mod sounds;
mod title;
mod votes;

pub use app::App;
use game_board::GameBoard;
use images::Images;
use layout::GameLayout;
use leaderboard::{Leaderboard, LeaderboardEntry};
use online::OnlineGame;
use redraw::Redraw;
use room_forms::RoomForms;
use solo::SoloGame;
use sounds::Sounds;
use title::Title;
use votes::Votes;

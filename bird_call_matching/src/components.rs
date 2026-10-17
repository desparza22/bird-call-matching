//! UI components. Each component lives in its own submodule; the ones used elsewhere are
//! re-exported here so callers can write `components::Sounds` instead of
//! `components::sounds::Sounds`.

mod app;
mod images;
mod multiplayer;
mod redraw;
mod score;
mod sounds;
mod title;
mod votes;

pub use app::App;
use images::Images;
use multiplayer::Multiplayer;
use redraw::Redraw;
use score::Score;
use sounds::Sounds;
use title::Title;
use votes::Votes;

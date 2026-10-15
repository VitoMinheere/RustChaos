//! `Game`'s methods, by what they're about. The struct itself, `Game::new` and the frame
//! (`Game::update`) are in the crate root.

mod blast;
mod creatures;
mod setpieces;
mod people;
pub(crate) use people::Route;
mod cars;
mod mission;
mod scenes;
mod actions;
mod combat;
mod draw;
mod snipe;
mod mib;

pub use snipe::Snipe;
mod bikes;
mod choppers;
mod barrels;
pub mod mines;
pub mod grenades;
mod litter;

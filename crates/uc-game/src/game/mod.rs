//! `Game`'s methods, by what they're about. The struct itself, `Game::new` and the frame
//! (`Game::update`) are in the crate root.

mod blast;
mod creatures;
mod setpieces;
mod people;
pub(crate) use people::Routes;
mod cars;
pub(crate) use cars::Streets;
mod mission;
pub(crate) use mission::Tally;
mod scenes;
mod actions;
pub use actions::Aiming;
pub(crate) use actions::special_item;
mod combat;
mod draw;
mod snipe;
mod sights;
pub use sights::Sight;
mod mib;

pub use snipe::Snipe;
mod bikes;
mod choppers;
mod barrels;
pub mod mines;
pub mod grenades;
mod litter;
mod cheats;
pub mod endless;
pub mod beat;

//! Renderer-independent Urban Chaos world: loads a mission's level from an install and
//! produces chunked, lit geometry, posed and animated people, and the surfaces they walk on. Any renderer
//! (the wgpu one in `uc-render`, or another) can draw the result.

pub mod character;
pub mod crinkle;
pub mod geometry;
pub mod ground;
pub mod level;
pub mod mav;

pub use character::{Figure, Model, People, Play};
pub use crinkle::Relief;
pub use ground::Ground;
pub use geometry::{ghost_page, reflect_page, Batch, Chunk, Layer, Vertex, WorldGeometry, ADDITIVE_PAGES, DECAL_PAGES, EXPLODE_PAGE, GHOST, GLOW, MIST, MOON_PAGE, REFLECT, SHADOW, SKY_PAGE, UNIT, UNTEXTURED};
pub use level::{Level, LoadOptions};

#[cfg(feature = "ears")] extern crate ears;
extern crate image;

#[cfg(feature = "ears")] pub mod music;
pub mod converters;
pub mod playback;
pub mod resizer;

#[cfg(feature = "ears")] pub use music::*;
pub use playback::*;

#[cfg(feature = "failure")] #[macro_use] extern crate failure;
#[cfg(feature = "gst")] extern crate gstreamer as gst;
#[cfg(feature = "gst")] extern crate gstreamer_app as gst_app;
#[cfg(feature = "termion")] extern crate termion;
extern crate image;

/// The converters themselves
pub mod converters;
/// High-level interactive TUI
pub mod interactive;
/// Functions to help with resizing math, like keeping aspect ratio
pub mod resizer;
/// A struct to help with zooming
pub mod zoomer;

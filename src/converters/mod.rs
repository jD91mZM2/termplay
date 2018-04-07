pub mod truecolor;

pub use self::truecolor::*;

use image::{GenericImage, Pixel, Primitive};
use std::{
    fmt,
    io::{self, Write}
};

/// A trait that converts an image to something displayable in the terminal
pub trait Converter {
    /// Write an image to specified io stream
    fn display<W, I, P, S>(&self, fmt: &mut W, image: &I) -> Result<(), io::Error>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = S>,
              S: Primitive + fmt::Display;
    /// Write an image to bytes that can be displayed in the terminal
    fn to_vec<W, I, P, S>(&self, image: &I) -> Vec<u8>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = S>,
              S: Primitive + fmt::Display
    {
        let mut buf = Vec::new();
        self.display(&mut buf, image).unwrap();
        buf
    }
}

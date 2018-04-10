pub mod color256;
pub mod sixel;
pub mod truecolor;

pub use self::color256::*;
pub use self::sixel::*;
pub use self::truecolor::*;

use image::{GenericImage, Pixel};
use std::io::{self, Write};

/// A trait that converts an image to something displayable in the terminal
pub trait Converter {
    /// Write an image to specified io stream
    fn display<W, I, P>(&self, fmt: &mut W, image: &I) -> io::Result<()>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = u8>;
    /// Write an image to bytes that can be displayed in the terminal
    fn to_vec<W, I, P>(&self, image: &I) -> Vec<u8>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = u8>
    {
        let mut buf = Vec::new();
        self.display(&mut buf, image).unwrap();
        buf
    }
    /// Where is x/y in the terminal on the image?
    /// For example, TrueColor/Color256 would just return directly,
    /// because one character is one pixel.
    fn actual_pos(&self, x: u32, y: u32) -> (u32, u32) {
        (x, y)
    }
}

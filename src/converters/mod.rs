#[cfg(feature = "sixel")] pub mod sixel;
pub mod color256;
pub mod halfblock;
pub mod truecolor;

#[cfg(feature = "sixel")] pub use self::sixel::*;
pub use self::color256::*;
pub use self::halfblock::*;
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

#[derive(Clone, Copy)]
/// An enum with all built-in converter types,
/// because trait objects don't work with generics.
pub enum DynamicConverter {
    #[cfg(feature = "sixel")] Sixel,
    Color256,
    HalfBlock,
    TrueColor
}
impl Converter for DynamicConverter {
    fn display<W, I, P>(&self, fmt: &mut W, image: &I) -> io::Result<()>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = u8>
    {
        match *self {
            #[cfg(feature = "sixel")] DynamicConverter::Sixel => Sixel.display(fmt, image),
            DynamicConverter::Color256 => Color256.display(fmt, image),
            DynamicConverter::HalfBlock => HalfBlock.display(fmt, image),
            DynamicConverter::TrueColor => TrueColor.display(fmt, image),
        }
    }
    fn actual_pos(&self, x: u32, y: u32) -> (u32, u32) {
        match *self {
            #[cfg(feature = "sixel")] DynamicConverter::Sixel => Sixel.actual_pos(x, y),
            DynamicConverter::Color256 => Color256.actual_pos(x, y),
            DynamicConverter::HalfBlock => HalfBlock.actual_pos(x, y),
            DynamicConverter::TrueColor => TrueColor.actual_pos(x, y)
        }
    }
}

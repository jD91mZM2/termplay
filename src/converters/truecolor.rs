use image::{GenericImage, Pixel, Primitive};
use std::{
    fmt,
    io::{self, Write}
};

pub struct TrueColor;

impl super::Converter for TrueColor {
    fn display<W, I, P, S>(&self, fmt: &mut W, image: &I) -> Result<(), io::Error>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = S>,
              S: Primitive + fmt::Display
    {
        for y in 0..image.height() {
            for x in 0..image.width() {
                let pixel = image.get_pixel(x, y).to_rgb().data;
                write!(fmt, "\x1b[48;2;{};{};{}m ", pixel[0], pixel[1], pixel[2])?;
            }
            write!(fmt, "\x1b[0m\r\n")?;
        }
        Ok(())
    }
}

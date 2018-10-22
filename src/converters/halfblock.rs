use image::{GenericImage, Pixel};
use std::io::{self, Write};

#[derive(Clone, Copy, Debug)]
pub struct HalfBlock;

impl super::Converter for HalfBlock {
    fn display<W, I, P>(&self, fmt: &mut W, image: &I) -> io::Result<()>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = u8>
    {
        for y in 0..image.height()/2 {
            if y > 0 {
                write!(fmt, "\r\n")?;
            }
            let y = y*2;
            for x in 0..image.width() {
                let pixel = image.get_pixel(x, y).to_rgb().data;
                let lower = if y+1 < image.height() {
                    image.get_pixel(x, y+1).to_rgb().data
                } else {
                    [0, 0, 0]
                };
                write!(
                    fmt,
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    pixel[0], pixel[1], pixel[2],
                    lower[0], lower[1], lower[2]
                )?;
            }
            write!(fmt, "\x1b[0m")?;
        }
        Ok(())
    }
    fn actual_pos(&self, x: u32, y: u32) -> (u32, u32) {
        (x, y * 2)
    }
}

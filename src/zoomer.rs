use image::{DynamicImage, GenericImage};
use std::cmp::{min, max};

/// A struct that helps with zooming
#[derive(Debug)]
pub struct Zoomer {
    x: u16,
    y: u16,
    level: u8
}

impl Default for Zoomer {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            level: 100
        }
    }
}
impl Zoomer {
    pub fn new() -> Self { Self::default() }
    /// Set where on the image to zoom into
    pub fn set_pos(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
    }
    /// Set zoom level in percent (100% shows whole image)
    pub fn set_level(&mut self, level: u8) {
        self.level = min(100, max(1, level));
    }
    pub fn x(&self) -> u16 { self.x }
    pub fn y(&self) -> u16 { self.y }
    pub fn level(&self) -> u8 { self.level }

    /// Return the bounds to crop the image to.
    /// old_width/old_height are the original image bounds.
    /// new_width/new_height are what the image will be resized to after the zoom.
    /// These can be left the same as the old if no resize occurs.
    pub fn bounds(&self, old_width: u32, old_height: u32, new_width: u32, new_height: u32) -> (u32, u32, u32, u32) {
        let x = min(self.x as u32, new_width) * (old_width / new_width);
        let y = min(self.y as u32, new_height) * (old_height / new_height);

        let level = self.level as f64 / 100.0;
        let level_x = (level * old_width as f64) as u32;
        let level_y = (level * old_height as f64) as u32;

        let x = min(x.saturating_sub(level_x / 2), old_width.saturating_sub(level_x));
        let y = min(y.saturating_sub(level_y / 2), old_height.saturating_sub(level_y));
        (x, y, level_x, level_y)
    }

    /// Zoom the image. This should be done before any resize.
    /// new_width/new_height are what the image will be resized to after the zoom.
    /// These can be left the same as the old if no resize occurs.
    pub fn crop(&self, image: &mut DynamicImage, new_width: u32, new_height: u32) -> DynamicImage {
        let (x, y, width, height) = self.bounds(image.width(), image.height(), new_width, new_height);
        image.crop(x, y, width, height)
    }
}

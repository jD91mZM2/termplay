//! A struct to help with zooming

use converters::Converter;

use image::{DynamicImage, GenericImageView};
use std::cmp::{min, max};

/// A struct that helps with zooming
#[derive(Debug)]
pub struct Zoomer<C: Converter> {
    x: u16,
    y: u16,
    level: u8,
    drag_start: Option<(u16, u16)>,
    drag_move: Option<(u16, u16)>,
    converter: C
}

impl<C: Converter> Zoomer<C> {
    /// Create a new zoomer
    pub fn new(converter: C) -> Self {
        Self {
            x: 0,
            y: 0,
            level: 100,
            drag_start: None,
            drag_move: None,
            converter
        }
    }
    /// Set where on the image to zoom into
    pub fn set_pos(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
    }
    /// Set zoom level in percent (100% shows whole image)
    pub fn set_level(&mut self, level: u8) {
        self.level = min(100, max(1, level));
    }
    /// Start dragging from x and y
    pub fn drag_start(&mut self, x: u16, y: u16) {
        self.drag_start = Some((x, y));
    }
    /// Drag to x and y
    pub fn drag_move(&mut self, x: u16, y: u16) {
        if let Some((ref mut drag_x, ref mut drag_y)) = self.drag_start {
            self.x = (max(0, self.x as i32 + (*drag_x as i32 - x as i32))) as u16;
            self.y = (max(0, self.y as i32 + (*drag_y as i32 - y as i32))) as u16;
            *drag_x = x;
            *drag_y = y;
        }
    }
    /// Stop dragging
    pub fn drag_stop(&mut self) {
        self.drag_start = None;
    }
    pub fn pos(&self) -> (u16, u16) { (self.x, self.y) }
    pub fn level(&self) -> u8 { self.level }
    pub fn is_dragging(&self) -> bool { self.drag_start.is_some() }

    /// Return the bounds to crop the image to.
    /// old_width/old_height are the original image bounds.
    /// new_width/new_height are what the image will be resized to after the zoom.
    /// These can be left the same as the old if no resize occurs.
    pub fn bounds(&self, old_width: u32, old_height: u32, new_width: u32, new_height: u32) -> (u32, u32, u32, u32) {
        let (x, y) = self.converter.actual_pos(self.x as u32, self.y as u32);

        let x = (min(x as u32, new_width) as f64 * (old_width as f64 / new_width as f64)) as u32;
        let y = (min(y as u32, new_height) as f64 * (old_height as f64 / new_height as f64)) as u32;

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

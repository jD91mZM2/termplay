//! Functions to help with resizing math, like keeping aspect ratio

/// Calculate the maximum width/height that fits within new_width/new_height,
/// but still keeps the aspect ratio.
pub fn keep_aspect_ratio(old_width: u32, old_height: u32, mut new_width: u32, mut new_height: u32) -> (u32, u32) {
    let old_ratio = old_width as f64 / old_height as f64;
    let new_ratio = new_width as f64 / new_height as f64;

    if new_ratio > old_ratio {
        // width is too big
        new_width = (old_width as f64 * (new_height as f64 / old_height as f64)) as u32;
    } else {
        // height is too big
        new_height = (old_height as f64 * (new_width as f64 / old_width as f64)) as u32;
    }
    (new_width, new_height)
}
#[deprecated(since = "2.0.2", note = "use apply_font_ratio, which supports a max width")]
pub fn with_font_ratio(width: u32, ratio: u8) -> u32 {
    (width as f64 * (ratio as f64 / 100.0 + 1.0)) as u32
}
/// Expand width to better match with the font ratio, unless it becomes more than max_width,
/// which in case it instead shrinks the height.
pub fn apply_pixel_ratio(ratio: u8, width: u32, height: u32, max_width: u32) -> (u32, u32) {
    let ratio = 1.0 + ratio as f64 / 100.0;
    let new_width = (width as f64 * ratio) as u32;
    if new_width <= max_width {
        (new_width, height)
    } else {
        (width, (height as f64 * (1.0 / ratio)) as u32)
    }
}

/// Functions to calculate the destination size
pub trait Sizer {
    /// Return destination size from old width/height
    fn get_size(&self, old_width: u32, old_height: u32) -> (u32, u32);
}

/// An implementation for the Sizer trait which keeps aspect ratio
/// and optionally applies pixel ratio.
#[derive(Clone, Debug)]
pub struct StandardSizer {
    pub new_width: u32,
    pub new_height: u32,
    pub ratio: Option<u8>
}
impl Sizer for StandardSizer {
    fn get_size(&self, old_width: u32, old_height: u32) -> (u32, u32) {
        let (mut width, mut height) = keep_aspect_ratio(old_width, old_height, self.new_width, self.new_height);
        if let Some(ratio) = self.ratio {
            let (w, h) = apply_pixel_ratio(ratio, width, height, self.new_width);
            width = w;
            height = h;
        }
        (width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aspect_ratio() {
        assert_eq!(keep_aspect_ratio(2, 1, 4, 5), (4, 2));
        assert_eq!(keep_aspect_ratio(1, 2, 5, 4), (2, 4));

        assert_eq!(keep_aspect_ratio(1092, 614, 167, 40), (71, 40));
    }
    #[test]
    fn test_font_ratio() {
        assert_eq!(apply_pixel_ratio(50, 5, 3, 10), (7, 3));
        assert_eq!(apply_pixel_ratio(50, 5, 3, 5),  (5, 2));
    }
}

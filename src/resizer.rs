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
pub fn with_font_ratio(width: u32, height: u32, ratio: u8) -> (u32, u32) {
    if width < height {
        (width, (height as f64 * (ratio as f64 / 100.0 + 1.0)) as u32)
     } else {
         ((width as f64 * (ratio as f64 / 100.0 + 1.0)) as u32, height)
     }
}

#[cfg(test)]
#[test]
pub fn test_aspect_ratio() {
    assert_eq!(keep_aspect_ratio(2, 1, 4, 5), (4, 2));
    assert_eq!(keep_aspect_ratio(1, 2, 5, 4), (2, 4));

    assert_eq!(keep_aspect_ratio(1092, 614, 167, 40), (71, 40));
}

#[cfg(test)]
#[test]
pub fn test_font_ratio() {
    assert_eq!(with_font_ratio(2, 4, 50), (3, 4));
    assert_eq!(with_font_ratio(4, 2, 50), (4, 3));
}

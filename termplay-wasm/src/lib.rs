extern crate image;
extern crate termplay;

use image::{GenericImage, FilterType};
use termplay::{
    converters::{Converter, TrueColor},
    resizer
};
use std::{
    ffi::CString,
    os::raw::c_char,
    ptr
};

fn _image_to_string(input: &[u8]) -> String {
    let image = match image::load_from_memory(input) {
        Ok(image) => image,
        Err(err) => return format!("failed to load image: {}", err)
    };

    let (width, height) = resizer::keep_aspect_ratio(image.width(), image.height(), 80, 24);

    let image = image.resize_exact(width, height, FilterType::Nearest);
    let bytes = TrueColor.to_vec(&image);

    String::from_utf8(bytes)
        .unwrap_or_else(|err| format!("failed to parse utf8: {}", err))
}

#[no_mangle]
pub extern fn slice_new(len: usize) -> *mut u8 {
    let vec = vec![0u8; len].into_boxed_slice();
    Box::into_raw(vec) as *mut u8
}
#[no_mangle]
pub extern fn slice_set(slice: *mut u8, len: usize, index: usize, val: u8) {
    let slice = unsafe { std::slice::from_raw_parts_mut(slice, len) };
    slice[index] = val;
}
#[no_mangle]
pub extern fn image_to_string(slice: *mut u8, len: usize) -> *const c_char {
    let slice = unsafe { std::slice::from_raw_parts_mut(slice, len) };
    let input: Box<[u8]> = unsafe { Box::from_raw(slice) };

    let string = _image_to_string(&input);

    match CString::new(string) {
        Ok(string) => string.into_raw(),
        Err(_) => ptr::null()
    }
}
#[no_mangle]
pub extern fn free(input: *mut c_char) {
    unsafe {
        CString::from_raw(input);
    }
}

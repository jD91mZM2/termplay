use image::{GenericImage, Pixel};
use std::{
    io::{self, Write},
    mem,
    os::raw::{c_uchar, c_int, c_void},
    ptr,
    slice
};

type Alloc = *mut c_void;
type Dither = *mut c_void;
type Output = *mut c_void;
type Status = c_int;
type WriteFn = extern "C" fn(data: *mut c_uchar, len: c_int, userdata: *mut c_void) -> Status;

const SIXEL_BUILTIN_XTERM256: c_int = 3;

#[link(name = "sixel")]
extern "C" {
    fn sixel_output_new(output: *mut Output, write_fn: WriteFn, userdata: *mut c_void, alloc: Alloc) -> Status;
    fn sixel_dither_get(kind: c_int) -> Dither;
    fn sixel_encode(data: *mut c_uchar, width: c_int, height: c_int, _: c_int, dither: Dither, output: Output) -> Status;
}

#[derive(Clone, Copy, Debug)]
pub struct Sixel;

impl super::Converter for Sixel {
    fn display<W, I, P>(&self, fmt: &mut W, image: &I) -> io::Result<()>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = u8>
    {
        extern "C" fn write_fn(data: *mut c_uchar, len: c_int, userdata: *mut c_void) -> c_int {
            unsafe {
                let output: &mut Vec<u8> = mem::transmute(userdata);
                output.write_all(slice::from_raw_parts(data, len as usize)).unwrap();
                0
            }
        }

        let mut data = Vec::with_capacity(image.width() as usize * image.height() as usize * 3);
        for y in 0..image.height() {
            for x in 0..image.width() {
                let pixel = image.get_pixel(x, y).to_rgb();
                data.push(pixel[0]);
                data.push(pixel[1]);
                data.push(pixel[2]);
            }
        }

        let data: *mut c_uchar = unsafe { mem::transmute(data.as_mut_ptr()) };

        let mut output: Vec<u8> = Vec::new();
        let mut sixel_output = ptr::null_mut();

        if unsafe {
            sixel_output_new(
                &mut sixel_output,
                write_fn,
                mem::transmute(&mut output),
                ptr::null_mut()
            )
        } != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "sixel_output_new error"));
        }

        let sixel_dither = unsafe { sixel_dither_get(SIXEL_BUILTIN_XTERM256) };

        let result = unsafe {
            sixel_encode(data, image.width() as i32, image.height() as i32, 0, sixel_dither, sixel_output)
        };

        if result == 0 {
            write!(fmt, "{}", ::std::str::from_utf8(&output).unwrap())
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "sixel_encode error"));
        }
    }
    fn actual_pos(&self, x: u32, y: u32) -> (u32, u32) {
        (x * 10, y * 10)
    }
}

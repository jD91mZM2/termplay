#[cfg(feature = "termion")] extern crate termion;
#[macro_use] extern crate clap;
#[macro_use] extern crate failure;
extern crate image;
extern crate termplay;

#[cfg(feature = "gst")] use image::ImageError;
#[cfg(feature = "gst")] use std::{borrow::Cow, fs};
use clap::{Arg, App};
use failure::Error;
use image::{GenericImage, Pixel};
use std::{
    io::{self, Write},
    process
};
#[cfg(feature = "gst")] use termplay::interactive::VideoPlayer;
use termplay::{
    converters::*,
    interactive::ImageViewer,
    resizer::{Sizer, StandardSizer}
};

#[derive(Clone, Copy)]
pub enum ConverterType {
    Color256,
    HalfBlock,
    Sixel,
    TrueColor
}
impl Converter for ConverterType {
    fn display<W, I, P>(&self, fmt: &mut W, image: &I) -> io::Result<()>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = u8>
    {
        match *self {
            ConverterType::Color256 => Color256.display(fmt, image),
            ConverterType::HalfBlock => HalfBlock.display(fmt, image),
            ConverterType::Sixel => Sixel.display(fmt, image),
            ConverterType::TrueColor => TrueColor.display(fmt, image),
        }
    }
    fn actual_pos(&self, x: u32, y: u32) -> (u32, u32) {
        match *self {
            ConverterType::Color256 => Color256.actual_pos(x, y),
            ConverterType::HalfBlock => HalfBlock.actual_pos(x, y),
            ConverterType::Sixel => Sixel.actual_pos(x, y),
            ConverterType::TrueColor => TrueColor.actual_pos(x, y)
        }
    }
}

fn main() {
    let code = if let Err(err) = do_main() {
        eprintln!("{}", err);
        1
    } else { 0 };
    process::exit(code);
}
fn do_main() -> Result<(), Error> {
    let app =
        App::new(crate_name!())
            .version(crate_version!())
            .author(crate_authors!())
            .about(crate_description!())
            .arg(Arg::with_name("path")
                .help("Specifies the path to the image/video to play")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("width")
                .help("Sets the width (defaults to the terminal size, or 80)")
                .short("w")
                .long("width")
                .takes_value(true))
            .arg(Arg::with_name("height")
                .help("Sets the height (defaults to the terminal size, or 24)")
                .short("h")
                .long("height")
                .takes_value(true))
            .arg(Arg::with_name("ratio")
                .help("Sets the terminal font ratio (only takes effect with some converters)")
                .long("ratio")
                .takes_value(true))
            .arg(Arg::with_name("converter")
                .help("Decides how the image should be displayed")
                .short("c")
                .long("converter")
                .takes_value(true)
                .possible_values(&["color256", "halfblock", "sixel", "truecolor"])
                .default_value("truecolor"))
            .arg(Arg::with_name("rate")
                .help("Sets the framerate")
                .short("r")
                .long("rate")
                .takes_value(true)
                .default_value("24"));
    #[cfg(feature = "termion")]
    let app = app
        .arg(Arg::with_name("quiet")
            .help("Ignores all the nice TUI things for simple image viewing")
            .short("q")
            .long("quiet"));
    let options = app.get_matches();

    let path = options.value_of_os("path").unwrap();

    let converter = match options.value_of("converter").unwrap() {
        "color256"  => ConverterType::Color256,
        "halfblock"  => ConverterType::HalfBlock,
        "sixel"     => ConverterType::Sixel,
        "truecolor" => ConverterType::TrueColor,
        _ => unreachable!()
    };

    let ratio = value_t!(options, "ratio", u8).ok();
    if ratio == Some(0) {
        bail!("ratio can't be zero");
    }

    #[cfg(feature = "termion")]
    let (width, height) = termion::terminal_size().map(|(w, h)| (w as u32, h as u32)).unwrap_or((80, 24));
    #[cfg(not(feature = "termion"))]
    let (width, height) = (80, 24);

    let (mut width, mut height) = converter.actual_pos(width, height);

    if let Ok(w) = value_t!(options, "width", u32) {
        width = w;
    }
    if let Ok(h) = value_t!(options, "height", u32) {
        height = h;
    }

    let sizer = StandardSizer {
        new_width: width,
        new_height: height,
        ratio: ratio
    };

    let mut stdout = io::stdout();
    stdout.lock();
    #[cfg(feature = "termion")]
    let mut stdin = io::stdin();
    #[cfg(feature = "termion")]
    stdin.lock();

    match image::open(path) {
        Ok(image) => {
            #[cfg(feature = "termion")]
            let mut image = image;

            let (width, height) = sizer.get_size(image.width(), image.height());

            let viewer = ImageViewer {
                converter: converter,
                width: width,
                height: height
            };

            #[cfg(feature = "termion")]
            let quiet = options.is_present("quiet");
            #[cfg(not(feature = "termion"))]
            let quiet = true;

            if quiet {
                viewer.display_image_quiet(&mut stdout, &image).map_err(Error::from)
            } else {
                #[cfg(feature = "termion")] {
                    viewer.display_image(&mut stdin, &mut stdout, &mut image).map_err(Error::from)
                }
                #[cfg(not(feature = "termion"))]
                unreachable!();
            }
        },
        #[cfg(feature = "gst")]
        Err(ImageError::IoError(_)) |
        Err(ImageError::UnsupportedError(_)) => {
            // Image failed, but file does exist.
            // Is it a video? Let's assume yes until proven otherwise.
            // What could possibly go wrong ¯\_(ツ)_/¯

            let rate = value_t!(options, "rate", u8).unwrap_or_else(|e| e.exit());

            if rate == 0 {
                bail!("rate can't be zero");
            }

            let path_str = path.to_str();

            // this really needs to be improved
            let uri = if path_str.is_some() && path_str.unwrap().contains("://") {
                Cow::Borrowed(path_str.unwrap())
            } else {
                // can't create glib::Value from OsString
                let path = fs::canonicalize(path)?;
                let path = path.to_str()
                    .ok_or_else(|| format_err!("Unfortunately, non-utf8 paths are not supported. I'm sorry :("))?;

                let mut uri = String::with_capacity(7 + path.len());
                uri.push_str("file://");
                uri.push_str(path);
                Cow::Owned(uri)
            };

            let player = VideoPlayer {
                converter: converter,
                sizer: sizer,
                rate: rate
            };
            player.play_video(&mut stdin, stdout, &uri)
        },
        Err(err) => Err(err.into())
    }
}

#[cfg(feature = "termion")] extern crate termion;
#[macro_use] extern crate clap;
#[macro_use] extern crate failure;
extern crate image;
extern crate termplay;

#[cfg(feature = "gst")] use image::ImageError;
#[cfg(feature = "gst")] use std::{borrow::Cow, fs};
use clap::{Arg, App};
use failure::Error;
use image::GenericImage;
use std::io;
#[cfg(feature = "gst")] use termplay::interactive::VideoPlayer;
use termplay::{
    converters::*,
    interactive::ImageViewer,
    resizer::{Sizer, StandardSizer}
};

fn main() -> Result<(), Error> {
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
                .help("Sets the terminal font ratio")
                .long("ratio")
                .takes_value(true))
            .arg(Arg::with_name("converter")
                .help("Decides how the image should be displayed")
                .short("c")
                .long("converter")
                .takes_value(true)
                .possible_values(&["color256", "halfblock", "sixel", "truecolor"])
                .default_value("halfblock"))
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
        #[cfg(feature = "sixel")] "sixel" => DynamicConverter::Sixel,
        "color256"  => DynamicConverter::Color256,
        "halfblock"  => DynamicConverter::HalfBlock,
        "truecolor" => DynamicConverter::TrueColor,
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
    #[cfg(feature = "termion")]
    let stdin = io::stdin();
    #[cfg(feature = "termion")]
    let mut stdin = stdin.lock();

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

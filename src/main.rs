#[macro_use]
extern crate clap;
extern crate ctrlc;
extern crate ears;
extern crate image;
#[macro_use]
extern crate lazy_static;
extern crate sixel_sys;
extern crate tempdir;
extern crate termion;
extern crate time;

macro_rules! flush {
    () => {
        io::stdout().flush().unwrap();
    }
}
macro_rules! nullify {
    ($cmd:expr) => {
        {
            $cmd
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
        }
    }
}
macro_rules! make_parse_macro {
    ($options:expr) => {
        macro_rules! parse {
            ($name:expr, $type:ty) => {
                match $options.value_of($name) {
                    None => None,
                    Some(num) => Some(match num.parse::<$type>() {
                        Ok(num) => num,
                        Err(_) => {
                            eprintln!(concat!("Value of --", $name, " is not a valid number"));
                            return Err(());
                        },
                    }),
                };
            }
        }
    }
}
macro_rules! check_cmd {
    ($cmd:expr, $arg:expr) => {
        print!(concat!("Checking ", $cmd, "... "));
        flush!();

        if let Err(err) = nullify!(Command::new($cmd).arg($arg)).spawn() {
            println!("{}FAILED{}", COLOR_RED, COLOR_RESET);
            eprintln!(concat!($cmd, ": {}"), err);
            return Err(());
        } else {
            println!("{}SUCCESS{}", COLOR_GREEN, COLOR_RESET);
        }
    }
}

mod colors;
#[macro_use]
mod img;
mod preprocess;
mod video;
mod ytdl;

use clap::{App, Arg, SubCommand};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

lazy_static! {
    static ref EXIT: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

fn main() {
    let status = do_main();
    process::exit(status);
}
fn do_main() -> i32 {
    let exit_clone = EXIT.clone();
    ctrlc::set_handler(move || exit_clone.store(true, AtomicOrdering::Relaxed)).unwrap();

    let opt_width = Arg::with_name("width")
        .help("The max width of the frame")
        .long("width")
        .short("w")
        .takes_value(true)
        .display_order(1);
    let opt_height = Arg::with_name("height")
        .help("The max height of the frame")
        .long("height")
        .short("h")
        .takes_value(true)
        .display_order(2);
    let opt_ratio = Arg::with_name("ratio")
        .help(
            "Change frame pixel width/height ratio (may or may not do anything)"
        )
        .long("ratio")
        .takes_value(true)
        .default_value("0");
    let opt_keep_size = Arg::with_name("keep-size")
        .help("Keep the frame size. Overrides -w and -h")
        .long("keep-size")
        .short("k")
        .display_order(3);
    let opt_rate = Arg::with_name("rate")
        .help("The framerate of the video")
        .long("rate")
        .short("r")
        .takes_value(true)
        .default_value("10");
    let opt_converter = Arg::with_name("converter")
        .help("How to convert the frame to ANSI.")
        .long("converter")
        .takes_value(true)
        .possible_values(&["truecolor", "256-color", "sixel"])
        .default_value("truecolor");

    let options = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommand(
            SubCommand::with_name("image")
                .about("Convert a single image to text")
                .arg(
                    Arg::with_name("IMAGE")
                        .help("The image to convert")
                        .index(1)
                        .required(true)
                )
                .arg(opt_width.clone())
                .arg(opt_height.clone())
                .arg(opt_ratio.clone())
                .arg(opt_keep_size.clone())
                .arg(opt_converter.clone())
        )
        .subcommand(
            SubCommand::with_name("preprocess")
                .about("Pre-process a video to play in your terminal")
                .long_about(
                    "This subcommand generates a directory to be used in `video`.\n\
                    If you submit a directory to `video`, that means the video is pre-processed.\n\
                    A pre-processed video is faster to play because it doesn't need to run\n\
                    ffmpeg again.\n\
                    This will also return a number of frames.\n\
                    The amount of frames will be required to give\n\
                    the `video` subcommand as well."
                )
                .arg(
                    Arg::with_name("VIDEO")
                        .help("The video file path to pre-process")
                        .index(1)
                        .required(true)
                )
                .arg(
                    Arg::with_name("output")
                        .help("The output directory to create")
                        .long("output")
                        .short("o")
                        .default_value("termplay-video")
                )
                .arg(opt_width.clone())
                .arg(opt_height.clone())
                .arg(opt_ratio.clone())
                .arg(opt_keep_size.clone())
                .arg(opt_converter.clone())
                .arg(opt_rate.clone())
        )
        .subcommand(
            SubCommand::with_name("video")
                .about("Play a video in your terminal")
                .arg(
                    Arg::with_name("VIDEO")
                        .help("The video file path to play")
                        .index(1)
                        .required(true)
                )
                .arg(
                    Arg::with_name("FRAMES")
                        .help(
                            "The FRAMES parameter is the number of frames processed. \
                            It will be returned when you pre-process a video"
                        )
                        .index(2)
                )
                .arg(opt_width.clone())
                .arg(opt_height.clone())
                .arg(opt_ratio.clone())
                .arg(opt_keep_size.clone())
                .arg(opt_converter.clone())
                .arg(opt_rate.clone())
        )
        .subcommand(
            SubCommand::with_name("ytdl")
                .about("Play any video from youtube-dl")
                .arg(
                    Arg::with_name("VIDEO")
                        .help("The video URL to play")
                        .index(1)
                        .required(true)
                )
                .arg(
                    Arg::with_name("format")
                        .help("Pass format to youtube-dl.")
                        .long("format")
                        .short("f")
                        .default_value("worstvideo+bestaudio")
                )
                .arg(opt_width)
                .arg(opt_height)
                .arg(opt_ratio)
                .arg(opt_keep_size)
                .arg(opt_converter)
                .arg(opt_rate)
        )
        .get_matches();

    match options.subcommand() {
        ("image", Some(options)) => img::main(options),
        ("preprocess", Some(options)) => preprocess::main(options),
        ("video", Some(options)) => video::main(options),
        ("ytdl", Some(options)) => ytdl::main(options),
        (..) => {
            eprintln!("No subcommand selected");
            eprintln!("Start with --help for help.");
            Err(())
        },
    }.map(|_| 0).unwrap_or(1)
}
fn allow_exit() -> Result<(), ()> {
    if ::EXIT.load(AtomicOrdering::Relaxed) {
        return Err(())
    }
    Ok(())
}

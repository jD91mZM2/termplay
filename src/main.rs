#[macro_use]
extern crate clap;
extern crate ctrlc;
extern crate ears;
extern crate image;
#[macro_use]
extern crate lazy_static;
extern crate tempdir;
extern crate terminal_size;
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
macro_rules! stderr {
	($fmt:expr) => {
		writeln!(io::stderr(), $fmt).unwrap();
	};
	($fmt:expr, $($arg:tt)*) => {
		writeln!(io::stderr(), $fmt, $($arg)*).unwrap();
	}
}
macro_rules! check_cmd {
	($cmd:expr, $arg:expr) => {
		print!(concat!("Checking ", $cmd, "... "));
		flush!();

		if let Err(err) = nullify!(Command::new($cmd).arg($arg)).spawn() {
			println!("{}FAILED{}", COLOR_RED, COLOR_RESET);
			stderr!(concat!($cmd, ": {}"), err);
			return 1;
		} else {
			println!("{}SUCCESS{}", COLOR_GREEN, COLOR_RESET);
		}
	}
}
macro_rules! allowexit {
	() => {
		if ::EXIT.load(atomic::Ordering::Relaxed) {
			print!("{}{}", CURSOR_SHOW, ALTERNATE_OFF);
			return 0;
		}
	}
}
macro_rules! make_parse_macro {
	($options:expr) => {
		macro_rules! parse {
			($name:expr, $type:tt) => {
				match $options.value_of($name) {
					None => None,
					Some(num) => Some(match num.parse::<$type>() {
						Ok(num) => num,
						Err(_) => {
							stderr!(concat!("--", $name, " is not a valid number"));
							return 1;
						},
					}),
				};
			}
		}
	}
}

mod colors;
mod img;
mod video;
mod ytdl;

use clap::{App, Arg, SubCommand};
use std::io;
use std::io::Write;
use std::process;
use std::sync::Arc;
use std::sync::atomic;

lazy_static! {
	static ref EXIT: Arc<atomic::AtomicBool> = Arc::new(atomic::AtomicBool::new(false));
}

fn main() {
	let status = do_main();
	process::exit(status);
}
fn do_main() -> i32 {
	let exit_clone = EXIT.clone();
	ctrlc::set_handler(move || exit_clone.store(true, atomic::Ordering::Relaxed)).unwrap();

	let opt_width = Arg::with_name("width")
		.help("The max width of the video")
		.long("width")
		.short("w")
		.takes_value(true)
		.display_order(1);
	let opt_height = Arg::with_name("height")
		.help("The max height of the video")
		.long("height")
		.short("h")
		.takes_value(true)
		.display_order(2);
	let opt_keep_size = Arg::with_name("keep-size")
		.help("Keep the video size. Overrides -w and -h")
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
		.help("How to convert the video.")
		.long("converter")
		.takes_value(true)
		.possible_values(&["truecolor", "256-color"])
		.default_value("truecolor");

	let options = App::new(crate_name!())
		.version(crate_version!())
		.author(crate_authors!())
		.about(crate_description!())
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
				.arg(opt_width.clone())
				.arg(opt_height.clone())
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
				.arg(opt_width.clone())
				.arg(opt_height.clone())
				.arg(opt_keep_size.clone())
				.arg(opt_converter.clone())
				.arg(opt_rate)
		)
		.subcommand(
			SubCommand::with_name("image")
				.about("Convert a single image to text")
				.arg(
					Arg::with_name("IMAGE")
						.help("The image to convert")
						.index(1)
						.required(true)
				)
				.arg(opt_width)
				.arg(opt_height)
				.arg(opt_keep_size)
				.arg(opt_converter)
		)
		.get_matches();

	match options.subcommand() {
		("ytdl", Some(options)) => ytdl::main(options),
		("video", Some(options)) => video::main(options),
		("image", Some(options)) => img::main(options),
		(..) => {
			stderr!("No subcommand selected");
			stderr!("Start with --help for help.");
			1
		},
	}
}

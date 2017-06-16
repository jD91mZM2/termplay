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

mod img;

use clap::{App, Arg};
use ears::{AudioController, Music};
use image::ImageFormat;
use std::cmp;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::process;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic;
use std::thread;
use std::time::Duration;
use tempdir::TempDir;

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

// Let's not care if the TERM variable is 'dumb'
// since we depend on escape sequences
const ALTERNATE_ON: &str = "\x1b[?1049h";
const ALTERNATE_OFF: &str = "\x1b[?1049l";
const COLOR_RESET: &str = "\x1b[0;0m";

const COLOR_RED: &str = "\x1b[0;31m";
const COLOR_GREEN: &str = "\x1b[0;32m";

enum Converter {
	TrueColor,
	Color256
}

fn main() {
	let status = do_main();
	process::exit(status);
}
fn do_main() -> i32 {
	let exit = Arc::new(atomic::AtomicBool::new(false));
	let exit_clone = exit.clone();
	ctrlc::set_handler(move || exit_clone.store(true, atomic::Ordering::Relaxed)).unwrap();

	macro_rules! allowexit {
		() => {
			if exit.load(atomic::Ordering::Relaxed) {
				return 0;
			}
		}
	}

	let options = App::new(crate_name!())
		.version(crate_version!())
		.author(crate_authors!())
		.about(crate_description!())
		.arg(
			Arg::with_name("VIDEO")
				.help("The video URL to play")
				.index(1)
				.required(true)
		)
		.arg(
			Arg::with_name("width")
				.help("The max width of the video")
				.long("width")
				.short("w")
				.takes_value(true)
				.display_order(1)
		)
		.arg(
			Arg::with_name("height")
				.help("The max height of the video")
				.long("height")
				.short("h")
				.takes_value(true)
				.display_order(2)
		)
		.arg(
			Arg::with_name("rate")
				.help("The framerate of the video")
				.long("rate")
				.short("r")
				.takes_value(true)
				.default_value("10")
		)
		.arg(
			Arg::with_name("converter")
				.help("How to convert the video.")
				.long("converter")
				.takes_value(true)
				.possible_values(&["truecolor", "256-color"])
				.default_value("truecolor")
		)
		.get_matches();

	let video_link = options.value_of("VIDEO").unwrap();

	macro_rules! parse {
		($name:expr, $type:tt) => {
			match options.value_of($name) {
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
	let width = parse!("width", u16);
	let height = parse!("height", u16);
	let rate = parse!("rate", u8).unwrap();
	let converter = match options.value_of("converter").unwrap() {
		"truecolor" => Converter::TrueColor,
		"256-color" => Converter::Color256,
		_ => unreachable!(),
	};

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
	check_cmd!("youtube-dl", "--version");
	check_cmd!("ffmpeg", "-version");

	println!();
	allowexit!();
	println!("Creating directory...");

	let dir = match TempDir::new("play-youtube") {
		Ok(dir) => dir,
		Err(err) => {
			println!("{}", err);
			return 1;
		},
	};
	let dir_path = dir.path();

	allowexit!();
	println!("Downloading video... {}", ALTERNATE_ON);

	match Command::new("youtube-dl")
	          .current_dir(dir_path)
	          .arg(video_link)
	          .status() {
		Ok(status) => {
			if !status.success() {
				println!("");
				return status.code().unwrap_or_default();
			}
		},
		Err(err) => {
			println!("{}", ALTERNATE_OFF);
			stderr!("youtube-dl: {}", err);
			return 1;
		},
	}

	println!("{}", ALTERNATE_OFF);
	allowexit!();
	println!("Finding newly created file...");

	let mut files = match fs::read_dir(dir_path) {
		Ok(files) => files,
		Err(err) => {
			stderr!("Could not read directory: {}", err);
			return 1;
		},
	};
	let video_file = match files.next() {
		Some(video_file) => {
			match video_file {
				Ok(video_file) => video_file,
				Err(err) => {
					stderr!("Could not access file: {}", err);
					return 1;
				},
			}
		},
		None => {
			stderr!("No file found. Deleted?");
			return 1;
		},
	};
	let video_path = video_file.path();
	if files.next().is_some() {
		stderr!("Warning: Could not safely assume file, multiple files exist");
	}

	allowexit!();
	println!("Starting conversion: Video -> Image...");

	let mut ffmpeg = match nullify!(
		Command::new("ffmpeg")
			.current_dir(dir_path)
			.arg("-i")
			.arg(&video_path)
			.arg("-r")
			.arg(rate.to_string())
			.arg("frame%d.png")
	).spawn() {
		Ok(ffmpeg) => ffmpeg,
		Err(err) => {
			stderr!("ffmpeg: {}", err);
			return 1;
		},
	};
	thread::sleep(Duration::from_secs(1));

	println!("Started new process.");
	println!("Converting: Image -> Text");

	let mut i = 1;
	let mut retries = 0;

	macro_rules! wait_for_ffmpeg {
		($err:expr) => {
			match ffmpeg.try_wait() {
				Ok(None) => {
					if retries >= 3 {
						ffmpeg.kill().ok(); // Only fails if it's closed. Shouldn't happen, but don't really care.
						stderr!("I have tried 3 times, still can't read the file.");
						stderr!("Did ffmpeg hang? Are you trolling me?");
						stderr!("I give up. Error: {}", $err);
						return 1;
					}
					retries += 1;
					println!("Failed. Retrying...");
					thread::sleep(Duration::from_secs(3));
					continue;
				},
				Ok(_) => {
					println!("Seems like we have reached the end");
					break;
				},
				Err(err) => {
					stderr!("Error trying to get running status: {}", err);
					return 1;
				},
			}
		}
	}

	loop {
		allowexit!();

		let s = i.to_string();
		let mut name = String::with_capacity(5 + s.len() + 4);
		name.push_str("frame");
		name.push_str(s.as_str());
		name.push_str(".png");

		print!("\rProcessing {}", name);
		flush!();
		let mut file = match OpenOptions::new().read(true).write(true).open(
			dir_path.join(name)
		) {
			Ok(file) => file,
			Err(err) => {
				println!();
				wait_for_ffmpeg!(err);
			},
		};

		let mut image = match image::load(BufReader::new(&mut file), ImageFormat::PNG) {
			Ok(image) => {
				retries = 0;
				i += 1;
				image
			},
			Err(err) => {
				println!();
				wait_for_ffmpeg!(err);
			},
		};
		image = img::fit(&image, width, height);
		let bytes = match converter {
			Converter::TrueColor => img::convert_true(&image),
			Converter::Color256 => img::convert_256(&image),
		}.into_bytes();

		// Previously reading has moved our cursor.
		// Let's move it back!
		if let Err(err) = file.seek(SeekFrom::Start(0)) {
			stderr!("Failed to seek to beginning of file: {}", err);
			return 1;
		}
		if let Err(err) = file.write_all(&bytes) {
			stderr!("Failed to write to file: {}", err);
			return 1;
		}
		if let Err(err) = file.set_len(bytes.len() as u64) {
			stderr!("Failed to trim. Error: {}", err);
			return 1;
		}
	}

	allowexit!();
	println!("Converting: Video -> Music {}", ALTERNATE_ON);

	if let Err(err) = Command::new("ffmpeg")
	       .current_dir(&dir_path)
	       .arg("-i")
	       .arg(&video_path)
	       .arg("sound.wav")
	       .status() {
		println!("{}", ALTERNATE_OFF);
		stderr!("Couldn't convert to audio. Error: {}", err);
		return 1;
	}
	println!("{}", ALTERNATE_OFF);

	let mut music = match Music::new(&dir_path.join("sound.wav").to_string_lossy()) {
		Some(music) => music,
		None => {
			stderr!("Couldn't open music file");
			return 1;
		},
	};

	println!("Ready to play. Press enter when you are... ");

	if let Err(err) = io::stdin().read_line(&mut String::new()) {
		stderr!("Failed to wait for user input!");
		stderr!("{}", err);
		stderr!("Starting anyways... I guess");
	}

	music.play();

	let optimal = 1_000_000_000 / rate as i64;
	let mut lag: i64 = 0;
	for i in 1..i - 1 {
		allowexit!();

		if lag < -optimal {
			lag += optimal;
			continue;
		}

		let start = time::precise_time_ns();

		let s = i.to_string();
		let mut name = String::with_capacity(5 + s.len() + 4);
		name.push_str("frame");
		name.push_str(s.as_str());
		name.push_str(".png");

		let mut file = match File::open(dir_path.join(name)) {
			Ok(file) => file,
			Err(err) => {
				stderr!("Failed to open file. Error: {}", err);
				return 1;
			},
		};

		// thread::sleep(Duration::from_millis(1000)); // Simulate lag

		let mut frame = String::new();
		if let Err(err) = file.read_to_string(&mut frame) {
			stderr!("Error reading file: {}", err);
			return 1;
		}

		println!("{}", frame);

		let elapsed = time::precise_time_ns() - start;
		let mut sleep = optimal - elapsed as i64;

		if lag < 0 {
			sleep += lag;
			lag = 0;
		}

		match sleep.cmp(&0) {
			cmp::Ordering::Greater => thread::sleep(Duration::new(0, sleep as u32)),
			cmp::Ordering::Equal => {},
			cmp::Ordering::Less => lag += sleep as i64,
		}
	}

	0
}

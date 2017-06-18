use clap::ArgMatches;
use colors::*;
use ears::{AudioController, Music};
use image;
use image::ImageFormat;
use img;
use std::cmp;
use std::env;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic;
use std::thread;
use std::time::Duration;
use tempdir::TempDir;
use time;

pub fn main(options: &ArgMatches, exit: Arc<atomic::AtomicBool>) -> i32 {
	let mut video_path = match env::current_dir() {
		Ok(path) => path,
		Err(err) => {
			stderr!("Could not get current directory");
			return 1;
		},
	};
	video_path.push(options.value_of("VIDEO").unwrap());

	if !video_path.exists() {
		stderr!("Video does not exist.");
		return 1;
	}

	make_allowexit_macro!(exit);
	make_parse_macro!(options);
	let width = parse!("width", u16);
	let height = parse!("height", u16);
	let rate = parse!("rate", u8).unwrap();
	let converter = options.value_of("converter").unwrap();

	check_cmd!("ffmpeg", "-version");

	println!();
	allowexit!();
	println!("Creating directory...");

	let dir = match TempDir::new("termplay") {
		Ok(dir) => dir,
		Err(err) => {
			println!("{}", err);
			return 1;
		},
	};
	let dir_path = dir.path();

	allowexit!();
	play(&video_path, dir_path, width, height, rate, converter, exit)
}
pub fn play(video_path: &Path, dir_path: &Path, width: Option<u16>, height: Option<u16>, rate: u8, converter: &str, exit: Arc<atomic::AtomicBool>) -> i32 {
	make_allowexit_macro!(exit);
	println!("Starting conversion: Video -> Image...");

	let mut ffmpeg = match nullify!(
		Command::new("ffmpeg")
			.current_dir(dir_path)
			.arg("-i")
			.arg(video_path)
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
				Ok(Some(i)) => {
					if !i.success() {
						stderr!("ffmpeg ended unsuccessfully.");
						stderr!("Exit code: {}", i.code().unwrap_or_default());
						return 1;
					}
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
		let bytes = img::convert(&image, converter).into_bytes();

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
	       .arg(video_path)
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

use clap::ArgMatches;
use colors::*;
use ears::{AudioController, Music};
use preprocess;
use std::cmp;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::thread;
use std::time::Duration;
use tempdir::TempDir;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use time;

pub fn main(options: &ArgMatches) -> i32 {
	let mut video_path = match env::current_dir() {
		Ok(path) => path,
		Err(_) => {
			stderr!("Could not get current directory");
			return 1;
		},
	};
	video_path.push(options.value_of("VIDEO").unwrap());

	if !video_path.exists() {
		stderr!("Video does not exist.");
		return 1;
	}
	let frames_param = options.value_of("FRAMES");
	make_parse_macro!(options);
	let width = parse!("width", u16);
	let height = parse!("height", u16);
	let ratio = parse!("ratio", u8).unwrap();
	let keep_size = options.is_present("keep-size");
	let rate = parse!("rate", u8).unwrap();
	let converter = options.value_of("converter").unwrap().parse().unwrap();

	if frames_param.is_none() && video_path.is_dir() {
		stderr!("Video is a directory (assuming pre-processed), but FRAMES isn't set.");
		stderr!("Run `termplay preprocess --help` for more info.");
		return 1;
	}
	if frames_param.is_some() && video_path.is_file() {
		stderr!("Warning: No reason to specify FRAMES. Video isn't pre-processed");
	}
	let mut frames = 0;

	let _tempdir;
	let dir_path: &Path;
	if video_path.is_file() {
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

		_tempdir = dir;
		dir_path = _tempdir.path();

		allowexit!();

		let result = preprocess::process(
			&mut frames,
			&preprocess::ProcessArgs {
				video_path: &video_path,
				dir_path: dir_path,
				width: width,
				height: height,
				ratio: ratio,
				keep_size: keep_size,
				rate: rate,
				converter: converter
			}
		);
		if result != 0 {
			return result;
		}
	} else {
		frames = match frames_param.unwrap().parse() {
			Ok(num) => num,
			Err(_) => {
				stderr!("FRAMES is not a valid number.");
				return 1;
			},
		};
		dir_path = &video_path;
	}

	play(dir_path, frames, rate)
}
pub fn play(dir_path: &Path, frames: u32, rate: u8) -> i32 {
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

	print!("{}{}", ALTERNATE_ON, CURSOR_HIDE);
	macro_rules! onexit {
		() => {
			print!("{}{}", CURSOR_SHOW, ALTERNATE_OFF)
		}
	}

	let raw = io::stdout().into_raw_mode();

	macro_rules! make_switch {
		($name:tt, $name_clone:tt) => {
			{
				let $name = Arc::new(AtomicBool::new(false));
				let $name_clone = $name.clone();
				($name, $name_clone)
			}
		}
	}
	macro_rules! toggle_switch {
		($name_clone:tt, $value:expr) => {
			$name_clone.store(
				$value,
				AtomicOrdering::Relaxed
			);
		}
	}

	let (higher, higher_clone) = make_switch!(lower, lower_clone);
	let (lower, lower_clone) = make_switch!(lower, lower_clone);
	let (pause, pause_clone) = make_switch!(pause, pause_clone);

	if raw.is_ok() {
		thread::spawn(move || for event in io::stdin().events() {
			let event = match event {
				Ok(event) => event,
				Err(_) => continue,
			};

			match event {
				Event::Key(Key::Char(' ')) => toggle_switch!(pause_clone, !pause_clone.load(AtomicOrdering::Relaxed)),
				Event::Key(Key::Up) => toggle_switch!(higher_clone, true),
				Event::Key(Key::Down) => toggle_switch!(lower_clone, true),
				Event::Key(Key::Ctrl('c')) => {
					::EXIT.store(true, AtomicOrdering::Relaxed);
				},
				_ => {},
			}
		});
	}

	music.play();

	let optimal = 1_000_000_000 / rate as i64;
	let mut lag: i64 = 0;

	let mut volume = 100;

	let max_show_volume = 3_000_000_000 / optimal; // 3 seconds
	let mut show_volume = -1;

	let mut i = 0;
	while i < frames {
		macro_rules! handle_volume {
			() => {
				if higher.load(AtomicOrdering::Relaxed) {
					higher.store(false, AtomicOrdering::Relaxed);

					volume = cmp::min(volume + 10, 100);
					music.set_volume(volume as f32 / 100.0);

					show_volume = max_show_volume;
				} else if lower.load(AtomicOrdering::Relaxed) {
					lower.store(false, AtomicOrdering::Relaxed);

					volume = cmp::max(volume - 10, 0);
					music.set_volume(volume as f32 / 100.0);

					show_volume = max_show_volume;
				}
			}
		}

		handle_volume!();
		if pause.load(AtomicOrdering::Relaxed) {
			music.pause();

			let duration = Duration::from_millis(50);
			while pause.load(AtomicOrdering::Relaxed) && !::EXIT.load(AtomicOrdering::Relaxed) {
				thread::sleep(duration);
				handle_volume!();
				print!("\r{}% ", volume);
				flush!();
			}
			print!("\r    ");

			music.play();
		}
		allowexit!({
			onexit!();
		});

		i += 1;

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
				onexit!();
				flush!();
				stderr!("Failed to open file.");
				stderr!("{}", err);
				return 1;
			},
		};

		// thread::sleep(Duration::from_millis(1000)); // Simulate lag

		let mut frame = String::new();
		if let Err(err) = file.read_to_string(&mut frame) {
			stderr!("Error reading file: {}", err);
			return 1;
		}

		print!("{}{}\r", CURSOR_TOP_LEFT, frame);
		if show_volume > 0 {
			show_volume -= 1;
			// We never clear the previous value.
			// Therefor the trailing space is necessary.
			print!("{}% ", volume);
			flush!();
		} else if show_volume == 0 {
			show_volume -= 1;

			print!("    ");
			flush!();
		}

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

	onexit!();
	0
}

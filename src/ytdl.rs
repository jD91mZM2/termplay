use clap::ArgMatches;
use colors::*;
use preprocess;
use std::fs;
use std::io;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic;
use tempdir::TempDir;
use video;

pub fn main(options: &ArgMatches) -> i32 {
	let video_link = options.value_of("VIDEO").unwrap();

	make_parse_macro!(options);
	let width = parse!("width", u16);
	let height = parse!("height", u16);
	let ratio = parse!("ratio", u8).unwrap();
	let keep_size = options.is_present("keep-size");
	let rate = parse!("rate", u8).unwrap();
	let converter = options.value_of("converter").unwrap().parse().unwrap();
	let format = options.value_of("format").unwrap();

	check_cmd!("youtube-dl", "--version");
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
	println!("Downloading video... {}", ALTERNATE_ON);

	match Command::new("youtube-dl")
		.current_dir(dir_path)
		.arg(video_link)
		.arg("--format")
		.arg(format)
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
	let mut frames = 0;
	let result = preprocess::process(
		&mut frames,
		&video_path,
		dir_path,
		width,
		height,
		ratio,
		keep_size,
		rate,
		converter
	);
	if result != 0 {
		return result;
	}

	video::play(dir_path, frames, rate)
}

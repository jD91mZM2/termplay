use clap::ArgMatches;
use colors::*;
use image;
use image::{FilterType, ImageFormat};
use img;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::{BufReader, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::thread;
use std::time::Duration;

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

	make_parse_macro!(options);
	let width = parse!("width", u16);
	let height = parse!("height", u16);
	let ratio = parse!("ratio", u8).unwrap();
	let keep_size = options.is_present("keep-size");
	let rate = parse!("rate", u8).unwrap();
	let converter = options.value_of("converter").unwrap().parse().unwrap();
	let output = options.value_of("output").unwrap();

	check_cmd!("ffmpeg", "-version");

	println!();
	allowexit!();
	println!("Creating directory...");
	if let Err(err) = fs::create_dir(output) {
		stderr!("Could not create directory!");
		stderr!("{}", err);
		return 1;
	}

	allowexit!();

	let mut frames = 0;
	let result = process(
		&mut frames,
		&ProcessArgs {
			video_path: &video_path,
			dir_path: Path::new(output),
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

	println!("Number of frames: {}", frames);
	0
}
pub struct ProcessArgs<'a> {
	pub video_path: &'a Path,
	pub dir_path: &'a Path,
	pub width: Option<u16>,
	pub height: Option<u16>,
	pub ratio: u8,
	pub keep_size: bool,
	pub rate: u8,
	pub converter: img::Converter
}
pub fn process(frames: &mut u32, args: &ProcessArgs) -> i32 {
	println!("Starting conversion: Video -> Image...");

	let mut ffmpeg = match nullify!(
		Command::new("ffmpeg")
			.current_dir(args.dir_path)
			.arg("-i")
			.arg(args.video_path)
			.arg("-r")
			.arg(args.rate.to_string())
			.arg("frame%d.png")
	).spawn() {
		Ok(ffmpeg) => ffmpeg,
		Err(err) => {
			stderr!("ffmpeg: {}", err);
			return 1;
		},
	};
	macro_rules! onexit {
		() => {
			let _ = ffmpeg.kill();
		}
	}
	thread::sleep(Duration::from_secs(1));

	println!("Started new process.");
	allowexit!({
		onexit!();
	});
	println!("Converting: Image -> Text");

	let mut i = 1;
	let mut retries = 0;

	macro_rules! wait_for_ffmpeg {
		($err:expr) => {
			match ffmpeg.try_wait() {
				Ok(None) => {
					if retries >= 3 {
						let _ = ffmpeg.kill();
						stderr!("I have tried 3 times, still can't read the file.");
						stderr!("Did ffmpeg hang? Are you trolling me by deleting files?");
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
						return i.code().unwrap_or_default();
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

	let (width, height) = img::find_size(args.converter, args.width, args.height, args.ratio);

	loop {
		allowexit!({
			onexit!();
		});

		let s = i.to_string();
		let mut name = String::with_capacity(5 + s.len() + 4);
		name.push_str("frame");
		name.push_str(s.as_str());
		name.push_str(".png");

		print!("\rProcessing {}", name);
		flush!();
		let mut file = match OpenOptions::new().read(true).write(true).open(
			args.dir_path
				.join(name)
		) {
			Ok(file) => file,
			Err(err) => {
				println!();
				wait_for_ffmpeg!(err);
			},
		};

		let image = match image::load(BufReader::new(&mut file), ImageFormat::PNG) {
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
		let bytes = scale_and_convert!(
			image,
			args.converter,
			width,
			height,
			args.ratio,
			args.keep_size
		).into_bytes();

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

	println!("Waiting for process to finish...");
	if let Ok(code) = ffmpeg.wait() {
		if !code.success() {
			println!("ffmpeg ended unsuccessfully.");
			return code.code().unwrap_or_default();
		}
	}

	allowexit!();
	println!("Converting: Video -> Music {}", ALTERNATE_ON);

	if let Err(err) = Command::new("ffmpeg")
		.current_dir(&args.dir_path)
		.arg("-i")
		.arg(args.video_path)
		.arg("sound.wav")
		.status()
	{
		println!("{}", ALTERNATE_OFF);
		stderr!("ffmpeg: {}", err);
		return 1;
	}
	println!("{}", ALTERNATE_OFF);

	*frames = i - 1;
	0
}

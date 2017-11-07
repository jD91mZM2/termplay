use allow_exit;
use clap::ArgMatches;
use colors::*;
use preprocess;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use tempdir::TempDir;
use video;

pub fn main(options: &ArgMatches) -> Result<(), ()> {
    let video_link = options.value_of("VIDEO").unwrap();

    make_parse_macro!(options);
    let converter = options.value_of("converter").unwrap().parse().unwrap();
    let format = options.value_of("format").unwrap();
    let height = parse!("height", u16);
    let keep_size = options.is_present("keep-size");
    let rate = parse!("rate", u8).unwrap();
    let ratio = parse!("ratio", u8, may be zero).unwrap();
    let width = parse!("width", u16);

    check_cmd!("youtube-dl", "--version");
    check_cmd!("ffmpeg", "-version");

    println!();
    allow_exit()?;
    println!("Creating directory...");

    let dir = TempDir::new("termplay").map_err(|err| {
        eprintln!("{}", err);
    })?;
    let dir_path = dir.path();

    allow_exit()?;
    println!("Downloading video... {}", ALTERNATE_ON);

    match Command::new("youtube-dl")
        .current_dir(dir_path)
        .arg(video_link)
        .arg("--format")
        .arg(format)
        .status() {
        Ok(status) => {
            if !status.success() {
                println!("{}", ALTERNATE_OFF);
                return Err(());
            }
        },
        Err(err) => {
            println!("{}", ALTERNATE_OFF);
            eprintln!("youtube-dl: {}", err);
            return Err(());
        },
    }

    println!("{}", ALTERNATE_OFF);
    allow_exit()?;
    println!("Finding newly created file...");

    let mut files = fs::read_dir(dir_path).map_err(|err| {
        eprintln!("Could not read directory: {}", err);
    })?;
    let video_file = match files.next() {
        Some(video_file) => {
            video_file.map_err(|err| {
                eprintln!("Could not access file: {}", err);
            })?
        },
        None => {
            eprintln!("No file found. Deleted?");
            return Err(());
        }
    };
    let video_path = video_file.path();
    if files.next().is_some() {
        eprintln!("Warning: Could not safely assume file, multiple files exist");
    }

    allow_exit()?;
    let mut frames = 0;
    preprocess::process(
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
    )?;

    video::play(dir_path, frames, rate)
}

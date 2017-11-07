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
    allow_exit()?;
    println!("Creating directory...");

    let dir = match TempDir::new("termplay") {
        Ok(dir) => dir,
        Err(err) => {
            println!("{}", err);
            return Err(());
        },
    };
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

    let mut files = match fs::read_dir(dir_path) {
        Ok(files) => files,
        Err(err) => {
            eprintln!("Could not read directory: {}", err);
            return Err(());
        },
    };
    let video_file = match files.next() {
        Some(video_file) => {
            match video_file {
                Ok(video_file) => video_file,
                Err(err) => {
                    eprintln!("Could not access file: {}", err);
                    return Err(());
                },
            }
        },
        None => {
            eprintln!("No file found. Deleted?");
            return Err(());
        },
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

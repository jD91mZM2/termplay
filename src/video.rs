use allow_exit;
use clap::ArgMatches;
use colors::*;
#[cfg(feature = "ears")]
use ears::{AudioController, Music};
use preprocess;
use std::cmp;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
#[cfg(feature = "ears")]
use std::sync::Arc;
#[cfg(feature = "ears")]
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::thread;
use std::time::Duration;
use tempdir::TempDir;
#[cfg(feature = "ears")]
use termion::event::{Event, Key};
#[cfg(feature = "ears")]
use termion::input::TermRead;
#[cfg(feature = "ears")]
use termion::raw::IntoRawMode;
use time;

pub fn main(options: &ArgMatches) -> Result<(), ()> {
    let mut video_path = env::current_dir().map_err(|_| {
        eprintln!("Could not get current directory");
    })?;
    video_path.push(options.value_of("VIDEO").unwrap());

    if !video_path.exists() {
        eprintln!("Video does not exist.");
        return Err(());
    }
    let frames_param = options.value_of("FRAMES");
    make_parse_macro!(options);
    let converter = options.value_of("converter").unwrap().parse().unwrap();
    let height = parse!("height", u16);
    let keep_size = options.is_present("keep-size");
    let rate = parse!("rate", u8).unwrap();
    let ratio = parse!("ratio", u8, may be zero).unwrap();
    let width = parse!("width", u16);

    if frames_param.is_none() && video_path.is_dir() {
        eprintln!("Video is a directory (assuming pre-processed), but FRAMES isn't set.");
        eprintln!("Run `termplay preprocess --help` for more info.");
        return Err(());
    }
    if frames_param.is_some() && video_path.is_file() {
        eprintln!("Warning: No reason to specify FRAMES. Video isn't pre-processed");
    }
    let mut frames = 0;

    let _tempdir;
    let dir_path: &Path;
    if video_path.is_file() {
        check_cmd!("ffmpeg", "-version");

        println!();
        allow_exit()?;
        println!("Creating directory...");

        let dir = TempDir::new("termplay").map_err(|err| {
            eprintln!("Failed to create temporary directory");
            eprintln!("Error: {}", err)
        })?;

        _tempdir = dir;
        dir_path = _tempdir.path();

        allow_exit()?;

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
    } else {
        frames = frames_param.unwrap().parse().map_err(|_| {
            eprintln!("FRAMES is not a valid number.");
        })?;
        dir_path = &video_path;
    }

    play(dir_path, frames, rate)
}

pub struct VideoExitGuard;

impl Drop for VideoExitGuard {
    fn drop(&mut self) {
        print!("{}{}", CURSOR_SHOW, ALTERNATE_OFF);
    }
}

pub fn play(dir_path: &Path, frames: u32, rate: u8) -> Result<(), ()> {
    #[cfg(feature = "ears")]
    let mut music = Music::new(&dir_path.join("sound.wav").to_string_lossy())
        .ok_or_else(|| {
            eprintln!("Couldn't open music file");
        })?;

    println!("Ready to play. Press enter when you are... ");

    if let Err(err) = io::stdin().read_line(&mut String::new()) {
        eprintln!("Failed to wait for user input!");
        eprintln!("{}", err);
        eprintln!("Starting anyways... I guess");
    }

    #[cfg(feature = "ears")]
    macro_rules! make_switch {
        ($name:ident, $name_clone:ident) => {
            {
                let $name = Arc::new(AtomicBool::new(false));
                let $name_clone = $name.clone();
                ($name, $name_clone)
            }
        }
    }
    #[cfg(feature = "ears")]
    macro_rules! toggle_switch {
        ($name_clone:ident, $value:expr) => {
            $name_clone.store(
                $value,
                AtomicOrdering::Relaxed
            );
        }
    }

    #[cfg(feature = "ears")]
    let (higher, higher_clone) = make_switch!(lower, lower_clone);
    #[cfg(feature = "ears")]
    let (lower, lower_clone) = make_switch!(lower, lower_clone);
    #[cfg(feature = "ears")]
    let (pause, pause_clone) = make_switch!(pause, pause_clone);

    #[cfg(feature = "ears")]
    let raw = io::stdout().into_raw_mode();

    #[cfg(feature = "ears")]
    { if raw.is_ok() {
        thread::spawn(move || for event in io::stdin().events() {
            // Relies on the OS to clean it up sadly since events here are blocking.
            let event = match event {
                Ok(event) => event,
                Err(_) => continue,
            };

            if ::EXIT.load(AtomicOrdering::Relaxed) {
                break;
            }

            match event {
                Event::Key(Key::Char(' ')) => toggle_switch!(pause_clone, !pause_clone.load(AtomicOrdering::Relaxed)),
                Event::Key(Key::Up) => toggle_switch!(higher_clone, true),
                Event::Key(Key::Down) => toggle_switch!(lower_clone, true),
                Event::Key(Key::Ctrl('c')) => {
                    ::EXIT.store(true, AtomicOrdering::Relaxed);
                    break;
                },
                _ => {},
            }
        });
    }}

    print!("{}{}", ALTERNATE_ON, CURSOR_HIDE);
    let _guard = VideoExitGuard;

    #[cfg(feature = "ears")]
    music.play();

    let optimal = 1_000_000_000 / rate as i64;
    let mut lag: i64 = 0;

    #[cfg(feature = "ears")]
    let mut volume = 100;

    #[cfg(feature = "ears")]
    let max_show_volume = 3_000_000_000 / optimal; // 3 seconds
    #[cfg(feature = "ears")]
    let mut show_volume = -1;

    let mut i = 0;
    while i < frames {
        #[cfg(feature = "ears")]
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

        #[cfg(feature = "ears")]
        handle_volume!();
        #[cfg(feature = "ears")]
        { if pause.load(AtomicOrdering::Relaxed) {
            #[cfg(feature = "ears")]
            music.pause();

            let duration = Duration::from_millis(50);
            while pause.load(AtomicOrdering::Relaxed) && !::EXIT.load(AtomicOrdering::Relaxed) {
                thread::sleep(duration);
                #[cfg(feature = "ears")]
                handle_volume!();
                print!("\r{}% ", volume);
                flush!();
            }
            print!("\r    ");

            #[cfg(feature = "ears")]
            music.play();
        }}
        allow_exit()?;

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

        let mut file = File::open(dir_path.join(name)).map_err(|err| {
            flush!();
            eprintln!("Failed to open file.");
            eprintln!("{}", err);
        })?;

        // thread::sleep(Duration::from_millis(1000)); // Simulate lag

        let mut frame = String::new();
        if let Err(err) = file.read_to_string(&mut frame) {
            eprintln!("Error reading file: {}", err);
            return Err(());
        }

        print!("{}{}\r", CURSOR_TOP_LEFT, frame);
        #[cfg(feature = "ears")]
        { if show_volume > 0 {
            show_volume -= 1;
            // We never clear the previous value.
            // Therefor the trailing space is necessary.
            print!("{}% ", volume);
            flush!();
        } else if show_volume == 0 {
            show_volume -= 1;

            print!("    ");
            flush!();
        }}

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

    Ok(())
}

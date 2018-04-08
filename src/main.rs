extern crate ears;
extern crate image;
extern crate termplay;

use image::{imageops, DynamicImage, GenericImage, FilterType};
use ears::Music;
use std::{
    cmp::min,
    io::{self, BufWriter, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration
};
use termplay::{
    Playback,
    MusicPlayback,
    converters::{Converter, TrueColor},
    resizer
};

fn main() {
    let player = Playback::new(30);
    let music = Music::new("test.wav").unwrap();
    let player = Arc::new(Mutex::new(MusicPlayback::new(player, music)));
    let player2 = Arc::clone(&player);

    thread::spawn(move || {
        push(&player2);
    });

    println!("Let the processing get ahead");
    thread::sleep(Duration::from_secs(3));
    println!("Go go go!");

    let stdout = io::stdout();
    stdout.lock();
    let mut stdout = BufWriter::new(stdout);

    writeln!(stdout, "\x1b[?1049h\x1b[?25l").unwrap();
    MusicPlayback::run(|| player.lock().unwrap(), |frame| {
        if let Some(frame) = frame {
            write!(stdout, "\x1b[;H").unwrap(); stdout.flush().unwrap();
            let (width, height) = resizer::keep_aspect_ratio(frame.width(), frame.height(), 167-3, 40-3);
            let (width, height) = resizer::with_font_ratio(width, height, 45);

            let frame = imageops::resize(&frame, width, height, FilterType::Nearest);
            TrueColor.display(&mut stdout, &frame).unwrap();
        } else {
            //let mut player = player.lock().unwrap();
            //player.pause();
            writeln!(stdout, "\x1b[;H\x1b[JHold on, you're going too fast! Waiting...").unwrap();
            stdout.flush().unwrap();
            thread::sleep(Duration::from_secs(1));
            //player.play();
        }
    });
    writeln!(stdout, "\x1b[?25h\x1b[?1049h").unwrap();
}

fn push(player: &Mutex<MusicPlayback<DynamicImage>>) {
    let frames = 9682;

    let mut i = 1;
    while i < frames {
        let mut buf = Vec::with_capacity(50);
        for _ in 0..min(frames-i, 50) {
            buf.push(image::open(format!("test/frame{}.png", i)).unwrap());
            i += 1;
        }
        player.lock().unwrap().playback.extend(buf);
    }
}

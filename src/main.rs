extern crate termplay;
extern crate image;

use image::{imageops, DynamicImage, GenericImage, FilterType};
use std::{
    cmp::min,
    io::{self, BufWriter, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration
};
use termplay::{
    Playback,
    converters::{Converter, TrueColor},
    resizer
};

fn main() {
    let player = Arc::new(Mutex::new(Playback::new(30)));
    let player2 = Arc::clone(&player);

    thread::spawn(move || {
        push(&player2);
    });

    println!("Let the processing get ahead");
    //thread::sleep(Duration::from_secs(5));
    println!("Go go go!");

    let stdout = io::stdout();
    stdout.lock();
    let mut stdout = BufWriter::new(stdout);

    println!("\x1b[?1049h\x1b[?25l");
    Playback::run(&player, |frame| {
        if let Some(frame) = frame {
            write!(stdout, "\x1b[;H").unwrap(); stdout.flush().unwrap();
            let (width, height) = resizer::keep_aspect_ratio(frame.width(), frame.height(), 167-3, 40-3);
            let (width, height) = resizer::with_font_ratio(width, height, 45);

            let frame = imageops::resize(&frame, width, height, FilterType::Nearest);
            TrueColor.display(&mut stdout, &frame).unwrap();
        } else {
            writeln!(stdout, "\x1b[;H\x1b[JHold on, you're going too fast! Waiting...").unwrap();
            stdout.flush().unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });
    println!("\x1b[?25h\x1b[?1049h");
}

fn push(player: &Mutex<Playback<DynamicImage>>) {
    let mut i = 1;
    while i < 849 {
        let mut buf = Vec::with_capacity(50);
        for _ in 0..min(849-i, 50) {
            buf.push(image::open(format!("test/frame{}.png", i)).unwrap());
            i += 1;
        }
        player.lock().unwrap().extend(buf);
    }
}

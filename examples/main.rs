extern crate ears;
extern crate image;
extern crate termion;
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
use termion::{
    event::{Event, Key, MouseEvent, MouseButton},
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode
};
use termplay::{
    Playback,
    MusicPlayback,
    converters::{Converter, TrueColor},
    resizer,
    zoomer::Zoomer
};

fn main() {
    let player = Playback::new(30);
    let music = Music::new("test.wav").unwrap();
    let player = Arc::new(Mutex::new(MusicPlayback::new(player, music)));
    let player2 = Arc::clone(&player);
    let player3 = Arc::clone(&player);

    let zoomer = Arc::new(Mutex::new(Zoomer::new()));
    let zoomer2 = Arc::clone(&zoomer);

    thread::spawn(move || {
        push(&player2);
    });
    thread::spawn(move || -> Result<(), io::Error> {
        let stdin = io::stdin();
        for event in stdin.events() {
            match event? {
                Event::Key(Key::Char(' ')) => {
                    let mut player = player3.lock().unwrap();
                    if player.playback.is_paused() {
                        player.play();
                    } else {
                        player.pause();
                    }
                },
                Event::Key(Key::Ctrl('c')) => {
                    let mut player = player3.lock().unwrap();
                    player.stop();
                },
                Event::Mouse(MouseEvent::Hold(x, y)) => {
                    let mut zoomer = zoomer2.lock().unwrap();
                    zoomer.set_pos(x, y);
                    let mut player = player3.lock().unwrap();
                    player.playback.redraw();
                },
                Event::Mouse(MouseEvent::Press(btn, x, y)) => {
                    let mut zoomer = zoomer2.lock().unwrap();
                    let level = zoomer.level();
                    zoomer.set_pos(x, y);
                    match btn {
                        MouseButton::WheelUp => zoomer.set_level(level.saturating_sub(5)),
                        MouseButton::WheelDown => zoomer.set_level(level + 5),
                        _ => ()
                    }
                    let mut player = player3.lock().unwrap();
                    player.playback.redraw();
                },
                _ => ()
            }
        }
        Ok(())
    });

    println!("Let the processing get ahead");
    thread::sleep(Duration::from_secs(3));
    println!("Go go go!");

    let stdout = io::stdout();
    stdout.lock();
    let stdout = match stdout.into_raw_mode() {
        Ok(stdout) => stdout,
        Err(err) => {
            eprintln!("error: {}", err);
            return;
        }
    };
    let stdout = MouseTerminal::from(stdout);
    let mut stdout = BufWriter::new(stdout);

    writeln!(stdout, "\x1b[?1049h\x1b[?25l").unwrap();
    MusicPlayback::run(|| player.lock().unwrap(), |frame| {
        if let Some(mut frame) = frame {
            write!(stdout, "\x1b[;H").unwrap(); stdout.flush().unwrap();
            let (width, height) = resizer::keep_aspect_ratio(frame.width(), frame.height(), 167-4, 40-4);
            let (width, height) = resizer::with_font_ratio(width, height, 45);

            let zoomer = zoomer.lock().unwrap();
            let frame = zoomer.crop(&mut frame, width, height);
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

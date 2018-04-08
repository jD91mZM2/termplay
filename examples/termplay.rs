#[macro_use] extern crate clap;
extern crate ears;
extern crate failure;
extern crate image;
extern crate termion;
extern crate termplay;

use clap::{Arg, App};
use failure::Error;
use image::{FilterType, GenericImage};
use std::{
    cell::RefCell,
    io::{self, Write},
    process
};
use termion::{
    cursor,
    event::{Event, Key, MouseEvent, MouseButton},
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen
};
use termplay::{
    converters::{Converter, TrueColor},
    resizer,
    Zoomer
};
//use ears::Music;
//use image::{imageops, DynamicImage, GenericImage, FilterType};
//use std::{
//    cmp::min,
//    io::{self, BufWriter, Write},
//    sync::{Arc, Mutex},
//    thread,
//    time::Duration
//};
//use termplay::{
//    Playback,
//    MusicPlayback,
//    converters::{Converter, TrueColor},
//    resizer,
//    zoomer::Zoomer
//};

pub struct Hide<W: Write>(W);
impl<W: Write> From<W> for Hide<W> {
    fn from(mut from: W) -> Self {
        write!(from, "{}", cursor::Hide).unwrap();
        Hide(from)
    }
}
impl<W: Write> Drop for Hide<W> {
    fn drop(&mut self) {
        write!(self.0, "{}", cursor::Show).unwrap();
    }
}
impl<W: Write> Write for Hide<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

fn main() {
    let code = if let Err(err) = do_main() {
        eprintln!("error: {}", err);
        1
    } else { 0 };
    process::exit(code);
}
fn do_main() -> Result<(), Error> {
    let options =
        App::new(crate_name!())
            .version(crate_version!())
            .author(crate_authors!())
            .about(crate_description!())
            .arg(Arg::with_name("path")
                .help("Specifies the path to the image/video to play")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("quiet")
                .help("Ignores all the nice TUI things for simple image viewing")
                .short("q")
                .long("quiet"))
            .arg(Arg::with_name("width")
                .help("Sets the width (defaults to the terminal size, or 80)")
                .short("w")
                .long("width")
                .takes_value(true))
            .arg(Arg::with_name("height")
                .help("Sets the height (defaults to the terminal size, or 24)")
                .short("h")
                .long("height")
                .takes_value(true))
            .arg(Arg::with_name("ratio")
                .help("Sets the terminal font ratio (only takes effect with some converters)")
                .long("ratio")
                .takes_value(true))
            .get_matches();

    let path = options.value_of("path").unwrap();

    let (mut width, mut height) = termion::terminal_size().map(|(w, h)| (w as u32, h as u32)).unwrap_or((80, 24));
    if let Ok(w) = value_t!(options, "width", u32) {
        width = w;
    }
    if let Ok(h) = value_t!(options, "height", u32) {
        height = h;
    }

    let mut stdout = io::stdout();
    stdout.lock();

    let mut image = image::open(path)?;

    if options.is_present("quiet") {
        let (mut width, mut height) = resizer::keep_aspect_ratio(image.width(), image.height(), width, height);
        if let Ok(ratio) = value_t!(options, "ratio", u8) {
            let (w, h) = resizer::with_font_ratio(width, height, ratio);
            width = w;
            height = h;
        }
        image = image.resize_exact(width, height, FilterType::Nearest);
        TrueColor.display(&mut stdout, &image)?;
        return Ok(());
    }

    let stdout = stdout.into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = Hide::from(stdout);
    let mut stdout = AlternateScreen::from(stdout);

    let zoomer = RefCell::new(Zoomer::new());

    let mut draw = || -> Result<(), io::Error> {
        let (mut width, mut height) = resizer::keep_aspect_ratio(image.width(), image.height(), width, height);
        if let Ok(ratio) = value_t!(options, "ratio", u8) {
            let (w, h) = resizer::with_font_ratio(width, height, ratio);
            width = w;
            height = h;
        }

        let zoomer = zoomer.borrow();
        let mut image = zoomer.crop(&mut image, width, height);

        image = image.resize_exact(width, height, FilterType::Nearest);

        write!(stdout, "{}", cursor::Goto(1, 1))?;
        TrueColor.display(&mut stdout, &image)?;
        Ok(())
    };
    draw()?;

    let stdin = io::stdin();
    stdin.lock();
    for event in stdin.events() {
        match event? {
            Event::Key(Key::Ctrl('c')) |
            Event::Key(Key::Char('q')) => {
                return Ok(());
            },
            Event::Mouse(MouseEvent::Hold(x, y)) => {
                zoomer.borrow_mut().set_pos(x, y);
                draw()?;
            },
            Event::Mouse(MouseEvent::Press(btn, x, y)) => {
                {
                    let mut zoomer = zoomer.borrow_mut();
                    let level = zoomer.level();
                    zoomer.set_pos(x, y);

                    match btn {
                        MouseButton::WheelUp => zoomer.set_level(level.saturating_sub(5)),
                        MouseButton::WheelDown => zoomer.set_level(level + 5),
                        _ => ()
                    }
                }
                draw()?;
            },
            _ => ()
        }
    }

    Ok(())
}

/*
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
                    if player.is_paused() {
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
                    player.redraw();
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
                    player.redraw();
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
        player.lock().unwrap().extend(buf);
    }
}
*/

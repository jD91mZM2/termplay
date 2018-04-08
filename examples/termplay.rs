#[macro_use] extern crate clap;
extern crate ears;
#[macro_use] extern crate failure;
extern crate image;
extern crate tempdir;
extern crate termion;
extern crate termplay;

use clap::{Arg, App};
use ears::Music;
use failure::Error;
use image::{FilterType, GenericImage, ImageError, Pixel};
use std::{
    cell::RefCell,
    fmt::Write as _FmtWrite,
    io::{self, Write},
    process::{self, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration
};
use termion::{
    cursor,
    event::{Event, Key, MouseEvent, MouseButton},
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen
};
use tempdir::TempDir;
use termplay::{
    converters::*,
    Playback,
    resizer,
    Zoomer
};

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

#[derive(Clone, Copy)]
pub enum ConverterType {
    TrueColor,
    Color256
}
impl Converter for ConverterType {
    fn display<W, I, P>(&self, fmt: &mut W, image: &I) -> Result<(), io::Error>
        where W: Write,
              I: GenericImage<Pixel = P>,
              P: Pixel<Subpixel = u8>
    {
        match *self {
            ConverterType::TrueColor => TrueColor.display(fmt, image),
            ConverterType::Color256 => Color256.display(fmt, image)
        }
    }
}

fn main() {
    let code = if let Err(err) = do_main() {
        eprintln!("{}", err);
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
                .takes_value(true)
                .required(true))
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
            .arg(Arg::with_name("converter")
                .help("Decides how the image should be displayed")
                .short("c")
                .long("converter")
                .takes_value(true)
                .possible_values(&["truecolor", "color256"])
                .default_value("truecolor"))
            .arg(Arg::with_name("rate")
                .help("Sets the framerate")
                .short("r")
                .long("rate")
                .takes_value(true)
                .default_value("25"))
            .get_matches();

    let path = options.value_of("path").unwrap();

    let converter = match options.value_of("converter").unwrap() {
        "truecolor" => ConverterType::TrueColor,
        "color256"  => ConverterType::Color256,
        _ => unreachable!()
    };

    let ratio = value_t!(options, "ratio", u8).ok();
    if ratio == Some(0) {
        bail!("ratio can't be zero");
    }

    let (mut width, mut height) = termion::terminal_size().map(|(w, h)| (w as u32, h as u32)).unwrap_or((80, 24));
    if let Ok(w) = value_t!(options, "width", u32) {
        width = w;
    }
    if let Ok(h) = value_t!(options, "height", u32) {
        height = h;
    }

    let mut stdout = io::stdout();
    stdout.lock();

    match image::open(path) {
        Ok(mut image) => {
            if options.is_present("quiet") {
                let (mut width, mut height) = resizer::keep_aspect_ratio(image.width(), image.height(), width, height);
                if let Some(ratio) = ratio {
                    let (w, h) = resizer::with_font_ratio(width, height, ratio);
                    width = w;
                    height = h;
                }
                image = image.resize_exact(width, height, FilterType::Nearest);
                converter.display(&mut stdout, &image)?;
                return Ok(());
            }

            let stdout = stdout.into_raw_mode()?;
            let stdout = MouseTerminal::from(stdout);
            let stdout = Hide::from(stdout);
            let mut stdout = AlternateScreen::from(stdout);

            let zoomer = RefCell::new(Zoomer::new());

            let mut draw = || -> Result<(), io::Error> {
                let (mut width, mut height) = resizer::keep_aspect_ratio(image.width(), image.height(), width, height);
                if let Some(ratio) = ratio {
                    let (w, h) = resizer::with_font_ratio(width, height, ratio);
                    width = w;
                    height = h;
                }

                let zoomer = zoomer.borrow();
                let mut image = zoomer.crop(&mut image, width, height);

                image = image.resize_exact(width, height, FilterType::Nearest);

                write!(stdout, "{}", cursor::Goto(1, 1)).unwrap();
                converter.display(&mut stdout, &image)?;
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
        },
        Err(ImageError::UnsupportedError(_)) => {
            // Image failed, but file does exist.
            // Is it a video? Let's assume yes until proven otherwise.
            // What could possibly go wrong ¯\_(ツ)_/¯

            let rate = value_t!(options, "rate", u8).unwrap_or_else(|e| e.exit());

            if rate == 0 {
                bail!("rate can't be zero");
            }

            writeln!(stdout, "Invoking ffmpeg: Version check").unwrap();
            let status =
                Command::new("ffmpeg")
                    .arg("-version")
                    .stdout(Stdio::null())
                    .status()?;
            if !status.success() {
                // this is allowed to not follow the lowercase-error syntax,
                // as it's a joke
                bail!("Somehow, even the simple command `ffmpeg -version` failed.");
            }

            let tmp = TempDir::new("termplay")?;

            writeln!(stdout, "Invoking ffmpeg: Video -> Audio").unwrap();
            let audio_path = tmp.path().join("audio.wav");
            {
                let mut stdout = AlternateScreen::from(&mut stdout);
                let mut child =
                    Command::new("ffmpeg")
                        .arg("-i")
                        .arg(path)
                        .arg(&audio_path)
                        .stderr(Stdio::piped())
                        .spawn()?;
                io::copy(child.stderr.as_mut().unwrap(), &mut stdout)?;
                if !child.wait()?.success() {
                    // give enough time to see error
                    thread::sleep(Duration::from_secs(3));
                    bail!("ffmpeg returned error during conversion");
                }
            }
            writeln!(stdout, "Invoking ffmpeg: Video -> Image (In the background)").unwrap();
            let mut child =
                Command::new("ffmpeg")
                    .arg("-i")
                    .arg(path)
                    .arg("-r")
                    .arg(rate.to_string())
                    .arg(tmp.path().join("frame%d.png"))
                    .stderr(Stdio::null())
                    .spawn()?;
            writeln!(stdout, "Waiting 3 seconds to give it a head start...").unwrap();
            thread::sleep(Duration::from_secs(3));
            writeln!(stdout, "Starting new thread to add images to buffer").unwrap();

            let mut player = Playback::new(rate);
            match audio_path.to_str().map(|string| Music::new(string)) {
                Some(Ok(music)) => player.set_music(music),
                Some(Err(err)) => {
                    child.kill()?;
                    bail!("audio error: {}", err);
                },
                None => {
                    child.kill()?;
                    bail!("utf-8 error in path");
                }
            }
            let player = Arc::new(Mutex::new(player));
            let player2 = Arc::clone(&player);

            let thread_buf = thread::spawn(move || -> Result<(), Error> {
                let mut i = 1;
                let mut path = tmp.into_path();
                loop {
                    let mut buf = Vec::new();
                    for _ in 0..50 {
                        let mut string = String::with_capacity(5 + 4 + 4); // "frame" + 4 + ".png"
                        write!(string, "frame{}.png", i).unwrap();
                        path.push(string);
                        match image::open(&path) {
                            Ok(image) => buf.push(image),
                            Err(ImageError::IoError(ref err))
                            if err.kind() == io::ErrorKind::NotFound => {
                                if child.try_wait().is_ok() {
                                    return Ok(());
                                } else {
                                    // too fast! slow down for ffmpeg!
                                    thread::sleep(Duration::from_secs(3));
                                    continue;
                                }
                            }
                            Err(err) => {
                                child.kill()?;
                                return Err(err.into());
                            }
                        }
                        path.pop();
                        i += 1;
                    }
                    let mut player = player2.lock().unwrap();
                    if player.is_stopped() {
                        return Ok(());
                    }
                    player.extend(buf);
                }
            });

            writeln!(stdout, "Waiting 3 seconds to give it a head start...").unwrap();
            thread::sleep(Duration::from_secs(3));

            writeln!(stdout, "Finally, starting!").unwrap();

            let player2 = Arc::clone(&player);
            let zoomer = Arc::new(Mutex::new(Zoomer::new()));
            let zoomer2 = Arc::clone(&zoomer);

            let stdout = stdout.into_raw_mode()?;
            let stdout = MouseTerminal::from(stdout);
            let stdout = Hide::from(stdout);
            let mut stdout = AlternateScreen::from(stdout);

            let thread_play = thread::spawn(move || -> Result<(), Error> {
                Playback::run(&player2, |frame| {
                    if let Some(mut image) = frame {
                        let (mut width, mut height) = resizer::keep_aspect_ratio(image.width(), image.height(), width, height);
                        if let Some(ratio) = ratio {
                            let (w, h) = resizer::with_font_ratio(width, height, ratio);
                            width = w;
                            height = h;
                        }

                        let zoomer = zoomer2.lock().unwrap();
                        let mut image = zoomer.crop(&mut image, width, height);

                        image = image.resize_exact(width, height, FilterType::Nearest);

                        write!(stdout, "{}", cursor::Goto(1, 1)).unwrap();
                        converter.display(&mut stdout, &image).unwrap();
                    } else {
                        //let mut player = player2.lock().unwrap();
                        //player.pause();
                        write!(stdout, "{}Can't keep up! Waiting 5 seconds!", cursor::Goto(1, 1)).unwrap();
                        stdout.flush().unwrap();
                        thread::sleep(Duration::from_secs(5));
                        //player.play();
                    }
                });
                player2.lock().unwrap().stop(); // mark as stopped so other threads can see it
                Ok(())
            });

            let stdin = io::stdin();
            stdin.lock();
            for event in stdin.events() {
                let mut player = player.lock().unwrap();
                if player.is_stopped() {
                    break;
                }
                match event? {
                    Event::Key(Key::Ctrl('c')) |
                    Event::Key(Key::Char('q')) => {
                        player.stop();
                        drop(player); // prevent deadlock
                        return thread_buf.join()
                                .map_err(|_| format_err!("failed to join thread"))
                                .and_then(|inner| inner)
                                .and(thread_play.join()
                                    .map_err(|_| format_err!("failed to join thread"))
                                    .and_then(|inner| inner));
                    },
                    Event::Key(Key::Char(' ')) => {
                        if player.is_paused() {
                            player.play();
                        } else {
                            player.pause();
                        }
                    },
                    Event::Mouse(MouseEvent::Hold(x, y)) => {
                        zoomer.lock().unwrap().set_pos(x, y);
                        player.redraw();
                    },
                    Event::Mouse(MouseEvent::Press(btn, x, y)) => {
                        let mut zoomer = zoomer.lock().unwrap();
                        let level = zoomer.level();
                        zoomer.set_pos(x, y);

                        match btn {
                            MouseButton::WheelUp => zoomer.set_level(level.saturating_sub(5)),
                            MouseButton::WheelDown => zoomer.set_level(level + 5),
                            _ => ()
                        }
                        player.redraw();
                    },
                    _ => ()
                }
            }
            bail!("this should never happen");
        },
        Err(err) => return Err(err.into())
    };

    Ok(())
}

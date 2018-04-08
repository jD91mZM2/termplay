#[macro_use] extern crate clap;
extern crate ears;
extern crate failure;
extern crate image;
extern crate termion;
extern crate termplay;

use clap::{Arg, App};
use failure::Error;
use image::{FilterType, GenericImage, Pixel};
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
    converters::*,
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
            .arg(Arg::with_name("converter")
                .help("Decides how the image should be displayed")
                .short("c")
                .long("converter")
                .possible_values(&["truecolor", "color256"])
                .default_value("truecolor")
                .takes_value(true))
            .get_matches();

    let path = options.value_of("path").unwrap();

    let converter = match options.value_of("converter").unwrap() {
        "truecolor" => ConverterType::TrueColor,
        "color256"  => ConverterType::Color256,
        _ => unreachable!()
    };

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
        if let Ok(ratio) = value_t!(options, "ratio", u8) {
            let (w, h) = resizer::with_font_ratio(width, height, ratio);
            width = w;
            height = h;
        }

        let zoomer = zoomer.borrow();
        let mut image = zoomer.crop(&mut image, width, height);

        image = image.resize_exact(width, height, FilterType::Nearest);

        write!(stdout, "{}", cursor::Goto(1, 1))?;
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

    Ok(())
}

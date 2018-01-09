#[cfg(feature = "screen_control")] use std::ffi::CString;
#[cfg(feature = "screen_control")] use std::os::raw::*;
use clap::ArgMatches;
use colors::*;
use image::{GenericImage, FilterType};
use image;
use std::cmp;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering as AtomicOrdering;
use std::sync::{Arc, Mutex};
use std::thread;
use termion::event::{Event, Key, MouseButton, MouseEvent};
use termion::input::{MouseTerminal, TermRead};
use termion::raw::IntoRawMode;
use video::VideoExitGuard;
use {allow_exit, img};

#[cfg(feature = "screen_control")]
type Window = c_int;
#[cfg(feature = "screen_control")]
type Xdo    = *const c_void;

#[cfg(feature = "screen_control")]
#[link(name = "xdo")]
extern "C" {
    fn xdo_new(display: *const c_char) -> Xdo;
    fn xdo_get_active_window(xdo: Xdo, window: *mut Window) -> c_int;
    fn xdo_focus_window(xdo: Xdo, window: Window) -> c_int;
    fn xdo_send_keysequence_window(xdo: Xdo, window: Window, keysequence: *const c_char, delay: c_uint) -> c_int;
}

struct Zoom {
    level: u8,
    x: u16,
    y: u16
}

pub fn main(options: &ArgMatches) -> Result<(), ()> {
    check_cmd!("maim", "--version");

    make_parse_macro!(options);
    let converter = options.value_of("converter").unwrap().parse().unwrap();
    let height = parse!("height", u16);
    let keep_size = options.is_present("keep-size");
    let ratio = parse!("ratio", u8, may be zero).unwrap();
    let width = parse!("width", u16);
    let window = parse!("WINDOW", i32, may be zero).unwrap();

    let (width, height) = img::find_size(converter, width, height, ratio);

    let zoom: Arc<Mutex<Zoom>> = Arc::new(Mutex::new(Zoom {
        level: 100,
        x: 0,
        y: 0
    }));
    let zoom_clone = Arc::clone(&zoom);

    let raw: Result<MouseTerminal<_>, _> = io::stdout().into_raw_mode().map(|inner| inner.into());
    {
        if raw.is_ok() {
            // Relies on the OS to clean it up sadly since events here are blocking.
            thread::spawn(move || {
                #[cfg(feature = "screen_control")] let display = CString::new(":0").unwrap();
                #[cfg(feature = "screen_control")] let xdo = unsafe { xdo_new(display.as_ptr()) };

                #[cfg(feature = "screen_control")] let mut current = 0;
                #[cfg(feature = "screen_control")] unsafe { xdo_get_active_window(xdo, &mut current as *mut c_int); }

                for event in io::stdin().events() {
                    let event = match event {
                        Ok(event) => event,
                        Err(_) => continue,
                    };

                    if ::EXIT.load(AtomicOrdering::Relaxed) {
                        break;
                    }

                    match event {
                        Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, x, y)) => {
                            let mut zoom = zoom_clone.lock().unwrap();
                            zoom.level = cmp::max(zoom.level.saturating_sub(10), 10);
                            zoom.x = x;
                            zoom.y = y;
                        },
                        Event::Mouse(MouseEvent::Press(MouseButton::WheelDown, _, _)) => {
                            let mut zoom = zoom_clone.lock().unwrap();
                            zoom.level = cmp::min(zoom.level + 10, 100);
                        },
                        Event::Key(Key::Ctrl('c')) => {
                            ::EXIT.store(true, AtomicOrdering::Relaxed);
                            break;
                        },
                        _event => {
                            #[cfg(feature = "screen_control")]
                            {
                                let keysequence = CString::new(match _event {
                                    Event::Key(Key::Char('\n')) => {
                                        String::from("Return")
                                    },
                                    Event::Key(Key::Char(' ')) => {
                                        String::from("space")
                                    },
                                    Event::Key(Key::Char(c)) => {
                                        c.to_string()
                                    },
                                    Event::Key(Key::Backspace) => {
                                        String::from("BackSpace")
                                    },
                                    Event::Key(Key::Ctrl(c)) => {
                                        let mut string = c.to_string();
                                        string.insert_str(0, "Ctrl+");
                                        string
                                    },
                                    _ => continue,
                                }).unwrap();
                                unsafe {
                                    xdo_focus_window(xdo, window);
                                    thread::sleep(::std::time::Duration::from_millis(200));
                                    xdo_send_keysequence_window(xdo, window, keysequence.as_ptr(), 0);
                                    xdo_focus_window(xdo, current);
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    print!("{}{}", ALTERNATE_ON, CURSOR_HIDE);
    let _guard = VideoExitGuard;

    loop {
        allow_exit()?;

        let output = Command::new("maim")
            .arg("-i")
            .arg(window.to_string())
            .output()
            .map_err(|err| eprintln!("Running command `maim` failed: {}", err))?;
        if !output.status.success() {
            eprintln!("Command `maim` exited with a non-zero result");
            break;
        }

        let mut image = image::load_from_memory(&output.stdout).map_err(|err| {
            eprintln!("Failed to load image: {}", err);
        })?;

        let (old_width, old_height) = image.dimensions();

        let old_ratio = old_width as f64 / old_height as f64;
        let new_ratio = width as f64 / height as f64;

        let scale = if new_ratio > old_ratio {
            // The new height is less than the old height, or
            // the new width is more than the old width.
            height as f64 / old_height as f64
        } else {
            width as f64 / old_width as f64
        };

        let width  = (old_width  as f64 * scale) as u32;
        let height = (old_height as f64 * scale) as u32;

        {
            let zoom = zoom.lock().unwrap();
            if zoom.level != 100 {
                if zoom.x as u32 <= width && zoom.y as u32 <= height {
                    let x = zoom.x as u32 * (old_width  as u32 / width as u32);
                    let y = zoom.y as u32 * (old_height as u32 / height as u32);

                    let level = zoom.level as f64 / 100.0;
                    let level_x = (old_width  as f64 * level) as u32;
                    let level_y = (old_height as f64 * level) as u32;

                    image = image.crop(x.saturating_sub(level_x), y.saturating_sub(level_y), level_x * 2, level_y * 2);
                }
            }
        }

        if !keep_size {
            // it's faster if I avoid the ratio check
            image = image.resize_exact(width, height, FilterType::Nearest);
        }
        let image = img::convert(&image, converter, ratio);

        print!(
            "{}{}",
            CURSOR_TOP_LEFT,
            image
        );
        flush!();
    }
    Ok(())
}

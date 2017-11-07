use clap::ArgMatches;
use colors::*;
use image;
#[cfg(feature = "screen_control")]
use std::ffi::CString;
use std::io::{self, Write};
#[cfg(feature = "screen_control")]
use std::os::raw::*;
use std::process::{Command, Stdio};
#[cfg(feature = "screen_control")]
use std::sync::atomic::Ordering as AtomicOrdering;
#[cfg(feature = "screen_control")]
use std::thread;
#[cfg(feature = "screen_control")]
use termion::event::Key;
#[cfg(feature = "screen_control")]
use termion::input::TermRead;
#[cfg(feature = "screen_control")]
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

    #[cfg(feature = "screen_control")]
    let raw = io::stdout().into_raw_mode();
    #[cfg(feature = "screen_control")]
    {
        if raw.is_ok() {
            // Relies on the OS to clean it up sadly since events here are blocking.
            thread::spawn(move || {
                let display = CString::new(":0").unwrap();
                let xdo = unsafe { xdo_new(display.as_ptr()) };

                let mut current = 0;
                unsafe { xdo_get_active_window(xdo, &mut current as *mut c_int); }

                for event in io::stdin().keys() {
                    let event = match event {
                        Ok(event) => event,
                        Err(_) => continue,
                    };

                    if ::EXIT.load(AtomicOrdering::Relaxed) {
                        break;
                    }

                    let keysequence = CString::new(match event {
                        Key::Char('\n') => {
                            String::from("Return")
                        },
                        Key::Char(' ') => {
                            String::from("space")
                        },
                        Key::Char(c) => {
                            c.to_string()
                        },
                        Key::Backspace => {
                            String::from("BackSpace")
                        },
                        Key::Ctrl('c') => {
                            ::EXIT.store(true, AtomicOrdering::Relaxed);
                            break;
                        },
                        Key::Ctrl(c) => {
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
                eprintln!("baii");
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

        let image = image::load_from_memory(&output.stdout).map_err(|err| {
            eprintln!("Failed to load image: {}", err);
        })?;

        print!(
            "{}{}",
            CURSOR_TOP_LEFT,
            img::scale_and_convert(image, converter, width, height, ratio, keep_size)
        );
        flush!();
    }
    Ok(())
}

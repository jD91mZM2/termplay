use clap::ArgMatches;
use colors::*;
use image;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use video::VideoExitGuard;
use {allow_exit, img};

pub fn main(options: &ArgMatches) -> Result<(), ()> {
    check_cmd!("maim", "--version");

    make_parse_macro!(options);
    let converter = options.value_of("converter").unwrap().parse().unwrap();
    let height = parse!("height", u16);
    let keep_size = options.is_present("keep-size");
    let ratio = parse!("ratio", u8, may be zero).unwrap();
    let width = parse!("width", u16);
    let window = options.value_of("WINDOW").unwrap();

    let (width, height) = img::find_size(converter, width, height, ratio);

    print!("{}{}", ALTERNATE_ON, CURSOR_HIDE);
    let _guard = VideoExitGuard;

    loop {
        allow_exit()?;

        let output = Command::new("maim")
            .arg("-i")
            .arg(window)
            .output()
            .map_err(|err| eprintln!("Running command `maim` failed: {}", err))?;
        if !output.status.success() {
            eprintln!("Command `maim` exited with a non-zero result");
            break;
        }

        let image = image::load_from_memory(&output.stdout).map_err(|err| {
            eprintln!("Failed to load image: {}", err);
        })?;

        println!(
            "{}",
            img::scale_and_convert(image, converter, width, height, ratio, keep_size)
        );
    }
    Ok(())
}

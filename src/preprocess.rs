use allow_exit;
use clap::ArgMatches;
use colors::*;
use image::{self, ImageFormat};
use img;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufReader, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

pub fn main(options: &ArgMatches) -> Result<(), ()> {
    let mut video_path = env::current_dir().map_err(|_| {
        eprintln!("Could not get current directory");
    })?;
    video_path.push(options.value_of("VIDEO").unwrap());

    if !video_path.exists() {
        eprintln!("Video does not exist.");
        return Err(());
    }

    make_parse_macro!(options);
    let converter = options.value_of("converter").unwrap().parse().unwrap();
    let height = parse!("height", u16);
    let keep_size = options.is_present("keep-size");
    let output = options.value_of("OUTPUT").unwrap();
    let rate = parse!("rate", u8).unwrap();
    let ratio = parse!("ratio", u8, may be zero).unwrap();
    let width = parse!("width", u16);

    check_cmd!("ffmpeg", "-version");

    println!();
    allow_exit()?;
    println!("Creating directory...");
    if let Err(err) = fs::create_dir(output) {
        eprintln!("Could not create directory!");
        eprintln!("{}", err);
        return Err(());
    }

    allow_exit()?;

    let mut frames = 0;
    process(
        &mut frames,
        &ProcessArgs {
            video_path: &video_path,
            dir_path: Path::new(output),
            width: width,
            height: height,
            ratio: ratio,
            keep_size: keep_size,
            rate: rate,
            converter: converter
        }
    )?;

    println!("Number of frames: {}", frames);
    Ok(())
}

struct ScopedChild(Child);

impl ::std::ops::Deref for ScopedChild {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ::std::ops::DerefMut for ScopedChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for ScopedChild {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

pub struct ProcessArgs<'a> {
    pub video_path: &'a Path,
    pub dir_path: &'a Path,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub ratio: u8,
    pub keep_size: bool,
    pub rate: u8,
    pub converter: img::Converter
}
pub fn process(frames: &mut u32, args: &ProcessArgs) -> Result<(), ()> {
    println!("Starting conversion: Video -> Image...");

    let mut ffmpeg = match nullify!(
        Command::new("ffmpeg")
            .current_dir(args.dir_path)
            .arg("-i")
            .arg(args.video_path)
            .arg("-r")
            .arg(args.rate.to_string())
            .arg("frame%d.png")
    ).spawn() {
        Ok(ffmpeg) => ScopedChild(ffmpeg),
        Err(err) => {
            eprintln!("ffmpeg: {}", err);
            return Err(());
        },
    };
    thread::sleep(Duration::from_secs(1));

    println!("Started new process.");
    allow_exit()?;
    println!("Converting: Image -> Text");

    let mut i = 1;
    let mut retries = 0;

    macro_rules! wait_for_ffmpeg {
        ($err:expr) => {
            match ffmpeg.try_wait() {
                Ok(None) => {
                    if retries >= 3 {
                        eprintln!("I have tried 3 times, still can't read the file.");
                        eprintln!("Did ffmpeg hang? Are you trolling me by deleting files?");
                        eprintln!("I give up. Error: {}", $err);
                        return Err(());
                    }
                    retries += 1;
                    println!("Failed. Retrying...");
                    thread::sleep(Duration::from_secs(3));
                    continue;
                },
                Ok(Some(i)) => {
                    if !i.success() {
                        eprintln!("ffmpeg ended unsuccessfully.");
                        return Err(());
                    }
                    println!("Seems like we have reached the end");
                    break;
                },
                Err(err) => {
                    eprintln!("Error trying to get running status: {}", err);
                    return Err(());
                },
            }
        }
    }

    let (width, height) = img::find_size(args.converter, args.width, args.height, args.ratio);

    loop {
        allow_exit()?;

        let s = i.to_string();
        let mut name = String::with_capacity(5 + s.len() + 4);
        name.push_str("frame");
        name.push_str(s.as_str());
        name.push_str(".png");

        print!("\rProcessing {}", name);
        flush!();
        let mut file = OpenOptions::new().read(true).write(true).open(args.dir_path.join(name))
            .map_err(|err| {
                println!();
                wait_for_ffmpeg!(err);
            })?;

        let image = match image::load(BufReader::new(&mut file), ImageFormat::PNG) {
            Ok(image) => {
                retries = 0;
                i += 1;
                image
            },
            Err(err) => {
                println!();
                wait_for_ffmpeg!(err);
            },
        };
        let bytes = img::scale_and_convert(
            image,
            args.converter,
            width,
            height,
            args.ratio,
            args.keep_size
        ).into_bytes();

        // Previously reading has moved our cursor.
        // Let's move it back!
        if let Err(err) = file.seek(SeekFrom::Start(0)) {
            eprintln!("Failed to seek to beginning of file: {}", err);
            return Err(());
        }
        if let Err(err) = file.write_all(&bytes) {
            eprintln!("Failed to write to file: {}", err);
            return Err(());
        }
        if let Err(err) = file.set_len(bytes.len() as u64) {
            eprintln!("Failed to trim. Error: {}", err);
            return Err(());
        }
    }

    println!("Waiting for process to finish...");
    if let Ok(code) = ffmpeg.wait() {
        if !code.success() {
            println!("ffmpeg ended unsuccessfully.");
            return Err(());
        }
    }

    allow_exit()?;
    println!("Converting: Video -> Music {}", ALTERNATE_ON);

    if let Err(err) = Command::new("ffmpeg")
        .current_dir(&args.dir_path)
        .arg("-i")
        .arg(args.video_path)
        .arg("sound.wav")
        .status()
    {
        println!("{}", ALTERNATE_OFF);
        eprintln!("ffmpeg: {}", err);
        return Err(());
    }
    println!("{}", ALTERNATE_OFF);

    *frames = i - 1;
    Ok(())
}

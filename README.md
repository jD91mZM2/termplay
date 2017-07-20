# termplay
*Name by the awesome [@tbodt](https://github.com/tbodt)*

Are you a terminal fanboy like me?  
Sure, but do you ever watch YouTube? In your terminal?

----------------------------------------------------

`termplay` is the tool to convert images to ANSI sequences.  
But it also supports playing videos... and YouTube...  
Written in the systems language Rust, it has some solid performance.

  - **Sixels**, **TrueColor** and **256-bit color**
    - Sixels are slower, but has really good quality. Doesn't seem to work on higher resolutions though.
    - TrueColor is any RGB color, so while the quality isn't great, the colors look fantastic!
    - 256-bit color is the closest representation
    - Choose whatever is supported by your terminal and sounds cool to you.
  - **Flexible**
    - Change framerate, size and more using command line switches
  - **Adapting size**
    - Automatically scales the image to fit your terminal

When playing a video:  
  - **Concurrency**
    - It's converting to ANSI while `ffmpeg` is still processing!
  - **Audio/Frame Sync**
    - If one frame in takes longer to load and the audio continues on,
    - don't just pretend nothing happened! Skip a few frames!
    - Get back on track!
  - **Controls!**
    - Pause video!
    - Control volume!

![Example image](http://i.imgur.com/dKzlbg0.png)  
[Or if you want to maintain ratio](http://i.imgur.com/jItzR8T.png)  
*(Landscape image from [pexels.com](https://www.pexels.com/photo/snow-capped-mountains-under-blue-sky-and-white-clouds-115045/))*

## Compatibility

This tool is tested in GNOME Terminal, Konsole and alacritty (glitchy but amazing framerate).  
Might not be fully or supported at all by whatever terminal you use.

## Using

### Image
```

termplay-image 
Convert a single image to text

USAGE:
    termplay image [FLAGS] [OPTIONS] <IMAGE>

FLAGS:
    -k, --keep-size    Keep the frame size. Overrides -w and -h
        --help         Prints help information
    -V, --version      Prints version information

OPTIONS:
    -w, --width <width>            The max width of the frame
    -h, --height <height>          The max height of the frame
        --converter <converter>    How to convert the frame to ANSI. [default: truecolor]  [values: truecolor, 256-color, sixel]
        --ratio <ratio>            Change frame pixel width/height ratio (may or may not do anything) [default: 0]

ARGS:
    <IMAGE>    The image to convert
```

### Video

```
termplay-video 
Play a video in your terminal

USAGE:
    termplay video [FLAGS] [OPTIONS] <VIDEO> [FRAMES]

FLAGS:
    -k, --keep-size    Keep the frame size. Overrides -w and -h
        --help         Prints help information
    -V, --version      Prints version information

OPTIONS:
    -w, --width <width>            The max width of the frame
    -h, --height <height>          The max height of the frame
        --converter <converter>    How to convert the frame to ANSI. [default: truecolor]  [values: truecolor, 256-color, sixel]
    -r, --rate <rate>              The framerate of the video [default: 10]
        --ratio <ratio>            Change frame pixel width/height ratio (may or may not do anything) [default: 0]

ARGS:
    <VIDEO>     The video file path to play
    <FRAMES>    The FRAMES parameter is the number of frames processed. It will be returned when you pre-process a video
```

### YouTube

Replace `video` with `ytdl`, and supply a URL as VIDEO, and boom!  
Watch from YouTube directly!

Also has `--format` (short `-f`) to supply formats to youtube-dl to change quality and stuff.

### Pre-processing

If you feel like playing a video multiple times on the same settings,  
you can **pre-process** a video.

That means doing all the processing part separately, so you can skip it if you do it multiple times.  
Example:
```
$ termplay preprocess video.mp4
Checking ffmpeg... SUCCESS

Creating directory...
Starting conversion: Video -> Image...
Started new process.
Converting: Image -> Text
Processing frame622.png
Seems like we have reached the end
Converting: Video -> Music
Number of frames: 621
$ termplay video termplay-video 621 # 'termplay-video' is the default name for the processed folder.
```

**Fun fact:**  
If you change the rate, you have to do it on both while pre-processing and while playing.  
Or... don't. And enjoy playing the video in fast or slow motion.

## Installing

... That said, it comes with a slight flaw. **For now**, you have to compile this yourself.  

### Compile time requirements

**Rust v1.18 or more** is required. See your Rust version with
```
rustc --version
```
Update rust with
```
rustup update stable
```

To install termplay, **you need anything [ears](https://github.com/jhasse/ears) requires.**  

Example:  
On Ubuntu, you would run
```
sudo apt install libopenal-dev libsndfile1-dev
```

### Runtime requirements

[libsixel](https://github.com/saitoha/libsixel) is ALWAYS needed (no matter if you use it or not). Example: `sudo apt install libsixel`  
To use the video features, you need [ffmpeg](https://ffmpeg.org/). Example: `sudo apt install ffmpeg`  
To use the ytdl features, you need [youtube-dl](https://github.com/rg3/youtube-dl/). Example: `sudo -H pip install --upgrade youtube-dl`  

### Compiling!

Other than that, [this project is hosted on crates.io](https://crates.io/crates/termplay).  
So to install you just need to run
```
cargo install termplay
```

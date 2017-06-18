# termplay
*Name by the awesome [@tbodt](https://github.com/tbodt)*

Are you a terminal fanboy like me?  
Sure, but do you ever watch YouTube? In your terminal?

----------------------------------------------------

`termplay` is the tool to convert images to ANSI sequences.  
But it also supports playing videos... and YouTube...  
Written in the systems language Rust, it has some solid performance.

 - **TrueColor** and **256-bit color**
   - Choose whatever is supported by your terminal!
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

![Example image](http://i.imgur.com/dKzlbg0.png)  
*(Landscape image from [pexels.com](https://www.pexels.com/photo/snow-capped-mountains-under-blue-sky-and-white-clouds-115045/))*

## Compatibility

This tool is tested in GNOME Terminal and Konsole.  
Might not be fully or supported at all by whatever terminal you use.

## Switches

```
termplay 0.1.0
LEGOlord208 <LEGOlord208@krake.one>
Play an image/video in your terminal!

USAGE:
    termplay [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    image    Convert a single image to text
    video    Play a video in your terminal
    ytdl     Play any video from youtube-dl
```

## Installing

... That said, it comes with a slight flaw. **For now**, you have to compile yourself.  
No big deal though.
The only external package libraries needed to install are the ones required by [ears](https://github.com/jhasse/ears).  

On Ubuntu, a full installation from nothing (not even Rust installed) would look like
```bash
curl https://sh.rustup.rs -sSf | sh
sudo apt install libopenal-dev libsndfile1-dev ffmpeg
sudo -H pip install --upgrade youtube-dl # Sudo is required if you're not using a single user python installation

git clone git@github.com:legolord208/termplay.git
cd termplay
cargo build --release
```
Poof! `targets/release/termplay` is created

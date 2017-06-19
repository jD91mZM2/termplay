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

## Using

### Image
```

[image]
```

### Video

```
[video]
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

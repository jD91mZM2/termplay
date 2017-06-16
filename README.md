# play-youtube

Are you a terminal fanboy like me?  
Sure, but do you ever watch YouTube? In your terminal?

----------------------------------------------------

`play-youtube` is the tool to play YouTube... Converted to ANSI escape sequences...  
Written in the systems language Rust, it has more performance than any bash script could.

 - **Concurrency**
   - It's converting to ANSI while `ffmpeg` is still processing!
 - **Audio/Frame Sync**
   - If one frame takes longer to load and the audio continues on,
   - don't just pretend nothing happened! Skip a few frames!
   - Get back on track!
 - **TrueColor** and **256-bit color**
   - Choose whatever is supported by your terminal!
 - **Adapting size**
   - Automatically scales the video to fit your terminal
 - **Flexible**
   - Change framerate, size and more using command line switches

## Compatibility

This tool is tested in GNOME Terminal and Konsole.  
Might not be fully or supported at all by whatever terminal you use.

## Switches

```
play-youtube 0.1.0
LEGOlord208 <LEGOlord208@krake.one>
Play YouTube in your terminal!

USAGE:
    play-youtube [OPTIONS] <VIDEO>

FLAGS:
        --help
            Prints help information

    -V, --version
            Prints version information


OPTIONS:
        --converter <converter>
            How to convert the video.
            Valid values are truecolor and 256-color.
            Default is truecolor.
    -h, --height <height>
            The max height of the video

    -r, --rate <rate>
            The framerate of the video

    -w, --width <width>
            The max width of the video


ARGS:
    <VIDEO>
            The video URL to play
```
Poof! `targets/release/play-youtube` is created

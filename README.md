# termplay
*Name by the awesome [@tbodt](https://github.com/tbodt)*

Are you a terminal fanboy like me?  
Sure, but do you ever watch YouTube? In your terminal?

----------------------------------------------------

`termplay` is the tool to convert images to ANSI sequences.  
But it also supports playing videos...  
Written in the systems language Rust, it has some solid performance.

  - **Multiple modes**
    - Sixels: Only supported by a few terminals, like xterm.
    - Unicode halfblock: This is the same as TrueColor below, but uses unicode half-blocks for smaller pixels.
    - TrueColor: Any RGB color, supported by most terminals.
    - 256 color: The closest representation of a color that can be fit within 1 byte. Use this if nothing else works.
  - **Flexible**
    - Change framerate, size and more using command line switches
  - **Adapting size**
    - Automatically scales the image to fit your terminal

Termplay also allows you to control the media, such as zoom in or pause the video.

![Example image](https://i.imgur.com/54MXrSk.png)  
*(Landscape image from [pexels.com](https://www.pexels.com/photo/beautiful-holiday-lake-landscape-358482/))*

## Using

### Image

```
termplay 2.0.0
jD91mZM2 <me@krake.one>
Play images/videos in your terminal

USAGE:
    termplay [FLAGS] [OPTIONS] <path>

FLAGS:
        --help       Prints help information
    -q, --quiet      Ignores all the nice TUI things for simple image viewing
    -V, --version    Prints version information

OPTIONS:
    -c, --converter <converter>    Decides how the image should be displayed [default: halfblock]  [possible values:
                                   color256, halfblock, sixel, truecolor]
    -h, --height <height>          Sets the height (defaults to the terminal size, or 24)
    -r, --rate <rate>              Sets the framerate [default: 24]
        --ratio <ratio>            Sets the terminal font ratio (only takes effect with some converters)
    -w, --width <width>            Sets the width (defaults to the terminal size, or 80)

ARGS:
    <path>    Specifies the path to the image/video to play
```

## Compiling

### Compile time requirements

**Rust v1.25 or more** is required. See your Rust version with
```
rustc --version
```
Update rust with
```
rustup update stable
```

### Compiling!

Other than that, [this project is hosted on crates.io](https://crates.io/crates/termplay).  
So to install you just need to run
```
cargo install termplay --example termplay
```

(Note: The `--example` part is a hack because normal binaries don't allow specifying separate dependencies)

### Features

Almost all dependencies are optional and can be disabled!

Default features:

  - termion: This is what enables the rich image viewer. Disabling this will disable almost everything.
  - gst: Video support!
  - sixel: Support for sixels, provided by [libsixel](https://github.com/saitoha/libsixel)

To disable default features, run  

```
cargo install termplay --example termplay --no-default-features
```

To enable specific features, run  
```
cargo install termplay --example termplay --no-default-features --features "..."
```
where `...` is a comma separated list of features.

### Arch Linux

If you just want to get this running on Arch Linux with the default features, you can use the  
[AUR Package](https://aur.archlinux.org/packages/termplay/)

### Ubuntu

Example:

```
sudo apt install libopenal-dev libsndfile1-dev libsixel-dev libgstreamer1.0-dev
cargo install termplay --example termplay
```

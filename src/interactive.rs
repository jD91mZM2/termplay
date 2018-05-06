#[cfg(feature = "gst")] use resizer::Sizer;
#[cfg(feature = "termion")] use zoomer::Zoomer;
use converters::Converter;

#[cfg(feature = "failure")] use failure::Error;
#[cfg(feature = "gst")] use gst::{self, prelude::*};
#[cfg(feature = "gst")] use gst_app;
#[cfg(feature = "gst")] use image::{self, GenericImage, ImageFormat};
#[cfg(feature = "gst")] use std::sync::{Arc, Mutex};
#[cfg(feature = "termion")] use std::io::Read;
use image::{DynamicImage, FilterType};
use std::io::{self, Write};
#[cfg(feature = "termion")]
use termion::{
    cursor,
    event::{Event, Key, MouseEvent, MouseButton},
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen
};

#[cfg(feature = "termion")]
struct Hide<W: Write>(W);
#[cfg(feature = "termion")]
impl<W: Write> From<W> for Hide<W> {
    fn from(mut from: W) -> Self {
        write!(from, "{}", cursor::Hide).unwrap();
        Hide(from)
    }
}
#[cfg(feature = "termion")]
impl<W: Write> Drop for Hide<W> {
    fn drop(&mut self) {
        write!(self.0, "{}", cursor::Show).unwrap();
    }
}
#[cfg(feature = "termion")]
impl<W: Write> Write for Hide<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[derive(Clone, Debug)]
/// A small interactive image viewer
pub struct ImageViewer<C: Converter + Copy> {
    pub converter: C,
    pub width: u32,
    pub height: u32
}
impl<C: Converter + Copy> ImageViewer<C> {
    /// Simply resize and display an image
    pub fn display_image_quiet<W: Write>(&self, stdout: &mut W, image: &DynamicImage) -> io::Result<()> {
        let image = image.resize_exact(self.width, self.height, FilterType::Nearest);
        self.converter.display(stdout, &image)
    }
    #[cfg(feature = "termion")]
    /// Display the image in a rich viewer with support from scrolling
    pub fn display_image<R, W>(&self, stdin: &mut R, stdout: &mut W, image: &mut DynamicImage) -> io::Result<()>
        where R: Read,
              W: Write
    {
        let stdout = stdout.into_raw_mode()?;
        let stdout = MouseTerminal::from(stdout);
        let stdout = Hide::from(stdout);
        let mut stdout = AlternateScreen::from(stdout);

        let mut zoomer = Zoomer::new(self.converter);

        let mut draw = |zoomer: &Zoomer<_>| -> io::Result<()> {
            let image = zoomer.crop(image, self.width, self.height);

            write!(stdout, "{}", cursor::Goto(1, 1)).unwrap();
            self.display_image_quiet(&mut stdout, &image)?;
            Ok(())
        };
        draw(&zoomer)?;

        for event in stdin.events() {
            match event? {
                Event::Key(Key::Ctrl('c')) |
                Event::Key(Key::Char('q')) => {
                    return Ok(());
                },
                Event::Key(Key::Char(c)) => {
                    let (mut x, mut y) = zoomer.pos();
                    let mut level = zoomer.level();
                    match c {
                        'w' => y = y.saturating_sub(2),
                        'a' => x = x.saturating_sub(2),
                        's' => y = y.saturating_add(2),
                        'd' => x = x.saturating_add(2),
                        '+' => zoomer.set_level(level.saturating_sub(5)),
                        '-' => zoomer.set_level(level + 5),
                        _   => ()
                    }
                    zoomer.set_pos(x, y);
                    draw(&zoomer)?;
                },
                Event::Mouse(MouseEvent::Press(btn, x, y)) => {
                    let level = zoomer.level();

                    match btn {
                        MouseButton::Left => zoomer.drag_start(x, y),
                        MouseButton::WheelUp => {
                            if level == 100 {
                                zoomer.set_pos(x, y);
                            }
                            zoomer.set_level(level.saturating_sub(5))
                        },
                        MouseButton::WheelDown => {
                            if level == 100 {
                                zoomer.set_pos(x, y);
                            }
                            zoomer.set_level(level + 5)
                        },
                        _ => ()
                    }
                    draw(&zoomer)?;
                },
                Event::Mouse(MouseEvent::Hold(x, y)) => {
                    zoomer.drag_move(x, y);
                    draw(&zoomer)?;
                },
                Event::Mouse(MouseEvent::Release(..)) => {
                    zoomer.drag_stop();
                },
                _ => ()
            }
        }
        Ok(())
    }
}

#[cfg(feature = "gst")]
#[derive(Debug, Fail)]
pub enum VideoError {
    #[fail(display = "failed to create {}", _0)]
    GstCreationError(&'static str)
}

#[cfg(feature = "gst")]
#[derive(Clone, Debug)]
/// A GStreamer-based interactive video player.
/// Because of some internal threading, this is cloned inside the play_video function.
/// So you will probably want to keep the converter and sizer small.
pub struct VideoPlayer<C: Converter + Copy + Send + 'static, S: Sizer + Clone + Send + 'static> {
    pub converter: C,
    pub sizer: S,
    pub rate: u8
}
#[cfg(feature = "gst")]
impl<C: Converter + Copy + Send + Sync, S: Sizer + Clone + Send + Sync> VideoPlayer<C, S> {
    fn image_from_sample(&self, sample: &gst::sample::SampleRef) -> Option<DynamicImage> {
        let buffer = sample.get_buffer()?;
        let map = buffer.map_readable()?;
        image::load_from_memory_with_format(&map, ImageFormat::PNM).ok()
    }
    fn display_image<W: Write>(
        &self,
        stdout: &mut W,
        zoomer: &Zoomer<C>,
        image: &mut DynamicImage
    ) {
        let (width, height) = self.sizer.get_size(image.width(), image.height());

        let image = zoomer.crop(image, width, height);

        let viewer = ImageViewer {
            converter: self.converter,
            width: width,
            height: height
        };

        write!(stdout, "{}", cursor::Goto(1, 1)).unwrap();
        let _ = viewer.display_image_quiet(&mut *stdout, &image);
    }
    /// Play the video on specified uri. Use file:// links for file paths.
    pub fn play_video<R, W>(&self, stdin: &mut R, stdout: W, uri: &str) -> Result<(), Error>
        where R: Read,
              W: Write + Send + 'static
    {
        gst::init()?;

        let source = gst::ElementFactory::make("playbin", None).ok_or(VideoError::GstCreationError("playbin"))?;
        let videorate = gst::ElementFactory::make("videorate", None).ok_or(VideoError::GstCreationError("videorate"))?;
        let pnmenc = gst::ElementFactory::make("pnmenc", None).ok_or(VideoError::GstCreationError("pnmenc"))?;
        let sink = gst::ElementFactory::make("appsink", None).ok_or(VideoError::GstCreationError("appsink"))?;
        let appsink = sink.clone()
            .downcast::<gst_app::AppSink>()
            .unwrap();

        videorate.set_property("max-rate", &(self.rate as i32))?;

        let elems = &[&videorate, &pnmenc, &sink];

        let bin = gst::Bin::new(None);
        bin.add_many(elems)?;
        gst::Element::link_many(elems)?;

        // make input for bin point to first element
        let sink = elems[0].get_static_pad("sink").unwrap();
        let ghost = gst::GhostPad::new("sink", &sink).ok_or(VideoError::GstCreationError("ghost pad"))?;
        ghost.set_active(true)?;
        bin.add_pad(&ghost)?;

        source.set_property("uri", &uri)?;
        source.set_property("video-sink", &bin.upcast::<gst::Element>())?;

        let zoomer = Arc::new(Mutex::new(Zoomer::new(self.converter)));

        let stdout = stdout.into_raw_mode()?;
        let stdout = MouseTerminal::from(stdout);
        let stdout = Hide::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let stdout = Arc::new(Mutex::new(stdout));

        let clone = self.clone();

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::new()
                .new_sample({
                    let stdout = Arc::clone(&stdout);
                    let zoomer = Arc::clone(&zoomer);
                    move |sink| {
                        let sample = match sink.pull_sample() {
                            Some(sample) => sample,
                            None => return gst::FlowReturn::Eos,
                        };
                        let mut stdout = stdout.lock().unwrap();
                        let zoomer = zoomer.lock().unwrap();
                        match clone.image_from_sample(&sample) {
                            Some(mut image) => {
                                clone.display_image(&mut *stdout, &zoomer, &mut image);
                                gst::FlowReturn::Ok
                            },
                            None => gst::FlowReturn::Error
                        }
                    }
                })
                .build()
        );

        source.set_state(gst::State::Playing).into_result()?;

        let mut volume: f64 = 1.0;
        let mut frame = None;

        let seek_time = gst::ClockTime::from_seconds(5);

        for event in stdin.events() {
            match event? {
                Event::Key(Key::Ctrl('c')) |
                Event::Key(Key::Char('q')) => {
                    break;
                },
                Event::Key(Key::Char(' ')) => {
                    let (result, state, _pending) = source.get_state(gst::CLOCK_TIME_NONE);
                    if result == gst::StateChangeReturn::Success {
                        if state == gst::State::Paused {
                            source.set_state(gst::State::Playing).into_result()?;
                            frame = None;
                        } else {
                            source.set_state(gst::State::Paused).into_result()?;
                            frame = appsink.pull_preroll().and_then(|sample| self.image_from_sample(&sample));
                        }
                    }
                },
                Event::Key(Key::Left) => {
                    if let Some(mut time) = source.query_position::<gst::ClockTime>() {
                        if time >= seek_time {
                            time -= seek_time;
                        } else {
                            time = gst::ClockTime(Some(0));
                        }

                        source.seek_simple(
                            gst::SeekFlags::FLUSH,
                            gst::format::GenericFormattedValue::from_time(time)
                        )?;
                    }
                },
                Event::Key(Key::Right) => {
                    if let Some(mut time) = source.query_position::<gst::ClockTime>() {
                        time += seek_time;

                        source.seek_simple(
                            gst::SeekFlags::FLUSH,
                            gst::format::GenericFormattedValue::from_time(time)
                        )?;
                    }
                },
                Event::Key(Key::Up) => {
                    if volume + 0.1 < 1.0 {
                        volume += 0.1;
                    }
                    source.set_property("volume", &volume)?;
                },
                Event::Key(Key::Down) => {
                    if volume - 0.1 > 0.0 {
                        volume -= 0.1;
                    }
                    source.set_property("volume", &volume)?;
                }
                Event::Key(Key::Char(c)) => {
                    let mut zoomer = zoomer.lock().unwrap();
                    let (mut x, mut y) = zoomer.pos();
                    let mut level = zoomer.level();
                    match c {
                        'w' => y = y.saturating_sub(2),
                        'a' => x = x.saturating_sub(2),
                        's' => y = y.saturating_add(2),
                        'd' => x = x.saturating_add(2),
                        '+' => zoomer.set_level(level.saturating_sub(5)),
                        '-' => zoomer.set_level(level + 5),
                        _   => ()
                    }
                    zoomer.set_pos(x, y);
                    if let Some(ref mut frame) = frame {
                        let _ = self.display_image(&mut *stdout.lock().unwrap(), &zoomer, frame);
                    }
                },
                Event::Mouse(MouseEvent::Press(btn, x, y)) => {
                    let mut zoomer = zoomer.lock().unwrap();
                    let level = zoomer.level();

                    match btn {
                        MouseButton::Left => zoomer.drag_start(x, y),
                        MouseButton::WheelUp => {
                            if level == 100 {
                                zoomer.set_pos(x, y);
                            }
                            zoomer.set_level(level.saturating_sub(5));
                        },
                        MouseButton::WheelDown => {
                            if level == 100 {
                                zoomer.set_pos(x, y);
                            }
                            zoomer.set_level(level + 5);
                        },
                        _ => ()
                    }
                    if let Some(ref mut frame) = frame {
                        let _ = self.display_image(&mut *stdout.lock().unwrap(), &zoomer, frame);
                    }
                },
                Event::Mouse(MouseEvent::Hold(x, y)) => {
                    let mut zoomer = zoomer.lock().unwrap();
                    zoomer.drag_move(x, y);
                    if let Some(ref mut frame) = frame {
                        let _ = self.display_image(&mut *stdout.lock().unwrap(), &zoomer, frame);
                    }
                },
                Event::Mouse(MouseEvent::Release(..)) => {
                    zoomer.lock().unwrap().drag_stop();
                },
                _ => ()
            }
        }
        source.set_state(gst::State::Null).into_result()?;
        Ok(())
    }
}

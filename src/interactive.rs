#[cfg(feature = "gst")] use resizer::Sizer;
#[cfg(feature = "termion")] use zoomer::Zoomer;
use converters::Converter;

#[cfg(feature = "failure")] use failure::Error;
#[cfg(feature = "gst")] use gst::{self, prelude::*};
#[cfg(feature = "gst")] use gst_app;
#[cfg(feature = "gst")] use image::{self, GenericImage, ImageFormat};
#[cfg(feature = "gst")] use std::sync::{Arc, Mutex};
#[cfg(feature = "termion")] use std::{cell::RefCell, io::Read};
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

        let zoomer = RefCell::new(Zoomer::new(self.converter));

        let mut draw = || -> io::Result<()> {
            let zoomer = zoomer.borrow();
            let image = zoomer.crop(image, self.width, self.height);

            write!(stdout, "{}", cursor::Goto(1, 1)).unwrap();
            self.display_image_quiet(&mut stdout, &image)?;
            Ok(())
        };
        draw()?;

        for event in stdin.events() {
            match event? {
                Event::Key(Key::Ctrl('c')) |
                Event::Key(Key::Char('q')) => {
                    return Ok(());
                },
                Event::Mouse(MouseEvent::Hold(x, y)) => {
                    zoomer.borrow_mut().set_pos(x, y);
                    draw()?;
                },
                Event::Mouse(MouseEvent::Press(btn, x, y)) => {
                    {
                        let mut zoomer = zoomer.borrow_mut();
                        let level = zoomer.level();
                        zoomer.set_pos(x, y);

                        match btn {
                            MouseButton::WheelUp => zoomer.set_level(level.saturating_sub(5)),
                            MouseButton::WheelDown => zoomer.set_level(level + 5),
                            _ => ()
                        }
                    }
                    draw()?;
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
    fn display_frame<W: Write>(
            &self,
            stdout: &Mutex<W>,
            zoomer: &Mutex<Zoomer<C>>,
            sample: &gst::sample::SampleRef
        ) -> gst::FlowReturn {
        macro_rules! unwrap_or_error {
            ($what:expr, $error:expr) => {
                match $what {
                    Some(inner) => inner,
                    None => {
                        return gst::FlowReturn::Error;
                    }
                }
            }
        }
        let mut stdout = stdout.lock().unwrap();
        let buffer = unwrap_or_error!(sample.get_buffer(), "failed to get buffer");
        let map = unwrap_or_error!(buffer.map_readable(), "failed to get map");
        match image::load_from_memory_with_format(&map, ImageFormat::PNM) {
            Ok(mut image) => {
                let (width, height) = self.sizer.get_size(image.width(), image.height());

                let zoomer = zoomer.lock().unwrap();
                let mut image = zoomer.crop(&mut image, width, height);

                let viewer = ImageViewer {
                    converter: self.converter,
                    width: width,
                    height: height
                };

                write!(stdout, "{}", cursor::Goto(1, 1)).unwrap();
                let _ = viewer.display_image_quiet(&mut *stdout, &image);
                gst::FlowReturn::Ok
            }
            Err(err) => {
                write!(stdout, "{}Failed to parse image: {}\r\n", cursor::Goto(1, 1), err).unwrap();
                gst::FlowReturn::Error
            }
        }
    }
    /// Play the video on specified uri. Use file:// links for file paths.
    pub fn play_video<R, W>(&self, stdin: &mut R, stdout: W, uri: &str) -> Result<(), Error>
        where R: Read,
              W: Write + Send + 'static
    {
        gst::init()?;

        let source = gst::ElementFactory::make("playbin", None).ok_or(VideoError::GstCreationError("playbin"))?;
        let videorate = gst::ElementFactory::make("videorate", None).ok_or(VideoError::GstCreationError("videorate"))?;
        let pngenc = gst::ElementFactory::make("pnmenc", None).ok_or(VideoError::GstCreationError("pngenc"))?;
        let sink = gst::ElementFactory::make("appsink", None).ok_or(VideoError::GstCreationError("appsink"))?;
        let appsink = sink.clone()
            .downcast::<gst_app::AppSink>()
            .unwrap();

        videorate.set_property("max-rate", &(self.rate as i32))?;

        let elems = &[&videorate, &pngenc, &sink];

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
                        clone.display_frame(&stdout, &zoomer, &sample)
                    }
                })
                .build()
        );

        source.set_state(gst::State::Playing).into_result()?;

        let mut frame = None;

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
                            frame = appsink.pull_preroll();
                        }
                    }
                },
                Event::Mouse(MouseEvent::Hold(x, y)) => {
                    zoomer.lock().unwrap().set_pos(x, y);
                    if let Some(ref frame) = frame {
                        let _ = self.display_frame(&stdout, &zoomer, frame);
                    }
                },
                Event::Mouse(MouseEvent::Press(btn, x, y)) => {
                    {
                        let mut zoomer = zoomer.lock().unwrap();
                        let level = zoomer.level();
                        zoomer.set_pos(x, y);

                        match btn {
                            MouseButton::WheelUp => zoomer.set_level(level.saturating_sub(5)),
                            MouseButton::WheelDown => zoomer.set_level(level + 5),
                            _ => ()
                        }
                    }
                    if let Some(ref frame) = frame {
                        let _ = self.display_frame(&stdout, &zoomer, frame);
                    }
                },
                _ => ()
            }
        }
        source.set_state(gst::State::Null).into_result()?;
        Ok(())
    }
}

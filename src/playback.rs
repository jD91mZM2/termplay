#[cfg(feature = "ears")]
use ears::{AudioController, Music};
use image::GenericImage;
use std::{
    cmp::min,
    collections::VecDeque,
    iter::Extend,
    sync::Mutex,
    thread,
    time::{Duration, Instant}
};

/// A convenience video player
pub struct Playback<I: GenericImage + Clone + 'static> {
    buffer: VecDeque<I>,
    buffer_start: usize,
    max_buf_size: usize,

    stopped: bool,
    paused: bool,
    redraw: bool,

    #[cfg(feature = "ears")]
    music: Option<Music>,

    delay: Duration,
    lag: Duration,
    last: Option<Instant>
}
impl<I: GenericImage + Clone + 'static> Default for Playback<I> {
    fn default() -> Self {
        Self {
            buffer: VecDeque::default(),
            buffer_start: 0,
            max_buf_size: 100,

            stopped: false,
            paused: false,
            redraw: false,

            #[cfg(feature = "ears")]
            music: None,

            delay: Duration::from_millis(1000 / 60),
            lag: Duration::new(0, 0),
            last: None
        }
    }
}
impl<I: GenericImage + Clone + 'static> Playback<I> {
    /// Create a new Playback with the specified FPS
    pub fn new(fps: u8) -> Self {
        Self {
            delay: Duration::from_millis(1000 / fps as u64),
            ..Default::default()
        }
    }
    /// Create a new Playback with a custom buffer size limit.
    /// This limit is not strict.
    /// It may be ignored in certain cases to avoid playback issues.
    pub fn with_buf_size(max_buf_size: usize, fps: u8) -> Self {
        Self {
            max_buf_size: max_buf_size,
            delay: Duration::from_millis(1000 / fps as u64),
            ..Default::default()
        }
    }
    #[cfg(feature = "ears")]
    /// Set the background music of this player.
    /// pause/play functions will also pause/play the music.
    pub fn set_music(&mut self, music: Music) {
        self.music = Some(music);
    }
    /// Push a new frame to the buffer
    pub fn push(&mut self, img: I) {
        self.buffer.push_back(img);
    }
    /// Mark a frame as passed and possibly clean up the buffer
    pub fn pop(&mut self) {
        while self.buffer.len() >= self.max_buf_size && self.buffer_start > 0 {
            self.buffer.pop_front();
            self.buffer_start -= 1;
        }
        self.buffer_start += 1;
    }
    /// Move the cursor (relative to the current position)
    /// Note: This does NOT change the music.
    /// So if you have a music, this WILL cause it to go out of sync.
    pub fn jump(&mut self, n: isize) {
        if n >= 0 {
            let val = self.buffer_start.saturating_add(n as usize);
            self.buffer_start = min(val, self.buffer.len());
        } else {
            self.buffer_start = self.buffer_start.saturating_sub((-n) as usize);
        }
    }
    /// Pause the playback. Resume with `play`.
    pub fn pause(&mut self) {
        #[cfg(feature = "ears")] {
            if let Some(ref mut music) = self.music {
                music.pause();
            }
        }
        self.last = None;
        self.redraw = false;
        self.paused = true;
    }
    /// Resume the playback. Also see `pause`.
    pub fn play(&mut self) {
        #[cfg(feature = "ears")] {
            if let Some(ref mut music) = self.music {
                music.play();
            }
        }
        self.last = None;
        self.redraw = false;
        self.paused = false;
    }
    /// Return true if player is paused, otherwise false
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    /// Quit the player. This will stop the `loop` function.
    pub fn stop(&mut self) {
        #[cfg(feature = "ears")] {
            if let Some(ref mut music) = self.music {
                music.stop();
            }
        }
        self.stopped = true;
    }
    /// Return true if player is stopped, otherwise false
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }
    /// Tell the main playback loop to send through one frame, even if it's paused.
    pub fn redraw(&mut self) {
        if self.paused {
            self.redraw = true;
        }
    }
    /// Retrieve the frame at the cursor position
    pub fn current(&self) -> Option<&I> {
        self.buffer.get(self.buffer_start)
    }

    /// Start the main playback loop, send frames via handler.
    pub fn run<F>(me: &Mutex<Self>, mut handler: F)
        where F: FnMut(Option<I>)
    {
        #[cfg(feature = "ears")] {
            if let Some(ref mut music) = me.lock().unwrap().music {
                music.play();
            }
        }
        loop {
            let now = Instant::now();
            let redraw;
            let paused;
            let delay;
            let current;
            {
                let mut me = me.lock().unwrap();
                if me.stopped {
                    return;
                }
                if !me.paused {
                    if let Some(last) = me.last {
                        me.lag += now - last;
                    }
                    while me.lag >= me.delay {
                        me.lag -= me.delay;
                        // skip a frame
                        me.pop();
                    }
                    me.last = Some(now);
                    redraw = false;
                    paused = false;
                    delay = me.delay;
                    current = me.current().cloned();
                } else {
                    redraw = me.redraw;
                    me.redraw = false;
                    paused = true;
                    delay = me.delay;
                    current = if redraw { me.current().cloned() } else { None };
                }
            };
            if !paused || redraw {
                let none = current.is_none();
                handler(current);
                if !paused && none && me.lock().unwrap().current().is_none() {
                    // If handler has not pushed any new images
                    return;
                }
            }
            if let Some(val) = delay.checked_sub(now.elapsed()) {
                thread::sleep(val);
            }
        }
    }
}
impl<I: GenericImage + Clone + 'static> Extend<I> for Playback<I> {
    fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        self.buffer.extend(iter);
    }
}

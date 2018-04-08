use image::GenericImage;
use std::{
    cmp::min,
    collections::VecDeque,
    iter::Extend,
    ops::{Deref, DerefMut},
    thread,
    time::{Duration, Instant}
};

/// A convenience video player
pub struct Playback<I: GenericImage + Clone + 'static> {
    buffer: VecDeque<I>,
    buffer_start: usize,
    max_buf_size: usize,

    paused: bool,
    delay: Duration,
    lag: Duration,
    last: Option<Instant>
}
impl<I: GenericImage + Clone + 'static> Default for Playback<I> {
    fn default() -> Self {
        Self {
            buffer: VecDeque::default(),
            buffer_start: 0,
            max_buf_size: 10_000,

            paused: false,
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
        self.last = None;
        self.paused = true;
    }
    /// Resume the playback. Also see `pause`.
    pub fn play(&mut self) {
        self.last = None;
        self.paused = false;
    }
    /// Retrieve the frame at the cursor position
    pub fn current(&self) -> Option<&I> {
        self.buffer.get(self.buffer_start)
    }

    /// Start the main playback loop, send frames via handler.
    pub fn run<P, D, F1, F2>(mut me: F1, mut handler: F2)
        where P: AsMut<Self>,
              D: Deref<Target = P> + DerefMut,
              F1: FnMut() -> D,
              F2: FnMut(Option<I>)
    {
        loop {
            let now = Instant::now();
            let paused;
            let delay;
            let current;
            {
                let mut me = me();
                let me: &mut Self = (*me).as_mut();
                if !me.paused {
                    //if let Some(last) = me.last {
                    //    me.lag += now - last;
                    //}
                    //while me.lag >= me.delay {
                    //    me.lag -= me.delay;
                    //    // skip a frame
                    //    me.pop();
                    //}
                    me.pop();
                    paused = false;
                    delay = me.delay;
                    current = me.current().cloned();
                } else {
                    paused = true;
                    delay = me.delay;
                    current = None;
                }
            };
            if !paused {
                let none = current.is_none();
                handler(current);
                if none && (*me()).as_mut().current().is_none() {
                    // If handler has not pushed any new images
                    return;
                }
            }
            if let Some(val) = delay.checked_sub(now.elapsed()) {
                thread::sleep(val);
            }
            (*me()).as_mut().last = Some(Instant::now());
        }
    }
}
impl<I: GenericImage + Clone + 'static> Extend<I> for Playback<I> {
    fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        self.buffer.extend(iter);
    }
}

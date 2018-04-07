use image::GenericImage;
use std::{
    collections::VecDeque,
    iter::Extend,
    sync::Mutex,
    thread,
    time::{Duration, Instant}
};

/// A convenience video player
pub struct Playback<I: GenericImage> {
    buffer: VecDeque<I>,
    buffer_start: usize,
    max_buf_size: usize,

    delay: Duration,
    last: Option<Instant>
}
impl<I: GenericImage> Default for Playback<I> {
    fn default() -> Self {
        Self {
            buffer: VecDeque::default(),
            buffer_start: 0,
            max_buf_size: 10_000,

            delay: Duration::from_millis(1000 / 60),
            last: None
        }
    }
}
impl<I: GenericImage + Clone> Playback<I> {
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
    pub fn jump(&mut self, n: isize) -> bool {
        if n >= 0 {
            if let Some(val) = self.buffer_start.checked_add(n as usize) {
                if val < self.buffer.len() {
                    self.buffer_start = val;
                    true
                } else { false }
            } else { false }
        } else {
            if let Some(val) = self.buffer_start.checked_sub(-n as usize) {
                self.buffer_start = val;
                true
            } else { false }
        }
    }
    /// Retrieve the frame at the cursor position
    pub fn current(&self) -> Option<&I> {
        self.buffer.get(self.buffer_start)
    }

    pub fn run<F>(me: &Mutex<Self>, mut handler: F)
        where F: FnMut(Option<I>)
    {
        loop {
            let now = Instant::now();
            let delay;
            let current = {
                let mut me = me.lock().unwrap();
                if let Some(last) = me.last {
                    let mut elapsed = now - last;
                    while elapsed >= me.delay {
                        elapsed -= me.delay;
                        // skip a frame
                        me.pop();
                    }
                }
                me.last = Some(now);
                delay = me.delay;
                me.current().cloned()
            };
            let none = current.is_none();
            handler(current);
            if none && me.lock().unwrap().current().is_none() {
                // If handler has not pushed any new images
                return;
            }
            if let Some(val) = delay.checked_sub(now.elapsed()) {
                thread::sleep(val);
            }
        }
    }
}
impl<I: GenericImage + Clone> Extend<I> for Playback<I> {
    fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        self.buffer.extend(iter);
    }
}

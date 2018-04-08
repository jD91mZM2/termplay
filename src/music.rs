use ears::{AudioController, Music};
use image::GenericImage;
use std::ops::{Deref, DerefMut};

use Playback;

pub struct MusicPlayback<I: GenericImage + Clone + 'static> {
    /// Warning: Do not modify the underlying playback cursor (e.g. jump/pause/play/run)
    pub playback: Playback<I>,
    music: Music
}
impl<I: GenericImage + Clone + 'static> MusicPlayback<I> {
    pub fn new(playback: Playback<I>, music: Music) -> Self {
        Self {
            playback: playback,
            music: music
        }
    }
    /// Pause the playback and music
    pub fn pause(&mut self) {
        self.music.pause();
        self.playback.pause();
    }
    /// Resume the playback and music
    pub fn play(&mut self) {
        self.music.play();
        self.playback.play();
    }
    /// Stop the playback and music
    pub fn stop(&mut self) {
        self.music.stop();
        self.playback.stop();
    }

    /// Start the underlying playback and music
    pub fn run<D, F1, F2>(mut me: F1, handler: F2)
        where D: Deref<Target = Self> + DerefMut,
              F1: FnMut() -> D,
              F2: FnMut(Option<I>)
    {
        me().music.play();
        Playback::run(me, handler);
    }
}

impl<I: GenericImage + Clone + 'static> AsMut<Playback<I>> for MusicPlayback<I> {
    fn as_mut(&mut self) -> &mut Playback<I> {
        &mut self.playback
    }
}

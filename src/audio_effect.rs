use audio_engine::SoundSource;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// Play the intro once,
// and then keep the music in loop
pub struct WithIntro<T: SoundSource, S: SoundSource> {
    in_intro: bool,
    intro: T,
    music: S,
}
impl<T: SoundSource, S: SoundSource> WithIntro<T, S> {
    pub fn new(intro: T, music: S) -> Self {
        Self {
            in_intro: true,
            intro,
            music,
        }
    }
}
impl<T: SoundSource, S: SoundSource> SoundSource for WithIntro<T, S> {
    fn channels(&self) -> u16 {
        self.music.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.music.sample_rate()
    }
    fn reset(&mut self) {
        self.music.reset();
    }
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        let mut len = 0;
        loop {
            break if self.in_intro {
                len += self.intro.write_samples(&mut buffer[len..]);
                if len < buffer.len() {
                    self.intro.reset();
                    self.in_intro = false;
                    continue;
                }
                len
            } else {
                len += self.music.write_samples(&mut buffer[len..]);
                len
            };
        }
    }
}

/// When slow_down is true, the sound will slow down, and stop.
/// This always keep the inner sound source in loop.
pub struct SlowDown<T: SoundSource> {
    inner: T,
    pub slow_down: Arc<AtomicBool>,
    in_buffer: Box<[i16]>,
    iter: f32,
    speed: f32,
}
impl<T: SoundSource> SlowDown<T> {
    pub fn new(inner: T) -> Self {
        let len = 100 * inner.channels() as usize;
        Self {
            inner,
            slow_down: Arc::new(false.into()),
            in_buffer: vec![0; len].into_boxed_slice(),
            iter: 100.0,
            speed: 1.0,
        }
    }
}
impl<T: SoundSource> SoundSource for SlowDown<T> {
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn reset(&mut self) {
        self.inner.reset();
        self.slow_down.store(false, Ordering::Relaxed);
        self.iter = 100.0;
        self.speed = 1.0;
    }
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        if !self.slow_down.load(Ordering::Relaxed) {
            let mut len = self.inner.write_samples(buffer);
            while len < buffer.len() {
                self.inner.reset();
                len += self.inner.write_samples(&mut buffer[len..]);
            }
            buffer.len()
        } else {
            let mut i = 0;
            let channels = self.inner.channels() as usize;
            while i < buffer.len() {
                if self.speed <= 0.0 {
                    return i;
                }
                if (self.iter + 1.0) * channels as f32 >= self.in_buffer.len() as f32 {
                    self.iter -= self.in_buffer.len() as f32 / channels as f32;
                    let mut len = self.inner.write_samples(&mut self.in_buffer);
                    while len < self.in_buffer.len() {
                        self.inner.reset();
                        len += self.inner.write_samples(&mut self.in_buffer[len..]);
                    }
                }
                let t = self.iter.fract();
                let j = self.iter as usize * channels;
                for c in 0..channels {
                    buffer[i + c] = (self.in_buffer[j + c] as f32 * t
                        + self.in_buffer[j + c + channels] as f32 * (1.0 - t))
                        as i16;
                }
                self.iter += 1.0 * self.speed;
                self.speed -=
                    1.0 / (0.5 * self.sample_rate() as f32) * (1.0 - self.speed).max(0.05);
                i += channels;
            }

            buffer.len()
        }
    }
}

use std::time::Instant;

#[derive(Debug, Clone)]
pub struct BeatSync {
    pub first: Instant,
    pub last: Instant,
    pub count: u32,
}

impl BeatSync {
    pub fn new() -> Self {
        let now = Instant::now();

        Self {
            first: now,
            last: now,
            count: 0,
        }
    }

    pub fn _reset(&mut self) {
        *self = Self::new()
    }

    pub fn trigger(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last).as_secs_f32() > 2.0 {
            self.first = now;
            self.count = 0;
        }

        self.last = now;
        self.count += 1;
    }

    /// Average number of beats per seconds
    pub fn rate(&self) -> f32 {
        let deltas = self.count.saturating_sub(1);
        if deltas > 1 {
            deltas as f32 / self.last.duration_since(self.first).as_secs_f32()
        } else {
            1.0
        }
    }

    /// Average number of beats per minute
    pub fn bpm(&self) -> f32 {
        60.0 * self.rate()
    }

    /// Interpolated number of beats since first trigger
    pub fn beat(&self) -> f32 {
        self.rate() * self.first.elapsed().as_secs_f32()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{ops::Sub, time::Duration};

    #[test]
    fn three_beats() {
        let mut sync = BeatSync::new();

        std::thread::sleep(Duration::from_millis(250));
        sync.trigger();
        assert!(sync.rate().sub(4.0).abs() < 0.1, "{}", sync.rate());
        assert!(sync.beat().sub(1.0).abs() < 0.1, "{}", sync.beat());

        std::thread::sleep(Duration::from_millis(250));
        sync.trigger();
        assert!(sync.rate().sub(4.0).abs() < 0.1, "{}", sync.rate());
        assert!(sync.beat().sub(2.0).abs() < 0.1, "{}", sync.beat());

        std::thread::sleep(Duration::from_millis(250));
        sync.trigger();
        assert!(sync.rate().sub(4.0).abs() < 0.1, "{}", sync.rate());
        assert!(sync.beat().sub(3.0).abs() < 0.1, "{}", sync.beat());
    }
}

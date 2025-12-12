//! FPS tracking for the status bar display.

use std::collections::VecDeque;
use std::time::Instant;

/// Sliding window FPS calculator for status bar display.
pub struct FpsTracker {
    samples: VecDeque<Instant>,
    current_fps: f32,
    averaging_period_secs: f32,
}

impl FpsTracker {
    pub const DEFAULT_AVERAGING_PERIOD: f32 = 2.0;

    pub fn new() -> Self {
        Self::with_period(Self::DEFAULT_AVERAGING_PERIOD)
    }

    pub fn with_period(averaging_period_secs: f32) -> Self {
        Self {
            samples: VecDeque::new(),
            current_fps: 0.0,
            averaging_period_secs,
        }
    }

    /// Record frame, return smoothed FPS.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        self.samples.push_back(now);

        // Remove samples older than the averaging window
        while let Some(t) = self.samples.front() {
            if now.duration_since(*t).as_secs_f32() > self.averaging_period_secs {
                self.samples.pop_front();
            } else {
                break;
            }
        }

        self.current_fps = self.samples.len() as f32 / self.averaging_period_secs;
        self.current_fps
    }

    #[allow(dead_code)]
    pub fn current(&self) -> f32 {
        self.current_fps
    }
}

impl Default for FpsTracker {
    fn default() -> Self {
        Self::new()
    }
}

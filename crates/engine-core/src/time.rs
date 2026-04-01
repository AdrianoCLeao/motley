use std::{collections::VecDeque, time::Instant};

pub const DEFAULT_FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 60.0;
pub const DEFAULT_MAX_FRAME_TIME_SECONDS: f64 = 0.25;
pub const DEFAULT_FPS_AVERAGE_WINDOW_SAMPLES: usize = 60;

#[derive(Debug, Clone, Copy)]
pub struct TimeConfig {
    pub fixed_timestep_seconds: f64,
    pub max_frame_time_seconds: f64,
    pub fps_average_window_samples: usize,
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            fixed_timestep_seconds: DEFAULT_FIXED_TIMESTEP_SECONDS,
            max_frame_time_seconds: DEFAULT_MAX_FRAME_TIME_SECONDS,
            fps_average_window_samples: DEFAULT_FPS_AVERAGE_WINDOW_SAMPLES,
        }
    }
}

#[derive(Debug)]
pub struct Time {
    config: TimeConfig,
    delta_seconds: f32,
    elapsed_seconds: f64,
    frame_count: u64,
    fps_instant: f32,
    fps_rolling: f32,
    jitter_seconds: f32,
    accumulator_seconds: f64,
    frame_history_seconds: VecDeque<f64>,
    frame_history_sum_seconds: f64,
    frame_history_sum_squares_seconds: f64,
    last_instant: Instant,
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

impl Time {
    pub fn new() -> Self {
        Self::with_config(TimeConfig::default())
    }

    pub fn with_config(config: TimeConfig) -> Self {
        let fixed_timestep_seconds = if config.fixed_timestep_seconds > 0.0 {
            config.fixed_timestep_seconds
        } else {
            DEFAULT_FIXED_TIMESTEP_SECONDS
        };
        let max_frame_time_seconds = if config.max_frame_time_seconds > 0.0 {
            config.max_frame_time_seconds
        } else {
            DEFAULT_MAX_FRAME_TIME_SECONDS
        };
        let fps_average_window_samples = if config.fps_average_window_samples > 0 {
            config.fps_average_window_samples
        } else {
            DEFAULT_FPS_AVERAGE_WINDOW_SAMPLES
        };

        Self {
            config: TimeConfig {
                fixed_timestep_seconds,
                max_frame_time_seconds,
                fps_average_window_samples,
            },
            delta_seconds: 0.0,
            elapsed_seconds: 0.0,
            frame_count: 0,
            fps_instant: 0.0,
            fps_rolling: 0.0,
            jitter_seconds: 0.0,
            accumulator_seconds: 0.0,
            frame_history_seconds: VecDeque::with_capacity(fps_average_window_samples),
            frame_history_sum_seconds: 0.0,
            frame_history_sum_squares_seconds: 0.0,
            last_instant: Instant::now(),
        }
    }

    pub fn advance(&mut self) {
        let now = Instant::now();
        let frame_time_seconds = (now - self.last_instant).as_secs_f64();
        self.last_instant = now;
        self.advance_by(frame_time_seconds);
    }

    pub fn advance_by(&mut self, frame_time_seconds: f64) {
        let clamped_frame_time = frame_time_seconds.clamp(0.0, self.config.max_frame_time_seconds);

        self.delta_seconds = clamped_frame_time as f32;
        self.elapsed_seconds += clamped_frame_time;
        self.frame_count = self.frame_count.saturating_add(1);
        self.accumulator_seconds += clamped_frame_time;

        if self.delta_seconds <= 0.0 {
            self.fps_instant = 0.0;
            self.recalculate_rolling_metrics();
            return;
        }

        self.fps_instant = 1.0 / self.delta_seconds;
        self.frame_history_seconds.push_back(clamped_frame_time);
        self.frame_history_sum_seconds += clamped_frame_time;
        self.frame_history_sum_squares_seconds += clamped_frame_time * clamped_frame_time;

        while self.frame_history_seconds.len() > self.config.fps_average_window_samples {
            if let Some(removed) = self.frame_history_seconds.pop_front() {
                self.frame_history_sum_seconds -= removed;
                self.frame_history_sum_squares_seconds -= removed * removed;
            }
        }

        self.recalculate_rolling_metrics();
    }

    fn recalculate_rolling_metrics(&mut self) {
        let frame_count = self.frame_history_seconds.len();

        if frame_count == 0 || self.frame_history_sum_seconds <= 0.0 {
            self.fps_rolling = 0.0;
            self.jitter_seconds = 0.0;
            return;
        }

        self.fps_rolling = (frame_count as f64 / self.frame_history_sum_seconds) as f32;

        if frame_count == 1 {
            self.jitter_seconds = 0.0;
            return;
        }

        let sample_count = frame_count as f64;
        let mean = self.frame_history_sum_seconds / sample_count;
        let mean_square = self.frame_history_sum_squares_seconds / sample_count;
        let variance = (mean_square - (mean * mean)).max(0.0);
        self.jitter_seconds = variance.sqrt() as f32;
    }

    pub fn fixed_steps(&mut self) -> FixedStepIterator<'_> {
        FixedStepIterator {
            accumulator_seconds: &mut self.accumulator_seconds,
            fixed_timestep_seconds: self.config.fixed_timestep_seconds,
        }
    }

    pub fn alpha(&self) -> f32 {
        if self.config.fixed_timestep_seconds <= 0.0 {
            return 0.0;
        }

        (self.accumulator_seconds / self.config.fixed_timestep_seconds) as f32
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    pub fn fixed_delta_seconds(&self) -> f32 {
        self.config.fixed_timestep_seconds as f32
    }

    pub fn elapsed_seconds(&self) -> f64 {
        self.elapsed_seconds
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn fps(&self) -> f32 {
        self.fps_rolling
    }

    pub fn instant_fps(&self) -> f32 {
        self.fps_instant
    }

    pub fn jitter_seconds(&self) -> f32 {
        self.jitter_seconds
    }

    pub fn jitter_milliseconds(&self) -> f32 {
        self.jitter_seconds * 1000.0
    }

    pub fn fps_average_window_samples(&self) -> usize {
        self.config.fps_average_window_samples
    }
}

pub struct FixedStepIterator<'a> {
    accumulator_seconds: &'a mut f64,
    fixed_timestep_seconds: f64,
}

impl<'a> Iterator for FixedStepIterator<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.fixed_timestep_seconds <= 0.0 {
            return None;
        }

        if *self.accumulator_seconds >= self.fixed_timestep_seconds {
            *self.accumulator_seconds -= self.fixed_timestep_seconds;
            return Some(self.fixed_timestep_seconds as f32);
        }

        None
    }
}

#[cfg(test)]
#[path = "time_tests.rs"]
mod tests;

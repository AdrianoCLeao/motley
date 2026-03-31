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
    accumulator_seconds: f64,
    frame_history_seconds: VecDeque<f64>,
    frame_history_sum_seconds: f64,
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
            accumulator_seconds: 0.0,
            frame_history_seconds: VecDeque::with_capacity(fps_average_window_samples),
            frame_history_sum_seconds: 0.0,
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
            return;
        }

        self.fps_instant = 1.0 / self.delta_seconds;
        self.frame_history_seconds.push_back(clamped_frame_time);
        self.frame_history_sum_seconds += clamped_frame_time;

        while self.frame_history_seconds.len() > self.config.fps_average_window_samples {
            if let Some(removed) = self.frame_history_seconds.pop_front() {
                self.frame_history_sum_seconds -= removed;
            }
        }

        if self.frame_history_sum_seconds > 0.0 {
            self.fps_rolling =
                (self.frame_history_seconds.len() as f64 / self.frame_history_sum_seconds) as f32;
        }
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
mod tests {
    use super::{Time, TimeConfig};

    fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn advance_by_caps_large_frame_time() {
        let mut time = Time::with_config(TimeConfig {
            fixed_timestep_seconds: 0.1,
            max_frame_time_seconds: 0.25,
            fps_average_window_samples: 4,
        });

        time.advance_by(1.5);
        let steps: Vec<f32> = time.fixed_steps().collect();

        assert_eq!(steps.len(), 2);
        assert_approx_eq(time.delta_seconds() as f64, 0.25, 1e-6);
        assert_approx_eq(time.elapsed_seconds(), 0.25, 1e-6);
        assert_eq!(time.frame_count(), 1);
        assert_approx_eq(time.alpha() as f64, 0.5, 1e-6);
        assert_approx_eq(time.instant_fps() as f64, 4.0, 1e-6);
        assert_approx_eq(time.fps() as f64, 4.0, 1e-6);
    }

    #[test]
    fn advance_by_negative_frame_time_is_clamped_to_zero() {
        let mut time = Time::with_config(TimeConfig {
            fixed_timestep_seconds: 0.1,
            max_frame_time_seconds: 1.0,
            fps_average_window_samples: 4,
        });

        time.advance_by(-1.0);
        let steps: Vec<f32> = time.fixed_steps().collect();

        assert!(steps.is_empty());
        assert_approx_eq(time.delta_seconds() as f64, 0.0, 1e-6);
        assert_approx_eq(time.elapsed_seconds(), 0.0, 1e-6);
        assert_eq!(time.frame_count(), 1);
        assert_approx_eq(time.alpha() as f64, 0.0, 1e-6);
        assert_approx_eq(time.instant_fps() as f64, 0.0, 1e-6);
        assert_approx_eq(time.fps() as f64, 0.0, 1e-6);
    }

    #[test]
    fn fixed_steps_keep_remainder_for_interpolation() {
        let mut time = Time::with_config(TimeConfig {
            fixed_timestep_seconds: 1.0 / 60.0,
            max_frame_time_seconds: 1.0,
            fps_average_window_samples: 8,
        });

        time.advance_by(0.05);
        let steps: Vec<f32> = time.fixed_steps().collect();

        assert_eq!(steps.len(), 3);
        assert!(time.alpha() >= 0.0);
        assert!(time.alpha() < 1.0);
    }

    #[test]
    fn rolling_fps_smooths_spikes() {
        let mut time = Time::with_config(TimeConfig {
            fixed_timestep_seconds: 1.0 / 60.0,
            max_frame_time_seconds: 1.0,
            fps_average_window_samples: 4,
        });

        time.advance_by(0.1);
        time.advance_by(0.1);
        time.advance_by(0.1);
        time.advance_by(1.0 / 60.0);

        assert!(time.instant_fps() > time.fps());
        assert_approx_eq(time.instant_fps() as f64, 60.0, 1e-3);
        assert_approx_eq(time.fps() as f64, 4.0 / (0.3 + 1.0 / 60.0), 1e-3);
    }

    #[test]
    fn rolling_window_discards_old_samples() {
        let mut time = Time::with_config(TimeConfig {
            fixed_timestep_seconds: 1.0 / 60.0,
            max_frame_time_seconds: 1.0,
            fps_average_window_samples: 2,
        });

        time.advance_by(0.1);
        time.advance_by(0.1);
        time.advance_by(0.05);

        assert_eq!(time.fps_average_window_samples(), 2);
        assert_approx_eq(time.fps() as f64, 2.0 / 0.15, 1e-6);
    }
}

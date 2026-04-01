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

#[test]
fn jitter_is_zero_for_stable_frame_time() {
    let mut time = Time::with_config(TimeConfig {
        fixed_timestep_seconds: 1.0 / 60.0,
        max_frame_time_seconds: 1.0,
        fps_average_window_samples: 16,
    });

    for _ in 0..10 {
        time.advance_by(1.0 / 60.0);
    }

    assert_approx_eq(time.jitter_seconds() as f64, 0.0, 1e-9);
    assert_approx_eq(time.jitter_milliseconds() as f64, 0.0, 1e-6);
}

#[test]
fn jitter_increases_when_frame_spike_occurs() {
    let mut time = Time::with_config(TimeConfig {
        fixed_timestep_seconds: 1.0 / 60.0,
        max_frame_time_seconds: 1.0,
        fps_average_window_samples: 8,
    });

    for _ in 0..6 {
        time.advance_by(1.0 / 60.0);
    }

    let baseline_jitter = time.jitter_seconds();
    time.advance_by(0.1);

    assert!(time.jitter_seconds() > baseline_jitter);
    assert!(time.jitter_milliseconds() > 1.0);
}

#[test]
fn long_pause_is_capped_and_recovers_without_backlog() {
    let mut time = Time::with_config(TimeConfig {
        fixed_timestep_seconds: 0.05,
        max_frame_time_seconds: 0.25,
        fps_average_window_samples: 8,
    });

    time.advance_by(5.0);
    let paused_steps: Vec<f32> = time.fixed_steps().collect();
    let paused_frame_delta = time.delta_seconds();

    time.advance_by(0.05);
    let resumed_steps: Vec<f32> = time.fixed_steps().collect();

    assert_eq!(paused_steps.len(), 5);
    assert_eq!(resumed_steps.len(), 1);
    assert_approx_eq(paused_frame_delta as f64, 0.25, 1e-6);
    assert_approx_eq(time.elapsed_seconds(), 0.30, 1e-6);
    assert_eq!(time.frame_count(), 2);
}

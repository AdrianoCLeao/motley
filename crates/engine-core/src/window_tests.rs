use super::frame_interval_from_refresh_rate;

#[test]
fn returns_none_when_vsync_is_disabled() {
    assert!(frame_interval_from_refresh_rate(false, Some(60_000)).is_none());
}

#[test]
fn uses_monitor_refresh_rate_when_available() {
    let interval = frame_interval_from_refresh_rate(true, Some(120_000)).expect("interval");
    assert!((interval.as_secs_f64() - 1.0 / 120.0).abs() < 1e-9);
}

#[test]
fn uses_default_refresh_rate_fallback() {
    let interval = frame_interval_from_refresh_rate(true, None).expect("interval");
    assert!((interval.as_secs_f64() - 1.0 / 60.0).abs() < 1e-9);
}

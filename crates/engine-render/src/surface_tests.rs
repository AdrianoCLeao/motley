use crate::surface::choose_present_mode;

#[test]
fn choose_present_mode_returns_fifo_when_vsync_is_enabled() {
    let mode = choose_present_mode(true, &[wgpu::PresentMode::Immediate]);

    assert_eq!(mode, wgpu::PresentMode::Fifo);
}

#[test]
fn choose_present_mode_prefers_low_latency_modes_when_vsync_is_disabled() {
    let mode = choose_present_mode(
        false,
        &[
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::FifoRelaxed,
            wgpu::PresentMode::Mailbox,
        ],
    );

    assert_eq!(mode, wgpu::PresentMode::Mailbox);
}

#[test]
fn choose_present_mode_falls_back_to_fifo_when_no_mode_matches() {
    let mode = choose_present_mode(false, &[]);

    assert_eq!(mode, wgpu::PresentMode::Fifo);
}

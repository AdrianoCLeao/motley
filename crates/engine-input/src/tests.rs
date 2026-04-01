use super::{apply_deadzone, BufferedWindowEvent, InputModule, InputState};
use engine_core::HardeningConfig;
use gilrs::{Axis, Button};
use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

#[test]
fn key_just_pressed_is_only_true_on_first_press_frame() {
    let mut input = InputState::default();

    input.begin_frame();
    input.process_key_input(KeyCode::Space, ElementState::Pressed, false);

    assert!(input.key_just_pressed(KeyCode::Space));
    assert!(input.key_held(KeyCode::Space));

    input.begin_frame();

    assert!(!input.key_just_pressed(KeyCode::Space));
    assert!(input.key_held(KeyCode::Space));
}

#[test]
fn repeat_press_does_not_create_new_just_pressed_event() {
    let mut input = InputState::default();

    input.begin_frame();
    input.process_key_input(KeyCode::KeyW, ElementState::Pressed, false);
    assert!(input.key_just_pressed(KeyCode::KeyW));

    input.begin_frame();
    input.process_key_input(KeyCode::KeyW, ElementState::Pressed, true);
    assert!(!input.key_just_pressed(KeyCode::KeyW));
    assert!(input.key_held(KeyCode::KeyW));
}

#[test]
fn key_release_moves_state_to_just_released() {
    let mut input = InputState::default();

    input.begin_frame();
    input.process_key_input(KeyCode::KeyA, ElementState::Pressed, false);

    input.begin_frame();
    input.process_key_input(KeyCode::KeyA, ElementState::Released, false);

    assert!(input.key_just_released(KeyCode::KeyA));
    assert!(!input.key_held(KeyCode::KeyA));
}

#[test]
fn mouse_delta_accumulates_and_resets_per_frame() {
    let mut input = InputState::default();

    input.begin_frame();
    input.process_cursor_position(10.0, 20.0);
    input.process_cursor_position(110.0, 40.0);

    assert_eq!(input.mouse_delta.x, 100.0);
    assert_eq!(input.mouse_delta.y, 20.0);

    input.begin_frame();
    assert_eq!(input.mouse_delta.x, 0.0);
    assert_eq!(input.mouse_delta.y, 0.0);
}

#[test]
fn mouse_button_press_and_release_have_frame_scoped_markers() {
    let mut input = InputState::default();

    input.begin_frame();
    input.process_mouse_button_input(MouseButton::Right, ElementState::Pressed);

    assert!(input.mouse_just_pressed(MouseButton::Right));
    assert!(input.mouse_held(MouseButton::Right));

    input.begin_frame();
    assert!(!input.mouse_just_pressed(MouseButton::Right));
    assert!(input.mouse_held(MouseButton::Right));

    input.process_mouse_button_input(MouseButton::Right, ElementState::Released);
    assert!(input.mouse_just_released(MouseButton::Right));
    assert!(!input.mouse_held(MouseButton::Right));
}

#[test]
fn gamepad_deadzone_filters_small_axis_values() {
    let mut input = InputState::default();
    input.begin_frame();
    input.process_gamepad_axis_input(0, Axis::LeftStickX, 0.1);

    assert_eq!(input.gamepad_axis(0, Axis::LeftStickX), 0.0);

    input.process_gamepad_axis_input(0, Axis::LeftStickX, 0.35);
    assert_eq!(input.gamepad_axis(0, Axis::LeftStickX), 0.35);
}

#[test]
fn gamepad_button_semantics_match_keyboard_semantics() {
    let mut input = InputState::default();
    input.set_gamepad_connected(0, true);

    input.begin_frame();
    input.process_gamepad_button_input(0, Button::South, true);
    assert!(input.gamepad_button_just_pressed(0, Button::South));
    assert!(input.gamepad_button_held(0, Button::South));

    input.begin_frame();
    assert!(!input.gamepad_button_just_pressed(0, Button::South));
    assert!(input.gamepad_button_held(0, Button::South));

    input.process_gamepad_button_input(0, Button::South, false);
    assert!(input.gamepad_button_just_released(0, Button::South));
    assert!(!input.gamepad_button_held(0, Button::South));
}

#[test]
fn deadzone_helper_is_symmetric() {
    assert_eq!(apply_deadzone(0.1, 0.15), 0.0);
    assert_eq!(apply_deadzone(-0.1, 0.15), 0.0);
    assert_eq!(apply_deadzone(0.2, 0.15), 0.2);
    assert_eq!(apply_deadzone(-0.2, 0.15), -0.2);
}

#[test]
fn buffered_events_soft_fail_when_capacity_is_reached() {
    let mut module = InputModule::new();
    module.configure_hardening(HardeningConfig {
        max_buffered_input_events: 1,
        ..HardeningConfig::default()
    });

    module.push_buffered_event(BufferedWindowEvent::MouseWheel { delta: 1.0 });
    module.push_buffered_event(BufferedWindowEvent::MouseWheel { delta: 2.0 });

    assert_eq!(module.buffered_events.len(), 1);
}

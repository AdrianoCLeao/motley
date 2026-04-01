use bevy_ecs::prelude::Resource;
use engine_core::Result;
use engine_math::Vec2;
use gilrs::{Axis, Button, EventType, Gilrs};
use std::collections::{HashMap, HashSet};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

pub const DEFAULT_GAMEPAD_DEADZONE: f32 = 0.15;

#[derive(Debug, Clone, Default)]
pub struct GamepadState {
    connected: bool,
    buttons_held: HashSet<Button>,
    buttons_pressed: HashSet<Button>,
    buttons_released: HashSet<Button>,
    axes: HashMap<Axis, f32>,
}

#[derive(Resource, Debug, Clone)]
pub struct InputState {
    keys_held: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    mouse_buttons_held: HashSet<MouseButton>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_buttons_released: HashSet<MouseButton>,
    pub mouse_position: Vec2,
    pub mouse_delta: Vec2,
    pub scroll_delta: f32,
    gamepads: HashMap<usize, GamepadState>,
    gamepad_deadzone: f32,
    last_cursor_position: Option<Vec2>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            keys_held: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            mouse_buttons_held: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            mouse_buttons_released: HashSet::new(),
            mouse_position: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            scroll_delta: 0.0,
            gamepads: HashMap::new(),
            gamepad_deadzone: DEFAULT_GAMEPAD_DEADZONE,
            last_cursor_position: None,
        }
    }
}

impl InputState {
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_buttons_pressed.clear();
        self.mouse_buttons_released.clear();
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = 0.0;

        for gamepad in self.gamepads.values_mut() {
            gamepad.buttons_pressed.clear();
            gamepad.buttons_released.clear();
        }
    }

    pub fn process_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => self.process_key_event(event),
            WindowEvent::MouseInput { state, button, .. } => {
                self.process_mouse_button_input(*button, *state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.process_cursor_position(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 120.0,
                };
                self.process_scroll_delta(scroll);
            }
            _ => {}
        }
    }

    pub fn process_key_event(&mut self, event: &winit::event::KeyEvent) {
        if let PhysicalKey::Code(code) = event.physical_key {
            self.process_key_input(code, event.state, event.repeat);
        }
    }

    pub fn process_key_input(&mut self, code: KeyCode, state: ElementState, repeat: bool) {
        match state {
            ElementState::Pressed if !repeat => {
                if self.keys_held.insert(code) {
                    self.keys_pressed.insert(code);
                }
            }
            ElementState::Released => {
                if self.keys_held.remove(&code) {
                    self.keys_released.insert(code);
                }
            }
            _ => {}
        }
    }

    pub fn process_mouse_button_input(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.mouse_buttons_held.insert(button) {
                    self.mouse_buttons_pressed.insert(button);
                }
            }
            ElementState::Released => {
                if self.mouse_buttons_held.remove(&button) {
                    self.mouse_buttons_released.insert(button);
                }
            }
        }
    }

    pub fn process_cursor_position(&mut self, x: f32, y: f32) {
        let next = Vec2::new(x, y);

        if let Some(last) = self.last_cursor_position {
            self.mouse_delta += next - last;
        }

        self.last_cursor_position = Some(next);
        self.mouse_position = next;
    }

    pub fn process_scroll_delta(&mut self, delta: f32) {
        self.scroll_delta += delta;
    }

    pub fn set_gamepad_connected(&mut self, gamepad_slot: usize, connected: bool) {
        let gamepad = self.gamepads.entry(gamepad_slot).or_default();
        gamepad.connected = connected;
        if !connected {
            gamepad.buttons_held.clear();
            gamepad.axes.clear();
        }
    }

    pub fn process_gamepad_button_input(
        &mut self,
        gamepad_slot: usize,
        button: Button,
        pressed: bool,
    ) {
        let gamepad = self.gamepads.entry(gamepad_slot).or_default();
        if pressed {
            if gamepad.buttons_held.insert(button) {
                gamepad.buttons_pressed.insert(button);
            }
        } else if gamepad.buttons_held.remove(&button) {
            gamepad.buttons_released.insert(button);
        }
    }

    pub fn process_gamepad_axis_input(&mut self, gamepad_slot: usize, axis: Axis, value: f32) {
        let normalized = apply_deadzone(value, self.gamepad_deadzone);
        let gamepad = self.gamepads.entry(gamepad_slot).or_default();
        gamepad.axes.insert(axis, normalized);
    }

    pub fn set_gamepad_deadzone(&mut self, deadzone: f32) {
        self.gamepad_deadzone = deadzone.clamp(0.0, 1.0);
    }

    pub fn key_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn key_just_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.mouse_buttons_held.contains(&button)
    }

    pub fn mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn mouse_just_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_released.contains(&button)
    }

    pub fn first_connected_gamepad(&self) -> Option<usize> {
        self.gamepads
            .iter()
            .find_map(|(slot, state)| state.connected.then_some(*slot))
    }

    pub fn gamepad_button_held(&self, gamepad_slot: usize, button: Button) -> bool {
        self.gamepads
            .get(&gamepad_slot)
            .is_some_and(|state| state.buttons_held.contains(&button))
    }

    pub fn gamepad_button_just_pressed(&self, gamepad_slot: usize, button: Button) -> bool {
        self.gamepads
            .get(&gamepad_slot)
            .is_some_and(|state| state.buttons_pressed.contains(&button))
    }

    pub fn gamepad_button_just_released(&self, gamepad_slot: usize, button: Button) -> bool {
        self.gamepads
            .get(&gamepad_slot)
            .is_some_and(|state| state.buttons_released.contains(&button))
    }

    pub fn gamepad_axis(&self, gamepad_slot: usize, axis: Axis) -> f32 {
        self.gamepads
            .get(&gamepad_slot)
            .and_then(|state| state.axes.get(&axis).copied())
            .unwrap_or(0.0)
    }
}

fn apply_deadzone(value: f32, deadzone: f32) -> f32 {
    if value.abs() < deadzone {
        0.0
    } else {
        value
    }
}

#[derive(Debug, Clone)]
enum BufferedWindowEvent {
    Key {
        code: KeyCode,
        state: ElementState,
        repeat: bool,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
    CursorMoved {
        x: f32,
        y: f32,
    },
    MouseWheel {
        delta: f32,
    },
}

pub struct InputModule {
    buffered_events: Vec<BufferedWindowEvent>,
    gilrs: Option<Gilrs>,
}

impl Default for InputModule {
    fn default() -> Self {
        Self::new()
    }
}

impl InputModule {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => Some(gilrs),
            Err(error) => {
                log::warn!(
                    target: "engine::input",
                    "Gamepad backend unavailable at startup: {}",
                    error
                );
                None
            }
        };

        Self {
            buffered_events: Vec::new(),
            gilrs,
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> Result<()> {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return Ok(());
                };

                self.buffered_events.push(BufferedWindowEvent::Key {
                    code,
                    state: event.state,
                    repeat: event.repeat,
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.buffered_events.push(BufferedWindowEvent::MouseButton {
                    button: *button,
                    state: *state,
                });
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.buffered_events.push(BufferedWindowEvent::CursorMoved {
                    x: position.x as f32,
                    y: position.y as f32,
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 120.0,
                };

                self.buffered_events
                    .push(BufferedWindowEvent::MouseWheel { delta: scroll });
            }
            _ => {}
        }

        Ok(())
    }

    pub fn pump(&mut self, input_state: &mut InputState) -> Result<()> {
        input_state.begin_frame();

        for event in self.buffered_events.drain(..) {
            match event {
                BufferedWindowEvent::Key {
                    code,
                    state,
                    repeat,
                } => input_state.process_key_input(code, state, repeat),
                BufferedWindowEvent::MouseButton { button, state } => {
                    input_state.process_mouse_button_input(button, state)
                }
                BufferedWindowEvent::CursorMoved { x, y } => {
                    input_state.process_cursor_position(x, y)
                }
                BufferedWindowEvent::MouseWheel { delta } => {
                    input_state.process_scroll_delta(delta)
                }
            }
        }

        if let Some(gilrs) = self.gilrs.as_mut() {
            while let Some(event) = gilrs.next_event() {
                let gamepad_slot: usize = event.id.into();
                match event.event {
                    EventType::Connected => input_state.set_gamepad_connected(gamepad_slot, true),
                    EventType::Disconnected => {
                        input_state.set_gamepad_connected(gamepad_slot, false)
                    }
                    EventType::ButtonPressed(button, _) | EventType::ButtonRepeated(button, _) => {
                        input_state.process_gamepad_button_input(gamepad_slot, button, true)
                    }
                    EventType::ButtonReleased(button, _) => {
                        input_state.process_gamepad_button_input(gamepad_slot, button, false)
                    }
                    EventType::ButtonChanged(button, value, _) => {
                        input_state.process_gamepad_button_input(
                            gamepad_slot,
                            button,
                            value >= input_state.gamepad_deadzone,
                        );
                    }
                    EventType::AxisChanged(axis, value, _) => {
                        input_state.process_gamepad_axis_input(gamepad_slot, axis, value)
                    }
                    EventType::Dropped | EventType::ForceFeedbackEffectCompleted => {}
                    _ => {}
                }
            }
        }

        log::trace!(target: "engine::input", "Input events pumped");
        Ok(())
    }

    pub fn backend_type_names(&self) -> (&'static str, &'static str) {
        (
            std::any::type_name::<winit::event::ElementState>(),
            std::any::type_name::<gilrs::GamepadId>(),
        )
    }
}

pub fn module_name() -> &'static str {
    "engine-input"
}

#[cfg(test)]
mod tests {
    use super::{apply_deadzone, InputState};
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
}

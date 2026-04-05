use crate::{EngineError, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

const DEFAULT_VSYNC_FALLBACK_REFRESH_RATE_MILLIHZ: u32 = 60_000;
const ESCAPE_CONFIRM_WINDOW: Duration = Duration::from_secs(2);

fn default_frame_interval() -> Duration {
    Duration::from_secs_f64(1000.0 / DEFAULT_VSYNC_FALLBACK_REFRESH_RATE_MILLIHZ as f64)
}

pub trait WindowLoop {
    fn window_created(&mut self, _window: Arc<Window>, _config: &WindowConfig) -> Result<()> {
        Ok(())
    }

    fn tick(&mut self) -> Result<()>;

    fn window_event(&mut self, _event: &WindowEvent) -> Result<()> {
        Ok(())
    }

    fn resized(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }

    fn title(&self) -> String {
        "Starman".to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Starman".to_owned(),
            width: 1280,
            height: 720,
            resizable: true,
            vsync: true,
        }
    }
}

impl WindowConfig {
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }
}

pub fn run_windowed<A>(config: WindowConfig, app: A) -> Result<()>
where
    A: WindowLoop + 'static,
{
    let initial_control_flow = if config.vsync {
        ControlFlow::Wait
    } else {
        ControlFlow::Poll
    };

    let event_loop = EventLoop::new()
        .map_err(|error| EngineError::Window(format!("failed to create event loop: {error}")))?;

    event_loop.set_control_flow(initial_control_flow);

    let mut runner = WindowRunner::new(config, app);
    let event_loop_result = event_loop.run_app(&mut runner);

    if let Some(error) = runner.take_error() {
        return Err(error);
    }

    event_loop_result
        .map_err(|error| EngineError::Window(format!("event loop exited with error: {error}")))
}

struct WindowRunner<A: WindowLoop> {
    config: WindowConfig,
    app: A,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    frame_interval: Option<Duration>,
    next_redraw_deadline: Option<Instant>,
    last_escape_press: Option<Instant>,
    error: Option<EngineError>,
}

impl<A: WindowLoop> WindowRunner<A> {
    fn new(config: WindowConfig, app: A) -> Self {
        Self {
            frame_interval: frame_interval_from_refresh_rate(config.vsync, None),
            config,
            app,
            window: None,
            window_id: None,
            next_redraw_deadline: None,
            last_escape_press: None,
            error: None,
        }
    }

    fn maybe_confirm_escape_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &WindowEvent,
    ) -> bool {
        let WindowEvent::KeyboardInput { event, .. } = event else {
            return false;
        };

        if event.state != ElementState::Pressed || event.repeat {
            return false;
        }

        if !matches!(event.physical_key, PhysicalKey::Code(KeyCode::Escape)) {
            return false;
        }

        let now = Instant::now();
        if let Some(last_press) = self.last_escape_press {
            if now.duration_since(last_press) <= ESCAPE_CONFIRM_WINDOW {
                log::info!(target: "engine::window", "Escape confirmation received, exiting application");
                event_loop.exit();
                self.last_escape_press = None;
                return true;
            }
        }

        self.last_escape_press = Some(now);
        log::warn!(
            target: "engine::window",
            "Press Escape again within 2 seconds to exit"
        );
        true
    }

    fn take_error(&mut self) -> Option<EngineError> {
        self.error.take()
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: EngineError) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl<A: WindowLoop> ApplicationHandler for WindowRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = WindowAttributes::default()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .with_resizable(self.config.resizable);

        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                self.frame_interval = frame_interval_from_refresh_rate(
                    self.config.vsync,
                    window
                        .current_monitor()
                        .and_then(|monitor| monitor.refresh_rate_millihertz()),
                );
                self.window_id = Some(window.id());
                self.next_redraw_deadline = Some(Instant::now());

                if let Err(error) = self.app.window_created(window.clone(), &self.config) {
                    self.fail(event_loop, error);
                    return;
                }

                if !self.config.vsync {
                    event_loop.set_control_flow(ControlFlow::Poll);
                }

                window.request_redraw();
                self.window = Some(window);
            }
            Err(error) => {
                self.fail(
                    event_loop,
                    EngineError::Window(format!("failed to create window: {error}")),
                );
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window_id != Some(window_id) {
            return;
        }

        if let Err(error) = self.app.window_event(&event) {
            self.fail(event_loop, error);
            return;
        }

        if self.maybe_confirm_escape_exit(event_loop, &event) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Err(error) = self.app.resized(size.width, size.height) {
                    self.fail(event_loop, error);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.app.tick() {
                    self.fail(event_loop, error);
                    return;
                }

                if let Some(window) = self.window.as_ref() {
                    window.set_title(&self.app.title());
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.config.vsync {
            event_loop.set_control_flow(ControlFlow::Poll);
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return;
        }

        let Some(window) = self.window.as_ref() else {
            return;
        };

        let frame_interval = self.frame_interval.unwrap_or_else(|| {
            debug_assert!(
                false,
                "vsync enabled but frame_interval was not initialized"
            );
            default_frame_interval()
        });

        let now = Instant::now();
        let deadline = self.next_redraw_deadline.unwrap_or(now);

        if now >= deadline {
            window.request_redraw();
            let next_deadline = now + frame_interval;
            self.next_redraw_deadline = Some(next_deadline);
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_deadline));
            return;
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
    }
}

fn frame_interval_from_refresh_rate(
    vsync_enabled: bool,
    refresh_rate_millihertz: Option<u32>,
) -> Option<Duration> {
    if !vsync_enabled {
        return None;
    }

    let millihertz = refresh_rate_millihertz
        .filter(|rate| *rate > 0)
        .unwrap_or(DEFAULT_VSYNC_FALLBACK_REFRESH_RATE_MILLIHZ);

    Some(Duration::from_secs_f64(1000.0 / millihertz as f64))
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;

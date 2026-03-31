use crate::{EngineError, Result};
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

const DEFAULT_VSYNC_FALLBACK_REFRESH_RATE_MILLIHZ: u32 = 60_000;

pub trait WindowLoop {
    fn tick(&mut self) -> Result<()>;

    fn resized(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }

    fn title(&self) -> String {
        "Motley".to_owned()
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
            title: "Motley".to_owned(),
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
    window: Option<Window>,
    window_id: Option<WindowId>,
    frame_interval: Option<Duration>,
    next_redraw_deadline: Option<Instant>,
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
            error: None,
        }
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
                self.frame_interval = frame_interval_from_refresh_rate(
                    self.config.vsync,
                    window
                        .current_monitor()
                        .and_then(|monitor| monitor.refresh_rate_millihertz()),
                );
                self.window_id = Some(window.id());
                self.next_redraw_deadline = Some(Instant::now());

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

                    if !self.config.vsync {
                        window.request_redraw();
                    }
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

        let Some(frame_interval) = self.frame_interval else {
            event_loop.set_control_flow(ControlFlow::Wait);
            window.request_redraw();
            return;
        };

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
mod tests {
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
}

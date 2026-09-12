use winit::event_loop::{ControlFlow, EventLoop};

use crate::window::window::WindowConfig;
use crate::app::runner::AppRunner;

pub struct App {
    pub window_config: WindowConfig,
}

impl App {
    pub fn new() -> Self {
        Self {
            window_config: WindowConfig::default(),
        }
    }

    pub fn set_window(mut self, config: WindowConfig) -> Self {
        self.window_config = config;
        self
    }

    pub fn run(mut self) {
        let event_loop = EventLoop::new().expect("Failed to create EventLoop");
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut runner = AppRunner::new(self);
        event_loop.run_app(&mut runner).expect("Error running event loop");
    }
}
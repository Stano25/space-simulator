use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::app::app::App;   

pub struct AppRunner {
    app: App,
    window: Option<Arc<Window>>,
}

impl AppRunner {
    pub fn new(app: App) -> Self {
        Self {
            app,
            window: None,
        }
    }
}

impl ApplicationHandler for AppRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let attributes = self.app.window_config.to_attributes();
        let raw_window = event_loop
            .create_window(attributes)
            .expect("Failed to create window");

        let window = Arc::new(raw_window);
        self.window = Some(window.clone());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::app::app::App;
use crate::input::input::InputState;
use crate::time::time::Time;
use crate::render::{context::RenderContext,
                    camera::Projection};

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
    fn resumed(&mut self, event_loop: &ActiveEventLoop) { // Called when the application enters the active state and is ready to initialize graphics.
        if self.window.is_some() { return; }

        let attributes = self.app.window_config.to_attributes();
        let raw_window = event_loop
            .create_window(attributes)
            .expect("Failed to create window");

        let window = Arc::new(raw_window);
        self.window = Some(window.clone());

        self.app.init_resources(window);

        self.app.startup_schedule.run(&mut self.app.world);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.app.render_schedule.run(&mut self.app.world);
            }
            WindowEvent::KeyboardInput { event, ..} => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    let mut input = self.app.world.resource_mut::<InputState>();

                    match event.state {
                        ElementState::Pressed => {
                            input.keyboard.pressed_keys.insert(key_code);
                            if !event.repeat {
                                input.keyboard.just_pressed_keys.insert(key_code);
                            }
                        }
                        ElementState::Released => {
                            input.keyboard.pressed_keys.remove(&key_code);
                            input.keyboard.just_released_keys.insert(key_code);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let mut input = self.app.world.resource_mut::<InputState>();

                match state {
                    ElementState::Pressed => {
                        input.mouse.pressed_buttons.insert(button);
                        input.mouse.just_pressed_buttons.insert(button);
                    }
                    ElementState::Released => {
                        input.mouse.pressed_buttons.remove(&button);
                        input.mouse.just_released_buttons.insert(button);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(x, y) => glam::vec2(x, y),
                    MouseScrollDelta::PixelDelta(pos) => glam::vec2(pos.x as f32, pos.y as f32),
                };

                let mut input = self.app.world.resource_mut::<InputState>();
                
                input.mouse.scroll_delta += scroll;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mut input = self.app.world.resource_mut::<InputState>();
                let new_position = glam::Vec2::new(position.x as f32, position.y as f32);

                let delta = new_position - input.mouse.position;

                input.mouse.delta += delta;
                input.mouse.position = new_position;
            }
            WindowEvent::Resized(new_size) => {
                if let Some(mut render_context) = self.app.world.get_resource_mut::<RenderContext>() {
                    render_context.resize(new_size.width, new_size.height);
                }

                if let Some(mut projection) = self.app.world.get_resource_mut::<Projection>() {
                    projection.resize(new_size.width as f32 / new_size.height as f32);
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.app.world.resource_mut::<Time>().update();

        self.app.update_schedule.run(&mut self.app.world);

        let mut input = self.app.world.resource_mut::<InputState>();
        input.clear_frame_states();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
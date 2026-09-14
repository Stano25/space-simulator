use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::app::app::App;
use crate::input::input::InputState;
use crate::render::context::RenderContext;
use crate::render::pipeline::{PipelineBuilder, PipelineRegistry};

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

        let render_context = pollster::block_on(RenderContext::new(window.clone()));

        let default_pipeline = PipelineBuilder::new(include_str!("../../../../assets/shaders/shader.wgsl"))
            .with_pixel_format(render_context.config.format)
            .build(&render_context.device);

        {
            let mut pipeline_registry = self.app.world.resource_mut::<PipelineRegistry>();
            pipeline_registry.pipelines.insert("default".into(), default_pipeline);
        }
        
        self.app.world.insert_resource(render_context);
        self.app.startup_schedule.run(&mut self.app.world);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
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
                            input.keyboard.just_pressed_keys.insert(key_code);
                        }
                        ElementState::Released => {
                            input.keyboard.pressed_keys.remove(&key_code);
                            input.keyboard.just_released_keys.insert(key_code);
                        }
                    }
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(mut render_context) = self.app.world.get_resource_mut::<RenderContext>() {
                    render_context.resize(new_size.width, new_size.height);
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.app.update_schedule.run(&mut self.app.world);

        let mut input = self.app.world.resource_mut::<InputState>();
        input.clear_frame_states();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
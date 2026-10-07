use std::sync::Arc;

use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::ScheduleSystem;

use crate::time::time::Time;
use crate::window::window::WindowConfig;
use crate::app::runner::AppRunner;
use crate::input::input::InputState;
use crate::render::{systems::render_system,
                    context::RenderContext,
                    pipeline::{PipelineBuilder, PipelineRegistry},
                    layout::{GpuLayout, EngineLayout},
                    camera::{GpuCamera, Projection}};


#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StartupSchedule;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UpdateSchedule;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderSchedule;

pub struct App {
    pub world: World,
    pub startup_schedule: Schedule,
    pub update_schedule: Schedule,
    pub render_schedule: Schedule,
    pub window_config: WindowConfig,
}

impl App {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            startup_schedule: Schedule::new(StartupSchedule),
            update_schedule: Schedule::new(UpdateSchedule),
            render_schedule: Schedule::new(RenderSchedule),
            window_config: WindowConfig::default(),
        }
    }

    pub fn set_window(mut self, config: WindowConfig) -> Self {
        self.window_config = config;
        self
    }

    pub fn add_startup_system<M>(mut self, system: impl IntoScheduleConfigs<ScheduleSystem, M>) -> Self { // Can add multiple systems at once. Each system is a function that has at least one ECS parameter. The system will be called once at the start of the application.
        self.startup_schedule.add_systems(system);
        self
    }

    pub fn add_system<M>(mut self, system: impl IntoScheduleConfigs<ScheduleSystem, M>) -> Self {
        self.update_schedule.add_systems(system);
        self
    }

    pub fn run(self) {
        // Create the event loop
        let event_loop = EventLoop::new().expect("Failed to create EventLoop");
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut runner = AppRunner::new(self);
        event_loop.run_app(&mut runner).expect("Error running event loop");
    }

    pub fn init_resources(&mut self, window: Arc<Window>) {
        self.world.insert_resource(InputState::default());
        self.world.insert_resource(PipelineRegistry::default());
        self.world.insert_resource(Time::new());
        self.render_schedule.add_systems(render_system);

        let render_context = pollster::block_on(RenderContext::new(window.clone()));

        let gpu_layout = GpuLayout::new(&render_context.device);

        let gpu_camera = GpuCamera::new(&render_context.device, &gpu_layout.get(EngineLayout::Camera));

        let projection = Projection::new(render_context.size.0 as f32 / render_context.size.1 as f32, 45.0_f32.to_radians(), 0.1, 100.0);

        let default_pipeline = PipelineBuilder::new(include_str!("../../../../assets/shaders/shader.wgsl"))
            .with_pixel_format(render_context.config.format)
            .with_layouts(&[&gpu_layout.get(EngineLayout::Material), &gpu_layout.get(EngineLayout::Camera)])
            .build(&render_context.device);

        {
            let mut pipeline_registry = self.world.resource_mut::<PipelineRegistry>();
            pipeline_registry.pipelines.insert("default".into(), default_pipeline);
        }

        self.world.insert_resource(projection);
        self.world.insert_resource(gpu_layout);
        self.world.insert_resource(gpu_camera);
        self.world.insert_resource(render_context);
    }
}

use winit::event_loop::{ControlFlow, EventLoop};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::ScheduleSystem;

use crate::window::window::WindowConfig;
use crate::app::runner::AppRunner;
use crate::input::input::InputState;

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
        let mut world = World::new();

        world.insert_resource(InputState::default());
        
        Self {
            world,
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

    pub fn run(mut self) {
        // Create the event loop
        let event_loop = EventLoop::new().expect("Failed to create EventLoop");
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut runner = AppRunner::new(self);
        event_loop.run_app(&mut runner).expect("Error running event loop");
    }
}
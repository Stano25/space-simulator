use std::time::Instant;

use bevy_ecs::prelude::*;

#[derive(Resource)]
pub struct Time {
    pub(crate) delta_time: f32,
    pub elapsed_time: f32,
    last_frame_time: Instant,
}

impl Time {
    pub fn new() -> Self {
        Self {
            delta_time: 0.0,
            elapsed_time: 0.0,
            last_frame_time: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame_time);
        self.delta_time = delta.as_secs_f32();
        self.elapsed_time += self.delta_time;
        self.last_frame_time = now;
    }

    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    pub fn elapsed_time(&self) -> f32 {
        self.elapsed_time
    }
}

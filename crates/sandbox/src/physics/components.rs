use bevy_ecs::prelude::*;
use glam::DVec3;

pub const RAILS_THRESHOLD: f64 = 10.0;

#[derive(Component)]
pub struct Velocity(pub DVec3);

#[derive(Component)]
pub struct Acceleration(pub DVec3);

#[derive(Component)]
pub struct CelestialBody {
    pub gm: f64, // Gravitational parameter: G * Mass
    pub softening_sq: f32, // Softening radius squared (prevents division by zero on near-collisions)
}

#[derive(Resource)]
pub struct TimeWarp {
    pub factor: f64,
    pub total_sim_time: f64,
    pub was_on_rails: bool, // tracks transition to physics from on-rails method
}

impl Default for TimeWarp {
    fn default() -> Self {
        Self {
            factor: 1.0,
            total_sim_time: 0.0,
            was_on_rails: false,
        }
    }
}

use engine::time::time::Time;
use engine::render::instance::Transform;

use bevy_ecs::prelude::*;
use glam::Vec3;

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Acceleration(pub Vec3);

#[derive(Component)]
pub struct CelestialBody {
    pub gm: f32, // Gravitational parameter: G * Mass
    pub softening_sq: f32, // Softening radius squared (prevents division by zero on near-collisions)
}

pub fn nbody_vverlet_system(
    time: Res<Time>,
    mut bodies: Query<(&mut Transform, &mut Velocity, &mut Acceleration, &CelestialBody)>,
) {
    let dt = time.delta_time();
    let half_dt = 0.5 * dt;

    // PHASE 1: KICK 1 + DRIFT -> All bodies move to time t + dt
    for (mut transform, mut velocity, mut accel, _) in bodies.iter_mut() {
        velocity.0 += accel.0 * half_dt; // Half-step velocity using existing a(t)
        transform.position += (velocity.0 * dt).as_dvec3(); // Full-step position using updated velocity

        accel.0 = Vec3::ZERO; // Clear acceleration: ready to accumulate new forces at t + dt
    }

    // PHASE 2: MUTUAL N-BODY GRAVITY AT NEW POSITIONS (t + dt)
    // Newton's 3rd Law: Pair (A, B) computes pull on both simultaneously
    let mut combinations = bodies.iter_combinations_mut();
    while let Some([
        (transf_a, _, mut accel_a, body_a),
        (transf_b, _, mut accel_b, body_b)
    ]) = combinations.fetch_next() {
        let r_vec = transf_b.position - transf_a.position;
        let dist_sq = r_vec.length_squared() + body_a.softening_sq.max(body_b.softening_sq) as f64;
        let dist = dist_sq.sqrt();
        let dist_cube = dist_sq * dist;

        let dir_over_dist_cube = r_vec / dist_cube;

        // Acceleration on A caused by B: a_A = (GM_B / r^3) * r
        accel_a.0 += dir_over_dist_cube.as_vec3() * body_b.gm;

        // Acceleration on B caused by A: a_B = -(GM_A / r^3) * r
        accel_b.0 -= dir_over_dist_cube.as_vec3() * body_a.gm;
    }

    // PHASE 3: KICK 2 (Complete velocity update with new accelerations)
    for (_, mut velocity, accel, _) in bodies.iter_mut() {
        velocity.0 += accel.0 * half_dt;
        // acc.0 is preserved here
    }
}

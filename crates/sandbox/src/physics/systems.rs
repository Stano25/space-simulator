use std::collections::HashMap;

use crate::physics::components::{CelestialBody, RAILS_THRESHOLD, TimeWarp, Velocity, Acceleration};
use crate::physics::kepler::KeplerOrbit;
use engine::time::time::Time;
use engine::render::instance::Transform;

use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};

pub fn propagation_system(
    time: Res<Time>,
    mut warp: ResMut<TimeWarp>,
    mut bodies: Query<(Entity, &mut Transform, &mut Velocity, &mut Acceleration, &CelestialBody, Option<&mut KeplerOrbit>)>,
) {
    let dt = time.delta_time() as f64;
    let sim_dt = dt * warp.factor;
    warp.total_sim_time += sim_dt;

    if warp.factor < RAILS_THRESHOLD {
        // ---------------------------------
        // Velocity verlet mode
        // ---------------------------------

        // Just exited rails -> Reset accelerations to zero
        if warp.was_on_rails {
            for (_, _, _, mut accel, _, _) in bodies.iter_mut() {
                accel.0 = Vec3::ZERO;
            }
            warp.was_on_rails = false;
        }

        // Sub-stepping for numerical stability
        const SUB_DT: f32 = 1.0 / 120.0;
        let steps = ((sim_dt / SUB_DT as f64).ceil() as u32).clamp(1, 150);
        let true_sub_dt = (sim_dt / steps as f64) as f32;
        let half_dt = 0.5 * true_sub_dt;

        for _ in 0..steps {
            // PHASE 1: KICK 1 + DRIFT -> All bodies move to time t + dt
            for (_, mut transform, mut velocity, mut accel, _, _) in bodies.iter_mut() {
                velocity.0 += accel.0 * half_dt as f32; // Half-step velocity using existing a(t)
                transform.position += (velocity.0 * true_sub_dt as f32).as_dvec3(); // Full-step position using updated velocity

                accel.0 = Vec3::ZERO; // Clear acceleration: ready to accumulate new forces at t + dt
            }

            // PHASE 2: MUTUAL N-BODY GRAVITY AT NEW POSITIONS (t + dt)
            // Newton's 3rd Law: Pair (A, B) computes pull on both simultaneously
            let mut combinations = bodies.iter_combinations_mut();
            while let Some([
                (_, transf_a, _, mut accel_a, body_a, _),
                (_, transf_b, _, mut accel_b, body_b, _)
            ]) = combinations.fetch_next() {
                let r_vec = transf_b.position - transf_a.position;
                let dist_sq = r_vec.length_squared() + body_a.softening_sq.max(body_b.softening_sq) as f64;
                let dist = dist_sq.sqrt();
                let dist_cube = dist_sq * dist;

                let dir_over_dist_cube = r_vec / dist_cube;

                // Acceleration on A caused by B: a_A = (GM_B / r^3) * r
                accel_a.0 += (dir_over_dist_cube * body_b.gm).as_vec3();

                // Acceleration on B caused by A: a_B = -(GM_A / r^3) * r
                accel_b.0 -= (dir_over_dist_cube * body_a.gm).as_vec3();
            }

            // PHASE 3: KICK 2 (Complete velocity update with new accelerations)
            for (_, _, mut velocity, accel, _, _) in bodies.iter_mut() {
                velocity.0 += accel.0 * half_dt as f32;
                // acc.0 is preserved here
            }
        }
    } else {
        // ---------------------------------
        // On-rails mode
        // ---------------------------------

        // Snapshot positions & masses so children can read parents without borrow conflicts
        let entity_lookup: HashMap<Entity, (DVec3, f64)> = bodies
            .iter()
            .map(|(e, tf, _, _, body, _)| (e, (tf.position, body.gm as f64)))
            .collect();

        // Just entered rails -> Recalculate orbits from current state
        if !warp.was_on_rails {
            for (_, transform, velocity, _, body, orbit_opt) in bodies.iter_mut() {
                if let Some(mut orbit) = orbit_opt {
                    let (parent_pos, mu) = match orbit.parent {
                        // Sub-satellite (e.g. Moon around Planet): Parent GM + Child GM
                        Some(parent_entity) => {
                            let (pos, parent_gm) = entity_lookup.get(&parent_entity).copied().unwrap_or((DVec3::ZERO, 0.0));
                            (pos, parent_gm + body.gm as f64)
                        }
                        // Root body (Stars or Circumbinary Planet): Keep orbit's own pre-calculated mu!
                        None => (DVec3::ZERO, orbit.gm),
                    };

                    *orbit = KeplerOrbit::from_state_vectors(
                        orbit.parent,
                        parent_pos,
                        transform.position,
                        velocity.0,
                        mu,
                        warp.total_sim_time,
                    );
                }
            }
            warp.was_on_rails = true;
        }

        // Evaluate analytical Kepler positions
        for (_, mut transform, mut velocity, _, _, orbit_opt) in bodies.iter_mut() {
            if let Some(orbit) = orbit_opt {
                let parent_pos = orbit.parent
                    .and_then(|p| entity_lookup.get(&p).map(|(pos, _)| *pos))
                    .unwrap_or(DVec3::ZERO);

                let (rel_pos, rel_vel) = orbit.eval_state_at_time(warp.total_sim_time);
                transform.position = parent_pos + rel_pos;
                velocity.0 = rel_vel;
            }
        }
    }
}

use std::collections::HashMap;

use crate::physics::components::{CelestialBody, RAILS_THRESHOLD, TimeWarp, Velocity, Acceleration};
use crate::physics::kepler::KeplerOrbit;
use engine::time::time::Time;
use engine::render::instance::Transform;

use bevy_ecs::prelude::*;
use glam::DVec3;

pub fn propagation_system(
    time: Res<Time>,
    mut warp: ResMut<TimeWarp>,
    mut bodies: Query<(Entity, &mut Transform, &mut Velocity, &mut Acceleration, &CelestialBody, Option<&mut KeplerOrbit>)>,
) {
    let sim_dt = time.delta_time() as f64 * warp.factor;
    let epoch_ref = warp.total_sim_time;
    warp.total_sim_time += sim_dt;

    if warp.factor < RAILS_THRESHOLD {
        physics_step(&mut warp, &mut bodies, sim_dt);
    } else {
        rails_step(&mut warp, &mut bodies, epoch_ref);
    }
}

fn physics_step(
    warp: &mut TimeWarp,
    bodies: &mut Query<(Entity, &mut Transform, &mut Velocity, &mut Acceleration, &CelestialBody, Option<&mut KeplerOrbit>)>,
    sim_dt: f64,
) {
    // Just exited rails -> Rebuild accelerations at the resumed on-rails state
    // BEFORE the first kick. Without this the first half-kick would use a stale
    // (zeroed) acceleration, losing a(t) * dt / 2 of velocity — a jolt at the
    // mode seam.
    if warp.was_on_rails {
        for (_, _, _, mut accel, _, _) in bodies.iter_mut() {
            accel.0 = DVec3::ZERO;
        }
        accumulate_gravity(bodies);
        warp.was_on_rails = false;
    }

    // Sub-stepping for numerical stability
    const SUB_DT: f64 = 1.0 / 120.0;
    let steps = ((sim_dt / SUB_DT).ceil() as u32).clamp(1, 150);
    let true_sub_dt = sim_dt / steps as f64;
    let half_dt = 0.5 * true_sub_dt;

    for _ in 0..steps {
        // PHASE 1: KICK 1 + DRIFT -> All bodies move to time t + dt
        for (_, mut transform, mut velocity, mut accel, _, _) in bodies.iter_mut() {
            velocity.0 += accel.0 * half_dt; // Half-step velocity using existing a(t)
            transform.position += velocity.0 * true_sub_dt; // Full-step position using updated velocity

            accel.0 = DVec3::ZERO; // Clear acceleration: ready to accumulate new forces at t + dt
        }

        // PHASE 2: MUTUAL N-BODY GRAVITY AT NEW POSITIONS (t + dt)
        accumulate_gravity(bodies);

        // PHASE 3: KICK 2 (Complete velocity update with new accelerations)
        for (_, _, mut velocity, accel, _, _) in bodies.iter_mut() {
            velocity.0 += accel.0 * half_dt;
            // acc.0 is preserved here
        }
    }
}

/// Newton's 3rd Law pairwise gravity, accumulated into `Acceleration` at the
/// bodies' CURRENT positions. Call only after accelerations have been zeroed:
/// Pair (A, B) computes the pull on both simultaneously.
fn accumulate_gravity(
    bodies: &mut Query<(Entity, &mut Transform, &mut Velocity, &mut Acceleration, &CelestialBody, Option<&mut KeplerOrbit>)>
) {
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
        accel_a.0 += dir_over_dist_cube * body_b.gm;

        // Acceleration on B caused by A: a_B = -(GM_A / r^3) * r
        accel_b.0 -= dir_over_dist_cube * body_a.gm;
    }
}

fn rails_step(
    warp: &mut TimeWarp,
    bodies: &mut Query<(Entity, &mut Transform, &mut Velocity, &mut Acceleration, &CelestialBody, Option<&mut KeplerOrbit>)>,
    epoch_ref: f64, // absolute time the current state vectors belong to
) {
    // Snapshot the full state (position, WORLD velocity, gm per entity, as of
    // `epoch_ref`) so parents and anchors can be read without borrow conflicts.
    let snapshot: HashMap<Entity, (DVec3, DVec3, f64)> = bodies
        .iter()
        .map(|(e, tf, vel, _, body, _)| (e, (tf.position, vel.0, body.gm)))
        .collect();

    // Just entered rails -> Rebuild orbiter orbits from the current state.
    // from_state_vectors expects PARENT-RELATIVE vectors, so the parent's own
    // motion is subtracted. Bodies without a KeplerOrbit are anchors: nothing
    // to rebuild, they simply stay where physics left them.
    if !warp.was_on_rails {
        for (_, transform, velocity, _, body, orbit_opt) in bodies.iter_mut() {
            if let Some(mut orbit) = orbit_opt {
                if let Some(&(parent_pos, parent_vel, parent_gm)) = snapshot.get(&orbit.parent) {
                    *orbit = KeplerOrbit::from_state_vectors(
                        orbit.parent,
                        transform.position - parent_pos,
                        velocity.0 - parent_vel,
                        parent_gm + body.gm,
                        epoch_ref,
                    );
                }
            }
        }
        warp.was_on_rails = true;
    }

    // Phase A: evaluate every orbiter's RELATIVE state at the new sim time.
    // Absence from `orbit_parents` marks an anchor.
    let mut orbit_parents: HashMap<Entity, Entity> = HashMap::new();
    let mut rel_states: HashMap<Entity, (DVec3, DVec3)> = HashMap::new();

    for (e, _, _, _, _, orbit_opt) in bodies.iter() {
        if let Some(orbit) = orbit_opt {
            orbit_parents.insert(e, orbit.parent);
            rel_states.insert(e, orbit.eval_state_at_time(warp.total_sim_time));
        }
    }

    // Phase B: compose world-space states by walking each parent chain up to
    // the first body WITHOUT an orbit (the anchor). The anchor's current state
    // starts the chain, so children are anchored to THIS frame's anchor
    // position instead of a stale snapshot or the origin.
    let mut abs_positions: HashMap<Entity, DVec3> = HashMap::new();
    let mut abs_velocities: HashMap<Entity, DVec3> = HashMap::new();

    for &entity in orbit_parents.keys() {
        let Some((anchor, chain)) = chain_to_anchor(entity, &orbit_parents) else {
            continue; // defensive: cyclic or broken parent data
        };
        let Some(&(anchor_pos, anchor_vel, _)) = snapshot.get(&anchor) else {
            continue; // defensive: anchor despawned -> orbiter freezes this frame
        };

        let mut pos = anchor_pos;
        let mut vel = anchor_vel;
        for &node in chain.iter().rev() { // anchor's child first, then down to `entity`
            let (rel_pos, rel_vel) = rel_states[&node];
            pos += rel_pos;
            vel += rel_vel;
        }

        abs_positions.insert(entity, pos);
        abs_velocities.insert(entity, vel);
    }

    // Phase C: write results back. Bodies without a KeplerOrbit (anchors) keep
    // their last physics-integrated transform: frozen on rails.
    for (e, mut transform, mut velocity, _, _, _) in bodies.iter_mut() {
        if let Some(pos) = abs_positions.get(&e) {
            transform.position = *pos;
        }
        if let Some(vel) = abs_velocities.get(&e) {
            velocity.0 = *vel;
        }
    }
}

/// Walks `start`'s parent chain (bodies WITH orbits only) up to the first
/// parent WITHOUT an orbit and returns that parent as the anchor, together
/// with the orbiter chain from the anchor's child down to `start`. Returns
/// None on cyclic parent data (defensive).
fn chain_to_anchor(
    start: Entity,
    orbit_parents: &HashMap<Entity, Entity>,
) -> Option<(Entity, Vec<Entity>)> {
    let mut chain = vec![start];
    loop {
        let last = *chain.last().unwrap();
        let parent = orbit_parents[&last]; // `last` always has an orbit here
        if chain.contains(&parent) {
            return None; // defensive: cyclic data would otherwise loop forever
        }
        if orbit_parents.contains_key(&parent) {
            chain.push(parent); // parent is itself an orbiter: keep walking up
        } else {
            return Some((parent, chain)); // parent has no orbit: it is the anchor
        }
    }
}

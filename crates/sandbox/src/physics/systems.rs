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
    // Just exited rails -> Reset accelerations to zero
    if warp.was_on_rails {
        for (_, _, _, mut accel, _, _) in bodies.iter_mut() {
            accel.0 = DVec3::ZERO;
        }
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
            accel_a.0 += dir_over_dist_cube * body_b.gm;

            // Acceleration on B caused by A: a_B = -(GM_A / r^3) * r
            accel_b.0 -= dir_over_dist_cube * body_a.gm;
        }

        // PHASE 3: KICK 2 (Complete velocity update with new accelerations)
        for (_, _, mut velocity, accel, _, _) in bodies.iter_mut() {
            velocity.0 += accel.0 * half_dt;
            // acc.0 is preserved here
        }
    }
}

fn rails_step(
    warp: &mut TimeWarp,
    bodies: &mut Query<(Entity, &mut Transform, &mut Velocity, &mut Acceleration, &CelestialBody, Option<&mut KeplerOrbit>)>,
    epoch_ref: f64, // absolute time the current state vectors belong to
) {
    // Snapshot the full hierarchy state (position, WORLD velocity, gm per entity,
    // as of `epoch_ref`) so parents can be read without borrow conflicts.
    let snapshot: HashMap<Entity, (DVec3, DVec3, f64)> = bodies
        .iter()
        .map(|(e, tf, vel, _, body, _)| (e, (tf.position, vel.0, body.gm)))
        .collect();

    // Just entered rails -> Rebuild orbits from the current state.
    // from_state_vectors expects PARENT-RELATIVE vectors, so the parent's own
    // motion is subtracted from the child's world velocity.
    if !warp.was_on_rails {
        for (_, transform, velocity, _, body, orbit_opt) in bodies.iter_mut() {
            if let Some(mut orbit) = orbit_opt {
                let (parent_pos, parent_vel, mu) = match orbit.parent {
                    // Orbiting another body (e.g. Moon around Planet): mu = parent GM + own GM
                    Some(parent_entity) => {
                        let (pos, vel, parent_gm) = snapshot.get(&parent_entity).copied()
                            .unwrap_or((DVec3::ZERO, DVec3::ZERO, 0.0));
                        (pos, vel, parent_gm + body.gm)
                    }
                    // Root body (Stars or Circumbinary Planet): Keep orbit's own pre-calculated mu!
                    None => (DVec3::ZERO, DVec3::ZERO, orbit.gm),
                };

                *orbit = KeplerOrbit::from_state_vectors(
                    orbit.parent,
                    parent_pos,
                    transform.position,
                    velocity.0 - parent_vel,
                    mu,
                    epoch_ref,
                );
            }
        }
        warp.was_on_rails = true;
    }

    // Phase A: evaluate every orbit's RELATIVE state at the new sim time.
    let mut parents: HashMap<Entity, Option<Entity>> = HashMap::new();
    let mut rel_states: HashMap<Entity, (DVec3, DVec3)> = HashMap::new();

    for (e, _, _, _, _, orbit_opt) in bodies.iter() {
        if let Some(orbit) = orbit_opt {
            parents.insert(e, orbit.parent);
            rel_states.insert(e, orbit.eval_state_at_time(warp.total_sim_time));
        }
    }

    // Phase B: compose world-space states by walking each parent chain, so a child
    // is anchored to THIS frame's parent position instead of a stale snapshot.
    // Roots (parent: None) anchor at DVec3::ZERO.
    let mut abs_positions: HashMap<Entity, DVec3> = HashMap::new();
    let mut abs_velocities: HashMap<Entity, DVec3> = HashMap::new();

    for (&entity, _) in parents.iter() {
        let chain = path_to_root(entity, &parents);

        let mut pos = DVec3::ZERO;
        let mut vel = DVec3::ZERO;
        for &node in chain.iter().rev() { // root first, then down to `entity`
            let (rel_pos, rel_vel) = rel_states[&node];
            pos += rel_pos;
            vel += rel_vel;
        }

        abs_positions.insert(entity, pos);
        abs_velocities.insert(entity, vel);
    }

    // Phase C: write results back. Bodies without a KeplerOrbit keep their
    // last physics-integrated transform.
    for (e, mut transform, mut velocity, _, _, _) in bodies.iter_mut() {
        if let Some(pos) = abs_positions.get(&e) {
            transform.position = *pos;
        }
        if let Some(vel) = abs_velocities.get(&e) {
            velocity.0 = *vel;
        }
    }
}

// Walks `start`'s parent chain up to a (possibly virtual) root and returns it as
// [start, ..., root]. A missing parent is treated as the root; cycles break the
// walk defensively.
fn path_to_root(start: Entity, parents: &HashMap<Entity, Option<Entity>>) -> Vec<Entity> {
    let mut chain = vec![start];
    while let Some(parent) = parents.get(&chain[chain.len() - 1]).copied().flatten() {
        if chain.contains(&parent) {
            break; // defensive: cyclic data would otherwise loop forever
        }
        chain.push(parent);
    }
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{Quat, Vec3};

    const DELTA: f32 = 1.0 / 60.0; // simulated 60 fps frame
    const TAU: f64 = std::f64::consts::TAU;
    const PI: f64 = std::f64::consts::PI;

    struct Fixture {
        world: World,
        schedule: Schedule,
        star_a: Entity,
        star_b: Entity,
        planet: Entity,
    }

    impl Fixture {
        fn new() -> Self {
            let mut world = World::new();

            // STAR A: circular binary orbit around barycenter
            let star_a = world.spawn((
                Transform { position: DVec3::new(3.2, 0.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::splat(1.8) },
                Velocity(DVec3::new(0.0, 0.0, 20.0f64.sqrt())), // sqrt(gm_bar/a) = sqrt(64/3.2)
                Acceleration(DVec3::new(-6.25, 0.0, 0.0)),
                CelestialBody { gm: 600.0, softening_sq: 1e-6 },
                KeplerOrbit {
                    parent: None,
                    gm: 64.0, // effective mu around barycenter: M_B^3 / (M_A + M_B)^2
                    a: 3.2,
                    e: 0.0,
                    periapsis_dir: DVec3::X,
                    semi_minor_dir: DVec3::Z,
                    mean_anomaly_epoch: 0.0,
                    epoch_time: 0.0,
                },
            )).id();

            // STAR B: anti-phase twin of STAR A
            let star_b = world.spawn((
                Transform { position: DVec3::new(-4.8, 0.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::splat(1.2) },
                Velocity(DVec3::new(0.0, 0.0, -(45.0f64.sqrt()))), // sqrt(216/4.8) = sqrt(45)
                Acceleration(DVec3::new(9.375, 0.0, 0.0)),
                CelestialBody { gm: 400.0, softening_sq: 1e-6 },
                KeplerOrbit {
                    parent: None,
                    gm: 216.0, // effective mu around barycenter: M_A^3 / (M_A + M_B)^2
                    a: 4.8,
                    e: 0.0,
                    periapsis_dir: -DVec3::X,
                    semi_minor_dir: -DVec3::Z,
                    mean_anomaly_epoch: 0.0,
                    epoch_time: 0.0,
                },
            )).id();

            // CIRCUMBINARY PLANET
            let planet = world.spawn((
                Transform { position: DVec3::new(0.0, 0.0, -24.0), rotation: Quat::IDENTITY, scale: Vec3::splat(0.6) },
                Velocity(DVec3::new((1000.5f64 / 24.0f64).sqrt(), 0.0, 0.0)),
                Acceleration(DVec3::new(0.0044, 0.0, 1.6693)),
                CelestialBody { gm: 0.5, softening_sq: 1e-6 },
                KeplerOrbit {
                    parent: None,
                    gm: 1000.5, // sees combined mass of both stars + planet
                    a: 24.0,
                    e: 0.0,
                    periapsis_dir: -DVec3::Z,
                    semi_minor_dir: DVec3::X,
                    mean_anomaly_epoch: 0.0,
                    epoch_time: 0.0,
                },
            )).id();

            world.insert_resource(Time::new());
            world.insert_resource(TimeWarp::default());

            let mut schedule = Schedule::default();
            schedule.add_systems(propagation_system);

            Self { world, schedule, star_a, star_b, planet }
        }

        fn pos(&mut self, e: Entity) -> DVec3 {
            self.world.get::<Transform>(e).unwrap().position
        }

        fn step(&mut self, factor: f64, delta: f32) {
            self.world.resource_mut::<TimeWarp>().factor = factor;
            self.world.resource_mut::<Time>().set_delta_time(delta);
            self.schedule.run(&mut self.world);
        }

        // Relative angle between the two stars in the X-Z plane, wrapped to (-pi, pi].
        fn opposition_phase(&mut self) -> f64 {
            let a = angle_xz(self.pos(self.star_a));
            let b = angle_xz(self.pos(self.star_b));
            wrap_pi(a - b + PI)
        }

        fn positions(&mut self) -> [DVec3; 3] {
            [self.star_a, self.star_b, self.planet].map(|e| self.pos(e))
        }
    }

    fn angle_xz(p: DVec3) -> f64 {
        p.z.atan2(p.x)
    }

    fn wrap_pi(x: f64) -> f64 {
        let x = x.rem_euclid(TAU);
        if x > PI { x - TAU } else { x }
    }

    // Untouched coordinates used as a bound check: entering rails at 10.5x must
    // only advance bodies by the legitimate one-frame arc (~0.8-1.1 units).
    // Before the fix, from_state_vectors' near-circular degeneracy teleported
    // bodies by ~2a (6.4 / 9.6 / 48 units).
    #[test]
    fn rails_entry_continues_physics_without_teleport() {
        let mut f = Fixture::new();

        // Physics at 1x for ~0.2 s
        for _ in 0..12 {
            f.step(1.0, DELTA);
        }

        let before = f.positions();
        assert!(!f.world.resource::<TimeWarp>().was_on_rails);
        f.step(10.5, DELTA); // crosses RAILS_THRESHOLD
        let after = f.positions();

        for (i, (b, a)) in before.iter().zip(after.iter()).enumerate() {
            let jump = (a - b).length();
            assert!(jump < 2.0, "body {i} teleported by {} (legit arc is ~1)", jump);
        }
        assert!(f.world.resource::<TimeWarp>().was_on_rails);
    }

    // On rails, all bodies are exact functions of total_sim_time with fixed
    // elements: the binary's anti-phase alignment must not drift over ~150 orbits.
    // Enters rails directly on the exact seeded conic (no physics warm-up): the
    // rebuild is f64-exact, so any drift beyond ~1e-6 would be a leak of per-frame
    // state into the orbit elements.
    #[test]
    fn rails_opposition_stays_stable_over_time() {
        let mut f = Fixture::new();

        f.step(10.5, DELTA); // enter rails immediately
        let phase_at_entry = f.opposition_phase();

        for _ in 0..400 {
            f.step(100.0, DELTA);
        }
        // 400 frames * 100x * 1/60s = ~667 s of sim time = ~149 binary orbits
        let drift = (f.opposition_phase() - phase_at_entry).abs();
        assert!(drift < 1e-6, "opposition drift over 149 orbits: {}", drift);
    }

    // Repeated physics <-> rails toggles must not degrade the deterministic
    // orbits: no teleport on any re-entry, radii stay conic-shaped, alignment
    // stays bounded. Note physics-mode phases the state with real 3-body
    // perturbations (the planet pulls the stars), so rails re-entry legitimately
    // re-stamps slightly different conics each cycle — tolerances here are set to
    // that perturbation scale, not to machine epsilon.
    #[test]
    fn toggle_roundtrip_preserves_orbits() {
        let mut f = Fixture::new();

        let mut phase_initial: f64 = 0.0;
        for cycle in 0..5 {
            for _ in 0..3 {
                f.step(1.0, DELTA);
            }
            let before = f.positions();
            f.step(10.5, DELTA);
            let after = f.positions();

            for (i, (b, a)) in before.iter().zip(after.iter()).enumerate() {
                let jump = (a - b).length();
                assert!(jump < 2.0, "cycle {cycle}: body {i} teleported by {}", jump);
            }

            if cycle == 0 {
                phase_initial = f.opposition_phase();
            }
            f.step(50.0, DELTA); // ride rails for a bit
        }

        let phase_final = f.opposition_phase();
        let drift = (phase_final - phase_initial).abs();
        assert!(drift < 0.02, "alignment drift across toggles: {}", drift);

        // Radii absorb only genuine 3-body per-cycle error; assert relative to
        // each body's orbit size (the planet's conic breathes at the binary's
        // 2-omega pulsation scale).
        let (ra, rb, rp) = (
            f.pos(f.star_a).length(),
            f.pos(f.star_b).length(),
            f.pos(f.planet).length(),
        );
        assert!((ra - 3.2).abs() / 3.2 < 1.6e-3, "star A radius drift: {}", (ra - 3.2).abs());
        assert!((rb - 4.8).abs() / 4.8 < 1.6e-3, "star B radius drift: {}", (rb - 4.8).abs());
        assert!((rp - 24.0).abs() / 24.0 < 2.0e-3, "planet radius drift: {}", (rp - 24.0).abs());
    }
}

use engine::time::time::Time;
use engine::render::instance::Transform;

use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};

pub const RAILS_THRESHOLD: f64 = 10.0;


#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Acceleration(pub Vec3);

#[derive(Component)]
pub struct CelestialBody {
    pub gm: f32, // Gravitational parameter: G * Mass
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

#[derive(Component)]
pub struct KeplerOrbit {
    pub parent: Entity,
    pub gm: f64, // G * M
    pub a: f64, // Semi-major axis (size of orbit)
    pub e: f64, // 0.0 = perfect circle, 0.0 < e < 1.0 = ellipse
    pub periapsis_dir: DVec3, // Unit vector pointing toward periapsis (closest approach)
    pub semi_minor_dir: DVec3, // Unit vector in orbital plane, perpendicular to periapsis in direction of motions
    pub mean_anomaly_epoch: f64, // Mean anomaly at epoch t0 (in radians)
    pub epoch_time: f64, // Time of epoch t0
}

impl KeplerOrbit {
    // Converts instantaneous Cartesian (r, v) into a KeplerOrbit
    pub fn from_state_vectors(
        parent: Entity,
        parent_pos: DVec3,
        pos: DVec3,
        vel: Vec3,
        gm: f64,
        current_time: f64
    ) -> Self {
        let r_vec = pos - parent_pos;
        let v_vec = vel.as_dvec3();
        let r = r_vec.length();
        let v_sq = v_vec.length_squared();

        // Specific angular momentum: h = r x v
        let h_vec = r_vec.cross(v_vec);
        let h = h_vec.length();

        // Eccentricity vector: e = (v x h)/mu - r/|r|
        let e_vec = v_vec.cross(h_vec) / gm - (r_vec / r);
        let e = e_vec.length().clamp(0.0, 0.999);

        // Semi-major axis from vis-viva equation: 1/a = 2/r - v^2/mu
        let a = 1.0 / (2.0 / r - v_sq / gm);

        // Perifocal basis vectors
        let periapsis_dir = if e > 1e-6 { e_vec.normalize() } else { r_vec.normalize() };
        let orbit_normal = h_vec.normalize();
        let semi_minor_dir = orbit_normal.cross(periapsis_dir).normalize();

        // Eccentric Anomaly at current moment
        let x_orb = r_vec.dot(periapsis_dir);
        let cos_e = ((a - r) / (a * e.max(1e-6))).clamp(-1.0, 1.0);
        let sin_e = (r_vec.dot(semi_minor_dir) / (a * (1.0 - e * e).sqrt())).clamp(-1.0, 1.0);
        let e_anom = sin_e.atan2(cos_e);

        // Mean Anomaly at epoch
        let m0 = e_anom - e * sin_e;

        Self {
            parent,
            gm,
            a,
            e,
            periapsis_dir,
            semi_minor_dir,
            mean_anomaly_epoch: m0,
            epoch_time: current_time,
        }
    }

    // Solves M = E - e*sin(E) for Eccentric Anomaly E using Newton-Raphson
    fn solve_eccentric_anomaly(&self, mean_anomaly: f64) -> f64 {
        let mut e_anom = mean_anomaly; // Good initial guess for small/moderate e

        for _ in 0..5 { // Converges to extreme precision in 3-5 iterations
            let delta = e_anom - self.e * e_anom.sin() - mean_anomaly;
            let delta_prime = 1.0 - self.e * e_anom.cos();
            e_anom -= delta / delta_prime;
        }

        e_anom
    }

    // Evaluates exact relative (Position, Velocity) at any absolute time t
    pub fn eval_state_at_time(&self, t: f64) -> (DVec3, Vec3) {
        // Mean motion: n = sqrt(gm / a^3)
        let n = (self.gm / (self.a * self.a * self.a)).sqrt();
        let m = (self.mean_anomaly_epoch + n * (t - self.epoch_time)).rem_euclid(std::f64::consts::TAU);

        // Solve Kepler's Equation for E
        let e_anom = self.solve_eccentric_anomaly(m);

        // 2D Coordinates in orbital plane
        let (sin_e, cos_e) = e_anom.sin_cos();
        let b = self.a * (1.0 - self.e * self.e).sqrt(); // Semi-minor axis

        let x_orb = self.a * (cos_e - self.e);
        let z_orb = b * sin_e;

        // Position relative to parent
        let rel_pos = self.periapsis_dir * x_orb + self.semi_minor_dir * z_orb;

        // Velocity in orbital plane: r_dot
        let r = self.a * (1.0 - self.e * cos_e);
        let v_factor = (self.gm * self.a).sqrt() / r;

        let vx_orb = -v_factor * sin_e;
        let vz_orb = v_factor * (1.0 - self.e * self.e).sqrt() * cos_e;

        let rel_vel = (self.periapsis_dir * vx_orb + self.semi_minor_dir * vz_orb).as_vec3();

        (rel_pos, rel_vel)
    }
}

pub fn orbit_system(
    time: Res<Time>,
    mut warp: ResMut<TimeWarp>,
    mut bodies: Query<(&mut Transform, &mut Velocity, &mut Acceleration, &CelestialBody)>,
) {
    let sim_time = time.delta_time();

    const STEP: f32 = 1.0 / 120.0;
    let steps: u32 = (sim_time / STEP).ceil() as u32;

    let dt = sim_time / steps as f32;
    let half_dt = 0.5 * dt;

    for _ in 0..steps {
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
}

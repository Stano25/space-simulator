use bevy_ecs::prelude::*;
use glam::DVec3;

/// A two-body conic orbit of a body around its `parent`, stored as classical
/// orbital elements stamped at an epoch. Bodies carrying this component are
/// propagated analytically while time is warped on rails; bodies without it
/// act as frozen rails anchors (see `systems::rails_step`).
#[derive(Component, Clone, Copy)]
pub struct KeplerOrbit {
    pub parent: Entity,
    pub gm: f64, // Two-body mu: parent G*M + own G*M
    pub a: f64, // Semi-major axis (size of orbit)
    pub e: f64, // 0.0 = perfect circle, 0.0 < e < 1.0 = ellipse
    pub periapsis_dir: DVec3, // Unit vector pointing toward periapsis (closest approach)
    pub semi_minor_dir: DVec3, // Unit vector in orbital plane, perpendicular to periapsis in direction of motion
    pub mean_anomaly_epoch: f64, // Mean anomaly at epoch t0 (in radians)
    pub epoch_time: f64, // Time of epoch t0
}

impl KeplerOrbit {
    // Converts instantaneous Cartesian state vectors into a KeplerOrbit.
    // `rel_pos` and `rel_vel` must be PARENT-RELATIVE (world state minus the
    // parent's world state); `mu` is the two-body parameter (parent G*M + own G*M).
    pub fn from_state_vectors(
        parent: Entity,
        rel_pos: DVec3,
        rel_vel: DVec3,
        mu: f64,
        current_time: f64,
    ) -> Self {
        let r_vec = rel_pos;
        let v_vec = rel_vel;
        let r = r_vec.length();
        let v_sq = v_vec.length_squared();

        // 1. Specific angular momentum: h = r x v
        let h_vec = r_vec.cross(v_vec);

        // 2. Eccentricity vector: e = (v x h)/mu - r/|r|
        let e_vec = v_vec.cross(h_vec) / mu - (r_vec / r);
        let e = e_vec.length().clamp(0.0, 0.999);

        // 3. Semi-major axis from vis-viva equation: 1/a = 2/r - v^2/mu
        let a = 1.0 / (2.0 / r - v_sq / mu);

        // 4. Perifocal basis vectors
        let periapsis_dir = if e > 1e-6 { e_vec.normalize() } else { r_vec.normalize() };
        let orbit_normal = h_vec.normalize();
        let semi_minor_dir = orbit_normal.cross(periapsis_dir).normalize();

        // 5. Eccentric Anomaly at current moment
        // Perifocal decomposition of r: x = a(cos E - e), y = a*sqrt(1-e^2)*sin E.
        // cos E = x/a + e avoids dividing by the near-zero eccentricity of circular orbits.
        let x_orb = r_vec.dot(periapsis_dir);
        let y_orb = r_vec.dot(semi_minor_dir);
        let cos_e = (x_orb / a + e).clamp(-1.0, 1.0);
        let sin_e = (y_orb / (a * (1.0 - e * e).sqrt())).clamp(-1.0, 1.0);
        let e_anom = sin_e.atan2(cos_e);

        // 6. Mean Anomaly at epoch
        let m0 = e_anom - e * sin_e;

        Self {
            parent,
            gm: mu,
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
    pub fn eval_state_at_time(&self, t: f64) -> (DVec3, DVec3) {
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

        let rel_vel = self.periapsis_dir * vx_orb + self.semi_minor_dir * vz_orb;

        (rel_pos, rel_vel)
    }
}

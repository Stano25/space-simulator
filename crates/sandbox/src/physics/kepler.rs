use bevy_ecs::prelude::*;
use glam::DVec3;

#[derive(Component, Clone, Copy)]
pub struct KeplerOrbit {
    pub parent: Option<Entity>,
    pub gm: f64, // parent G*M + current G*M
    pub a: f64, // Semi-major axis (size of orbit)
    pub e: f64, // 0.0 = perfect circle, 0.0 < e < 1.0 = ellipse
    pub periapsis_dir: DVec3, // Unit vector pointing toward periapsis (closest approach)
    pub semi_minor_dir: DVec3, // Unit vector in orbital plane, perpendicular to periapsis in direction of motions
    pub mean_anomaly_epoch: f64, // Mean anomaly at epoch t0 (in radians)
    pub epoch_time: f64, // Time of epoch t0
}

impl KeplerOrbit {
    // Converts instantaneous Cartesian (r, v) into a KeplerOrbit
    // `pos` and `vel` are parent-RELATIVE state vectors.
    pub fn from_state_vectors(
        parent: Option<Entity>,
        parent_pos: DVec3,
        pos: DVec3,
        vel: DVec3,
        gm: f64,
        current_time: f64,
    ) -> Self {
        let r_vec = pos - parent_pos;
        let v_vec = vel;
        let r = r_vec.length();
        let v_sq = v_vec.length_squared();

        // 1. Specific angular momentum: h = r x v
        let h_vec = r_vec.cross(v_vec);

        // 2. Eccentricity vector: e = (v x h)/mu - r/|r|
        let e_vec = v_vec.cross(h_vec) / gm - (r_vec / r);
        let e = e_vec.length().clamp(0.0, 0.999);

        // 3. Semi-major axis from vis-viva equation: 1/a = 2/r - v^2/mu
        let a = 1.0 / (2.0 / r - v_sq / gm);

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

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    fn dist(a: DVec3, b: DVec3) -> f64 {
        (a - b).length()
    }

    // The seeded binary star: circular orbit around the barycenter at origin.
    // Exact f64 state vectors must round-trip losslessly.
    #[test]
    fn circular_state_roundtrips_exactly() {
        let pos = DVec3::new(3.2, 0.0, 0.0);
        let vel = DVec3::new(0.0, 0.0, 20.0f64.sqrt()); // sqrt(64/3.2) = sqrt(20)
        let orbit = KeplerOrbit::from_state_vectors(None, DVec3::ZERO, pos, vel, 64.0, 0.0);

        // Elements must be a perfect circle
        assert!((orbit.e - 0.0).abs() < EPS, "e = {}", orbit.e);
        assert!((orbit.a - 3.2).abs() < EPS, "a = {}", orbit.a);

        // eval at epoch reproduces the snapshot state (pre-fix this flipped by pi: pos = -2.8)
        let (p, v) = orbit.eval_state_at_time(0.0);
        assert!(dist(p, pos) < EPS, "pos diverged: {}", dist(p, pos));
        assert!(dist(v, vel) < EPS, "vel diverged: {}", dist(v, vel));

        // 2000 periods later the body is back at the snapshot position
        let period = std::f64::consts::TAU * ((3.2f64 * 3.2 * 3.2) / 64.0).sqrt();
        let (p2, _) = orbit.eval_state_at_time(2000.0 * period);
        assert!(dist(p2, pos) < 1e-9, "pos after 2000 periods: {}", dist(p2, pos));
    }

    // An f32-truncated circular velocity (as stored when Velocity was Vec3) yields
    // an eccentricity of ~1.6e-5 with the periapsis on the far side, so the anomaly
    // legitimately reconstructs to E = pi (apoapsis in that skewed basis). That is
    // SELF-CONSISTENT: what matters is that the state round-trips — eval at epoch
    // reproduces the snapshot instead of teleporting to a different orbit branch.
    #[test]
    fn truncated_velocity_state_roundtrips() {
        let pos = DVec3::new(3.2, 0.0, 0.0);
        let vel = DVec3::new(0.0, 0.0, 4.4721); // f32 truncation of sqrt(20) = 4.472136
        let orbit = KeplerOrbit::from_state_vectors(None, DVec3::ZERO, pos, vel, 64.0, 0.0);

        let (p, v) = orbit.eval_state_at_time(0.0);
        assert!(dist(p, pos) < 1e-3, "pos round-trip error: {}", dist(p, pos));
        assert!(dist(v, vel) < 1e-2, "vel round-trip error: {}", dist(v, vel));

        // The elements hold a valid bound ellipse of the right size, whatever side
        // the (degenerate) periapsis ended up on.
        assert!((orbit.a - 3.2).abs() < 1e-3, "a = {}", orbit.a);
    }

    // The two binary stars' mean motions must stay commensurate: their anti-phase
    // alignment must not drift apart over many orbits (post-fix: both periods are
    // f64-exact equal, so drift is at machine epsilon level).
    #[test]
    fn binary_stars_stay_antiphase() {
        let a_orbit = KeplerOrbit::from_state_vectors(
            None, DVec3::ZERO,
            DVec3::new(3.2, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 20.0f64.sqrt()),
            64.0, 0.0,
        );
        let b_orbit = KeplerOrbit::from_state_vectors(
            None, DVec3::ZERO,
            DVec3::new(-4.8, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 45.0f64.sqrt()), // sqrt(216/4.8) = sqrt(45)
            216.0, 0.0,
        );

        let angle = |p: DVec3, orbit: &KeplerOrbit| {
            // Convert world position back to the orbit's in-plane anomaly reference:
            // the perifocal decomposition angle is enough for relative comparison.
            p.dot(orbit.semi_minor_dir).atan2(p.dot(orbit.periapsis_dir))
        };

        for t in [500.0, 2000.0, 10000.0] { // 223 / 894 / 4470 orbits
            let (pa, _) = a_orbit.eval_state_at_time(t);
            let (pb, _) = b_orbit.eval_state_at_time(t);
            let (p0a, _) = a_orbit.eval_state_at_time(0.0);
            let (p0b, _) = b_orbit.eval_state_at_time(0.0);
            let phase_a = angle(pa, &a_orbit) - angle(p0a, &a_orbit);
            let phase_b = angle(pb, &b_orbit) - angle(p0b, &b_orbit);
            // Both advance by the same in-plane phase: n_A == n_B exactly (f64 seeds).
            assert!(
                (phase_a - phase_b).abs() < 1e-9,
                "phase drift at t={}: {} vs {} (delta {})",
                t, phase_a, phase_b, (phase_a - phase_b).abs()
            );
        }
    }
}

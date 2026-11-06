use std::sync::Arc;

use engine::render::camera::{Camera};
use engine::render::{context::RenderContext,
                     mesh::{Mesh, Vertex}};
use engine::render::material::Material;
use engine::render::layout::{EngineLayout, LayoutRegistry};
use engine::render::texture::{Texture};
use engine::render::instance::{Transform};

use crate::physics::{ components::{Velocity, Acceleration, CelestialBody},
                      kepler::KeplerOrbit};

use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3, Quat};

const TH_VERTICES: &[Vertex] = &[
    Vertex { position: [ 0.0,  0.6,  0.0], uv: [0.5, 0.0] }, // 0: Apex
    Vertex { position: [ 0.0, -0.4,  0.7], uv: [0.0, 1.0] }, // 1: Front
    Vertex { position: [ 0.6, -0.4, -0.4], uv: [1.0, 1.0] }, // 2: Back-Right
    Vertex { position: [-0.6, -0.4, -0.4], uv: [0.5, 1.0] }, // 3: Back-Left
];
const TH_INDICES: &[u16] = &[
    0, 1, 2, // Front-Right
    0, 3, 1, // Front-Left
    0, 2, 3, // Back
    1, 3, 2, // Bottom
];


pub fn test_system(mut commands: Commands, render_context: Res<RenderContext>, gpu_layout: Res<LayoutRegistry>) {
    commands.spawn( Camera::from_direction(
        glam::DVec3::new(0.0, 48.0, 48.0),
        glam::Vec3::new(0.0, -0.7071, -0.7071)
    ));

    let th_mesh = Mesh::new(&render_context.device, TH_VERTICES, TH_INDICES);

    // Materials
    let texture = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/roman.png"), "roman.png").unwrap();
    let material = Material::new(&render_context.device, &gpu_layout.get(EngineLayout::Material), Arc::new(texture));

    // ===========================================================================
    // STAR A
    // Mass: 600.0 | Distance to Barycenter: 3.2 | Period: ~4.5s
    // ===========================================================================
    let star_a_pos = DVec3::new(3.2, 0.0, 0.0);
    let star_a_vel = Vec3::new(0.0, 0.0, 4.4721); // Moving +Z
    let star_a_acc = Vec3::new(-6.25, 0.0, 0.0);  // Initial gravity from Star B (-X)

    let star_a_orbit = KeplerOrbit {
        parent: None,
        gm: 64.0, // Effective mu for Star A around barycenter: M_B^3 / (M_A + M_B)^2
        a: 3.2,
        e: 0.0,
        periapsis_dir: DVec3::X,
        semi_minor_dir: DVec3::Z,
        mean_anomaly_epoch: 0.0,
        epoch_time: 0.0,
    };

    // ===========================================================================
    // STAR B
    // Mass: 400.0 | Distance to Barycenter: 4.8 | Period: ~4.5s (180° opposite)
    // ===========================================================================
    let star_b_pos = DVec3::new(-4.8, 0.0, 0.0);
    let star_b_vel = Vec3::new(0.0, 0.0, -6.7082); // Moving -Z
    let star_b_acc = Vec3::new(9.375, 0.0, 0.0);   // Initial gravity from Star A (+X)

    let star_b_orbit = KeplerOrbit {
        parent: None,
        gm: 216.0, // Effective mu for Star B around barycenter: M_A^3 / (M_A + M_B)^2
        a: 4.8,
        e: 0.0,
        periapsis_dir: -DVec3::X, // 180 degrees opposite Star A!
        semi_minor_dir: -DVec3::Z,
        mean_anomaly_epoch: 0.0,
        epoch_time: 0.0,
    };

    // ===========================================================================
    // 3. CIRCUMBINARY PLANET
    // Orbits the common barycenter outside both stars | Distance: 24.0 | Period: ~23.3s
    // ===========================================================================
    let planet_pos = DVec3::new(0.0, 0.0, -24.0);
    let planet_vel = Vec3::new(6.4566, 0.0, 0.0); // Moving +X
    let planet_acc = Vec3::new(0.0044, 0.0, 1.6693); // Combined pull from both stars (+Z)

    let planet_orbit = KeplerOrbit {
        parent: None,
        gm: 1000.5, // Sees combined mass of both stars + planet (600 + 400 + 0.5)
        a: 24.0,
        e: 0.0,
        periapsis_dir: -DVec3::Z,
        semi_minor_dir: DVec3::X,
        mean_anomaly_epoch: 0.0,
        epoch_time: 0.0,
    };

    // ===========================================================================
    // Entity spawning
    // ===========================================================================

    // STAR A
    commands.spawn((
        Transform { position: star_a_pos, rotation: Quat::IDENTITY, scale: Vec3::splat(1.8) },
        Velocity(star_a_vel),
        Acceleration(star_a_acc),
        CelestialBody { gm: 600.0, softening_sq: 2.25 },
        star_a_orbit,
        th_mesh.clone(),
        material.clone()
    ));

    // STAR B
    commands.spawn((
        Transform { position: star_b_pos, rotation: Quat::IDENTITY, scale: Vec3::splat(1.2) },
        Velocity(star_b_vel),
        Acceleration(star_b_acc),
        CelestialBody { gm: 400.0, softening_sq: 1.44 },
        star_b_orbit,
        th_mesh.clone(),
        material.clone()
    ));

    // CIRCUMBINARY PLANET
    commands.spawn((
        Transform { position: planet_pos, rotation: Quat::IDENTITY, scale: Vec3::splat(0.6) },
        Velocity(planet_vel),
        Acceleration(planet_acc),
        CelestialBody { gm: 0.5, softening_sq: 0.25 },
        planet_orbit,
        th_mesh.clone(),
        material.clone()
    ));
}

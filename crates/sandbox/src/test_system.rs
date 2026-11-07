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

// ===========================================================================
// Example solar system: one star at the origin with two planets in circular
// orbits around it. The star carries NO KeplerOrbit: it is the frozen rails
// anchor, and both planets orbit it parent-relative (see rails_step).
// ===========================================================================

const STAR_GM: f64 = 1000.0;
const NEAR_GM: f64 = 0.5;
const FAR_GM: f64 = 1.0;
const NEAR_R: f64 = 10.0; // Planet 1 orbit radius | Period ~6.28 s
const FAR_R: f64 = 28.0;  // Planet 2 orbit radius | Period ~29.4 s

pub fn test_system(mut commands: Commands, render_context: Res<RenderContext>, gpu_layout: Res<LayoutRegistry>) {
    commands.spawn( Camera::from_direction(
        glam::DVec3::new(0.0, 48.0, 48.0),
        glam::Vec3::new(0.0, -0.7071, -0.7071)
    ));

    let th_mesh = Mesh::new(&render_context.device, TH_VERTICES, TH_INDICES);

    // Materials
    let texture = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/roman.png"), "roman.png").unwrap();
    let material = Material::new(&render_context.device, &gpu_layout.get(EngineLayout::Material), Arc::new(texture));

    // Initial accelerations: exact pairwise pulls at t = 0 so physics mode
    // starts clean. The planets sit on opposite sides of the star, separated
    // by NEAR_R + FAR_R along X.
    let separation = NEAR_R + FAR_R;
    let star_acc = DVec3::new(
        NEAR_GM / (NEAR_R * NEAR_R) - FAR_GM / (FAR_R * FAR_R),
        0.0, 0.0,
    );
    let near_acc = DVec3::new(
        -(STAR_GM / (NEAR_R * NEAR_R) + FAR_GM / (separation * separation)),
        0.0, 0.0,
    );
    let far_acc = DVec3::new(
        STAR_GM / (FAR_R * FAR_R) + NEAR_GM / (separation * separation),
        0.0, 0.0,
    );

    // ===========================================================================
    // Entity spawning
    // ===========================================================================

    // STAR — no KeplerOrbit: it is the anchor everything orbits on rails
    let star = commands.spawn((
        Transform { position: DVec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::splat(2.0) },
        Velocity(DVec3::ZERO),
        Acceleration(star_acc),
        CelestialBody { gm: STAR_GM, softening_sq: 1e-6 },
        th_mesh.clone(),
        material.clone()
    )).id();

    // NEAR PLANET — circular orbit: v = sqrt(mu / r) with the two-body mu
    commands.spawn((
        Transform { position: DVec3::new(NEAR_R, 0.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::splat(0.6) },
        Velocity(DVec3::new(0.0, 0.0, ((STAR_GM + NEAR_GM) / NEAR_R).sqrt())),
        Acceleration(near_acc),
        CelestialBody { gm: NEAR_GM, softening_sq: 1e-6 },
        KeplerOrbit {
            parent: star,
            gm: STAR_GM + NEAR_GM,
            a: NEAR_R,
            e: 0.0,
            periapsis_dir: DVec3::X,
            semi_minor_dir: DVec3::Z,
            mean_anomaly_epoch: 0.0,
            epoch_time: 0.0,
        },
        th_mesh.clone(),
        material.clone()
    ));

    // FAR PLANET — circular orbit on the opposite side, same orbital sense
    commands.spawn((
        Transform { position: DVec3::new(-FAR_R, 0.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::splat(0.9) },
        Velocity(DVec3::new(0.0, 0.0, -((STAR_GM + FAR_GM) / FAR_R).sqrt())),
        Acceleration(far_acc),
        CelestialBody { gm: FAR_GM, softening_sq: 1e-6 },
        KeplerOrbit {
            parent: star,
            gm: STAR_GM + FAR_GM,
            a: FAR_R,
            e: 0.0,
            periapsis_dir: -DVec3::X,
            semi_minor_dir: -DVec3::Z,
            mean_anomaly_epoch: 0.0,
            epoch_time: 0.0,
        },
        th_mesh.clone(),
        material.clone()
    ));
}

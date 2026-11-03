use std::sync::Arc;

use engine::render::camera::{Camera};
use engine::render::{context::RenderContext,
                     mesh::{Mesh, Vertex}};
use engine::render::material::Material;
use engine::render::layout::{EngineLayout, LayoutRegistry};
use engine::render::texture::{Texture};
use engine::render::instance::{Transform};

use crate::physics::orbit::{Velocity, Acceleration, CelestialBody};

use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3, Quat};

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], uv: [0.4131759, 0.00759614], }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], uv: [0.0048659444, 0.43041354], }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], uv: [0.28081453, 0.949397], }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], uv: [0.85967, 0.84732914], }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], uv: [0.9414737, 0.2652641], }, // E
];
const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];

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
    let camera = Camera::from_direction(
        glam::DVec3::new(0.0, 0.0, 2.0),
        glam::Vec3::new(0.0, 0.0, -1.0)
    );

    let mesh = Mesh::new(&render_context.device, VERTICES, INDICES);

    let th_mesh = Mesh::new(&render_context.device, TH_VERTICES, TH_INDICES);

    let transform = Transform {
        position: DVec3::new(0.0, 10.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::new(1.0, 1.0, 1.0),
    };

    // ===========================================================================
    // 1. CENTRAL STAR (Sun-like)
    // ===========================================================================
    let star_transform = Transform {
        position: DVec3::new(0.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::splat(2.4), // Visually prominent
    };

    // Counter-velocity to cancel planets' momentum and prevent solar system drift
    let star_velocity = Velocity(Vec3::new(-0.0018, 0.0, -0.0127));

    // Initial gravitational pull from both planets on the star
    let star_acceleration = Acceleration(Vec3::new(0.0076, 0.0, -0.0004));

    let star_body = CelestialBody {
        gm: 1000.0,        // Heavy central mass
        softening_sq: 4.0, // Physical radius squared (~2.0 radius)
    };

    // ===========================================================================
    // 2. INNER PLANET (Fast, rocky world)
    // Distance: r = 14.0 | Orbit Period: ~10.4 seconds
    // ===========================================================================
    let inner_transform = Transform {
        position: DVec3::new(14.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::splat(0.7),
    };

    // v = sqrt(GM / r) = sqrt(1000 / 14) ≈ 8.4515 (moving along +Z)
    let inner_velocity = Velocity(Vec3::new(0.0, 0.0, 8.4515));

    // a = -GM / r^2 = -1000 / 196 ≈ -5.102 (pointing toward the star at origin)
    let inner_acceleration = Acceleration(Vec3::new(-5.102, 0.0, 0.0));

    let inner_body = CelestialBody {
        gm: 1.5,            // 1:666 mass ratio to star
        softening_sq: 0.36, // ~0.6 radius
    };

    // ===========================================================================
    // 3. OUTER PLANET (Slower gas giant)
    // Distance: r = 28.0 | Orbit Period: ~29.4 seconds
    // ===========================================================================
    let outer_transform = Transform {
        position: DVec3::new(0.0, 0.0, -28.0), // Placed 90° away on the Z-axis
        rotation: Quat::IDENTITY,
        scale: Vec3::splat(1.1),
    };

    // v = sqrt(GM / r) = sqrt(1000 / 28) ≈ 5.9761 (moving along +X)
    let outer_velocity = Velocity(Vec3::new(5.9761, 0.0, 0.0));

    // a = GM / r^2 = 1000 / 784 ≈ 1.2755 (pointing toward the star along +Z)
    let outer_acceleration = Acceleration(Vec3::new(0.0, 0.0, 1.2755));

    let outer_body = CelestialBody {
        gm: 0.3,            // Smaller mass
        softening_sq: 0.81, // ~0.9 radius
    };

    let texture2 = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/roman.png"), "roman.png").unwrap();

    let texture = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/happy-tree.png"), "happy-tree.png").unwrap();

    let material = Material::new(&render_context.device, &gpu_layout.get(EngineLayout::Material), Arc::new(texture));

    let material2 = Material::new(&render_context.device, &gpu_layout.get(EngineLayout::Material), Arc::new(texture2));

    commands.spawn((mesh, material, transform));
    commands.spawn((th_mesh.clone(), material2.clone(),
        star_transform,
        star_velocity,
        star_acceleration,
        star_body
    ));
    commands.spawn((th_mesh.clone(), material2.clone(),
        inner_transform,
        inner_velocity,
        inner_acceleration,
        inner_body
    ));
    commands.spawn((th_mesh.clone(), material2.clone(),
        outer_transform,
        outer_velocity,
        outer_acceleration,
        outer_body,
    ));
    commands.spawn(camera);
}

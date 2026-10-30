use std::sync::Arc;

use engine::render::camera::{Camera};
use engine::render::{context::RenderContext,
                     mesh::{Mesh, Vertex}};
use engine::render::material::Material;
use engine::render::layout::{EngineLayout, LayoutRegistry};
use engine::render::texture::{Texture};
use engine::render::instance::{Transform};

use crate::physics::vverlet::{Velocity, Acceleration, CelestialBody};

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

    let th_transform1 = Transform {
        position: DVec3::new(2.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::new(1.0, 1.0, 1.0),
    };

    let th_transform2 = Transform {
        position: DVec3::new(0.0, -10.0, -5.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::new(1.0, 1.0, 1.0),
    };

    let th_velocity1 = Velocity(Vec3::new( 6.0, 0.0, 0.0));
    let th_velocity2 = Velocity(Vec3::new(-6.0, 0.0, 0.0));

    let th_acceleration1 = Acceleration(Vec3::new( 1.0, 0.0, 0.0));
    let th_acceleration2 = Acceleration(Vec3::new(-1.0, 0.0, 0.0));

    let th_celest1 = CelestialBody {
        gm: 3000.0,
        softening_sq: 1.0,
    };

    let th_celest2 = CelestialBody {
        gm: 2000.0,
        softening_sq: 1.0,
    };

    let texture2 = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/roman.png"), "roman.png").unwrap();

    let texture = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/happy-tree.png"), "happy-tree.png").unwrap();

    let material = Material::new(&render_context.device, &gpu_layout.get(EngineLayout::Material), Arc::new(texture));

    let material2 = Material::new(&render_context.device, &gpu_layout.get(EngineLayout::Material), Arc::new(texture2));

    commands.spawn((mesh, material, transform));
    commands.spawn((th_mesh.clone(), material2.clone(), th_transform1, th_velocity1, th_acceleration1, th_celest1));
    commands.spawn((th_mesh.clone(), material2.clone(), th_transform2, th_velocity2, th_acceleration2, th_celest2));
    commands.spawn(camera);
}

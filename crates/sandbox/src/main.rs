use std::sync::Arc;

use engine::app::app::App;
use engine::render::texture;
use engine::window::window::WindowConfig;
use engine::render::{context::RenderContext, mesh::{Mesh, Vertex}};
use engine::render::material::Material;
use engine::render::layout::GpuLayout;
use engine::render::texture::Texture;

use bevy_ecs::prelude::*;

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

fn main() {
    App::new()
        .set_window(WindowConfig {
            title: "Space Simulator".to_string(),
            width: 1280,
            height: 720,
        })
        .add_startup_system(test_mesh)
        .run();
}

fn test_mesh(mut commands: Commands, render_context: Res<RenderContext>, gpu_layout: Res<GpuLayout>) {
    let mesh = Mesh::new(&render_context.device, VERTICES, INDICES);

    let texture = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/happy-tree.png"), "happy-tree.png").unwrap();

    let material = Material::new(&render_context.device, &gpu_layout.material, Arc::new(texture));

    commands.spawn((mesh, material));
}
use engine::app::app::App;
use engine::window::window::WindowConfig;
use engine::render::{context::RenderContext, mesh::{Mesh, Vertex}};

use bevy_ecs::prelude::*;

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }, // E
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

fn test_mesh(mut commands: Commands, render_context: Res<RenderContext>) {
    let mesh = Mesh::new(&render_context.device, VERTICES, INDICES);

    commands.spawn(mesh);
}
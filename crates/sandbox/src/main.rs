use core::time;
use std::sync::Arc;

use engine::app::app::App;
use engine::render::camera::{self, Camera};
use engine::window::window::WindowConfig;
use engine::render::{context::RenderContext, mesh::{Mesh, Vertex}};
use engine::render::material::Material;
use engine::render::layout::{EngineLayout, GpuLayout};
use engine::render::texture::{self, Texture};
use engine::render::camera::{GpuCamera, CameraUniform, Projection, SAFE_FRAC_PI_2};
use engine::input::input::InputState;
use engine::time::time::Time;

use bevy_ecs::prelude::*;
use winit::keyboard::KeyCode;

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], uv: [0.4131759, 0.00759614], }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], uv: [0.0048659444, 0.43041354], }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], uv: [0.28081453, 0.949397], }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], uv: [0.85967, 0.84732914], }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], uv: [0.9414737, 0.2652641], }, // E
];

const VERTICES2: &[Vertex] = &[
    Vertex { position: [-0.5, 2.0, 0.0], uv: [0.0, 1.0], }, // Bottom-left
    Vertex { position: [0.5, 2.0, 0.0], uv: [1.0, 1.0], }, // Bottom-right
    Vertex { position: [0.5, 3.0, 0.0], uv: [1.0, 0.0], }, // Top-right
    Vertex { position: [-0.5, 3.0, 0.0], uv: [0.0, 0.0], }, // Top-left
];

const INDICES2: &[u16] = &[
    3, 0, 1,
    3, 1, 2,
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
        .add_startup_system(test)
        .add_system(camera_system)
        .run();
}

fn test(mut commands: Commands, render_context: Res<RenderContext>, gpu_layout: Res<GpuLayout>) {
    let camera =  Camera::from_direction(
        glam::Vec3::new(0.0, 0.0, 2.0),
        glam::Vec3::new(0.0, 0.0, -1.0)
    );

    let mesh = Mesh::new(&render_context.device, VERTICES, INDICES);

    let mesh2 = Mesh::new(&render_context.device, VERTICES2, INDICES2);

    let texture2 = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/roman.png"), "roman.png").unwrap();

    let texture = Texture::from_bytes(&render_context.device, &render_context.queue, include_bytes!("../../../assets/textures/happy-tree.png"), "happy-tree.png").unwrap();

    let material = Material::new(&render_context.device, &gpu_layout.get(EngineLayout::Material), Arc::new(texture));

    let material2 = Material::new(&render_context.device, &gpu_layout.get(EngineLayout::Material), Arc::new(texture2));

    commands.spawn((mesh, material));
    commands.spawn((mesh2, material2));
    commands.spawn(camera);
}

fn camera_system(mut query: Query<&mut Camera>, render_context: Res<RenderContext>, camera_gpu: Res<GpuCamera>, input: Res<InputState>, time: Res<Time>, projection: Res<Projection>) {
    let speed: f32 = 4.0;
    let sensitivity: f32 = 0.2;
    let delta_time = time.delta_time();

    for mut camera in query.iter_mut() {
        let mut camera_uniform = CameraUniform::new();

        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward = glam::Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = glam::Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();

        if input.is_key_pressed(KeyCode::KeyW) {
            camera.position += forward * speed * delta_time;
        }
        if input.is_key_pressed(KeyCode::KeyS) {
            camera.position -= forward * speed * delta_time;
        }
        if input.is_key_pressed(KeyCode::KeyA) {
            camera.position -= right * speed * delta_time;
        }
        if input.is_key_pressed(KeyCode::KeyD) {
            camera.position += right * speed * delta_time;
        }

        let (pitch_sin, pitch_cos) = camera.pitch.sin_cos();
        let scrollward = glam::Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * input.get_scroll_delta().y * speed * sensitivity * delta_time;

        if input.is_key_pressed(KeyCode::Space){
            camera.position += glam::Vec3::Y * speed * delta_time;
        }
        if input.is_key_pressed(KeyCode::ShiftLeft){
            camera.position -= glam::Vec3::Y * speed * delta_time;
        }

        if input.is_button_pressed(winit::event::MouseButton::Left) {
            camera.yaw += input.get_mouse_delta().x * sensitivity * delta_time;
            camera.pitch -= input.get_mouse_delta().y * sensitivity * delta_time;
        }

        camera.pitch = camera.pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        camera_uniform.update_view_proj(&camera, &projection);
        render_context.queue.write_buffer(&camera_gpu.uniform_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }
}

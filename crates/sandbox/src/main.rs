use engine::app::app::App;
use engine::render::camera::{Camera};
use engine::window::window::WindowConfig;
use engine::render::{context::{RenderContext, RenderConfig}};
use engine::render::camera::{GpuCamera, CameraUniform, Projection, SAFE_FRAC_PI_2};
use engine::input::input::InputState;
use engine::time::time::Time;

use sandbox::physics::vverlet::nbody_vverlet_system;
use sandbox::render::systems::sandbox_render_system;
use sandbox::render::skybox::{setup_skybox_system, update_skybox_system};
use sandbox::test_system::test_system;

use bevy_ecs::prelude::*;
use winit::keyboard::KeyCode;


fn main() {
    App::new()
        .set_window_config(WindowConfig {
            title: "Space Simulator".to_string(),
            width: 1280,
            height: 720,
        })
        .set_render_config( RenderConfig {
            vsync: true,
            ..Default::default()
        })

        // STARTUP (runs once at launch)
        .add_startup_system((test_system, setup_skybox_system))

        // UPDATE (runs every tick in about_to_wait)
        .add_system((camera_system, update_skybox_system, nbody_vverlet_system))

        // RENDER (runs on WindowEvent::RedrawRequested)
        .set_render_system(sandbox_render_system)
        .run();
}

fn camera_system(mut query: Query<&mut Camera>, render_context: Res<RenderContext>, camera_gpu: Res<GpuCamera>, input: Res<InputState>, time: Res<Time>, projection: Res<Projection>) {
    let speed: f64 = 4.0;
    let sensitivity: f32 = 0.2;
    let delta_time = time.delta_time() as f64;
    let delta_time_f32 = time.delta_time() as f32;

    for mut camera in query.iter_mut() {
        let mut camera_uniform = CameraUniform::new();

        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();

        let yaw_sin = yaw_sin as f64;
        let yaw_cos = yaw_cos as f64;

        let forward = glam::DVec3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = glam::DVec3::new(-yaw_sin, 0.0, yaw_cos).normalize();

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

        let pitch_sin = pitch_sin as f64;
        let pitch_cos = pitch_cos as f64;

        let scrollward = glam::DVec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * input.get_scroll_delta().y as f64 * speed * sensitivity as f64 * delta_time;

        if input.is_key_pressed(KeyCode::Space){
            camera.position += glam::DVec3::Y * speed * delta_time;
        }
        if input.is_key_pressed(KeyCode::ShiftLeft){
            camera.position -= glam::DVec3::Y * speed * delta_time;
        }

        if input.is_button_pressed(winit::event::MouseButton::Left) {
            camera.yaw += input.get_mouse_delta().x * sensitivity * delta_time_f32;
            camera.pitch -= input.get_mouse_delta().y * sensitivity * delta_time_f32;
        }

        camera.pitch = camera.pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        camera_uniform.update_view_proj(&camera, &projection);
        render_context.queue.write_buffer(&camera_gpu.uniform_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }
}

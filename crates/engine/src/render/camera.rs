use std::f32::consts::FRAC_PI_2;

use glam::{Mat4, Vec3};
use glam::camera::rh::{view::look_at_mat4, proj::directx::perspective};

use bevy_ecs::prelude::*;

use wgpu::util::DeviceExt;

pub const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Component)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self { position, yaw, pitch }
    }

    pub fn from_direction(position: Vec3, direction: Vec3) -> Self {
        let dir = direction.normalize();

        let pitch = dir.y.clamp(-1.0, 1.0).asin();

        let yaw = dir.z.atan2(dir.x);

        Self { position, yaw, pitch }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
        let (pitch_sin, pitch_cos) = self.pitch.sin_cos();

        let forward = Vec3::new(
            pitch_cos * yaw_cos,
            pitch_sin,
            pitch_cos * yaw_sin,
        ).normalize();

        look_at_mat4(self.position, self.position + forward, Vec3::Y)
    }
}

#[derive(Resource)]
pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self { aspect, fovy, znear, zfar }
    }

    pub fn resize(&mut self, new_aspect: f32) {
        self.aspect = new_aspect;
    }

    pub fn calc_matrix(&self) -> Mat4 {
        perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).to_cols_array_2d();
    }
}

#[derive(Resource)]
pub struct GpuCamera {
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl GpuCamera {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let camera_uniform = CameraUniform::new();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
            label: Some("Camera Bind Group"),
        });

        Self {
            uniform_buffer: camera_buffer,
            bind_group: camera_bind_group,
        }
    }
}

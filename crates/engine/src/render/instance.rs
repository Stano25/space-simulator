use bevy_ecs::prelude::*;
use glam::{DVec3, Quat, Vec3};
use std::sync::Arc;
use crate::render::{material::GpuMaterial, mesh::GpuMesh};


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model_matrix: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn from_transform(transform: &Transform, camera_pos: DVec3) -> Self {
        let relative_pos_f64: DVec3 = transform.position - camera_pos;

        let relative_pos_f32: Vec3 = relative_pos_f64.as_vec3();

        let model_matrix = glam::Mat4::from_scale_rotation_translation(
            transform.scale,
            transform.rotation,
            relative_pos_f32,
        );
        Self {
            model_matrix: model_matrix.to_cols_array_2d(),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Transform {
    pub position: DVec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Resource)]
pub struct InstanceManager {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub capacity: usize,
}

impl InstanceManager {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, initial_capacity: usize) -> Self {
        let size = (initial_capacity * std::mem::size_of::<InstanceRaw>()) as u64;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Instance Bind Group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            buffer,
            bind_group,
            capacity: initial_capacity,
        }
    }

    pub fn ensure_capacity(&mut self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout, required_capacity: usize) {
        if required_capacity < self.capacity { return; }

        let new_capacity = (self.capacity * 2).max(required_capacity);
        let new_size = (new_capacity * std::mem::size_of::<InstanceRaw>()) as u64;

        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: new_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let new_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Instance Bind Group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: new_buffer.as_entire_binding(),
            }],
        });

        self.buffer = new_buffer;
        self.bind_group = new_bind_group;
        self.capacity = new_capacity;
    }
}

pub struct DrawBatch {
    pub mesh: Arc<GpuMesh>,
    pub material: Arc<GpuMaterial>,
    pub start_instance: u32,
    pub count: u32,
}

pub struct BatchGroup {
    pub mesh: Arc<GpuMesh>,
    pub material: Arc<GpuMaterial>,
    pub instances: Vec<InstanceRaw>,
}

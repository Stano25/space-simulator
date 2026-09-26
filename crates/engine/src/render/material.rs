use std::sync::Arc;

use bevy_ecs::prelude::*;

use crate::render::texture::Texture;

pub struct GpuMaterial {
    pub bind_group: wgpu::BindGroup,
    pub diffuse_texture: Arc<Texture>,
}

#[derive(Component, Clone)]
pub struct Material {
    pub gpu_material: Arc<GpuMaterial>,
}

impl Material {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, diffuse_texture: Arc<Texture>) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("Material Bind Group"),
        });

        Self { gpu_material: Arc::new(GpuMaterial { bind_group, diffuse_texture }) }
    }
}
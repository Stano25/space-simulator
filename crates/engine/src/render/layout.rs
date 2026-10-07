use bevy_ecs::prelude::*;
use std::any::TypeId;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayoutId {
    type_id: TypeId,
    index: usize,
}

pub trait IntoLayoutKey: Copy + 'static {
    fn into_key(self) -> LayoutId;
}

#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum EngineLayout {
    Material,
    Camera,
}

impl IntoLayoutKey for EngineLayout {
    fn into_key(self) -> LayoutId {
        LayoutId {
            type_id: TypeId::of::<Self>(),
            index: self as usize,
        }
    }
}

#[derive(Resource)]
pub struct GpuLayout {
    pub gpu_layouts: HashMap<LayoutId, wgpu::BindGroupLayout>,
}

impl GpuLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Material Bind Group Layout"),
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
            ],
            label: Some("Camera Bind Group Layout"),
        });

        let mut gpu_layouts: HashMap<LayoutId, wgpu::BindGroupLayout> = HashMap::new();
        gpu_layouts.insert(EngineLayout::Material.into_key(), material_layout);
        gpu_layouts.insert(EngineLayout::Camera.into_key(), camera_layout);
        GpuLayout {
            gpu_layouts
        }
    }
    // Direct access
    pub fn get(&self, key: impl IntoLayoutKey) -> &wgpu::BindGroupLayout {
            let key_id = key.into_key();
            self.gpu_layouts
                .get(&key_id)
                .unwrap_or_else(|| panic!("BindGroupLayout not found for key: {:?}", key_id))
        }
    // Safe access
    pub fn try_get(&self, key: impl IntoLayoutKey) -> Option<&wgpu::BindGroupLayout> {
        self.gpu_layouts.get(&key.into_key())
    }
}

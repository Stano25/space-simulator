use bevy_ecs::prelude::*;
use wgpu::util::DeviceExt;

use crate::keys::{SandboxLayout, SandboxPipeline};
use engine::render::{
    context::RenderContext,
    camera::{Camera, Projection},
    layout::LayoutRegistry,
    pipeline::PipelineRegistry,
    texture::Texture,
};
use glam::Vec4;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyboxUniforms {
    pub inv_view_proj: [[f32; 4]; 4],
}

#[derive(Resource)]
pub struct GpuSkybox {
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

pub fn setup_skybox_system(
    render_context: Res<RenderContext>,
    mut layout_registry: ResMut<LayoutRegistry>,
    mut pipeline_registry: ResMut<PipelineRegistry>,
    mut commands: Commands,
) {
    let device = &render_context.device;
    let queue = &render_context.queue;

    let initial_uniforms = SkyboxUniforms {
        inv_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
    };

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Skybox Uniform Buffer"),
        contents: bytemuck::cast_slice(&[initial_uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Baking the galaxy cubemap once at startup (2048x2048 per face)
    let cubemap = bake_cubemap(device, queue, 2048);

    let skybox_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Skybox Bind Group Layout"),
        entries: &[
            // Binding 0: Uniform Buffer (inv_view_proj + time)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Binding 1: Baked Cubemap
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            // Binding 2: Cubemap Sampler
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    layout_registry.insert(SandboxLayout::Skybox, skybox_layout);
    let layout = layout_registry.get(SandboxLayout::Skybox);

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Skybox Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&cubemap.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&cubemap.sampler),
            },
        ],
    });

    // Create and register the runtime Skybox Render Pipeline
    let runtime_shader = device.create_shader_module(wgpu::include_wgsl!("../../../../assets/shaders/skybox.wgsl"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Skybox Runtime Pipeline Layout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });

    let runtime_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Skybox Runtime Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &runtime_shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &runtime_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: render_context.config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    pipeline_registry.insert(SandboxPipeline::Skybox, runtime_pipeline);

    commands.insert_resource(GpuSkybox { uniform_buffer, bind_group });
}

pub fn update_skybox_system(
    skybox: Option<Res<GpuSkybox>>,
    render_context: Res<RenderContext>,
    projection: Res<Projection>,
    camera_query: Query<&Camera>,
) {
    let (Some(skybox), Some(camera)) = (skybox, camera_query.iter().next()) else {
        return;
    };

    // Calculating inverted proj * view matrix for the
    // skybox shader to get the view direction per pixel
    let mut view = camera.calc_matrix();
    view.w_axis = Vec4::new(0.0, 0.0, 0.0, 1.0);

    let view_proj = projection.calc_matrix() * view;
    let inv_view_proj = view_proj.inverse();

    let uniforms = SkyboxUniforms {
        inv_view_proj: inv_view_proj.to_cols_array_2d(),
    };

    render_context.queue.write_buffer(
        &skybox.uniform_buffer,
        0,
        bytemuck::cast_slice(&[uniforms]),
    );
}

// Bakes the galaxy into a 6-face cubemap texture using the compute shader
fn bake_cubemap(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: u32
) -> Texture {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../../../assets/shaders/bake_cubemap.wgsl"));

    // Create the cubemap texture with STORAGE (write) and TEXTURE_BINDING (read)
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Baked Galaxy Cubemap"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let storage_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Galaxy Compute Storage View"),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });

    let cube_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Galaxy Cube Sample View"),
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Galaxy Cubemap Sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    // Auto-layout compute pipeline
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Bake Galaxy Pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bake Galaxy Bind Group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&storage_view),
        }],
    });

    // Dispatch
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(size / 16, size / 16, 6);
    }
    queue.submit(std::iter::once(encoder.finish()));

    Texture {
        texture,
        view: cube_view,
        sampler,
    }
}

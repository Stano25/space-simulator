use bevy_ecs::prelude::*;
use wgpu::util::DeviceExt;

use crate::keys::{SandboxLayout, SandboxPipeline};
use engine::render::{
    camera::{Camera, Projection},
    context::RenderContext,
    layout::LayoutRegistry,
    pipeline::PipelineRegistry,
};
use engine::time::time::Time;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyboxUniforms {
    pub inv_view_proj: [f32; 16],
    pub time: f32,
    pub _pad: [f32; 3],
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

    // 1. Create and insert BindGroupLayout using SandboxLayout::Skybox
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Skybox Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    layout_registry.insert(SandboxLayout::Skybox, bind_group_layout);
    let layout = layout_registry.get(SandboxLayout::Skybox);

    // 2. Uniform Buffer and BindGroup
    let uniform = SkyboxUniforms {
        inv_view_proj: [0.0; 16],
        time: 0.0,
        _pad: [0.0; 3],
    };

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Skybox Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Skybox Bind Group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    // 3. Pipeline creation and registration using SandboxPipeline::Skybox
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../../../assets/shaders/skybox.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Skybox Pipeline Layout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Skybox Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
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

    pipeline_registry.insert(SandboxPipeline::Skybox, pipeline);

    commands.insert_resource(GpuSkybox {
        uniform_buffer,
        bind_group,
    });
}

pub fn update_skybox_system(
    time: Res<Time>,
    skybox: Option<Res<GpuSkybox>>,
    render_context: Res<RenderContext>,
    projection: Res<Projection>,
    camera_query: Query<&Camera>,
) {
    let (Some(skybox), Some(camera)) = (skybox, camera_query.iter().next()) else {
        return;
    };

    let mut view = camera.calc_matrix();
    view.w_axis = glam::Vec4::new(0.0, 0.0, 0.0, 1.0);

    let view_proj = projection.calc_matrix() * view;
    let inv_view_proj = view_proj.inverse();

    let uniforms = SkyboxUniforms {
        inv_view_proj: inv_view_proj.to_cols_array(),
        time: time.elapsed_time,
        _pad: [0.0; 3],
    };

    render_context.queue.write_buffer(
        &skybox.uniform_buffer,
        0,
        bytemuck::cast_slice(&[uniforms]),
    );
}

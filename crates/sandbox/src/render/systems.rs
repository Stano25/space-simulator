use bevy_ecs::prelude::*;

use engine::render::{
    camera::{GpuCamera, Camera},
    context::RenderContext,
    instance::{InstanceManager, InstanceRaw, Transform, BatchGroup, DrawBatch},
    layout::{EngineLayout, LayoutRegistry},
    material::Material,
    mesh::{GpuMesh, Mesh},
    pipeline::{EnginePipeline, PipelineRegistry},
    material::GpuMaterial,
};

use std::collections::HashMap;
use std::sync::Arc;

use crate::keys::SandboxPipeline;
use crate::render::skybox::GpuSkybox;

pub fn sandbox_render_system(
    mut render_context: ResMut<RenderContext>,
    pipeline_registry: Res<PipelineRegistry>,
    query: Query<(&Mesh, &Material, &Transform)>,
    camera_gpu: Res<GpuCamera>,
    mut instance_manager: ResMut<InstanceManager>,
    layouts: Res<LayoutRegistry>,
    skybox: Option<Res<GpuSkybox>>,
    camera_query: Query<&Camera>,
) {
    let current_surface_texture = render_context.surface.get_current_texture();
    let drawable = match current_surface_texture {
        wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        wgpu::CurrentSurfaceTexture::Timeout => return,
        wgpu::CurrentSurfaceTexture::Outdated => {
            render_context.update_surface();
            return;
        }
        _ => return,
    };

    type BatchKey = (*const GpuMesh, *const GpuMaterial);

    let mut groups: HashMap<BatchKey, BatchGroup> = HashMap::new();

    let Some(camera) = camera_query.iter().next() else {
        return;
    };

    for (mesh, material, transform) in query.iter() {
        let key = (Arc::as_ptr(&mesh.gpu_mesh), Arc::as_ptr(&material.gpu_material));
        let batch_group = groups.entry(key).or_insert_with(|| BatchGroup {
            mesh: Arc::clone(&mesh.gpu_mesh),
            material: Arc::clone(&material.gpu_material),
            instances: Vec::new(),
        });

        let instance_raw = InstanceRaw::from_transform(transform, camera.position);
        batch_group.instances.push(instance_raw);
    }

    let mut all_instances: Vec<InstanceRaw> = Vec::new();
    let mut batches: Vec<DrawBatch> = Vec::new();

    for (_, group) in groups {
        let start_instance = all_instances.len() as u32;
        let count = group.instances.len() as u32;

        all_instances.extend_from_slice(&group.instances);

        batches.push(DrawBatch {
            mesh: group.mesh,
            material: group.material,
            start_instance,
            count,
        });
    }

    instance_manager.ensure_capacity(
        &render_context.device,
        &layouts.get(EngineLayout::Instance),
        all_instances.len(),
    );

    render_context.queue.write_buffer(
        &instance_manager.buffer,
        0,
        bytemuck::cast_slice(&all_instances),
    );

    let image_view = drawable.texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut command_encoder = render_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Sandbox Render Encoder"),
    });

    let render_pass_descriptor = wgpu::RenderPassDescriptor {
        label: Some("Main Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &image_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.001, g: 0.002, b: 0.004, a: 1.0 }),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &render_context.depth_texture.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        occlusion_query_set: None,
        multiview_mask: None,
        timestamp_writes: None,
    };

    {
        let mut render_pass = command_encoder.begin_render_pass(&render_pass_descriptor);

        // 1. DRAW SKYBOX (Background)
        if let (Some(skybox_pipeline), Some(skybox)) = (
            pipeline_registry.try_get(SandboxPipeline::Skybox),
            skybox.as_ref(),
        ) {
            render_pass.set_pipeline(skybox_pipeline);
            render_pass.set_bind_group(0, &skybox.bind_group, &[]);
            render_pass.draw(0..3, 0..1); // 1 fullscreen triangle
        }

        // 2. DRAW OPAQUE MESHES (Foreground)
        if let Some(default_pipeline) = pipeline_registry.try_get(EnginePipeline::Default) {
            render_pass.set_pipeline(default_pipeline);

            render_pass.set_bind_group(1, &camera_gpu.bind_group, &[]);
            render_pass.set_bind_group(2, &instance_manager.bind_group, &[]);

            for batch in batches {
                render_pass.set_bind_group(0, &batch.material.bind_group, &[]);

                render_pass.set_vertex_buffer(0, batch.mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(batch.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(
                    0..batch.mesh.index_count,
                    0,
                    batch.start_instance..(batch.start_instance + batch.count),
                );
            }
        }
    }

    render_context.queue.submit(std::iter::once(command_encoder.finish()));
    render_context.queue.present(drawable);
}

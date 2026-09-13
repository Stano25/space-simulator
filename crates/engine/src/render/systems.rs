use bevy_ecs::prelude::*;

use crate::render::{context::RenderContext};

pub fn render_system(mut render_context: ResMut<RenderContext>) {
    let current_surface_texture = render_context.surface.get_current_texture();
    let drawable = match current_surface_texture {
        wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        wgpu::CurrentSurfaceTexture::Timeout => {
            return;
        }
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            let (width, height) = render_context.size;
            render_context.resize(width, height);
            return;
        }
        _ => return,
    };

    let image_view_descriptor = wgpu::TextureViewDescriptor::default();
    let image_view = drawable.texture.create_view(&image_view_descriptor);
    
    let command_encoder_descriptor = wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    };
    let mut command_encoder = render_context.device.create_command_encoder(&command_encoder_descriptor);

    let color_attachment = wgpu::RenderPassColorAttachment {
        view: &image_view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.2, g: 0.3, b: 0.3, a: 1.0, }),
            store: wgpu::StoreOp::Store,
        },
        depth_slice: None,
    };

    let render_pass_descriptor = wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(color_attachment)],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        multiview_mask: None,
        timestamp_writes: None,
    };

    command_encoder.begin_render_pass(&render_pass_descriptor);
    render_context.queue.submit(std::iter::once(command_encoder.finish()));
    
    render_context.queue.present(drawable);
}
use std::collections::HashMap;

use bevy_ecs::prelude::Resource;

use crate::render::mesh::Vertex;

#[derive(Resource, Default)]
pub struct PipelineRegistry{
    pub pipelines: HashMap<String, wgpu::RenderPipeline>
}

pub struct PipelineBuilder<'a> {
    shader_path: String,
    vertex_entry: String,
    fragment_entry: String,
    layouts: Vec<Option<&'a wgpu::BindGroupLayout>>,
    pixel_format: wgpu::TextureFormat
}

impl<'a> PipelineBuilder<'a> {
    pub fn new(shader_path: impl Into<String>) -> Self {
        Self { 
            shader_path: shader_path.into(),
            vertex_entry: String::from("vs_main"),
            fragment_entry: String::from("fs_main"),
            layouts: Vec::new(),
            pixel_format: wgpu::TextureFormat::Rgba8UnormSrgb
        }
    }

    pub fn with_vertex_entry(mut self, entry: impl Into<String>) -> Self {
        self.vertex_entry = entry.into();
        self
    }

    pub fn with_fragment_entry(mut self, entry: impl Into<String>) -> Self {
        self.fragment_entry = entry.into();
        self
    }

    pub fn with_pixel_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.pixel_format = format;
        self
    }

    pub fn with_layout(mut self, layout: &'a wgpu::BindGroupLayout) -> Self {
        self.layouts.push(Some(layout));
        self
    }

    pub fn with_layouts(mut self, layouts: &[&'a wgpu::BindGroupLayout]) -> Self {
        self.layouts = layouts.iter().map(|layout| Some(*layout)).collect();
        self
    }

    pub fn build(self, device: &wgpu::Device) -> wgpu::RenderPipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(self.shader_path.into())
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &self.layouts, 
            immediate_size: 0 
        });

        let render_targets = wgpu::ColorTargetState {
            format: self.pixel_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState { 
                module: &shader_module, 
                entry_point: Some(self.vertex_entry.as_str()), 
                compilation_options: wgpu::PipelineCompilationOptions::default(), 
                buffers: &[Some(Vertex::desc())] 
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some(self.fragment_entry.as_str()),
                targets: &[Some(render_targets)],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }), 
            primitive: wgpu::PrimitiveState { 
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None, 
                front_face: wgpu::FrontFace::Ccw, 
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,  
                conservative: false 
            }, 
            depth_stencil: None, 
            multisample: wgpu::MultisampleState { 
                count: 1, 
                mask: !0, 
                alpha_to_coverage_enabled: false
            }, 
            multiview_mask: None, 
            cache: None 
        });

        render_pipeline
    }
}
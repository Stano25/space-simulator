use std::sync::Arc;

use winit::window::Window;
use bevy_ecs::prelude::Resource;

#[derive(Resource)]
pub struct RenderContext {
    instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: (u32, u32),
    window: Arc<Window>,
}

impl RenderContext {
    pub async fn new(window: Arc<Window>) -> Self{
        let size = window.inner_size();

        let instance_descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        };
        let instance = wgpu::Instance::new(instance_descriptor);

        let surface = instance.create_surface(window.clone()).expect("Failed to create surface");

        let adapter_descriptor = wgpu::RequestAdapterOptions{
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false, // Only for browser, not needed for desktop
        };
        let adapter = instance.request_adapter(&adapter_descriptor).await.expect("Failed to request adapter");

        let device_descriptor = wgpu::DeviceDescriptor {
            label: Some("Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        };
        let (device, queue) = adapter.request_device(&device_descriptor).await.expect("Failed to request device");

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format  = surface_capabilities.formats.iter()
            .copied().filter(|f| f.is_srgb())
            .next().unwrap_or(surface_capabilities.formats[0]);
        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],   
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto
        };
        surface.configure(&device, &surface_configuration);

        Self {
            instance: instance,
            surface: surface,
            device: device,
            queue: queue,
            config: surface_configuration,
            size: (size.width, size.height),
            window: window,
        }
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            self.size = (new_width, new_height);
            self.config.width = new_width;
            self.config.height = new_height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn update_surface(&mut self) {
        self.surface = self.instance.create_surface(self.window.clone()).unwrap();
        self.surface.configure(&self.device, &self.config);
    }
}
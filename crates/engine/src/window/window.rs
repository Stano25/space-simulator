use winit::dpi::LogicalSize;
use winit::window::{Window, WindowAttributes};

#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Window".to_string(),
            width: 1280,
            height: 720,
        }
    }
}

impl WindowConfig {
    pub fn to_attributes(&self) -> WindowAttributes {
        WindowAttributes::default()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(self.width, self.height))
    }
}
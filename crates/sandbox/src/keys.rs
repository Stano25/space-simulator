use std::any::TypeId;
use engine::render::{
    layout::{IntoLayoutKey, LayoutId},
    pipeline::{IntoPipelineKey, PipelineId},
};

// ==========================================
// Sandbox Bind Group Layouts
// ==========================================
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SandboxLayout {
    Skybox,
}

impl IntoLayoutKey for SandboxLayout {
    fn into_key(self) -> LayoutId {
        LayoutId {
            type_id: TypeId::of::<Self>(),
            index: self as usize,
        }
    }
}

// ==========================================
// Sandbox Render Pipelines
// ==========================================
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SandboxPipeline {
    Skybox,
}

impl IntoPipelineKey for SandboxPipeline {
    fn into_key(self) -> PipelineId {
        PipelineId {
            type_id: TypeId::of::<Self>(),
            index: self as usize,
        }
    }
}

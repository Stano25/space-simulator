# Space Simulator

Rust workspace (edition 2024): reusable engine crate + simulator game crate.

## Layout
- `crates/engine` — game-agnostic engine (app/event loop, window, input, time, wgpu render)
- `crates/sandbox` — binary wiring ECS systems into `engine::App`; all gameplay lives here
- `assets/shaders` — WGSL shaders; embedded with `include_str!`/`include_wgsl!` relative paths

## Commands
- `cargo run -p sandbox` — run the simulator
- `cargo check` — must pass without warnings

## Architecture rules
- `engine` must never depend on `sandbox` or know about game content; anything simulation-specific goes in `sandbox`
- New engine subsystems get their own directory module with `mod.rs` (see `render/`, `input/`)

## ECS conventions (bevy_ecs as library, custom `App`)
- Systems are plain functions named `*_system`, registered in `sandbox/src/main.rs` via
  `.add_startup_system` (once), `.add_system` (every tick), `.set_render_system` (RedrawRequested)
- Order dependencies explicitly: `.system_name.before(after_name)`
- Shared state: `#[derive(Resource)]` resources; entities: components (see `physics/components.rs`)

## Style
- Physics/positions: `f64`/`glam::DVec3`; GPU buffers: `f32` structs via `bytemuck` (Pod)
- GPU resources: build pipelines through `PipelineBuilder`/`PipelineRegistry`, bind groups via `LayoutRegistry`

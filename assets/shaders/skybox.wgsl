struct Uniforms {
    inv_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var sky_tex: texture_cube<f32>;
@group(0) @binding(2) var sky_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ndc_xy: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(
        f32((in_vertex_index << 1u) & 2u),
        f32(in_vertex_index & 2u)
    );
    let ndc_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    out.position = vec4<f32>(ndc_xy, 1.0, 1.0);
    out.ndc_xy = ndc_xy;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ndc = vec4<f32>(in.ndc_xy, 1.0, 1.0);
    let world_pos = uniforms.inv_view_proj * ndc;
    let dir = normalize(world_pos.xyz / world_pos.w);

    let sky_color = textureSample(sky_tex, sky_sampler, dir).rgb;

    return vec4<f32>(sky_color, 1.0);
}

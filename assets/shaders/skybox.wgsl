struct Uniforms {
    inv_view_proj: mat4x4<f32>,
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

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

fn hash3(p: vec3<f32>) -> vec3<f32> {
    var q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yxz + 33.33);
    return fract((q.xxy + q.yxx) * q.zyx);
}

fn star_color(seed: f32) -> vec3<f32> {
    if (seed < 0.65) {
        return vec3<f32>(0.95, 0.97, 1.0); // Neutral / White
    } else if (seed < 0.85) {
        return vec3<f32>(0.75, 0.88, 1.0); // Hot Blue (Class O/B)
    } else {
        return vec3<f32>(1.0, 0.82, 0.65); // Warm Yellow/Orange (Class K/M)
    }
}

// Single, robust star generator for ALL layers (No neighbor loop needed!)
fn render_star_layer(
    dir: vec3<f32>,
    scale: f32,
    threshold: f32,
    brightness: f32,
    core_sharpness: f32,
    halo_size: f32
) -> vec3<f32> {
    let grid = floor(dir * scale);
    let rnd = hash3(grid);

    if (rnd.x < threshold) {
        return vec3<f32>(0.0);
    }

    // Keep offset safely inside [-0.25, 0.25] so stars never touch cell borders
    let star_raw_pos = grid + 0.5 + (rnd - 0.5) * 0.5;
    let star_dir = normalize(star_raw_pos);

    let diff = (dir - star_dir) * scale;
    let d = length(diff);

    // Realistic optical point-source: blistering core + soft airy halo
    let core = exp(-d * d * core_sharpness);
    let halo = exp(-d * halo_size) * 0.12;
    let glow = core + halo;

    let color = star_color(rnd.z);
    return color * glow * brightness;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ndc = vec4<f32>(in.ndc_xy, 1.0, 1.0);
    let world_pos = uniforms.inv_view_proj * ndc;
    let dir = normalize(world_pos.xyz / world_pos.w);

    var final_color = vec3<f32>(0.001, 0.002, 0.004);

    // LAYER 1: Dense microscopic background stars
    final_color += render_star_layer(dir, 400.0, 0.940, 0.6, 500.0, 30.0);

    // LAYER 2: Medium visible stars
    final_color += render_star_layer(dir, 160.0, 0.970, 1.4, 300.0, 22.0);

    // LAYER 3: Hero prominent stars (Pure intense white-hot cores + soft halos)
    final_color += render_star_layer(dir, 60.0, 0.990, 3.2, 160.0, 12.0);

    return vec4<f32>(final_color, 1.0);
}

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

// ---------------------------------------------------------------------------
// Star layer: seamless, no cube faces, no projection.
//
// The view direction is scaled by `scale` and a plain 3D integer grid is laid
// over the result. Every cell may hold one star at a jittered 3D position. The
// star is then pushed onto the unit sphere (normalize) and its distance to the
// pixel is measured in 3D between two unit vectors, so the profile is perfectly
// circular everywhere on the sky.
//
// Why there are no seams: a 3D grid has no faces or edges that mean anything to
// the sky. The cell lookup is the same function of the direction in every
// direction, and each star is kept or dropped purely from its own fixed
// position (never from the view), so nothing can pop or get cut.
//
// Lookup safety: only stars whose radius is within SHELL_HALF of `scale` are
// kept. The pixel point p = dir * scale lies on that shell, so any kept star
// within ~0.7 cell units of the pixel is always inside the 3x3x3 neighbourhood.
// Glow beyond that distance is below 1e-4, which is invisible.
// ---------------------------------------------------------------------------
const SHELL_HALF: f32 = 0.5;
const JITTER: f32 = 0.7;

fn render_star_layer(
    dir: vec3<f32>,
    scale: f32,
    threshold: f32,
    brightness: f32,
    core_sharpness: f32,
    halo_size: f32
) -> vec3<f32> {
    let base_cell = floor(dir * scale);

    // Decorrelates the layers, which would otherwise hash identical integer cells.
    let layer_seed = vec3<f32>(scale * 13.7);

    var star_accum = vec3<f32>(0.0);

    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let cell = base_cell + vec3<f32>(f32(x), f32(y), f32(z));

                let rnd = hash3(cell + layer_seed);
                if (rnd.x < threshold) {
                    continue;
                }

                // Jittered 3D position inside the cell
                let jit = hash3(cell + layer_seed + vec3<f32>(41.7, 13.3, 29.1));
                let star_pos = cell + 0.5 + (jit - 0.5) * JITTER;

                // Keep only stars near the shell the sky sphere sits in
                let r = length(star_pos);
                if (abs(r - scale) > SHELL_HALF) {
                    continue;
                }

                // Push onto the unit sphere, measure the isotropic distance
                let star_dir = star_pos / r;
                let d = length(dir - star_dir) * scale;

                let core = exp(-d * d * core_sharpness);
                let halo = exp(-d * halo_size) * 0.12;
                let glow = core + halo;

                // rnd.x is uniform in [threshold, 1), so remap it to [0, 1) for the color
                let color_seed = (rnd.x - threshold) / (1.0 - threshold);
                star_accum += star_color(color_seed) * glow * brightness;
            }
        }
    }

    return star_accum;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ndc = vec4<f32>(in.ndc_xy, 1.0, 1.0);
    let world_pos = uniforms.inv_view_proj * ndc;
    let dir = normalize(world_pos.xyz / world_pos.w);

    var final_color = vec3<f32>(0.001, 0.002, 0.004);

    // LAYER 1: Dense microscopic background stars
    final_color += render_star_layer(dir, 250.0, 0.885, 0.6, 500.0, 30.0);

    // LAYER 2: Medium visible stars
    final_color += render_star_layer(dir, 100.0, 0.943, 1.4, 300.0, 22.0);

    // LAYER 3: Hero prominent stars
    final_color += render_star_layer(dir, 40.0, 0.981, 3.2, 160.0, 14.0);

    return vec4<f32>(final_color, 1.0);
}

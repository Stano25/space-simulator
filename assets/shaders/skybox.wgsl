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
    @builtin(position) clip_position: vec4<f32>,
    @location(0) ndc_xy: vec2<f32>, // custom interpolated NDC attribute
};

// Fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(
        f32((in_vertex_index << 1u) & 2u),
        f32(in_vertex_index & 2u)
    );
    let ndc_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    // Draw at the far plane (depth = 1.0)
    out.clip_position = vec4<f32>(ndc_xy, 1.0, 1.0);
    out.ndc_xy = ndc_xy;
    return out;
}

// 3D pseudo-random hash returning 3 random floats
fn hash3(p: vec3<f32>) -> vec3<f32> {
    var q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yxz + 33.33);
    return fract((q.xxy + q.yxx) * q.zyx);
}

// Blackbody-inspired star colors (cool blue, white, warm yellow/orange)
fn star_color(seed: f32) -> vec3<f32> {
    if (seed < 0.6) {
        return vec3<f32>(0.9, 0.95, 1.0); // Neutral white / faint blue
    } else if (seed < 0.85) {
        return vec3<f32>(0.7, 0.85, 1.0); // Bright Blue (hot star)
    } else {
        return vec3<f32>(1.0, 0.75, 0.5); // Red/Orange (cool giant)
    }
}

// Single procedural star layer
fn render_star_layer(
    dir: vec3<f32>,
    scale: f32,
    threshold: f32,
    twinkle_speed: f32,
    is_hero: bool
) -> vec3<f32> {
    let grid = floor(dir * scale);
    let rnd = hash3(grid);

    // Only generate a star if the cell's random value beats the threshold
    if (rnd.x < threshold) {
        return vec3<f32>(0.0);
    }

    // Offset the star position inside its cell
    let star_pos = (rnd - 0.5) * 0.7;
    // where the pixel is in the cell
    let cell_fract = fract(dir * scale) - 0.5;
    let diff = cell_fract - star_pos;

    // Project diff onto the plane perpendicular to the ray (dir).
    // This gives the exact angular distance from the line of sight to the star,
    // preventing stars from being clipped by radial depth.
    let diff_perp = diff - dot(diff, dir) * dir;

    let d = length(diff_perp);

    // Point intensity falloff
    var glow = smoothstep(0.08, 0.0, d);

    // If it's a hero star, add 4-point cross diffraction spikes
    if (is_hero) {
        // Construct an orthonormal 2D basis (tangent & bitangent) perpendicular to the view ray
        let ref_up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(dir.y) > 0.99);
        let tangent = normalize(cross(ref_up, dir));
        let bitangent = cross(dir, tangent);

        // Project the 3D offset into 2D camera plane coordinates
        let delta = abs(vec2<f32>(dot(diff_perp, tangent), dot(diff_perp, bitangent)));

        let spike = max(
            smoothstep(0.015, 0.0, delta.x) * smoothstep(0.35, 0.0, delta.y),
            smoothstep(0.015, 0.0, delta.y) * smoothstep(0.35, 0.0, delta.x)
        );
        glow += spike * 0.6;
    }

    // Twinkling animation (desynchronized by random seed)
    let twinkle = 0.5 + 0.5 * sin(uniforms.time * twinkle_speed + rnd.y * 6.2831);
    let color = star_color(rnd.z);

    return color * glow * twinkle;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct the 3D world ray from screen coordinates
    let ndc = vec4<f32>(in.ndc_xy, 1.0, 1.0);
    let world_pos = uniforms.inv_view_proj * ndc;
    let dir = normalize(world_pos.xyz / world_pos.w);

    var final_color = vec3<f32>(0.001, 0.002, 0.004); // Deep space background tint

    // LAYER 1: Background star dust (dense, tiny, static)
    let dust = hash3(floor(dir * 600.0));
    if (dust.x > 0.988) {
        final_color += vec3<f32>(0.2, 0.25, 0.3) * smoothstep(0.988, 1.0, dust.x);
    }

    // LAYER 2: Main starfield (medium density, twinkling, colored)
    final_color += render_star_layer(dir, 180.0, 0.982, 3.5, false) * 1.5;

    // LAYER 3: Hero stars (rare, intense, diffraction spikes)
    final_color += render_star_layer(dir, 55.0, 0.995, 1.5, true) * 3.5;

    return vec4<f32>(final_color, 1.0);
}

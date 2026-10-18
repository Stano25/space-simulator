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

// ---------------------------------------------------------------------------
// Continuous 3D Voronoi Stars
// ---------------------------------------------------------------------------
struct VoronoiResult {
    dist: f32,
    seed: f32,
};

fn voronoi_stars_3d(p: vec3<f32>) -> VoronoiResult {
    let fl = floor(p);
    let fr = p - fl; // Fractional position inside cell
    let i0 = vec3<i32>(fl);

    var min_dist = 10.0;
    var star_seed = 0.0;

    // 3x3x3 search guarantees continuous distance across all cell boundaries
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let cell = vec3<i32>(x, y, z);

                // Deterministic integer seed for cell
                let rnd = hash_star_u32(i0 + cell);

                // Jittered point position inside neighbor cell
                let pt = vec3<f32>(cell) + rnd;
                let diff = pt - fr;
                let d = length(diff);

                if (d < min_dist) {
                    min_dist = d;
                    star_seed = rnd.y; // Independent seed for color & twinkle
                }
            }
        }
    }

    return VoronoiResult(min_dist, star_seed);
}

// Renders dense, crisp Voronoi stars with power-curve sharpening
fn render_voronoi_star_layer(
    dir: vec3<f32>,
    scale: f32,
    radius: f32,           // Detection radius (larger = denser starfield!)
    core_power: f32,       // Exponent (higher = needle-thin pinprick)
    brightness: f32,
    twinkle_speed: f32,
    density_boost: f32
) -> vec3<f32> {
    let p = dir * scale;
    let res = voronoi_stars_3d(p);

    // Expand radius in the galactic band so stars cluster naturally
    let effective_radius = radius * (1.0 + density_boost);

    let d = res.dist;
    if (d >= effective_radius) {
        return vec3<f32>(0.0);
    }

    // Twinkling animation
    let twinkle = 0.85 + 0.15 * sin(uniforms.time * twinkle_speed + res.seed * 6.2831);

    // Normalized falloff [1.0 at center -> 0.0 at radius edge]
    let falloff = 1.0 - (d / effective_radius);

    // Power-curve sharpening: turns wide detection radii into needle-sharp pinpricks!
    let glow = pow(falloff, core_power) * twinkle;

    let color = get_vivid_star_color(res.seed);
    return color * glow * brightness;
}

// ---------------------------------------------------------------------------
// 1. Full 2-Round PCG Hash (STARS ONLY - 100% Avalanche, Zero Lines)
// ---------------------------------------------------------------------------
fn hash_star_u32(p: vec3<i32>) -> vec3<f32> {
    var v = vec3<u32>(p) * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> vec3<u32>(16u);
    // Round 2: Critical for breaking grid lines along coordinate axes!
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return vec3<f32>(v) * (1.0 / 4294967296.0);
}

fn get_vivid_star_color(seed: f32) -> vec3<f32> {
    if (seed < 0.20) {
        return vec3<f32>(0.60, 0.78, 1.00); // Class O/B: Intense Ice Blue
    } else if (seed < 0.42) {
        return vec3<f32>(0.88, 0.94, 1.00); // Class A: Diamond White
    } else if (seed < 0.68) {
        return vec3<f32>(1.00, 0.94, 0.82); // Class F/G: Warm Sun Yellow
    } else if (seed < 0.88) {
        return vec3<f32>(1.00, 0.70, 0.32); // Class K: Vibrant Orange
    } else {
        return vec3<f32>(1.00, 0.42, 0.22); // Class M: Deep Red-Orange
    }
}

// ---------------------------------------------------------------------------
// 2. High-Speed FP32 Noise (NEBULA - Smooth, Fast, Zero Seams)
// ---------------------------------------------------------------------------
fn hash13_fast(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.zyx + 31.32);
    return fract((q.x + q.y) * q.z);
}

fn noise3d_fast(p: vec3<f32>) -> f32 {
    let fl = floor(p);
    let f = p - fl;
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash13_fast(fl + vec3<f32>(0.0, 0.0, 0.0)), hash13_fast(fl + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash13_fast(fl + vec3<f32>(0.0, 1.0, 0.0)), hash13_fast(fl + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash13_fast(fl + vec3<f32>(0.0, 0.0, 1.0)), hash13_fast(fl + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash13_fast(fl + vec3<f32>(0.0, 1.0, 1.0)), hash13_fast(fl + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

fn fbm(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i = 0; i < 4; i++) {
        v += amp * noise3d_fast(pos);
        pos = pos * 2.04 + vec3<f32>(1.2, 5.8, 3.4);
        amp *= 0.5;
    }
    return v;
}

// ---------------------------------------------------------------------------
// 3. Galactic Plane & Nebula
// ---------------------------------------------------------------------------
const GALACTIC_POLE: vec3<f32> = vec3<f32>(0.35, 0.85, 0.38);
const GALACTIC_CORE: vec3<f32> = vec3<f32>(0.85, -0.42, 0.31);

struct GalacticCoords {
    disk_mask: f32,
    core_mask: f32,
};

fn get_galactic_coords(dir: vec3<f32>) -> GalacticCoords {
    let lat = abs(dot(dir, normalize(GALACTIC_POLE)));
    let disk_mask = exp(-lat * lat * 7.5);

    let core_angle = max(dot(dir, normalize(GALACTIC_CORE)), 0.0);
    let core_mask = pow(core_angle, 6.0) * exp(-lat * lat * 14.0);

    return GalacticCoords(disk_mask, core_mask);
}

fn render_milky_way(dir: vec3<f32>, gal: GalacticCoords) -> vec3<f32> {
    if (gal.disk_mask < 0.005) {
        return vec3<f32>(0.0);
    }

    // Small positive offset (+2.0): Never crosses zero (kills NVIDIA seam),
    // but stays small (p in [2.8, 8.4]) to preserve full float precision!
    let p = (dir + vec3<f32>(2.0, 2.0, 2.0)) * 2.8;

    let warp = vec3<f32>(
        fbm(p + vec3<f32>(1.7, 9.2, 0.4)),
        fbm(p + vec3<f32>(8.3, 2.8, 5.1)),
        fbm(p + vec3<f32>(4.1, 6.5, 3.9))
    );
    let gas = fbm(p + warp * 1.15);

    let rift_p = (dir + vec3<f32>(3.0, 3.0, 3.0)) * 6.5 + warp * 0.75;
    let dust_noise = fbm(rift_p + vec3<f32>(3.1, 7.4, 1.2));
    let dust_absorption = smoothstep(0.26, 0.70, dust_noise);

    let gas_profile = smoothstep(0.20, 0.80, gas);
    let raw_intensity = (gal.disk_mask * 0.65 + gal.core_mask * 0.90) * gas_profile;

    var intensity = raw_intensity * (0.05 + 0.95 * dust_absorption);

    let grain = hash13_fast(floor((dir + vec3<f32>(2.0, 2.0, 2.0)) * 500.0));
    intensity *= (0.90 + 0.20 * grain);

    intensity = 0.90 * (intensity / (0.38 + intensity));

    if (intensity <= 0.001) {
        return vec3<f32>(0.0);
    }

    let col_void  = vec3<f32>(0.015, 0.008, 0.035);
    let col_plum  = vec3<f32>(0.280, 0.100, 0.260);
    let col_rose  = vec3<f32>(0.840, 0.380, 0.640);
    let col_peach = vec3<f32>(1.000, 0.740, 0.540);

    var col = mix(col_void, col_plum, smoothstep(0.0, 0.28, intensity));
    col = mix(col, col_rose, smoothstep(0.28, 0.65, intensity));
    col = mix(col, col_peach, smoothstep(0.65, 0.90, intensity));
    col = mix(col, vec3<f32>(1.00, 0.66, 0.44), gal.core_mask * 0.45);

    return col * intensity;
}

// ---------------------------------------------------------------------------
// 4. Star Layer (Full PCG Hashing: Zero Lines, True Random Distribution)
// ---------------------------------------------------------------------------
// const SHELL_HALF: f32 = 0.5;
// const JITTER: f32 = 0.7;

// fn render_star_layer(
//     dir: vec3<f32>,
//     scale: f32,
//     base_threshold: f32,
//     brightness: f32,
//     core_sharpness: f32,
//     halo_size: f32,
//     density_boost: f32
// ) -> vec3<f32> {
//     let base_cell = vec3<i32>(floor(dir * scale));
//     let layer_seed = vec3<i32>(i32(scale * 137.0), i32(scale * 311.0), i32(scale * 577.0));

//     let threshold = clamp(base_threshold - density_boost, 0.75, 0.999);

//     var star_accum = vec3<f32>(0.0);

//     for (var z = -1; z <= 1; z++) {
//         for (var y = -1; y <= 1; y++) {
//             for (var x = -1; x <= 1; x++) {
//                 let cell = base_cell + vec3<i32>(x, y, z);

//                 // Full 2-round PCG: guarantees complete bit avalanche
//                 let rnd = hash_star_u32(cell + layer_seed);
//                 if (rnd.x < threshold) {
//                     continue;
//                 }

//                 let jit = hash_star_u32(cell + layer_seed + vec3<i32>(413, 137, 291));
//                 let star_pos = vec3<f32>(cell) + 0.5 + (jit - 0.5) * JITTER;

//                 let r = length(star_pos);
//                 if (abs(r - scale) > SHELL_HALF) {
//                     continue;
//                 }

//                 let star_dir = star_pos / r;
//                 let d = length(dir - star_dir) * scale;

//                 let core = exp(-d * d * core_sharpness);
//                 let halo = exp(-d * halo_size) * 0.10;

//                 let base_color = get_vivid_star_color(jit.y);
//                 let core_color = mix(base_color, vec3<f32>(1.0, 1.0, 1.0), core * 0.85);
//                 star_accum += (core_color * core + base_color * halo * 1.6) * brightness;
//             }
//         }
//     }

//     return star_accum;
// }

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ndc = vec4<f32>(in.ndc_xy, 1.0, 1.0);
    let world_pos = uniforms.inv_view_proj * ndc;
    let dir = normalize(world_pos.xyz / world_pos.w);

    var final_color = vec3<f32>(0.002, 0.003, 0.005);

    let gal = get_galactic_coords(dir);

    // 1. The Milky Way
    final_color += render_milky_way(dir, gal);

    // 2. DENSE VORONOI STARS
    // LAYER 1: Deep sparkling carpet (thousands of micro-stars)
    final_color += render_voronoi_star_layer(
        dir,
        220.0,                // scale
        0.15,                 // radius (boosted from 0.045 -> 10x more stars!)
        3.2,                  // core_power (tight needle pinpricks)
        0.75,                 // brightness
        1.2,                  // twinkle speed
        gal.disk_mask * 0.35  // cluster inside the Milky Way
    );

    // LAYER 2: Medium visible stars
    final_color += render_voronoi_star_layer(
        dir,
        100.0,                // scale
        0.12,                 // radius
        2.6,                  // core_power
        1.40,                 // brightness
        2.2,                  // twinkle speed
        gal.disk_mask * 0.20
    );

    // LAYER 3: Hero prominent stars (Bright, colorful, distinct)
    final_color += render_voronoi_star_layer(
        dir,
        30.0,                 // scale
        0.09,                 // radius
        1.8,                  // core_power
        3.20,                 // brightness
        0.8,                  // twinkle speed
        0.0                   // uniform across the sky
    );

    return vec4<f32>(final_color, 1.0);
}

@group(0) @binding(0)
var output_cubemap: texture_storage_2d_array<rgba8unorm, write>;

// ---------------------------------------------------------------------------
// 1. Cubemap Face to 3D Direction Vector
// ---------------------------------------------------------------------------
fn pixel_to_dir(coord: vec2<f32>, face: u32, size: f32) -> vec3<f32> {
    let uv = (coord / size) * 2.0 - 1.0;
    var v = vec3<f32>(0.0);
    switch (face) {
        case 0u: { v = vec3<f32>( 1.0, -uv.y, -uv.x); } // +X
        case 1u: { v = vec3<f32>(-1.0, -uv.y,  uv.x); } // -X
        case 2u: { v = vec3<f32>( uv.x,  1.0,  uv.y); } // +Y
        case 3u: { v = vec3<f32>( uv.x, -1.0, -uv.y); } // -Y
        case 4u: { v = vec3<f32>( uv.x, -uv.y,  1.0); } // +Z
        case 5u: { v = vec3<f32>(-uv.x, -uv.y, -1.0); } // -Z
        default: { v = vec3<f32>(0.0, 0.0, 1.0); }
    }
    return normalize(v);
}

// ---------------------------------------------------------------------------
// 2. Hash & Color Utilities
// ---------------------------------------------------------------------------
fn hash13_fast(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.zyx + 31.32);
    return fract((q.x + q.y) * q.z);
}

// Full 2-round PCG Integer Hash (100% Avalanche, Zero Lines)
fn hash_star_u32(p: vec3<i32>) -> vec3<f32> {
    var v = vec3<u32>(p) * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> vec3<u32>(16u);
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return vec3<f32>(v) * (1.0 / 4294967296.0);
}

// ---------------------------------------------------------------------------
// High-Chroma Spectral Colors (Vivid & Rich)
// ---------------------------------------------------------------------------
fn get_vivid_star_color(seed: f32) -> vec3<f32> {
    if (seed < 0.22) {
        return vec3<f32>(0.45, 0.70, 1.00); // Class O/B: Saturated Electric Blue
    } else if (seed < 0.45) {
        return vec3<f32>(0.90, 0.95, 1.00); // Class A: Diamond White
    } else if (seed < 0.70) {
        return vec3<f32>(1.00, 0.92, 0.70); // Class F/G: Warm Sun Gold
    } else if (seed < 0.88) {
        return vec3<f32>(1.00, 0.62, 0.20); // Class K: Saturated Flame Orange
    } else {
        return vec3<f32>(1.00, 0.35, 0.15); // Class M: Deep Ruby Red-Orange
    }
}

// ---------------------------------------------------------------------------
// 3. Fast 3D Noise & Nebula Math
// ---------------------------------------------------------------------------
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
// 4. Procedural Spherical Shell Stars (Crisp, Tiny, Highly Saturated)
// ---------------------------------------------------------------------------
const SHELL_HALF: f32 = 0.5;
const JITTER: f32 = 0.7;

fn render_procedural_star_layer(
    dir: vec3<f32>,
    scale: f32,
    base_threshold: f32,
    brightness: f32,
    core_sharpness: f32,   // High = needle-thin pinprick
    halo_size: f32,        // Falloff rate
    halo_strength: f32,    // 0.0 for Layer 1 (kills all blur/bloat)
    white_core_mix: f32,   // Low value = keeps 80-90% pure color so it NEVER turns grey in motion
    density_boost: f32
) -> vec3<f32> {
    let base_cell = vec3<i32>(floor(dir * scale));
    let layer_seed = vec3<i32>(i32(scale * 137.0), i32(scale * 311.0), i32(scale * 577.0));

    let threshold = clamp(base_threshold - density_boost, 0.70, 0.999);

    var star_accum = vec3<f32>(0.0);

    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let cell = base_cell + vec3<i32>(x, y, z);

                let rnd = hash_star_u32(cell + layer_seed);
                if (rnd.x < threshold) {
                    continue;
                }

                let jit = hash_star_u32(cell + layer_seed + vec3<i32>(413, 137, 291));
                let star_pos = vec3<f32>(cell) + 0.5 + (jit - 0.5) * JITTER;

                let r = length(star_pos);
                if (abs(r - scale) > SHELL_HALF) {
                    continue;
                }

                let star_dir = star_pos / r;
                let d = length(dir - star_dir) * scale;

                let core = exp(-d * d * core_sharpness);
                let halo = exp(-d * halo_size) * halo_strength;

                let base_color = get_vivid_star_color(jit.y);

                // Preserve vivid color saturation (prevents turning into grey specks when moving)
                let core_color = mix(base_color, vec3<f32>(1.0, 1.0, 1.0), core * white_core_mix);
                star_accum += (core_color * core + base_color * halo) * brightness;
            }
        }
    }

    return star_accum;
}

// ---------------------------------------------------------------------------
// 5. Analytical Fibonacci Hero Stars (Tuned slightly smaller)
// ---------------------------------------------------------------------------
fn render_hero_stars_analytical(dir: vec3<f32>, cubemap_size: f32) -> vec3<f32> {
    var hero_accum = vec3<f32>(0.0);

    let pixels_per_radian = cubemap_size * (2.0 / 3.14159265);

    for (var i = 0u; i < 16u; i++) {
        let y = 1.0 - (f32(i) / 15.0) * 2.0;
        let radius_at_y = sqrt(max(1.0 - y * y, 0.0));
        let theta = f32(i) * 2.3999632;

        let star_pos = vec3<f32>(cos(theta) * radius_at_y, y, sin(theta) * radius_at_y);

        let cos_dist = dot(dir, star_pos);

        if (cos_dist > 0.998) {
            let angle = acos(clamp(cos_dist, -1.0, 1.0));
            let d_px = angle * pixels_per_radian;

            // Tightened core (~5-6px) and slightly tighter halo
            let core = exp(-d_px * d_px * 0.085);
            let halo = exp(-d_px * 0.28) * 0.28;

            let base_color = get_vivid_star_color(fract(f32(i) * 0.618033 + 0.1));
            let core_color = mix(base_color, vec3<f32>(1.0, 1.0, 1.0), core * 0.85);

            hero_accum += (core_color * core + base_color * halo) * 3.4;
        }
    }

    return hero_accum;
}

// ---------------------------------------------------------------------------
// Compute Entry Point (Runs ONCE at launch)
// ---------------------------------------------------------------------------
@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = f32(textureDimensions(output_cubemap).x);
    if (id.x >= u32(size) || id.y >= u32(size)) {
        return;
    }

    let dir = pixel_to_dir(vec2<f32>(f32(id.x) + 0.5, f32(id.y) + 0.5), id.z, size);

    var final_color = vec3<f32>(0.0);

    // 1. Nebula / Milky Way
    let gal = get_galactic_coords(dir);
    final_color += render_milky_way(dir, gal);

    // 2. LAYER 1: Microscopic Stardust Carpet
    // Tightened back to true tiny pinpricks (zero halo, 90% saturated color)
    final_color += render_procedural_star_layer(
        dir,
        210.0,              // scale
        0.815,              // threshold (frequent)
        0.70,               // brightness
        380.0,              // core sharpness (needle pinpricks!)
        50.0,               // halo size
        0.0,                // halo_strength = 0.0 (NO fuzzy halo, pure crisp specks)
        0.10,               // white_core_mix = 0.10 (90% pure color, never desaturates)
        gal.disk_mask * 0.04
    );

    // 3. LAYER 2: Medium Visible Stars
    // Tightened to small ~2-3px crisp dots
    final_color += render_procedural_star_layer(
        dir,
        75.0,               // scale
        0.950,              // threshold
        1.35,               // brightness
        220.0,              // core sharpness (~2-3px tight disc)
        38.0,               // halo size
        0.05,               // subtle, restrained aura
        0.30,               // white_core_mix (70% pure color)
        0.0
    );

    // 4. LAYER 3: Hero Landmark Beacons (Fibonacci Analytical)
    final_color += render_hero_stars_analytical(dir, size);

    // Write directly into the cubemap face!
    textureStore(output_cubemap, id.xy, id.z, vec4<f32>(final_color, 1.0));
}

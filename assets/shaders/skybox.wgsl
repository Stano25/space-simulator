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
// Hash & Math Utilities
// ---------------------------------------------------------------------------
fn hash3(p: vec3<f32>) -> vec3<f32> {
    var q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yxz + 33.33);
    return fract((q.xxy + q.yxx) * q.zyx);
}

fn hash13(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.zyx + 31.32);
    return fract((q.x + q.y) * q.z);
}

// ---------------------------------------------------------------------------
// Vivid Astronomical Star Colors (Morgan-Keenan Spectral Classes)
// ---------------------------------------------------------------------------
fn get_vivid_star_color(seed: f32) -> vec3<f32> {
    if (seed < 0.20) {
        return vec3<f32>(0.60, 0.78, 1.00); // Class O/B: Intense Ice Blue (Rigel)
    } else if (seed < 0.42) {
        return vec3<f32>(0.88, 0.94, 1.00); // Class A: Crisp Diamond White (Sirius)
    } else if (seed < 0.68) {
        return vec3<f32>(1.00, 0.94, 0.82); // Class F/G: Warm Sun Yellow (Capella)
    } else if (seed < 0.88) {
        return vec3<f32>(1.00, 0.70, 0.32); // Class K: Vibrant Orange (Arcturus)
    } else {
        return vec3<f32>(1.00, 0.42, 0.22); // Class M: Deep Red-Orange (Betelgeuse / Antares)
    }
}

// ---------------------------------------------------------------------------
// 3D Value Noise & Fractal Brownian Motion (FBM)
// ---------------------------------------------------------------------------
fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash13(i + vec3<f32>(0.0, 0.0, 0.0)), hash13(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash13(i + vec3<f32>(0.0, 1.0, 0.0)), hash13(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash13(i + vec3<f32>(0.0, 0.0, 1.0)), hash13(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash13(i + vec3<f32>(0.0, 1.0, 1.0)), hash13(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
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
        v += amp * noise3d(pos);
        pos = pos * 2.08 + vec3<f32>(1.2, 5.8, 3.4);
        amp *= 0.5;
    }
    return v;
}

// ---------------------------------------------------------------------------
// Galactic Plane Orientation
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

    // Tight, localized Sagittarius core bulge
    let core_angle = max(dot(dir, normalize(GALACTIC_CORE)), 0.0);
    let core_mask = pow(core_angle, 6.0) * exp(-lat * lat * 14.0);

    return GalacticCoords(disk_mask, core_mask);
}

// ---------------------------------------------------------------------------
// Milky Way Glow (Vibrant Cosmic Rose, Lilac & Champagne Palette)
// ---------------------------------------------------------------------------
fn render_milky_way(dir: vec3<f32>, gal: GalacticCoords) -> vec3<f32> {
    if (gal.disk_mask < 0.005) {
        return vec3<f32>(0.0);
    }

    // 1. Turbulent gas clouds
    let p = dir * 2.8;
    let warp = vec3<f32>(
        fbm(p + vec3<f32>(1.7, 9.2, 0.4)),
        fbm(p + vec3<f32>(8.3, 2.8, 5.1)),
        fbm(p + vec3<f32>(4.1, 6.5, 3.9))
    );
    let gas = fbm(p + warp * 1.15);

    // 2. The Great Rift: High-contrast dark molecular dust lanes
    let rift_p = dir * 6.5 + warp * 0.75;
    let dust_noise = fbm(rift_p + vec3<f32>(3.1, 7.4, 1.2));
    let dust_absorption = smoothstep(0.26, 0.70, dust_noise);

    // 3. Higher contrast gas emission (carves out deeper voids so colors pop!)
    let gas_profile = smoothstep(0.20, 0.80, gas);
    let raw_intensity = (gal.disk_mask * 0.65 + gal.core_mask * 0.90) * gas_profile;

    // Rich interstellar dust veins
    var intensity = raw_intensity * (0.05 + 0.95 * dust_absorption);

    // Subtle micro-starlight grain
    let grain = hash13(floor(dir * 600.0));
    intensity *= (0.90 + 0.20 * grain);

    // 4. Soft-knee compression (Luminance ceiling at 0.90)
    intensity = 0.90 * (intensity / (0.60 + intensity));

    if (intensity <= 0.001) {
        return vec3<f32>(0.0);
    }

    // 5. Vibrant Astrophotography Palette (High Chroma / Zero Ashy Grey):
    let col_void  = vec3<f32>(0.015, 0.008, 0.035); // Deep midnight violet-indigo
    let col_plum  = vec3<f32>(0.280, 0.100, 0.260); // Rich dark plum/burgundy dust
    let col_rose  = vec3<f32>(0.840, 0.380, 0.640); // Vibrant, glowing cosmic rose-lilac
    let col_peach = vec3<f32>(1.000, 0.740, 0.540); // Radiant warm champagne / peach core

    var col = mix(col_void, col_plum, smoothstep(0.0, 0.28, intensity));
    col = mix(col, col_rose, smoothstep(0.28, 0.65, intensity));
    col = mix(col, col_peach, smoothstep(0.65, 0.90, intensity));

    // Dual-tone warmth: The Sagittarius core glows with saturated golden-peach
    col = mix(col, vec3<f32>(1.00, 0.66, 0.44), gal.core_mask * 0.45);

    return col * intensity;
}

// ---------------------------------------------------------------------------
// Star Layer (Uncorrelated Colors + Retained Saturation)
// ---------------------------------------------------------------------------
const SHELL_HALF: f32 = 0.5;
const JITTER: f32 = 0.7;

fn render_star_layer(
    dir: vec3<f32>,
    scale: f32,
    base_threshold: f32,
    brightness: f32,
    core_sharpness: f32,
    halo_size: f32,
    density_boost: f32
) -> vec3<f32> {
    let base_cell = floor(dir * scale);
    let layer_seed = vec3<f32>(scale * 13.7);

    // Controlled threshold: only Layer 1 receives a subtle boost
    let threshold = clamp(base_threshold - density_boost, 0.75, 0.999);

    var star_accum = vec3<f32>(0.0);

    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let cell = base_cell + vec3<f32>(f32(x), f32(y), f32(z));

                let rnd = hash3(cell + layer_seed);
                if (rnd.x < threshold) {
                    continue;
                }

                let jit = hash3(cell + layer_seed + vec3<f32>(41.7, 13.3, 29.1));
                let star_pos = cell + 0.5 + (jit - 0.5) * JITTER;

                let r = length(star_pos);
                if (abs(r - scale) > SHELL_HALF) {
                    continue;
                }

                let star_dir = star_pos / r;
                let d = length(dir - star_dir) * scale;

                let core = exp(-d * d * core_sharpness);
                let halo = exp(-d * halo_size) * 0.10;

                // Color is seeded by an independent channel (jit.y)
                let base_color = get_vivid_star_color(jit.y);

                // Anti-bleach trick: The core saturates to white-hot at center,
                // but the halo preserves 100% of the rich spectral color!
                let core_color = mix(base_color, vec3<f32>(1.0, 1.0, 1.0), core * 0.85);
                star_accum += (core_color * core + base_color * halo * 1.6) * brightness;
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

    var final_color = vec3<f32>(0.004, 0.003, 0.006);

    let gal = get_galactic_coords(dir);

    // 1. The Milky Way Core & Dust Lanes (Never overexposed)
    final_color += render_milky_way(dir, gal);

    // 2. Stars
    // LAYER 1: Microscopic stellar dust (gentle +2.5% boost in the galactic plane)
    final_color += render_star_layer(dir, 240.0, 0.910, 0.55, 480.0, 30.0, gal.disk_mask * 0.025);

    // LAYER 2: Visible stars (ZERO density boost: stays uniform and natural)
    final_color += render_star_layer(dir, 95.0, 0.965, 1.25, 280.0, 20.0, 0.0);

    // LAYER 3: Hero prominent stars (ZERO density boost: rare and prominent)
    final_color += render_star_layer(dir, 38.0, 0.988, 3.0, 160.0, 12.0, 0.0);

    return vec4<f32>(final_color, 1.0);
}

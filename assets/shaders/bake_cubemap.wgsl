@group(0) @binding(0)
var output_cubemap: texture_storage_2d_array<rgba8unorm, write>;

// Cubemap Face to 3D Direction Vector
fn pixel_to_dir(coord: vec2<f32>, face: u32, size: f32) -> vec3<f32> {
    let face_coords = (coord / size) * 2.0 - 1.0;
    var v = vec3<f32>(0.0);
    switch (face) {
        case 0u: { v = vec3<f32>(           1.0, -face_coords.y, -face_coords.x); } // +X
        case 1u: { v = vec3<f32>(          -1.0, -face_coords.y,  face_coords.x); } // -X
        case 2u: { v = vec3<f32>( face_coords.x,            1.0,  face_coords.y); } // +Y
        case 3u: { v = vec3<f32>( face_coords.x,           -1.0, -face_coords.y); } // -Y
        case 4u: { v = vec3<f32>( face_coords.x, -face_coords.y,            1.0); } // +Z
        case 5u: { v = vec3<f32>(-face_coords.x, -face_coords.y,           -1.0); } // -Z
        default: { v = vec3<f32>(0.0, 0.0, 1.0); }
    }
    return normalize(v);
}

// PCG (Permuted Congruential Generator) + LCG (Linear Congruential Generator) from Numerical Recipes
// this implementation is from the Research paper "Hash Functions for GPU Rendering" (JCGT)
fn hash_u32_3d(p: vec3<i32>) -> vec3<f32> {
    // Linear Congruential Generator -> guarantees every number is visited only once
    var v = vec3<u32>(p) * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> vec3<u32>(16u);
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return vec3<f32>(v) / 4294967296.0;
}
fn hash_u32_1d(p: vec3<i32>) -> f32 {
    // Linear Congruential Generator -> guarantees every number is visited only once
    var v = vec3<u32>(p) * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> vec3<u32>(16u);
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    // avoids possible combination where the same pattern of x,y,z
    // could produce the same hash for different inputs
    let h = v.x + v.y * 31u + v.z * 137u;
    return f32(h) / 4294967296.0;
}

// ---------------------------------------------------------------------------
// 3D Noise & Nebula Math
// ---------------------------------------------------------------------------
fn noise3d(p: vec3<f32>) -> f32 {
    let fl = floor(p);
    let f = p - fl;
    // Ken Perlin’s Quintic S-Curve ( 6f^5 − 15f^4 + 10f^3 )
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    let i0 = vec3<i32>(fl);

    let c000 = hash_u32_1d(i0 + vec3<i32>(0, 0, 0));
    let c100 = hash_u32_1d(i0 + vec3<i32>(1, 0, 0));
    let c010 = hash_u32_1d(i0 + vec3<i32>(0, 1, 0));
    let c110 = hash_u32_1d(i0 + vec3<i32>(1, 1, 0));

    let c001 = hash_u32_1d(i0 + vec3<i32>(0, 0, 1));
    let c101 = hash_u32_1d(i0 + vec3<i32>(1, 0, 1));
    let c011 = hash_u32_1d(i0 + vec3<i32>(0, 1, 1));
    let c111 = hash_u32_1d(i0 + vec3<i32>(1, 1, 1));

    return mix(
        mix(mix(c000, c100, u.x), mix(c010, c110, u.x), u.y),
        mix(mix(c001, c101, u.x), mix(c011, c111, u.x), u.y),
        u.z
    );
}

// Fractal Brownian Motion (FBM) for Nebula Density
fn fbm(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i = 0; i < 7; i++) {
        v += amp * noise3d(pos);
        pos = pos * 2.02 + vec3<f32>(1.2, 5.8, 3.4);
        amp *= 0.52;
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

// ---------------------------------------------------------------------------
// Astrophysical Simulator Model (Mass -> Temp/Luminosity -> Distance/Extinction)
// ---------------------------------------------------------------------------
struct AstroStar {
    color: vec3<f32>,
    core_sharpness: f32,
    halo_size: f32,
    brightness: f32,
};

fn evaluate_astrophysical_star(
    u_mass: f32,       // Random float [0, 1]
    u_dist: f32,       // Random float [0, 1]
    gal_disk: f32,     // Galactic disk mask (denser dust in Milky Way)
    is_hero: bool
) -> AstroStar {
    // 1. Initial Mass Function (Salpeter power law: dwarfs are common, giants are rare)
    let mass = 0.15 * pow(max(1.0 - u_mass * 0.995, 0.001), -0.85);

    // 2. Main Sequence Temperature (K) & Intrinsic Luminosity (L_sun)
    let temp = 5780.0 * sqrt(mass);
    let luminosity = pow(mass, 3.5);

    // 3. Distance distribution (volume-weighted)
    let dist_range = select(vec2<f32>(25.0, 350.0), vec2<f32>(10.0, 120.0), is_hero);
    let dist = mix(dist_range.x, dist_range.y, pow(u_dist, 0.40));

    // 4. Flux reaching camera (Inverse-square law)
    let flux = luminosity / (dist * dist);

    // 5. Interstellar Reddening (Extinction)
    // Blue light attenuates ~5x faster than red light through galactic dust
    let dust_tau = (dist / dist_range.y) * (0.05 + gal_disk * 0.90);
    let extinction = exp(-vec3<f32>(0.35, 0.85, 1.80) * dust_tau);

    // 6. Color from Temperature (Blackbody curve)
    var base_col = vec3<f32>(1.0);
    if (temp < 3600.0) {
        base_col = vec3<f32>(1.00, 0.38, 0.15); // Deep Red (Class M)
    } else if (temp < 5200.0) {
        base_col = vec3<f32>(1.00, 0.65, 0.25); // Flame Orange (Class K)
    } else if (temp < 6800.0) {
        base_col = vec3<f32>(1.00, 0.92, 0.75); // Warm Yellow/Sun (Class G)
    } else if (temp < 9800.0) {
        base_col = vec3<f32>(0.92, 0.96, 1.00); // Diamond White (Class A/F)
    } else {
        base_col = vec3<f32>(0.50, 0.72, 1.00); // Saturated Blue Giant (Class O/B)
    }

    // Shift color according to dust extinction
    let final_color = base_col * extinction;

    // 7. Apparent Size & Brightness (Logarithmic Optical Point Spread Function)
    // Brighter flux = wider perceived core & halo
    let log_flux = clamp(log(1.0 + flux * 12.0), 0.35, 3.5);

    var out: AstroStar;
    out.color = final_color;
    out.core_sharpness = select(240.0 / log_flux, 0.085 / log_flux, is_hero);
    out.halo_size = select(42.0 / log_flux, 0.28 / log_flux, is_hero);
    out.brightness = select(clamp(log_flux * 0.65, 0.75, 1.70), clamp(log_flux * 1.25, 2.5, 4.2), is_hero);

    return out;
}

fn render_milky_way(dir: vec3<f32>, gal: GalacticCoords) -> vec3<f32> {
    if (gal.disk_mask < 0.001) {
        return vec3<f32>(0.0);
    }

    let p = dir * 2.8;

    let warp = vec3<f32>(
        fbm(p + vec3<f32>(1.7, 9.2, 0.4)),
        fbm(p + vec3<f32>(8.3, 2.8, 5.1)),
        fbm(p + vec3<f32>(4.1, 6.5, 3.9))
    );
    let gas = fbm(p + warp * 1.15);

    let rift_p = dir * 6.5 + warp * 0.75;
    let dust_noise = fbm(rift_p + vec3<f32>(3.1, 7.4, 1.2));
    let dust_absorption = smoothstep(0.24, 0.72, dust_noise);

    let gas_profile = pow(gas, 1.55);
    let raw_intensity = (gal.disk_mask * 0.65 + gal.core_mask * 0.90) * gas_profile;

    var intensity = raw_intensity * (0.05 + 0.95 * dust_absorption);

    let grain = hash_u32_1d(vec3<i32>(floor(dir * 500.0)));
    intensity *= (0.92 + 0.16 * grain);

    intensity = 0.90 * (intensity / (0.38 + intensity));

    let col_void  = vec3<f32>(0.015, 0.008, 0.035);
    let col_plum  = vec3<f32>(0.280, 0.100, 0.260);
    let col_rose  = vec3<f32>(0.840, 0.380, 0.640);
    let col_peach = vec3<f32>(1.000, 0.740, 0.540);

    var col = mix(col_void, col_plum, smoothstep(0.0, 0.25, intensity));
    col = mix(col, col_rose, smoothstep(0.25, 0.62, intensity));
    col = mix(col, col_peach, smoothstep(0.62, 0.90, intensity));
    col = mix(col, vec3<f32>(1.00, 0.66, 0.44), gal.core_mask * 0.45);

    return col * intensity;
}

// ---------------------------------------------------------------------------
// Procedural Star Layer (Supports Astrophysical Physics for Layer 2)
// ---------------------------------------------------------------------------
const SHELL_HALF: f32 = 0.5;
const JITTER: f32 = 0.7;

fn render_procedural_star_layer(
    dir: vec3<f32>,
    scale: f32,
    base_threshold: f32,
    brightness: f32,
    core_sharpness: f32,
    halo_size: f32,
    halo_strength: f32,
    white_core_mix: f32,
    density_boost: f32,
    is_astrophysical: bool, // true = evaluates Mass, Distance & Extinction!
    gal_disk: f32
) -> vec3<f32> {
    let base_cell = vec3<i32>(floor(dir * scale));
    let layer_seed = vec3<i32>(i32(scale * 137.0), i32(scale * 311.0), i32(scale * 577.0));

    let threshold = clamp(base_threshold - density_boost, 0.70, 0.999);

    var star_accum = vec3<f32>(0.0);

    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let cell = base_cell + vec3<i32>(x, y, z);

                let rnd = hash_u32_3d(cell + layer_seed);
                if (rnd.x < threshold) {
                    continue;
                }

                let jit = hash_u32_3d(cell + layer_seed + vec3<i32>(413, 137, 291));
                let star_pos = vec3<f32>(cell) + 0.5 + (jit - 0.5) * JITTER;

                let r = length(star_pos);
                if (abs(r - scale) > SHELL_HALF) {
                    continue;
                }

                let star_dir = star_pos / r;
                let d = length(dir - star_dir) * scale;

                if (is_astrophysical) {
                    // LAYER 2: Physical Main Sequence Model
                    let astro = evaluate_astrophysical_star(jit.y, jit.z, gal_disk, false);
                    let core = exp(-d * d * astro.core_sharpness);
                    let halo = exp(-d * astro.halo_size) * halo_strength;

                    let core_color = mix(astro.color, vec3<f32>(1.0, 1.0, 1.0), core * white_core_mix);
                    star_accum += (core_color * core + astro.color * halo) * astro.brightness;
                } else {
                    // LAYER 1: Fast Microscopic Stardust
                    let core = exp(-d * d * core_sharpness);
                    let halo = exp(-d * halo_size) * halo_strength;

                    // Neutral stardust tint
                    let base_color = mix(vec3<f32>(0.90, 0.95, 1.0), vec3<f32>(1.0, 0.85, 0.65), jit.y);
                    let core_color = mix(base_color, vec3<f32>(1.0, 1.0, 1.0), core * white_core_mix);
                    star_accum += (core_color * core + base_color * halo) * brightness;
                }
            }
        }
    }

    return star_accum;
}

// ---------------------------------------------------------------------------
// 6. Analytical Fibonacci Hero Stars (Astrophysical Model on 16 Landmarks)
// ---------------------------------------------------------------------------
fn render_hero_stars_analytical(dir: vec3<f32>, cubemap_size: f32, gal_disk: f32) -> vec3<f32> {
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

            // LAYER 3: Physical Model on Hero Stars
            let u_mass = fract(f32(i) * 0.618033 + 0.1);
            let u_dist = fract(f32(i) * 0.324718 + 0.5);
            let astro = evaluate_astrophysical_star(u_mass, u_dist, gal_disk, true);

            let core = exp(-d_px * d_px * astro.core_sharpness);
            let halo = exp(-d_px * astro.halo_size) * 0.28;

            let core_color = mix(astro.color, vec3<f32>(1.0, 1.0, 1.0), core * 0.65);
            hero_accum += (core_color * core + astro.color * halo) * astro.brightness;
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
    // redundant in this case, but its a good safety check because GPUs execute threads in groups of 16x16
    if (id.x >= u32(size) || id.y >= u32(size)) {
        return;
    }

    let dir = pixel_to_dir(vec2<f32>(f32(id.x) + 0.5, f32(id.y) + 0.5), id.z, size);

    var final_color = vec3<f32>(0.0);

    // 1. Nebula / Milky Way
    let gal = get_galactic_coords(dir);
    final_color += render_milky_way(dir, gal);

    // 2. LAYER 1: Microscopic Stardust Carpet (Fast, dense background field)
    final_color += render_procedural_star_layer(
        dir,
        210.0,              // scale
        0.815,              // threshold
        0.70,               // brightness
        380.0,              // core sharpness (needle pinpricks!)
        50.0,               // halo size
        0.0,                // no halo
        0.10,               // 90% saturated
        gal.disk_mask * 0.04,
        false,              // is_astrophysical = false (fast micro dust)
        gal.disk_mask
    );

    // 3. LAYER 2: Medium Visible Stars (Astrophysical Model: Mass, Distance & Extinction!)
    final_color += render_procedural_star_layer(
        dir,
        75.0,               // scale
        0.950,              // threshold
        1.35,               // brightness
        220.0,              // base core sharpness
        38.0,               // base halo size
        0.05,               // subtle halo
        0.30,               // white core mix
        0.0,                // density boost
        true,               // is_astrophysical = true!
        gal.disk_mask
    );

    // 4. LAYER 3: Hero Landmark Beacons (Astrophysical Model on 16 Landmarks!)
    final_color += render_hero_stars_analytical(dir, size, gal.disk_mask);

    // 5. Anti-Banding Dither (TPDF)
    let rnd_dither = hash_u32_3d(vec3<i32>(id));
    let dither = (rnd_dither.x - rnd_dither.y) * (1.0 / 255.0);
    final_color += vec3<f32>(dither);
    // if dir.x == 0.0 {
    //     final_color = vec3<f32>(1.0, 0.0, 0.0);
    // }

    // Write directly into the cubemap face!
    textureStore(output_cubemap, id.xy, id.z, vec4<f32>(final_color, 1.0));
}

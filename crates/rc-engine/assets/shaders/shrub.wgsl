// Shrub pass: instanced class meshes, colour = the instance's LightShrubs palette entry, GS TFX = MODULATE,
// GS fog (one F per instance), ShrubProc's draw-distance and fade rules. Rust side: shrub_render.rs;
// lighting: rc_formats::shrub_light.
//
// Instance data (`insts[mesh tag]`): the class -> Bevy-world matrix (the game's column-major instance
// matrix with game_to_bevy applied after it; the mesh is already in class units x scale / 1024, which is
// exactly how the VU1 matrix ShrubProc builds applies to the raw s16 positions), the bounding-sphere
// centre with the run-time draw distance D in w, the instance origin with the billboard fade distance F
// in w (-1 = class without a billboard), and the 24 packed RGBA8 palette entries (0x80 = 1.0).
//
// ShrubProc rules (level01 0x29cdf0, shrub_render.rs module doc), z = view depth of the centre:
// * D - z < 0: the instance is not drawn;
// * no billboard: vertex alpha = min(trunc((D - z) * 4096), 0x8000) >> 8 (fades over the last 8 units);
// * billboard class: z < F: alpha 0x80; F <= z < F + 8: alpha (0x8000 - trunc((z - F) * 4096)) >> 8
//   (the mesh half of the cross-fade); beyond (and F = 0): billboard only (shrub_billboard.wgsl).
// * wind sway (`sway[mesh tag]`, rewritten every frame by shrub_render::update_sway from
//   rc_formats::shrub::wind_sway): ShrubProc shears the instance columns in game space, x += sx * z and
//   y += sy * z (Z up, about the origin); in Bevy axes (x, z, -y) that is x += sx * y, z -= sy * y.
// * texture: MODULATE, GS mip rule level = round(log2(z / 32) + K) clamped to 0..MXL (z = per-pixel camera
//   depth in integer units; K = the ad-gif's TEX1 K, per vertex in info bits 20..31), as tfrag.wgsl / tie.wgsl.
// * VU1 56467 writes the same F for every vertex from the instance origin depth (qw 4 = the translation
//   column): here the tfrag slope / offset on the origin depth, clamped max(far_int) then min(near_int).
// `params.misc.x` selects the ShrubProc list: 0 = always, 1 = opaque list (alpha = 0x80; TEST_1 0x5320b,
// AREF 0x20), 2 = fading list (alpha < 0x80; TEST_1 0x530cb, AREF 0x0c). The GS_ATEST_* defs select the
// half of that list's alpha-test split this draw is (gs_state.rs).
// Point lights (`LightShrubs`' third light, crate::world_lights): when the instance's nibble list names any, the
// palette entry gets the merged light on that entry's class normal added (clamped at 243), as the game relights
// the palette. With an empty list (0xffff) nothing changes. (Billboards keep the load-time average colour.)
// The list and the bank ride in the `sway` buffer (after the shears), the class normals in the instance record,
// so the material layout is the same as without point lights (shrub_render.rs `write_record`).

#import bevy_pbr::{
    mesh_functions,
    view_transformations::{position_world_to_clip, position_world_to_view},
}
#import rerac::world_lights::{WorldLight, NO_LIGHTS, instance_light, instance_lit_packed}

struct ShrubFog {
    color: vec4<f32>,
    params: vec4<f32>,
    // x: the texture option (crate::graphics; sample_world).
    tex_mode: vec4<f32>,
}

struct ShrubParams {
    // x = variant (see above), y = MXL, z = near (32).
    misc: vec4<f32>,
}

struct ShrubInst {
    model: mat4x4<f32>,
    // xyz = bounding-sphere centre (Bevy world), w = run-time draw distance D (world units).
    centre: vec4<f32>,
    // xyz = instance origin (Bevy world), w = billboard fade distance F, or -1.
    origin: vec4<f32>,
    palette: array<u32, 24>,
    // The class normals (s16: x | y << 16, z) the point lights use.
    normals: array<vec2<u32>, 24>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tex_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<storage, read> fog: ShrubFog;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> params: ShrubParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<storage, read> insts: array<ShrubInst>;
// n shears, then n (point-light nibble list as u32 bits, 0), then the bank: 8 × (pos.xy, pos.zw, col.xy, col.zw).
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<storage, read> sway: array<vec2<f32>>;

// The texture option (crate::graphics, `fog.tex_mode.x`): 0 Original = the GS level round(L) clamped to 0..MXL (one
// level, bilinear inside it); 1 Smooth = L unrounded, the two nearest levels blended by the sampler (no switch line);
// 2 Sharp = the GPU's own level from the UV footprint `duv` (dx, dy), anisotropic. The sampler's mip filter is linear:
// an integer level reads that level alone, as the GS does.
fn sample_world(uv: vec2<f32>, l: f32, mxl: f32, duv: vec4<f32>) -> vec4<f32> {
    let mode = fog.tex_mode.x;
    if mode > 1.5 {
        return textureSampleGrad(tex, tex_sampler, uv, duv.xy, duv.zw);
    }
    let level = select(clamp(floor(l + 0.5), 0.0, mxl), clamp(l, 0.0, mxl), mode > 0.5);
    return textureSampleLevel(tex, tex_sampler, uv, level);
}

fn shrub_count() -> u32 { return (arrayLength(&sway) - 32u) / 2u; }

fn bank() -> array<WorldLight, 8> {
    let b = 2u * shrub_count();
    var ls: array<WorldLight, 8>;
    for (var k = 0u; k < 8u; k += 1u) {
        let o = b + 4u * k;
        ls[k] = WorldLight(vec4<f32>(sway[o], sway[o + 1u]), vec4<f32>(sway[o + 2u], sway[o + 3u]));
    }
    return ls;
}

fn class_normal(p: vec2<u32>) -> vec3<f32> {
    return vec3<f32>(f32(bitcast<i32>(p.x << 16u) >> 16u), f32(bitcast<i32>(p.x) >> 16u), f32(bitcast<i32>(p.y << 16u) >> 16u)) / 32768.0;
}

struct ShrubVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    // palette entry VU1 reads the colour from (the vertex normal index, or the 6-vertex quirk's entry)
    // | TEX1 K (s12, 1/16 units) << 20
    @location(2) info: u32,
}

struct ShrubVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(linear) color: vec4<f32>,
    @location(2) @interpolate(flat) fog: f32,
    // Camera depth in integer units, perspective-correct (= n / Q per pixel).
    @location(3) depth: f32,
    @location(4) @interpolate(flat) k: f32,
}

fn culled() -> ShrubVertexOutput {
    var out: ShrubVertexOutput;
    out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    return out;
}

@vertex
fn vertex(v: ShrubVertex) -> ShrubVertexOutput {
    let tag = mesh_functions::get_tag(v.instance_index);
    let inst = &insts[tag];
    let z = -position_world_to_view((*inst).centre.xyz).z;
    let d = (*inst).centre.w;
    if (d - z < 0.0) { return culled(); }
    let f = (*inst).origin.w;
    var alpha: f32;
    if (f < 0.0) {
        alpha = floor(min(trunc((d - z) * 4096.0), 32768.0) / 256.0);
    } else {
        let iz = trunc(max(z, 0.0) * 4096.0) - f * 4096.0;
        if (f == 0.0 || iz >= 32768.0) { return culled(); }
        alpha = select(floor((32768.0 - iz) / 256.0), 128.0, iz < 0.0);
    }
    let variant = params.misc.x;
    if ((variant == 1.0 && alpha < 128.0) || (variant == 2.0 && alpha >= 128.0)) { return culled(); }

    var out: ShrubVertexOutput;
    let sw = sway[tag];
    var off = ((*inst).model * vec4<f32>(v.position, 0.0)).xyz;
    off = vec3<f32>(off.x + sw.x * off.y, off.y, off.z - sw.y * off.y);
    let world = off + (*inst).model[3].xyz;
    out.position = position_world_to_clip(world);
    out.uv = v.uv;
    out.depth = -position_world_to_view(world).z * 1024.0;
    out.k = f32(bitcast<i32>(v.info) >> 20u) / 16.0;
    let entry = (v.info & 0xfffu) % 24u;
    var rgba = (*inst).palette[entry];
    let list = bitcast<u32>(sway[shrub_count() + tag].x) & 0xffffu;
    if (list != NO_LIGHTS) {
        let il = instance_light(list, bank(), (*inst).centre.xyz);
        if (il.hit) { rgba = instance_lit_packed(rgba, il, class_normal((*inst).normals[entry]), (*inst).model, 243.0); }
    }
    let c = unpack4x8unorm(rgba) * (255.0 / 128.0);
    out.color = vec4<f32>(c.rgb, alpha / 128.0);
    let origin_depth = -position_world_to_view((*inst).origin.xyz).z;
    let fv = min(max(origin_depth * 1024.0 * fog.params.x + fog.params.y, fog.params.z), fog.params.w);
    out.fog = trunc(fv) / 255.0;
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: ShrubVertexOutput) -> @location(0) vec4<f32> {
    let duv = vec4<f32>(dpdx(in.uv), dpdy(in.uv));
    let t = sample_world(in.uv, log2(in.depth / params.misc.z) + in.k, params.misc.y, duv);
    var rgb = min(t.rgb * in.color.rgb, vec3<f32>(1.0));
    if (fog.color.w > 0.5) {
        rgb = mix(fog.color.rgb, rgb, in.fog);
    }
    let a_s = min(floor(round(t.a * 255.0) * round(in.color.a * 128.0) / 128.0), 255.0);
    // GS TEST_1 alpha test (ATST GEQUAL AREF, AFAIL RGB_ONLY): which half of the split this draw is (gs_state.rs).
#ifdef GS_ATEST_PASS
    if (a_s < f32(#{GS_AREF})) { discard; }
#endif
#ifdef GS_ATEST_FAIL
    if (a_s >= f32(#{GS_AREF})) { discard; }
#endif
    return vec4<f32>(srgb_to_linear(rgb), a_s / 128.0);
}

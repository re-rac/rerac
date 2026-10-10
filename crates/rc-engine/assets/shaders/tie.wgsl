// Tie pass: instanced class meshes, colour = the instance's LightTies slot table, GS TFX = MODULATE,
// GS fog (one F per instance), GS mip rule. Rust side: tie_render.rs; lighting: rc_formats::tie_light.
//
// Instance data (`insts[mesh tag]`): the class -> Bevy-world matrix (the game's column-major instance
// matrix with game_to_bevy applied after it; it may scale, shear and mirror, so the entity transform is
// not used for positions), the bounding-sphere centre (Bevy world) with the draw distance in w, the radius,
// and 64 packed RGBA8 colours (0x80 = 1.0).
//
// Per-frame TieProc decision (`lods[mesh tag]`, tie_lod.rs): x = LOD (3 = culled) | F << 8, y = k,
// z = VU qw4.w = 256k, w = qw4.z = 256 - 256k (f32 bits). Vertices of the other LODs are dropped;
// VU1 program 13507 writes the same F for every vertex of an instance (qw 5.w).
// Fat vertices (13507 L25..L39): position + k * delta (`mulx.xyz vf05, vf05, vf27`, `add.xyzw vf05, vf05,
// vf28`); colour per lane = low byte of avg * w + c0 * z with avg = (c1 + c2) * 0.5, all in VU floats
// (`mulay/maddy vf29, vf30, vf27`, `mulaw ACC, vf29, vf27`, `maddz vf11, vf11, vf27`), palette lanes
// 0x4b000000 + byte. Dinky vertices: the slot colour as is (`lq vf11, 838(vi09)`).
// Texture: same GS rules as tfrag.wgsl (MODULATE, raw display-encoded bytes, As = At * Af >> 7, mip level
// round(log2(z / 32) + K) clamped to 0..MXL).
// Point lights (`LightTies`' third light, crate::world_lights): when the instance's nibble list names any, each slot
// colour the vertex reads (c0, and c1 / c2 of a fat vertex, before the VU blend) gets the merged light on its class
// normal added, clamped at 243. With an empty list (0xffff) nothing changes. The class normals ride in the
// instance record, the lists (one vec4 per instance, x) and the bank (16 vec4, f32 bits) after the per-frame LOD
// words (no bindings of their own: a changed material layout reorders draws and changes lightless frames).

#import bevy_pbr::{
    mesh_functions,
    view_transformations::{position_world_to_clip, position_world_to_view},
}
#import rerac::world_lights::{WorldLight, NO_LIGHTS, InstanceLight, instance_light, instance_lit_packed}

struct TieFog {
    color: vec4<f32>,
    params: vec4<f32>,
    // x: the texture option (crate::graphics; sample_world).
    tex_mode: vec4<f32>,
}

struct TieParams {
    // x = near (32), y = MXL, z = 1: tint by LOD.
    misc: vec4<f32>,
}

struct TieInst {
    model: mat4x4<f32>,
    // xyz = bounding-sphere centre (Bevy world), w = draw distance (world units).
    centre: vec4<f32>,
    // x = bounding radius (world units).
    misc: vec4<f32>,
    colors: array<u32, 64>,
    // The class normals (s16: x | y << 16, z) the point lights use.
    normals: array<vec2<u32>, 64>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tex_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<storage, read> fog: TieFog;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> params: TieParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<storage, read> insts: array<TieInst>;

@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<storage, read> lods: array<vec4<u32>>;

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

// `lods` = n instance words, n (nibble list, -, -, -), then the bank.
fn tie_count() -> u32 { return (arrayLength(&lods) - 16u) / 2u; }

fn bank() -> array<WorldLight, 8> {
    let b = 2u * tie_count();
    var ls: array<WorldLight, 8>;
    for (var k = 0u; k < 8u; k += 1u) {
        ls[k] = WorldLight(bitcast<vec4<f32>>(lods[b + 2u * k]), bitcast<vec4<f32>>(lods[b + 2u * k + 1u]));
    }
    return ls;
}

// Slot `j`'s colour with the instance's merged point light.
fn lit_slot(rgba: u32, j: u32, il: InstanceLight, tag: u32) -> u32 {
    let p = insts[tag].normals[j];
    let n = vec3<f32>(f32(bitcast<i32>(p.x << 16u) >> 16u), f32(bitcast<i32>(p.x) >> 16u), f32(bitcast<i32>(p.y << 16u) >> 16u)) / 32768.0;
    return instance_lit_packed(rgba, il, n, insts[tag].model, 243.0);
}

struct TieVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    // light slot | TEX1 K (s12, 1/16) << 20
    @location(2) info: u32,
    // morph slot 1 | slot 2 << 6 | fat << 12 | LOD << 13
    @location(3) morph: u32,
    @location(4) delta: vec3<f32>,
}

struct TieVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(linear) color: vec4<f32>,
    @location(2) @interpolate(flat) fog: f32,
    // Camera depth in raw units, perspective-correct (= n / Q per pixel).
    @location(3) depth: f32,
    @location(4) @interpolate(flat) k: f32,
    // Debug tint: 0 none, 1 LOD 1, 2 LOD 2.
    @location(5) @interpolate(flat) tint: u32,
}

// VU FMAC product of two non-negative floats, truncated (rc_formats ps2::mul; tie_lod::vu_mul).
fn vu_mul(a: u32, b: u32) -> u32 {
    let ea = i32((a >> 23u) & 0xffu);
    let eb = i32((b >> 23u) & 0xffu);
    if (ea == 0 || eb == 0) { return 0u; }
    let ma = (a & 0x7fffffu) | 0x800000u;
    let mb = (b & 0x7fffffu) | 0x800000u;
    let ah = ma >> 12u;
    let al = ma & 0xfffu;
    let bh = mb >> 12u;
    let bl = mb & 0xfffu;
    let mid = ah * bl + al * bh;
    let low = ((mid & 0xfffu) << 12u) + al * bl;
    let top = ah * bh + (mid >> 12u) + (low >> 24u);
    var m = top;
    var e = ea + eb - 126;
    if (top < 0x800000u) {
        m = (top << 1u) | ((low >> 23u) & 1u);
        e = ea + eb - 127;
    }
    if (e < 1) { return 0u; }
    if (e > 255) { return 0x7fffffffu; }
    return (u32(e) << 23u) | (m & 0x7fffffu);
}

// VU FMAC sum of two non-negative floats, truncated (ps2::add; tie_lod::vu_add).
fn vu_add(a: u32, b: u32) -> u32 {
    let ea = i32(a >> 23u);
    let eb = i32(b >> 23u);
    if (ea == 0) { return b; }
    if (eb == 0) { return a; }
    var hi = a;
    var lo = b;
    var eh = ea;
    var el = eb;
    if (eb > ea) {
        hi = b;
        lo = a;
        eh = eb;
        el = ea;
    }
    let d = u32(eh - el);
    if (d >= 25u) { return hi; }
    let s = ((hi & 0x7fffffu) | 0x800000u) + (((lo & 0x7fffffu) | 0x800000u) >> d);
    if (s >= 0x1000000u) { return (u32(eh + 1) << 23u) | ((s >> 1u) & 0x7fffffu); }
    return (u32(eh) << 23u) | (s & 0x7fffffu);
}

// One colour lane of a fat vertex (tie_lod::vu_fat_color): bytes c0, c1, c2, weights w, z (f32 bits).
fn vu_fat_lane(c0: u32, c1: u32, c2: u32, w: u32, z: u32) -> u32 {
    let avg = vu_add(vu_mul(0x4b000000u | c1, 0x3f000000u), vu_mul(0x4b000000u | c2, 0x3f000000u));
    return vu_add(vu_mul(avg, w), vu_mul(0x4b000000u | c0, z)) & 0xffu;
}

@vertex
fn vertex(v: TieVertex) -> TieVertexOutput {
    var out: TieVertexOutput;
    let tag = mesh_functions::get_tag(v.instance_index);
    let inst = &insts[tag];
    let lod = lods[tag];
    let vlod = (v.morph >> 13u) & 3u;
    if ((lod.x & 3u) != vlod) {
        // Culled this frame (3), or a vertex of another LOD: the triangle is clipped away.
        out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        return out;
    }
    let k = bitcast<f32>(lod.y);
    let world = ((*inst).model * vec4<f32>(v.position + k * v.delta, 1.0)).xyz;
    out.position = position_world_to_clip(world);
    out.uv = v.uv;
    var c0 = (*inst).colors[v.info & 63u];
    let list = lods[tie_count() + tag].x & 0xffffu;
    var il = InstanceLight(vec3<f32>(0.0), vec3<f32>(0.0), 0.0, false);
    if (list != NO_LIGHTS) {
        il = instance_light(list, bank(), (*inst).centre.xyz);
        if (il.hit) { c0 = lit_slot(c0, v.info & 63u, il, tag); }
    }
    var rgba = c0;
    if (((v.morph >> 12u) & 1u) != 0u) {
        var c1 = (*inst).colors[v.morph & 63u];
        var c2 = (*inst).colors[(v.morph >> 6u) & 63u];
        if (il.hit) {
            c1 = lit_slot(c1, v.morph & 63u, il, tag);
            c2 = lit_slot(c2, (v.morph >> 6u) & 63u, il, tag);
        }
        rgba = 0u;
        for (var i = 0u; i < 32u; i += 8u) {
            let lane = vu_fat_lane((c0 >> i) & 0xffu, (c1 >> i) & 0xffu, (c2 >> i) & 0xffu, lod.z, lod.w);
            rgba |= lane << i;
        }
    }
    out.color = unpack4x8unorm(rgba) * (255.0 / 128.0);
    out.depth = -position_world_to_view(world).z * 1024.0;
    out.fog = f32(lod.x >> 8u) / 255.0;
    out.k = f32(bitcast<i32>(v.info) >> 20u) / 16.0;
    out.tint = select(0u, vlod, params.misc.z > 0.5);
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: TieVertexOutput) -> @location(0) vec4<f32> {
    let duv = vec4<f32>(dpdx(in.uv), dpdy(in.uv));
    let t = sample_world(in.uv, log2(in.depth / params.misc.x) + in.k, params.misc.y, duv);
    var rgb = min(t.rgb * in.color.rgb, vec3<f32>(1.0));
    if (fog.color.w > 0.5) {
        rgb = mix(fog.color.rgb, rgb, in.fog);
    }
    if (in.tint == 1u) { rgb = mix(rgb, vec3<f32>(1.0, 0.0, 0.0), 0.5); }
    if (in.tint == 2u) { rgb = mix(rgb, vec3<f32>(0.0, 0.2, 1.0), 0.5); }
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

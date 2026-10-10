// Shrub billboards (far LOD sprites), as ShrubProc builds them on the EE. Rust side and the full rule with
// addresses: shrub_billboard.rs; formats: rc_formats::shrub::{shrub_fade, billboard_extent, billboard_corners}.
//
// Per instance (`insts[info & 0xffffff]`): game-space origin t + F (the class's billboard byte; 0 = billboard
// only), Bevy-space bounding-sphere centre + run-time draw distance D, world extent (W, H, Z), average lit colour.
// Per vertex: corner = info >> 24 (GS strip order).
//
// z = view depth of the centre. Listed when D - z >= 0 and (F = 0 or z >= F); alpha (GS 0..0x80):
//   F != 0 and z < F + 8: a = min(trunc(z * 4096) - F * 4096, 0x8000) >> 8   (cross-fade with the mesh)
//   otherwise:           a = min(trunc((D - z) * 4096) >> 1, 0x8000) >> 8
// params.misc.x = draw: 0 = pass 1 (a == 0x80) texels with As >= 0x60 (Z write), 1 = pass 1 texels with
// As < 0x60 (TEST_1 AFAIL = RGB_ONLY: no Z), 2 = pass 2 (a in 1..0x7f; alpha test NEVER + RGB_ONLY: no Z).
// The vertex shader lists by pass (x < 1.5: pass 1); the As split is the GS_ATEST_* defs (gs_state.rs).
// Corners: d = (t - eye) * rsqrt(|t - eye|^2); p = t + y * W * (d.y, -d.x, 0) + (zc * H + Z) * (0, 0, 1).
// Colour: MODULATE with RGBAQ = (average colour, a); fog F per vertex from the corner depth (tfrag line);
// mip level round(log2(z / 32) + K) clamped to 0..MXL, per pixel.

#import bevy_pbr::{
    mesh_view_bindings::view,
    view_transformations::{position_world_to_clip, position_world_to_view},
}

struct BillboardFog {
    color: vec4<f32>,
    params: vec4<f32>,
    // x: the texture option (crate::graphics; sample_world).
    tex_mode: vec4<f32>,
}

struct BillboardParams {
    // x = draw (see above), y = MXL, z = K (mip levels), w = near (32).
    misc: vec4<f32>,
}

struct BillboardInst {
    // xyz = instance origin (game axes), w = F.
    origin: vec4<f32>,
    // xyz = bounding-sphere centre (Bevy world), w = D.
    centre: vec4<f32>,
    // (W, H, Z, 0) in world units.
    extent: vec4<f32>,
    // x = r | g << 8 | b << 16 (0x80 = 1.0).
    colour: vec4<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tex_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<storage, read> fog: BillboardFog;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> params: BillboardParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<storage, read> insts: array<BillboardInst>;

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

struct BillboardVertex {
    @location(0) position: vec3<f32>,
    @location(1) info: u32,
}

struct BillboardVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(linear) color: vec4<f32>,
    @location(2) @interpolate(linear) fog: f32,
    // Camera depth in integer units, perspective-correct (= n / Q per pixel).
    @location(3) depth: f32,
}

fn culled() -> BillboardVertexOutput {
    var out: BillboardVertexOutput;
    out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    return out;
}

@vertex
fn vertex(v: BillboardVertex) -> BillboardVertexOutput {
    let inst = &insts[v.info & 0xffffffu];
    let corner = v.info >> 24u;
    let z = -position_world_to_view((*inst).centre.xyz).z;
    let d = (*inst).centre.w;
    if (d - z < 0.0) { return culled(); }
    let far = min(i32(trunc((d - z) * 4096.0)) >> 1u, 0x8000) >> 8u;
    let f = i32((*inst).origin.w);
    var a = far;
    if (f != 0) {
        let iz = i32(trunc(max(z, 0.0) * 4096.0)) - f * 4096;
        if (iz < 0) { return culled(); }
        let fade = min(iz, 0x8000) >> 8u;
        a = select(fade, far, fade == 128);
    }
    let draw = params.misc.x;
    if (draw < 1.5) {
        if (a != 128) { return culled(); }
    } else if (a <= 0 || a >= 128) {
        return culled();
    }

    // Eye in game axes (Bevy (x, y, z) = game (x, z, -y)).
    let e = view.world_position;
    let eye = vec3<f32>(e.x, -e.z, e.y);
    let t = (*inst).origin.xyz;
    let r = t - eye;
    let q = inverseSqrt((r.x * r.x + r.y * r.y) + r.z * r.z);
    let dx = r.x * q;
    let dy = r.y * q;
    // Corner table 0x1c3130: (y, z) = (-.5, 1), (.5, 1), (-.5, 0), (.5, 0); ST = (0, 0), (1, 0), (0, 1), (1, 1).
    let cy = select(-0.5, 0.5, (corner & 1u) == 1u);
    let cz = select(1.0, 0.0, corner >= 2u);
    let ext = (*inst).extent;
    let h = cy * ext.x;
    let vz = cz * ext.y + ext.z;
    let g = vec3<f32>(t.x + h * dy, t.y - h * dx, t.z + vz);
    let world = vec3<f32>(g.x, g.z, -g.y);

    var out: BillboardVertexOutput;
    out.position = position_world_to_clip(world);
    out.uv = vec2<f32>(f32(corner & 1u), f32(corner >> 1u));
    let c = unpack4x8unorm((*inst).colour.x) * (255.0 / 128.0);
    out.color = vec4<f32>(c.rgb, f32(a) / 128.0);
    let depth = -position_world_to_view(world).z * 1024.0;
    out.depth = depth;
    out.fog = trunc(min(max(depth * fog.params.x + fog.params.y, fog.params.z), fog.params.w)) / 255.0;
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: BillboardVertexOutput) -> @location(0) vec4<f32> {
    let duv = vec4<f32>(dpdx(in.uv), dpdy(in.uv));
    let t = sample_world(in.uv, log2(in.depth / params.misc.w) + params.misc.z, params.misc.y, duv);
    let a_s = min(floor(round(t.a * 255.0) * round(in.color.a * 128.0) / 128.0), 255.0);
    // GS TEST_1 alpha test (ATST GEQUAL AREF, AFAIL RGB_ONLY): which half of the split this draw is (gs_state.rs).
#ifdef GS_ATEST_PASS
    if (a_s < f32(#{GS_AREF})) { discard; }
#endif
#ifdef GS_ATEST_FAIL
    if (a_s >= f32(#{GS_AREF})) { discard; }
#endif
    var rgb = min(t.rgb * in.color.rgb, vec3<f32>(1.0));
    if (fog.color.w > 0.5) {
        rgb = mix(fog.color.rgb, rgb, in.fog);
    }
    return vec4<f32>(srgb_to_linear(rgb), a_s / 128.0);
}

// Tfrag base pass: GS TFX = MODULATE, then GS fog (PRIM FGE = 1), no lighting.
//
// GS texture function MODULATE (TCC = 1): Cv = (Ct * Cf) >> 7, Av = (At * Af) >> 7, clamped to 0xff.
// The vertex colour attribute already carries rgba / 128 (0x80 = 1.0), so here out = tex * color.
// The texture's alpha channel holds the raw GS texel alpha (0..0x80, stored as byte / 255).
//
// Interpolation (PRIM 0x7c: IIP = 1 Gouraud, TME, FGE, ABE): the GS interpolates RGBA and F linearly in
// screen space and only ST perspective-correctly (divided by the interpolated Q). So `color` and `fog`
// are @interpolate(linear) (WGSL noperspective), `uv` keeps WGSL's default perspective interpolation.
//
// Alpha: the tfrag pass runs with ALPHA_1 = 0x8000000044 (A = Cs, B = Cd, C = As, D = Cd, FIX 0x80:
// Cout = ((Cs - Cd) * As >> 7) + Cd) and TEST_1 = 0x5360b (ATE, ATST = GEQUAL, AREF = 0x60,
// AFAIL = RGB_ONLY: a failing pixel still writes its blended RGB, only Z is not written).
// The fragment returns alpha = As / 128. A batch whose As is 0x80 everywhere is one opaque draw (the blend
// is then Cs); any other batch is two draws of the same mesh (gs_state.rs): GS_ATEST_PASS keeps As >= AREF
// (blended, Z written), GS_ATEST_FAIL keeps As < AREF (blended, no Z).
// The texture is Rgba8Unorm holding the raw display-encoded bytes the GS sees, so the product is
// computed in display space (as the GS does) and only then converted to linear for Bevy's sRGB
// view target, which re-encodes it to exactly the GS value.
//
// Fog (docs/plan/game_camera_fog.md): the VU1 tfrag program writes the F lane of XYZF2 as
// trunc(clamp(w + qw661.w, qw656.y, qw656.z)), where w is the projection's 4th output
// = camera depth (integer units) * slope. The GS interpolates F linearly in screen space and
// blends RGB only: C = FOGCOL + (C - FOGCOL) * F / 255 (F = 255: no fog).
//
// LOD (docs/formats/tfrag_rac1.md §3b; tfrag_render.rs / tfrag_lod.rs for the data): every strip list of every
// tfrag is in the mesh; `modes[tfrag]` is this frame's `TfragProc` decision (0 = culled, 2 = clipping path,
// else the VU1 MSCAL entry 6 / 8 / 0xa / 0xe / 0x10 / 0x14) and a vertex of another list is dropped. The rest
// replays the VU1 program for that entry:
//   L18 (modes 8, 0xa, 0xe) morphs LOD-01 positions, L17 (0xe, 0x10) LOD-0 positions (after L18, so LOD-0
//   vertices blend toward already-morphed LOD-01 parents in mode 0xe):
//     t.xy = clamp(w * qw66a.xy + qw66b.xy, 0, (0.5, 1))  = (u/2, 1 - u),  w = the vertex's own VU w
//     pos' = (P1 + P2) * t.x + pos * t.y,  col' = ftoi0((C1 + C2) * t.x + col * t.y)
//   The VU blends clip-space xyzw, which is affine in the world position, so blending world positions and
//   projecting afterwards is the same map. UVs do not morph.
//   L26 (mode 8, LOD-01) / L25 (mode 0xe, LOD-0): if both parents have w < qw66a.w (depth beyond D0 / D1),
//   the morphed position is not stored and the vertex-info entry is replaced by parent 1's (UV and position).
//   L48 / L47: the same collapse test for the "extra" entries (parent 1 from unk_indices_2), after the morph.
// w = depth * slope with the fog slope (negative), exactly the VU's w lane before the fog offset.
//
// Mip level (GS, TEX1 LCM = 0, L = 0): LOD = log2(1 / |Q|) + K with Q = qw656.x / w = n / z (z = camera depth
// in raw units, n = 32), so LOD = log2(z / 32) + K. MMIN = LINEAR_MIPMAP_NEAREST: level = round(LOD) clamped to
// 0..MXL (LOD < 0 is magnification, MMAG = LINEAR on level 0), bilinear inside the level. Q is interpolated
// perspective-correctly per pixel, so z is the exact per-pixel depth.
//
// Point lights (`LightTfrags`' point pass, crate::world_lights): when the tfrag's nibble list names any, every
// position slot the vertex reads gets them added to its baked RGBA (world_lights::tfrag_lit) before the morph
// blends, as VU1 reads the relit RGBA block. With an empty list (0xffff) nothing changes. The lists (one u32 per
// tfrag) and the bank (8 × 8 f32 as u32 bits) follow the draw modes in `modes`; a slot's normal is packed in its
// last word. (No bindings of their own: a changed material layout reorders draws and changes lightless frames.)

#import bevy_pbr::view_transformations::{position_world_to_clip, position_world_to_view}
#import rerac::world_lights::{WorldLight, NO_LIGHTS, tfrag_lit}

struct TfragFog {
    // rgb = FOGCOL (display-encoded 0..1), w = 1 when fog is enabled.
    color: vec4<f32>,
    // (slope per integer unit of depth, offset qw661.w, lower clamp qw656.y, upper clamp qw656.z).
    params: vec4<f32>,
    // x: the texture option (crate::graphics; sample_world).
    tex_mode: vec4<f32>,
}

struct TfragLod {
    k666: vec4<f32>,
    k667: vec4<f32>,
    k668: vec4<f32>,
    k669: vec4<f32>,
    // x = VU w per raw unit of depth, y = 1: tint by LOD, z = near (32).
    misc: vec4<f32>,
    // y = MXL.
    tex: vec4<f32>,
}

struct Slot {
    pos: vec3<f32>,
    color: u32,
    p1: u32,
    p2: u32,
    // 0 none, 1 = written by a LOD-01 primary, 2 = by a LOD-0 primary.
    tier: u32,
    // The LightTfrags record's normal: azimuth | elevation << 8 | 1 << 16 (0: no record).
    normal: u32,
}

struct VInfo {
    uv: vec2<f32>,
    slot: u32,
    // 0 common, 1 LOD-01, 2 LOD-0 (collapse candidates).
    tier: u32,
    p1: u32,
    p2: u32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tex_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<storage, read> fog: TfragFog;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> lod: TfragLod;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<storage, read> slots: array<Slot>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<storage, read> vinfos: array<VInfo>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var<storage, read> modes: array<u32>;

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

// `modes` = n draw modes, n nibble lists, then the bank.
fn tfrag_count() -> u32 { return (arrayLength(&modes) - 64u) / 2u; }

fn bank() -> array<WorldLight, 8> {
    let b = 2u * tfrag_count();
    var ls: array<WorldLight, 8>;
    for (var k = 0u; k < 8u; k += 1u) {
        let o = b + 8u * k;
        ls[k] = WorldLight(
            bitcast<vec4<f32>>(vec4<u32>(modes[o], modes[o + 1u], modes[o + 2u], modes[o + 3u])),
            bitcast<vec4<f32>>(vec4<u32>(modes[o + 4u], modes[o + 5u], modes[o + 6u], modes[o + 7u])),
        );
    }
    return ls;
}

// N = (cos az · cos el, sin az · cos el, sin el) in game axes (256 steps per turn), in Bevy axes (x, z, −y);
// w = 1 when the slot has a light record.
fn slot_normal(p: u32) -> vec4<f32> {
    let a = f32(p & 0xffu) * (6.2831853 / 256.0);
    let e = f32((p >> 8u) & 0xffu) * (6.2831853 / 256.0);
    return vec4<f32>(cos(a) * cos(e), sin(e), -sin(a) * cos(e), f32((p >> 16u) & 1u));
}

// This vertex's tfrag's nibble list (set once per vertex, read by `own`).
var<private> point_list: u32 = 0xffffu;

struct TfragVertex {
    @location(0) position: vec3<f32>,
    // (global vertex-info index, tfrag index | strip list << 16 | TEX1 K (s12, 1/16 units) << 18)
    @location(1) tref: vec2<u32>,
}

struct TfragVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // Gouraud RGBA / 128, screen-space linear like the GS.
    @location(1) @interpolate(linear) color: vec4<f32>,
    // GS F / 255, screen-space linear like the GS.
    @location(2) @interpolate(linear) fog: f32,
    // Camera depth in raw units, perspective-correct (= n / Q per pixel).
    @location(3) depth: f32,
    // Debug tint: 0 none, 1 LOD 1, 2 LOD 2, 3 clipping path.
    @location(4) @interpolate(flat) tint: u32,
    // TEX1 K of the triangle's ad-gif, in mip levels.
    @location(5) @interpolate(flat) k: f32,
}

// Position (world, game units) and colour (0..255) of a VU position slot.
struct Vtx {
    pos: vec3<f32>,
    col: vec4<f32>,
}

fn depth_raw(p: vec3<f32>) -> f32 {
    // Camera-space depth in the game's integer units (Bevy view space looks down -z, game units).
    return -position_world_to_view(p).z * 1024.0;
}

fn w_of(p: vec3<f32>) -> f32 { return depth_raw(p) * lod.misc.x; }

fn own(s: u32) -> Vtx {
    let sl = slots[s];
    let c = unpack4x8unorm(sl.color) * 255.0;
    if (point_list == NO_LIGHTS) { return Vtx(sl.pos, c); }
    return Vtx(sl.pos, tfrag_lit(point_list, bank(), c, sl.pos, slot_normal(sl.normal)));
}

fn blend(c: Vtx, p1: Vtx, p2: Vtx, t: vec2<f32>) -> Vtx {
    return Vtx((p1.pos + p2.pos) * t.x + c.pos * t.y, floor((p1.col + p2.col) * t.x + c.col * t.y));
}

// A common or LOD-01 slot after L18 / L26 in mode m.
fn value01(s: u32, m: u32) -> Vtx {
    let sl = slots[s];
    let c = own(s);
    if (sl.tier != 1u || !(m == 8u || m == 10u || m == 14u)) { return c; }
    let p1 = own(sl.p1);
    let p2 = own(sl.p2);
    if (m == 8u && w_of(p1.pos) < lod.k666.w && w_of(p2.pos) < lod.k666.w) { return c; }
    let t = clamp(w_of(c.pos) * lod.k666.xy + lod.k668.xy, vec2<f32>(0.0), vec2<f32>(0.5, 1.0));
    return blend(c, p1, p2, t);
}

// Any slot after all morph passes of mode m.
fn value(s: u32, m: u32) -> Vtx {
    let sl = slots[s];
    if (sl.tier != 2u) { return value01(s, m); }
    let c = own(s);
    if (!(m == 14u || m == 16u)) { return c; }
    let p1 = value01(sl.p1, m);
    let p2 = value01(sl.p2, m);
    if (m == 14u && w_of(p1.pos) < lod.k667.w && w_of(p2.pos) < lod.k667.w) { return c; }
    let t = clamp(w_of(c.pos) * lod.k667.xy + lod.k669.xy, vec2<f32>(0.0), vec2<f32>(0.5, 1.0));
    return blend(c, p1, p2, t);
}

@vertex
fn vertex(v: TfragVertex) -> TfragVertexOutput {
    var out: TfragVertexOutput;
    let tf = v.tref.y & 0xffffu;
    let list = (v.tref.y >> 16u) & 3u;
    let m = modes[tf];
    // Strip list the mode draws: LOD 2 for MSCAL 6, LOD 1 for 8 / 0xa, LOD 0 otherwise (incl. the clip path).
    let mlist = select(select(0u, 1u, m == 8u || m == 10u), 2u, m == 6u);
    if (m == 0u || mlist != list) {
        // Not drawn this frame: all three vertices of the triangle land on one point outside the clip volume.
        out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        return out;
    }
    point_list = modes[tfrag_count() + tf] & 0xffffu;
    var e = vinfos[v.tref.x];
    if ((m == 8u && e.tier == 1u) || (m == 14u && e.tier == 2u)) {
        let thr = select(lod.k667.w, lod.k666.w, m == 8u);
        let pe = vinfos[e.p1];
        if (w_of(value(pe.slot, m).pos) < thr && w_of(value(e.p2, m).pos) < thr) { e = pe; }
    }
    let r = value(e.slot, m);
    out.position = position_world_to_clip(r.pos);
    out.uv = e.uv;
    out.color = r.col / 128.0;
    let depth = depth_raw(r.pos);
    out.depth = depth;
    // VU: w = depth * slope (the w row of the matrix), += qw661.w, min qw656.z, max qw656.y, ftoi4.
    let f = max(min(depth * fog.params.x + fog.params.y, fog.params.w), fog.params.z);
    out.fog = trunc(f) / 255.0;
    out.k = f32(bitcast<i32>(v.tref.y << 2u) >> 20u) / 16.0;
    out.tint = select(0u, select(list, 3u, m == 2u), lod.misc.y > 0.5);
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: TfragVertexOutput) -> @location(0) vec4<f32> {
    let duv = vec4<f32>(dpdx(in.uv), dpdy(in.uv));
    // GS mip level: round(log2(z / n) + K), clamped to 0..MXL (or the texture option's level: sample_world).
    let t = sample_world(in.uv, log2(in.depth / lod.misc.z) + in.k, lod.tex.y, duv);
    var rgb = min(t.rgb * in.color.rgb, vec3<f32>(1.0));
    if (fog.color.w > 0.5) {
        rgb = mix(fog.color.rgb, rgb, in.fog);
    }
    if (in.tint == 1u) { rgb = mix(rgb, vec3<f32>(1.0, 0.0, 0.0), 0.5); }
    if (in.tint == 2u) { rgb = mix(rgb, vec3<f32>(0.0, 0.2, 1.0), 0.5); }
    if (in.tint == 3u) { rgb = mix(rgb, vec3<f32>(0.0, 1.0, 0.0), 0.5); }
    // As = (At * Af) >> 7 in GS units (At raw 0..0x80, Af 0x80 = 1.0), clamped to 0xff; FGE leaves it alone.
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

// Moby metal ("shine") pass: the class's metal packets (texture −2 chrome / −3 glass), drawn after the moby's
// own packets when MobyProc's shine gate passes (crate::moby_lod). Rust side: moby_render.rs
// (MobyMetalMaterial, metal meshes), moby_lod.rs (shine alpha, sphere-map basis E).
// Spec: docs/plan/moby_skinning_lighting.md §6 and §9.
//
// Vertex (VU0 104691 single-shot entries 0x111 / 0x121 / 0x12D = 3 / 2 / 1 matrices, called per vertex by
// fun_001ee650's metal tail 0x1ef65c):
//   M     = Σ w_k/256 · F[j_k] (joints straight from the palette, no slot cache; count ≤ 1 → F[j0])
//   pos   = trunc(M·p) (ftoi0), s16 wrap (ppach), then the moby matrix with the depth bias below
//   n'    = M0·n.x + M1·n.y + M2·n.z (not normalised; n from the boot-ELF (cos, sin) table)
//   RGB   = low byte of 0x70 + Σ_k ⌊32·C_k·max(L_k·n', 0)⌋ (vf28..30 = C·0.25, ambient 0x70 on the 65536
//           bias, one truncating add per light; no rsqrt, no back-light term, no saturation: `ppach` +
//           `ppacb`); α = the shine alpha (`sb` over byte 3)
//   ST    = (ftoi12(e.xy + 1)) >> 1 / 4096, e = E_0·n'.x + E_1·n'.y + E_2·n'.z (E from MobyProc, per moby)
// VU1 13859 L2 (MSCAL 0x0a) with the per-moby matrix plus (0, 0, 1000, 0) on its translation row (0x212b08):
// the GS Z grows by 1000·Q = 1000·n/z, i.e. clip.z += 1000·n/(1024·2^24) in the port's GameProjection
// (depth = Z/2^24, clip.w = depth in game units).
// GS: PRIM 0x7c (tristrip, IIP, TME, FGE, ABE); TEX0 = the level's chrome (128×128) or glass (64×64) PSMT8
// map, TCC 1, TFX MODULATE; TEX1 MXL 0 / MMAG linear; CLAMP_1 = 5 (clamp); ALPHA_1 0x8000000044; TEST_1
// 0x5360b (the moby's, restored after a fade). The GS_ATEST_* defs select the half of the alpha-test split.

#import bevy_pbr::{
    mesh_functions,
    view_transformations::{position_world_to_clip, position_world_to_view},
}
#ifdef DISPLAY_BLEND_MIX
#import rerac::display_blend::gs_mix
#endif

struct MobyFog {
    color: vec4<f32>,
    params: vec4<f32>,
}

struct MobyInst {
    model: mat4x4<f32>,
    light_rows: array<vec4<f32>, 3>,
    light_colors: array<vec4<f32>, 3>,
    neg_k: vec4<f32>,
    ambient: vec4<f32>,
    misc: vec4<u32>,
}

struct MobyLod {
    // x = vertex alpha of the moby's own packets, y = flags, z = shine alpha, w = 0.
    misc: vec4<u32>,
    // Sphere-map basis rows E_0..E_2 (xyz).
    e: array<vec4<f32>, 3>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tex_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<storage, read> fog: MobyFog;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<storage, read> insts: array<MobyInst>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<storage, read> palette: array<mat4x4<f32>>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<storage, read> normal_table: array<vec2<f32>>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var<storage, read> lods: array<MobyLod>;

// 1000·n / 1024 / 2^24 with n = 32 (NEAR, integer units).
const METAL_Z_BIAS: f32 = 1000.0 * 32.0 / 1024.0 / 16777216.0;

struct MetalVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    // x = azimuth | elevation << 8 | joint count << 16; y = joints; z = weights (10 bits each).
    @location(2) skin: vec4<u32>,
}

struct MetalVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) st: vec2<f32>,
    @location(1) @interpolate(linear) color: vec4<f32>,
    @location(2) @interpolate(linear) fog: f32,
}

fn s16(x: i32) -> i32 { return (x << 16u) >> 16u; }

@vertex
fn vertex(v: MetalVertex) -> MetalVertexOutput {
    var out: MetalVertexOutput;
    let tag = mesh_functions::get_tag(v.instance_index);
    let inst = &insts[tag];
    let lod = &lods[tag];
    let base = (*inst).misc.x;

    let count = (v.skin.x >> 16u) & 0xffu;
    let j = vec3<u32>(v.skin.y & 0xffu, (v.skin.y >> 8u) & 0xffu, (v.skin.y >> 16u) & 0xffu);
    var m = palette[base + j.x];
    if (count >= 2u) {
        let w = vec3<f32>(f32(v.skin.z & 0x3ffu), f32((v.skin.z >> 10u) & 0x3ffu), f32((v.skin.z >> 20u) & 0x3ffu)) / 256.0;
        m = palette[base + j.x] * w.x + palette[base + j.y] * w.y;
        if (count >= 3u) {
            m = m + palette[base + j.z] * w.z;
        }
    }

    let sp = clamp(trunc((m * vec4<f32>(v.position, 1.0)).xyz), vec3<f32>(-2147483520.0), vec3<f32>(2147483520.0));
    let pi = vec3<i32>(s16(i32(sp.x)), s16(i32(sp.y)), s16(i32(sp.z)));
    let world = ((*inst).model * vec4<f32>(vec3<f32>(pi), 1.0)).xyz;
    out.position = position_world_to_clip(world);
    out.position.z = out.position.z + METAL_Z_BIAS;

    let ta = normal_table[v.skin.x & 0xffu];
    let te = normal_table[(v.skin.x >> 8u) & 0xffu];
    let n = vec3<f32>(ta.x * te.x, ta.y * te.x, te.y);
    let np = (m[0] * n.x + m[1] * n.y + m[2] * n.z).xyz;
    let d = max((*inst).light_rows[0] * np.x + (*inst).light_rows[1] * np.y + (*inst).light_rows[2] * np.z, vec4<f32>(0.0));
    // ACC = 65536 + 0x70/128, then += C'_k·d_k one term at a time: each add truncates to the 1/128 grid.
    let c0 = floor((*inst).light_colors[0].xyz * d.x * 32.0);
    let c1 = floor((*inst).light_colors[1].xyz * d.y * 32.0);
    let c2 = floor((*inst).light_colors[2].xyz * d.z * 32.0);
    let rgb = (vec3<i32>(112) + vec3<i32>(c0) + vec3<i32>(c1) + vec3<i32>(c2)) & vec3<i32>(0xff);
    out.color = vec4<f32>(vec3<f32>(rgb), f32((*lod).misc.z)) / 128.0;

    let e = (*lod).e[0].xy * np.x + (*lod).e[1].xy * np.y + (*lod).e[2].xy * np.z;
    let q = vec2<i32>(trunc((e + vec2<f32>(1.0)) * 4096.0)) >> vec2<u32>(1u);
    out.st = vec2<f32>(q) / 4096.0;

    let depth = -position_world_to_view(world).z * 1024.0;
    let fg = max(min(depth * fog.params.x + fog.params.y, fog.params.w), fog.params.z);
    out.fog = trunc(fg) / 255.0;
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: MetalVertexOutput) -> @location(0) vec4<f32> {
    let t = textureSample(tex, tex_sampler, in.st);
    var rgb = min(t.rgb * in.color.rgb, vec3<f32>(1.0));
    if (fog.color.w > 0.5) {
        rgb = mix(fog.color.rgb, rgb, in.fog);
    }
    let a_s = min(floor(round(t.a * 255.0) * round(in.color.a * 128.0) / 128.0), 255.0);
#ifdef GS_ATEST_PASS
    if (a_s < f32(#{GS_AREF})) { discard; }
#endif
#ifdef GS_ATEST_FAIL
    if (a_s >= f32(#{GS_AREF})) { discard; }
#endif
    // A moby drawn in moby order (gs_state `EffectTested` / `EffectLowAlpha`): on display bytes (crate::display_blend).
#ifdef DISPLAY_BLEND_MIX
    return gs_mix(round(rgb * 255.0), a_s);
#else
    return vec4<f32>(srgb_to_linear(rgb), a_s / 128.0);
#endif
}

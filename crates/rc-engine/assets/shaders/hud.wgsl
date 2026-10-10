// The 2D pass (docs/plan/hud_text.md §2; Rust side: hud_render.rs, text_render.rs).
//
// HUD_PRIMS: the GS primitives (HudSprite, DrawTexturedQuad, DrawRectOverlay) into the 512x416 offscreen
// target. Vertices are in game pixels already shifted by the GS's -0.5 / GPU +0.5 (so they sit on integer
// target coordinates); UVs in texels. Fragment = GS texture unit + MODULATE on raw GS bytes, output
// (Cs / 255, As / 128) for the (Cs - Cd)·As + Cd blend (ALPHA_1 0x44) set by GsPass::Hud.
//
// HUD_COMPOSITE: the UI material that puts the offscreen image on the main camera, bilinear or nearest (the HUD option),
// premultiplied display bytes -> straight linear colour for the sRGB target's alpha blend; `composite_static` the
// static layer's (premultiplied blend).
//
// HUD_ADD: the static layer's noise pass (ALPHA_1 0x68, added with FIX = the vertex alpha).

#import bevy_ui::ui_vertex_output::UiVertexOutput

#ifdef HUD_PRIMS
// Material2d bind group (MATERIAL_2D_BIND_GROUP_INDEX).
@group(2) @binding(0) var atlas: texture_2d<f32>;

struct HudVertex {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) rgba: u32,
    // atlas x | y << 16, texture w | h << 16, flags (1 = textured, 2 = REPEAT, 4 = NEAREST, 8 = ALPHA_1 0x48), 0
    @location(3) tex: vec4<u32>,
    // x0, x1, y0, y1 (inclusive pixels)
    @location(4) scissor: vec4<u32>,
}

struct HudVarying {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // Flat like the GS (IIP = 0): one colour per primitive.
    @location(1) @interpolate(flat) rgba: vec4<u32>,
    @location(2) @interpolate(flat) tex: vec4<u32>,
    @location(3) @interpolate(flat) scissor: vec4<u32>,
}

@vertex
fn vertex(v: HudVertex) -> HudVarying {
    var out: HudVarying;
    out.position = vec4<f32>(v.position.x / 256.0 - 1.0, 1.0 - v.position.y / 208.0, 0.5, 1.0);
    out.uv = v.uv;
    out.rgba = vec4<u32>(v.rgba & 0xffu, (v.rgba >> 8u) & 0xffu, (v.rgba >> 16u) & 0xffu, v.rgba >> 24u);
    out.tex = v.tex;
    out.scissor = v.scissor;
    return out;
}

// Texel p of the texture at the atlas rectangle `t`, CLAMP to its own size (or REPEAT, flag 2: CLAMP_1 = 0), as
// 0..255 bytes.
fn texel(t: vec4<u32>, p: vec2<i32>) -> vec4<f32> {
    let origin = vec2<i32>(i32(t.x & 0xffffu), i32(t.x >> 16u));
    let size = vec2<i32>(i32(t.y & 0xffffu), i32(t.y >> 16u));
    var q = clamp(p, vec2<i32>(0), size - vec2<i32>(1));
    if (t.z & 2u) != 0u {
        q = ((p % size) + size) % size;
    }
    return round(textureLoad(atlas, origin + q, 0) * 255.0);
}

@fragment
fn fragment(in: HudVarying) -> @location(0) vec4<f32> {
    let px = vec2<u32>(floor(in.position.xy));
    if px.x < in.scissor.x || px.x > in.scissor.y || px.y < in.scissor.z || px.y > in.scissor.w {
        discard;
    }
    let cf = vec4<f32>(in.rgba);
    var cs: vec3<f32>;
    var a: f32;
    if (in.tex.z & 1u) != 0u {
        // Bilinear (TEX1 0x261 inherited from the AA blit): texel centres at +0.5, sample position in 1/16
        // texel (12.4 UV), weights k/16, truncated.
        let s = vec2<i32>(round((in.uv - vec2<f32>(0.5)) * 16.0));
        let i0 = vec2<i32>(s.x >> 4u, s.y >> 4u);
        let f = vec2<f32>(vec2<i32>(s.x & 15, s.y & 15)) / 16.0;
        let t00 = texel(in.tex, i0);
        let t10 = texel(in.tex, i0 + vec2<i32>(1, 0));
        let t01 = texel(in.tex, i0 + vec2<i32>(0, 1));
        let t11 = texel(in.tex, i0 + vec2<i32>(1, 1));
        var ct = floor(mix(mix(t00, t10, f.x), mix(t01, t11, f.x), f.y));
        if (in.tex.z & 4u) != 0u {
            // TEX1 MMAG / MMIN NEAREST: the texel under the sample point.
            ct = texel(in.tex, vec2<i32>(floor(in.uv)));
        }
        // MODULATE: C = Ct·Cf >> 7, A = At·Af >> 7, clamped to 0xff.
        cs = min(floor(ct.rgb * cf.rgb / 128.0), vec3<f32>(255.0));
        a = min(floor(ct.a * cf.a / 128.0), 255.0);
#ifdef HUD_ADD
        // ALPHA_1 0x68, FIX = the vertex alpha: Cd + (Cs·FIX >> 7); the texture's alpha takes no part. Coverage 0.
        return vec4<f32>(floor(cs * cf.a / 128.0) / 255.0, 0.0);
#endif
    } else {
        cs = cf.rgb;
        a = cf.a;
    }
    // Premultiplied (the pipeline blends One / OneMinusSrcAlpha): ALPHA_1 0x44 `Cs·As + Cd·(1 − As)`; flag 8 = 0x48
    // `Cs·As + Cd` (coverage 0).
    let cov = select(a / 128.0, 0.0, (in.tex.z & 8u) != 0u);
    return vec4<f32>(cs / 255.0 * (a / 128.0), cov);
}
#endif

#ifdef HUD_COMPOSITE
@group(1) @binding(0) var hud_image: texture_2d<f32>;
// x: 1 bilinear (the HUD option's Original: the TV showed the picture smoothed), 0 nearest (Sharp pixels).
@group(1) @binding(1) var<uniform> scaling: vec4<f32>;

// The image at `uv`: the pixel under it (nearest), or the four around it weighted (bilinear, edges clamped). Both on
// the stored values (premultiplied display bytes), as a display scaler works on the signal. Bilinear reads half a pixel
// to the left: the game's display copy (crate::aa_blit, B) passes the finished screen, HUD included, through a GS
// bilinear tap at U = X, which reads texels x − 1 and x half each; as a box of one game pixel at the frame's
// resolution that is the bilinear sample half a pixel left. Down, the copy's stretch is centred (the plain sample).
fn sample_layer(uv: vec2<f32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(hud_image));
    if scaling.x < 0.5 {
        let p = clamp(vec2<i32>(floor(uv * vec2<f32>(size))), vec2<i32>(0), size - vec2<i32>(1));
        return textureLoad(hud_image, p, 0);
    }
    let q = uv * vec2<f32>(size) - vec2<f32>(1.0, 0.5);
    let p0 = vec2<i32>(floor(q));
    let f = q - floor(q);
    let hi = size - vec2<i32>(1);
    let a = textureLoad(hud_image, clamp(p0, vec2<i32>(0), hi), 0);
    let b = textureLoad(hud_image, clamp(p0 + vec2<i32>(1, 0), vec2<i32>(0), hi), 0);
    let c = textureLoad(hud_image, clamp(p0 + vec2<i32>(0, 1), vec2<i32>(0), hi), 0);
    let d = textureLoad(hud_image, clamp(p0 + vec2<i32>(1, 1), vec2<i32>(0), hi), 0);
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn composite(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // The target's own size: 512×416, wider in 16:9 (crate::display; crate::hud_render widens the layer).
    let c = sample_layer(in.uv);
    if c.a <= 0.0 {
        discard;
    }
    let straight = clamp(c.rgb / c.a, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(to_linear(straight), min(c.a, 1.0));
}

// The static layer (hud_render::HudStaticComposite, premultiplied blend): its display-space premultiplied colour in
// linear light added over what is under it, that scaled by 1 - coverage. Exact over black; the noise (coverage 0)
// adds.
@fragment
fn composite_static(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // The target's own size: 512×416, wider in 16:9 (crate::display; crate::hud_render widens the layer).
    let c = sample_layer(in.uv);
    let rgb = to_linear(clamp(c.rgb, vec3<f32>(0.0), vec3<f32>(1.0)));
    let a = clamp(c.a, 0.0, 1.0);
    if a <= 0.0 && max(rgb.r, max(rgb.g, rgb.b)) <= 0.0 {
        discard;
    }
    return vec4<f32>(rgb, a);
}
#endif

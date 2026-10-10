// Anti-aliasing Original: one axis of one of the game's full-screen bilinear copies, at the frame's resolution (Rust
// side and the addresses: aa_blit.rs).
//
// In game units along the axis (one game pixel = 1; a frame pixel is p = 384 / height across, 416 / height down), frame
// pixel i (centre t) reads the frame averaged over [s - 0.5, s + 0.5], s = (t - a) * k: the box of one game pixel a GS
// bilinear tap is (at one frame pixel per game pixel, exactly the GS's bilinear). Pixels at t >= `last` are not drawn
// (they keep their value). The frame's edge pixels are extended (CLAMP). The averaging is on the display values
// (sRGB), as the GS filters the bytes in the frame buffer.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;

struct AaCopy {
    // x = k, y = a, z = last.
    params: vec4<f32>,
    // x = 0 across, 1 down.
    axis: vec4<u32>,
}
@group(0) @binding(2) var<uniform> aa: AaCopy;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

const MAX_TAPS: i32 = 32;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(screen_texture));
    let px = vec2<i32>(floor(in.position.xy));
    let down = aa.axis.x == 1u;
    let n = select(dims.x, dims.y, down);
    let i = select(px.x, px.y, down);
    let p = select(384.0, 416.0, down) / f32(dims.y);
    let t = (f32(i) + 0.5) * p;
    if (t >= aa.params.z) { return textureLoad(screen_texture, px, 0); }
    let s = (t - aa.params.y) * aa.params.x;
    let lo = s - 0.5;
    let hi = s + 0.5;
    let j0 = clamp(i32(floor(lo / p)), 0, n - 1);
    let j1 = clamp(i32(floor(hi / p)), j0, min(n - 1, j0 + MAX_TAPS - 1));
    var acc = vec4<f32>(0.0);
    for (var j = j0; j <= j1; j += 1) {
        // Edge pixels reach past the frame (CLAMP).
        let v0 = select(f32(j) * p, lo, j == 0);
        let v1 = select(f32(j + 1) * p, hi, j == n - 1);
        let w = max(0.0, min(v1, hi) - max(v0, lo));
        if (w > 0.0) {
            let c = textureLoad(screen_texture, select(vec2<i32>(j, px.y), vec2<i32>(px.x, j), down), 0);
            acc += w * vec4<f32>(to_srgb(c.rgb), c.a);
        }
    }
    // The box is one game unit long: the weights sum to 1.
    return vec4<f32>(to_linear(acc.rgb), acc.a);
}

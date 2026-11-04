// Bloom (#523): what lights itself, blurred and added back over the frame.
//
// The glowing parts are drawn on their own (lit windows, the pages that light themselves,
// lamps, headlights, muzzle flashes and fire: `Renderer::draw_glowing`), then brought down to
// a quarter of the size while what's too dim to glow is taken off, blurred across and down
// twice, and added to the frame.

struct Params {
    // One texel of the texture read, in its UVs.
    texel: vec2<f32>,
    // The blur's way (1,0 or 0,1), how far apart its taps are.
    dir: vec2<f32>,
    // x: what's taken off before blurring (too dim to glow); y: how strongly it's added back.
    k: vec4<f32>,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(1) @binding(0) var<uniform> p: Params;

struct V {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One triangle over the whole target.
@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> V {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var o: V;
    o.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2<f32>(x, y);
    return o;
}

// To a quarter of the size: four filtered taps, each the mean of a 2x2, make the mean of the
// 4x4 under the pixel; less what's too dim to glow.
@fragment
fn fs_prefilter(in: V) -> @location(0) vec4<f32> {
    let t = p.texel;
    let c = (textureSample(src, samp, in.uv + vec2<f32>(-t.x, -t.y)).rgb
           + textureSample(src, samp, in.uv + vec2<f32>(t.x, -t.y)).rgb
           + textureSample(src, samp, in.uv + vec2<f32>(-t.x, t.y)).rgb
           + textureSample(src, samp, in.uv + vec2<f32>(t.x, t.y)).rgb) * 0.25;
    return vec4<f32>(max(c - vec3<f32>(p.k.x), vec3<f32>(0.0)), 1.0);
}

// A Gaussian along `dir`: nine texels in five filtered taps.
@fragment
fn fs_blur(in: V) -> @location(0) vec4<f32> {
    let step = p.texel * p.dir;
    var c = textureSample(src, samp, in.uv).rgb * 0.2270270;
    c = c + textureSample(src, samp, in.uv + step * 1.3846154).rgb * 0.3162162;
    c = c + textureSample(src, samp, in.uv - step * 1.3846154).rgb * 0.3162162;
    c = c + textureSample(src, samp, in.uv + step * 3.2307692).rgb * 0.0702703;
    c = c + textureSample(src, samp, in.uv - step * 3.2307692).rgb * 0.0702703;
    return vec4<f32>(c, 1.0);
}

// Added over the frame (the pipeline adds it).
@fragment
fn fs_composite(in: V) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(src, samp, in.uv).rgb * p.k.y, 1.0);
}

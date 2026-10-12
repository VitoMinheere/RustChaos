// Textured, vertex-lit geometry with alpha cut-outs and distance fog.
// Colours are multiplied in gamma space, as Direct3D did in 1999.

struct Globals {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
    // x = fog start, y = fog end (square units), z = 1 to enable
    fog: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn vs_main(@location(0) pos: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> VOut {
    var o: VOut;
    o.clip = g.view_proj * vec4<f32>(pos, 1.0);
    o.uv = uv;
    o.color = color;
    o.world = pos;
    return o;
}

// Blob shadows: the vertex colour (black, or red for blood), alpha from the round falloff
// texture times the vertex alpha, fading out with the fog.
@fragment
fn fs_shadow(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    return vec4<f32>(in.color.rgb, t.a * in.color.a * (1.0 - f));
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    // Cut-out transparency (fences, windows, foliage): no sorting needed.
    if (t.a < 0.5) {
        discard;
    }
    var c = t.rgb * in.color.rgb;
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    c = mix(c, g.fog_color.rgb, f);
    return vec4<f32>(c, 1.0);
}

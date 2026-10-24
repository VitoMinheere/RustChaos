// Textured, vertex-lit geometry with alpha cut-outs and distance fog.
// Colours are multiplied in gamma space, as Direct3D did in 1999.

struct Globals {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
    // x = fog start, y = fog end (square units), z = 1 to enable
    fog: vec4<f32>,
    // Dynamic lights (`NIGHT_dlight`): xyz where (square units), w how far it reaches; and
    // what each adds to the light. x of `light_count` is how many there are.
    lights: array<vec4<f32>, 16>,
    light_rgb: array<vec4<f32>, 16>,
    light_count: vec4<f32>,
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

// Marks on the ground (`POLY_PAGE_FOOTPRINT`, `_TYRETRACK`, `_TYRESKID`): the texture times
// the vertex colour, alpha the texture's times the vertex's (`D3DTBLEND_MODULATEALPHA`), going
// into the fog as the ground under them does.
@fragment
fn fs_decal(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    return vec4<f32>(mix(t.rgb * in.color.rgb, g.fog_color.rgb, f), t.a * in.color.a);
}

// A person's own shadow (#241, `POLY_PAGE_SHADOW`): what it falls on darkened by half the
// map's coverage (`GetShadowPixelMapping`) times the vertex alpha (its fade from them), and
// nothing off the map. Gone into the fog as the ground is.
@fragment
fn fs_smap(in: VOut) -> @location(0) vec4<f32> {
    if (in.uv.x < 0.0 || in.uv.x > 1.0 || in.uv.y < 0.0 || in.uv.y > 1.0) {
        discard;
    }
    let t = textureSample(tex, samp, in.uv);
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    return vec4<f32>(0.0, 0.0, 0.0, t.a * 0.5 * in.color.a * (1.0 - f));
}

// Where a puddle shows (#268): only the stencil is written, wherever its texture isn't
// clear (`POLY_PAGE_PUDDLE`'s alpha test, `D3DCMP_NOTEQUAL` 0).
@fragment
fn fs_mask(in: VOut) -> @location(0) vec4<f32> {
    if (textureSample(tex, samp, in.uv).a * in.color.a < 0.02) {
        discard;
    }
    return vec4<f32>(0.0);
}

// The sky behind everything and the moon added over it (`POLY_PAGE_SKY`, `_MOON`): the
// texture times the vertex colour, with no fog.
@fragment
fn fs_sky(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    return vec4<f32>(t.rgb * in.color.rgb, 1.0);
}

// What the dynamic lights add at `p`: each its colour, falling off evenly to nothing at its
// edge, by the engine's own distance (`QDIST3`: the longest side and a quarter of the others).
fn dynamic_light(p: vec3<f32>) -> vec3<f32> {
    var add = vec3<f32>(0.0);
    let n = u32(g.light_count.x);
    for (var i = 0u; i < n; i = i + 1u) {
        let l = g.lights[i];
        let d = abs(l.xyz - p);
        let big = max(d.x, max(d.y, d.z));
        let dist = big + (d.x + d.y + d.z - big) * 0.25;
        if (dist < l.w) {
            add = add + g.light_rgb[i].rgb * (1.0 - dist / l.w);
        }
    }
    return add;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    // Cut-out transparency (fences, windows, foliage): no sorting needed.
    if (t.a < 0.5) {
        discard;
    }
    var c = t.rgb * min(in.color.rgb + dynamic_light(in.world), vec3<f32>(1.0));
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    c = mix(c, g.fog_color.rgb, f);
    return vec4<f32>(c, 1.0);
}

// A ghost (#186), first pass: only its depth, so the second shows the nearest of it and not
// the limbs behind. The texture's cut-outs still cut. (The colour isn't written.)
@fragment
fn fs_ghost_depth(in: VOut) -> @location(0) vec4<f32> {
    if (textureSample(tex, samp, in.uv).a < 0.5) {
        discard;
    }
    return vec4<f32>(0.0);
}

// A ghost, second pass: the texture times the vertex colour, see-through by the vertex
// alpha, fading into the fog.
@fragment
fn fs_ghost(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    if (t.a < 0.5) {
        discard;
    }
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    return vec4<f32>(mix(t.rgb * in.color.rgb, g.fog_color.rgb, f), in.color.a * (1.0 - f));
}

// Sprites of fire (`POLY_PAGE_EXPLODE1`): the texture times the vertex colour, blended by
// its alpha, fading into the fog.
@fragment
fn fs_sprite(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv) * in.color;
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    return vec4<f32>(mix(t.rgb, g.fog_color.rgb, f), t.a);
}

// Explosions: the same round blob, added to what's behind it rather than blended over it,
// so fire and dust brighten the scene. The falloff is squared, which the soft blob of a
// shadow doesn't want but a flame does: it keeps a hot middle instead of a wide haze.
// Premultiplied, so both blend factors can be one.
// Ribbons of smoke (`POLY_PAGE_SMOKER`): the texture times the vertex colour times the vertex
// alpha (`RS_AlphaPremult`), added to what's behind, less in the fog. The page wraps
// (`D3DTADDRESS_WRAP`): the coordinates are wrapped here, sampled at the gradients of the
// unwrapped ones so there's no seam where they wrap.
@fragment
fn fs_additive(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSampleGrad(tex, samp, fract(in.uv), dpdx(in.uv), dpdy(in.uv));
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    let a = in.color.a * (1.0 - f);
    return vec4<f32>(t.rgb * in.color.rgb * a, a);
}

@fragment
fn fs_glow(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    let a = t.a * t.a * in.color.a * (1.0 - f);
    return vec4<f32>(in.color.rgb * a, a);
}

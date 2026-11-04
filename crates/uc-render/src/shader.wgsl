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
    // Relief (#474), for pages with a normal map: how much the ambient light changes with
    // the cosine to its direction (rgb) and how strong the relief is (w); the ambient
    // light's direction (`AMB_NORM`); and the static lights nearest the camera, as `lights`.
    relief: vec4<f32>,
    amb_dir: vec4<f32>,
    lamps: array<vec4<f32>, 16>,
    lamp_rgb: array<vec4<f32>, 16>,
    lamp_count: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world: vec3<f32>,
    // The way a one-sided face faces (zero for one drawn from both sides).
    @location(3) facing: vec3<f32>,
};

@vertex
fn vs_main(@location(0) pos: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>,
           @location(3) facing: vec4<f32>) -> VOut {
    var o: VOut;
    o.facing = facing.xyz;
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

// Seen from behind a one-sided face: a warehouse's outside wall from inside, or its inside
// wall from the street, which lie back to back in the same place.
fn behind(in: VOut) -> bool {
    return dot(in.facing, g.cam_pos.xyz - in.world) < 0.0;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    if (behind(in)) {
        discard;
    }
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

@group(2) @binding(0) var ntex: texture_2d<f32>;
@group(2) @binding(1) var nsamp: sampler;

// What a light at `l` (xyz, reaching w) adds at `p`, as `dynamic_light` has it: 0 to 1.
fn reach(l: vec4<f32>, p: vec3<f32>) -> f32 {
    let d = abs(l.xyz - p);
    let big = max(d.x, max(d.y, d.z));
    let dist = big + (d.x + d.y + d.z - big) * 0.25;
    return max(1.0 - dist / l.w, 0.0);
}

// A wall, floor or roof page with a normal map (#474): lit as `fs_main`, and then by the
// normal map. The original lights a face by its own normal only through the ambient light,
// `amb * (192 + 64 cos) / 256` to `AMB_NORM`; here that cosine is the normal map's at each
// pixel. The lights (static and dynamic) light by distance alone in the original; here each
// also lights the relief, brighter where the normal map turns towards it and darker where it
// turns away, by as much as it lights the face. A flat normal map changes nothing.
//
// The page's way across and down is found from how its UVs change over the screen (no
// tangents in the vertices), and the face's normal from its position's: the side the camera
// is on.
@fragment
fn fs_relief(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    let nt = textureSample(ntex, nsamp, in.uv).xyz * 2.0 - 1.0;
    let dp1 = dpdx(in.world);
    let dp2 = dpdy(in.world);
    let duv1 = dpdx(in.uv);
    let duv2 = dpdy(in.uv);
    if (behind(in) || t.a < 0.5) {
        discard;
    }
    var n = normalize(cross(dp1, dp2));
    if (dot(n, g.cam_pos.xyz - in.world) < 0.0) {
        n = -n;
    }
    // d(world)/du and d(world)/dv.
    let det = duv1.x * duv2.y - duv1.y * duv2.x;
    var p = n;
    if (abs(det) > 1e-12) {
        let tu = (dp1 * duv2.y - dp2 * duv1.y) / det;
        let tv = (dp2 * duv1.x - dp1 * duv2.x) / det;
        let tu1 = normalize(tu - n * dot(tu, n));
        let tv1 = normalize(tv - n * dot(tv, n));
        p = normalize(tu1 * nt.x + tv1 * nt.y + n * nt.z);
    }
    let k = g.relief.w;
    var light = in.color.rgb + dynamic_light(in.world);
    light = light + g.relief.rgb * (dot(p, g.amb_dir.xyz) - dot(n, g.amb_dir.xyz)) * k;
    // Each light: as much again as it adds, times how much more (or less) the relief faces it.
    for (var i = 0u; i < u32(g.lamp_count.x); i = i + 1u) {
        let l = g.lamps[i];
        let w = reach(l, in.world);
        if (w <= 0.0) {
            continue;
        }
        let to = normalize(l.xyz - in.world);
        light = light + g.lamp_rgb[i].rgb * w * (max(dot(p, to), 0.0) - max(dot(n, to), 0.0)) * k;
    }
    for (var i = 0u; i < u32(g.light_count.x); i = i + 1u) {
        let l = g.lights[i];
        let w = reach(l, in.world);
        if (w <= 0.0) {
            continue;
        }
        let to = normalize(l.xyz - in.world);
        light = light + g.light_rgb[i].rgb * w * (max(dot(p, to), 0.0) - max(dot(n, to), 0.0)) * k;
    }
    var c = t.rgb * clamp(light, vec3<f32>(0.0), vec3<f32>(1.0));
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    c = mix(c, g.fog_color.rgb, f);
    return vec4<f32>(c, 1.0);
}

// Pages that light themselves (`textype.txt`'s 'S', and the lit windows an 'I' page draws
// over itself from the next page): the texture at its own colour, whatever the light
// (`D3DTBLEND_DECAL`), its clear parts cut out (`D3DCMP_NOTEQUAL` 0), into the fog as the
// rest.
@fragment
fn fs_unlit(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    if (behind(in)) {
        discard;
    }
    if (t.a < 0.5) {
        discard;
    }
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    return vec4<f32>(mix(t.rgb, g.fog_color.rgb, f), 1.0);
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

// Bloom (#523): a self-lit page or lit window as it glows: its colour, fading to nothing in
// the fog rather than to the fog's colour.
@fragment
fn fs_emit(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    if (behind(in) || t.a < 0.5) {
        discard;
    }
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    return vec4<f32>(t.rgb * (1.0 - f), 1.0);
}

// Bloom: a sprite of fire as it glows, by its alpha, fading in the fog.
@fragment
fn fs_sprite_emit(in: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv) * in.color;
    let d = distance(in.world, g.cam_pos.xyz);
    let f = clamp((d - g.fog.x) / max(g.fog.y - g.fog.x, 0.001), 0.0, 1.0) * g.fog.z;
    return vec4<f32>(t.rgb * t.a * (1.0 - f), t.a);
}

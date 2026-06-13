#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Draws a soft, dark halo around the sprite's silhouette. Works by finding the
// distance from the current fragment to the nearest "on" sprite texel and
// mapping that distance through a linear falloff. Because the fragment
// position is continuous while the texel grid is integer, the distance varies
// smoothly across screen pixels — no 2-pixel banding from quantisation.
//
// The quad this renders on is inflated relative to the sprite quad (see
// ShadowMaterial), so `params.x` (inflate) maps the inflated uv back into
// sprite-space [0,1] where the sprite texture lives.
@group(2) @binding(0) var sprite_tex: texture_2d<f32>;
@group(2) @binding(1) var sprite_sampler: sampler;
// params: x = inflate, y = radius (in sprite pixels), z = max alpha, w = unused
@group(2) @binding(2) var<uniform> params: vec4<f32>;

fn sample_a(uv: vec2<f32>) -> f32 {
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return 0.0;
    }
    return textureSampleLevel(sprite_tex, sprite_sampler, uv, 0.0).a;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let inflate = params.x;
    let radius_px = params.y;
    let max_alpha = params.z;

    let local = (in.uv - vec2(0.5)) * inflate + vec2(0.5);
    let dims = vec2<f32>(textureDimensions(sprite_tex));

    let center_a = sample_a(local);

    // Fragment's position and "home" texel in sprite-pixel units.
    let frag_px = local * dims;
    let home_center = floor(frag_px) + vec2(0.5);

    // Walk the surrounding texel grid out to radius_px (+ a margin). For each
    // sprite-on texel found, distance from the fragment to that texel's centre
    // (in pixels) is what feeds the falloff. The minimum wins.
    let R = 3;
    var min_d = 1e6;
    for (var dy = -R; dy <= R; dy = dy + 1) {
        for (var dx = -R; dx <= R; dx = dx + 1) {
            let cand_center = home_center + vec2(f32(dx), f32(dy));
            let cand_uv = cand_center / dims;
            let a = sample_a(cand_uv);
            if a > 0.5 {
                let d = length(frag_px - cand_center);
                min_d = min(min_d, d);
            }
        }
    }

    // Linear ramp: 1 right against the silhouette, 0 radius_px pixels past
    // the silhouette's edge. min_d is measured to texel *centres*, so even
    // the closest neighbouring fragment is ~1 px away from the silhouette
    // texel — subtract that 1 so the band right next to the sprite goes
    // full intensity instead of getting clipped at ~1/radius.
    let cover = clamp(1.0 - max(min_d - 1.0, 0.0) / radius_px, 0.0, 1.0);

    // Suppress shadow inside the sprite (sharp cutoff so the darkest band sits
    // right against the silhouette instead of being soft-faded one pixel out).
    let outside = 1.0 - step(0.5, center_a);

    let shadow_a = cover * outside * max_alpha;
    if shadow_a < 0.01 {
        discard;
    }
    return vec4(0.0, 0.0, 0.0, shadow_a);
}

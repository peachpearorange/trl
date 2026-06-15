#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var sprite_tex: texture_2d<f32>;
@group(2) @binding(1) var sprite_sampler: sampler;
@group(2) @binding(2) var<uniform> outline_color: vec4<f32>;

fn safe_alpha(px: vec2<i32>, dims: vec2<i32>) -> f32 {
    if px.x < 0 || px.y < 0 || px.x >= dims.x || px.y >= dims.y {
        return 0.0;
    }
    return textureLoad(sprite_tex, px, 0).a;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(sprite_tex));
    let pad = 1;
    let padded = vec2<f32>(dims + vec2(pad * 2));
    let texel = vec2<i32>(floor(in.uv * padded)) - vec2(pad);

    let alpha = safe_alpha(texel, dims);
    if alpha > 0.1 {
        discard;
    }

    var max_a = 0.0;
    max_a = max(max_a, safe_alpha(texel + vec2(-1i, 0i), dims));
    max_a = max(max_a, safe_alpha(texel + vec2( 1i, 0i), dims));
    max_a = max(max_a, safe_alpha(texel + vec2( 0i,-1i), dims));
    max_a = max(max_a, safe_alpha(texel + vec2( 0i, 1i), dims));

    if max_a > 0.1 {
        return outline_color;
    }

    discard;
    return vec4(0.0);
}

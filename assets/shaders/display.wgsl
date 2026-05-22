#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var screen_texture: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;
@group(2) @binding(2) var entity_texture: texture_2d<f32>;
@group(2) @binding(3) var entity_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(screen_texture));
    let coord = vec2<i32>(floor(in.uv * vec2<f32>(dims)));
    let bg = textureLoad(screen_texture, coord, 0);
    let fg = textureLoad(entity_texture, coord, 0);
    let color = vec4<f32>(mix(bg.rgb, fg.rgb, fg.a), max(bg.a, fg.a));
    let screen_y = i32(floor(in.position.y));
    let scanline = 1.0 - f32((screen_y / 2) % 2) * 0.12;
    return vec4<f32>(color.rgb * scanline, color.a);
}

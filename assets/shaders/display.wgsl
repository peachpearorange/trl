#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var screen_texture: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(screen_texture));
    let coord = vec2<i32>(floor(in.uv * vec2<f32>(dims)));
    return textureLoad(screen_texture, coord, 0);
}

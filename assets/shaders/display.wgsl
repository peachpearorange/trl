#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var screen_texture: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;
@group(2) @binding(2) var entity_texture: texture_2d<f32>;
@group(2) @binding(3) var entity_sampler: sampler;
@group(2) @binding(4) var<uniform> time: f32;
@group(2) @binding(5) var<uniform> world_offset: vec2<i32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(screen_texture));
    let coord = vec2<i32>(floor(in.uv * vec2<f32>(dims)));
    let bg_raw = textureLoad(screen_texture, coord, 0);
    let is_liquid = bg_raw.a > 0.5 && bg_raw.a < 0.999;
    var bg = bg_raw;
    if is_liquid {
        let world = vec2<f32>(coord + world_offset);
        let wave_x = i32(floor(sin(world.y * 0.15 + time * 1.5) * 2.0));
        let wave_y = i32(floor(cos(world.x * 0.12 + time * 1.2) * 1.5));
        let wcoord = clamp(coord + vec2<i32>(wave_x, wave_y), vec2<i32>(0), dims - 1);
        let sampled = textureLoad(screen_texture, wcoord, 0);
        bg = vec4<f32>(sampled.rgb, bg_raw.a);
    }
    let fg = textureLoad(entity_texture, coord, 0);
    let color = vec4<f32>(mix(bg.rgb, fg.rgb, fg.a), max(bg.a, fg.a));
    let screen_y = i32(floor(in.position.y));
    let scanline = 1.0 - f32((screen_y / 2) % 2) * 0.12;
    return vec4<f32>(color.rgb * scanline, color.a);
}

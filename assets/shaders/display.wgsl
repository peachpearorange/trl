#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var screen_texture: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;
@group(2) @binding(2) var<uniform> cam_pos: vec4<f32>;

// Returns a pseudo-random rgb in [-1, 1]^3 for a given pixel coordinate.
fn noise3(pixel: vec2<f32>) -> vec3<f32> {
    let r = fract(sin(dot(pixel, vec2(127.1, 311.7))) * 43758.5453) * 2.0 - 1.0;
    let g = fract(sin(dot(pixel + vec2(31.7, 17.3), vec2(269.5, 183.3))) * 43758.5453) * 2.0 - 1.0;
    let b = fract(sin(dot(pixel + vec2(7.1, 53.1), vec2(419.2, 371.9))) * 43758.5453) * 2.0 - 1.0;
    return vec3(r, g, b);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(screen_texture, screen_sampler, in.uv);

    // Background pixels (camera cleared to alpha=0) pass through untouched.
    if color.a < 0.5 {
        return color;
    }

    // Scanlines: darken alternating pairs of pixel rows (matches 2x game pixel scale).
    if (i32(in.position.y) / 2 % 2) == 0 {
        color = color * 0.88;
    }

    // World-space noise — sticks to sprites as the camera moves.
    // Camera2d: 1 world unit = 1 screen pixel; each game texel = 2 screen pixels.
    let dims = vec2<f32>(textureDimensions(screen_texture));
    let offset = in.position.xy - dims * 0.5;
    // Y is flipped: screen Y-down vs world Y-up.
    let world_pos = cam_pos.xy + vec2(offset.x, -offset.y);
    // Quantise to game-texel grid (2×2 screen pixels per texel).
    let noise = noise3(floor(world_pos / 2.0));
    color = vec4(
        clamp(color.r + noise.r * 0.02, 0.0, 1.0),
        clamp(color.g + noise.g * 0.02, 0.0, 1.0),
        clamp(color.b + noise.b * 0.02, 0.0, 1.0),
        color.a,
    );

    return color;
}

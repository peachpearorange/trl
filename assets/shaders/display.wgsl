#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var screen_texture: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;
@group(2) @binding(2) var entity_texture: texture_2d<f32>;
@group(2) @binding(3) var entity_sampler: sampler;
@group(2) @binding(4) var<uniform> time: f32;
@group(2) @binding(5) var<uniform> world_offset: vec2<i32>;
@group(2) @binding(6) var<uniform> player_screen_pos: vec2<f32>;
@group(2) @binding(7) var fov_tex: texture_2d<f32>;
@group(2) @binding(8) var fov_sampler: sampler;
@group(2) @binding(9) var<uniform> map_dims: vec2<f32>;
@group(2) @binding(10) var<uniform> scale: f32;

// World units per tile. Mirrors `TILE_SIZE` in main.rs (SPRITE_TEXELS * SCREEN_PIXELS_PER_TEXEL).
const TILE_SIZE: f32 = 40.0;

// FOV brightness for a screen pixel: map physical pixel -> world units (inverse of the camera
// projection captured by world_offset/scale) -> tile, and read the shared per-tile lightmap.
// Returns 1.0 (no dim) while the lightmap is the 1x1 placeholder before the first level loads.
fn fov_brightness(coord: vec2<i32>) -> f32 {
    let dims = vec2<f32>(textureDimensions(fov_tex));
    if dims.x < 2.0 {
        return 1.0;
    }
    let px = vec2<f32>(f32(coord.x) + 0.5, f32(coord.y) + 0.5);
    let world = vec2<f32>(px.x + f32(world_offset.x),
                          -(px.y + f32(world_offset.y))) / scale;
    // Integer tile coords sit on tile centres (see tile_screen_pos), so round to the nearest
    // tile rather than floor — flooring would shift the sampled brightness half a tile.
    let tile = vec2<i32>(round(vec2<f32>(world.x / TILE_SIZE + map_dims.x * 0.5,
                                         map_dims.y * 0.5 - world.y / TILE_SIZE)));
    if tile.x < 0 || tile.y < 0 || tile.x >= i32(map_dims.x) || tile.y >= i32(map_dims.y) {
        return 0.0;
    }
    return textureLoad(fov_tex, tile, 0).r;
}

fn sample_composited(coord: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
    let c = clamp(coord, vec2<i32>(0), dims - 1);
    let bg_raw = textureLoad(screen_texture, c, 0);
    let fg = textureLoad(entity_texture, c, 0);
    return vec4<f32>(mix(bg_raw.rgb, fg.rgb, fg.a), max(bg_raw.a, fg.a));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(screen_texture));
    let coord = vec2<i32>(floor(in.uv * vec2<f32>(dims)));
    let bg_raw = textureLoad(screen_texture, coord, 0);
    let is_liquid = bg_raw.a > 0.5 && bg_raw.a < 0.999;
    var bg = bg_raw;
    if is_liquid {
        let world = vec2<f32>(coord + world_offset);
        let wave_x = i32(floor(cos(world.y * 0.12 + time * 1.2) * 1.5));
        let wave_y = i32(floor(sin(world.x * 0.15 + time * 1.5) * 2.0));
        let wcoord = clamp(coord + vec2<i32>(wave_x, wave_y), vec2<i32>(0), dims - 1);
        let sampled = textureLoad(screen_texture, wcoord, 0);
        bg = vec4<f32>(sampled.rgb, bg_raw.a);
    }
    let fg = textureLoad(entity_texture, coord, 0);
    let color = vec4<f32>(mix(bg.rgb, fg.rgb, fg.a), max(bg.a, fg.a));

    // chromatic aberration scaling with distance from player
    let fcoord = vec2<f32>(coord);
    let delta = fcoord - player_screen_pos;
    let dist = length(delta);
    let dir = select(vec2<f32>(0.0), normalize(delta), dist > 1.0);
    let aberration = dist * 0.0012;
    let offset = vec2<i32>(round(dir * aberration));
    let r = sample_composited(coord + offset, dims).r;
    let b = sample_composited(coord - offset, dims).b;
    let aberrated = vec4<f32>(r, color.g, b, color.a);

    // Single FOV overlay over the fully composited scene: fade both tiles and entities by the
    // visibility of the tile under this pixel, hiding anything standing on out-of-view tiles.
    let dimmed = aberrated.rgb * fov_brightness(coord);

    let screen_y = i32(floor(in.position.y));
    let scanline = f32((screen_y / 2) % 2) * 0.018;
    return vec4<f32>(mix(dimmed, vec3<f32>(0.72, 1.0, 0.74), scanline), aberrated.a);
}

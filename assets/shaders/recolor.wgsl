#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var sprite_tex: texture_2d<f32>;
@group(2) @binding(1) var sprite_sampler: sampler;
@group(2) @binding(2) var<uniform> primary: vec4<f32>;
@group(2) @binding(3) var<uniform> secondary: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(sprite_tex, sprite_sampler, in.uv);
    if sample.a < 0.01 {
        discard;
    }
    let lum = dot(sample.rgb, vec3(0.299, 0.587, 0.114));
    let color = mix(primary.rgb, secondary.rgb, lum);
    return vec4(color, sample.a);
}

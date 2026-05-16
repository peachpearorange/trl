#import bevy_ui::ui_vertex_output::UiVertexOutput

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // in.position.y is the fragment's screen-space y coordinate in pixels.
    // Alternate rows are darkened to produce a CRT scanline look.
    let is_dark = (i32(in.position.y) % 2) == 0;
    let alpha = select(0.0, 0.22, is_dark);
    return vec4<f32>(0.0, 0.0, 0.0, alpha);
}

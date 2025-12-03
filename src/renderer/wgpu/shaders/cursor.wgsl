@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // cursor color (RGBA linear)
    return vec4<f32>(0.95, 0.95, 0.95, 0.7);
}

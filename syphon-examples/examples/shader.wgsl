// Simple test pattern shader

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full screen triangle
    let x = f32(vertex_index % 2u) * 4.0 - 1.0; // 0, 1, 0 -> -1, 3, -1
    let y = f32(vertex_index / 2u) * 4.0 - 1.0; // 0, 0, 1 -> -1, -1, 3
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Return color from clear value
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

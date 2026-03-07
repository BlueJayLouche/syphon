// BGRA to RGBA conversion compute shader
// Processes pixels in parallel using GPU compute

struct Uniforms {
    width: u32,
    height: u32,
    stride: u32,
    _padding: u32,
};

@group(0) @binding(0)
var<storage, read> input_buffer: array<u32>;

@group(0) @binding(1)
var<storage, read_write> output_buffer: array<u32>;

@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    // Bounds check
    if (x >= uniforms.width || y >= uniforms.height) {
        return;
    }

    // Calculate indices
    // Input uses stride (may include padding), output is tightly packed
    let src_pixel_idx = y * uniforms.stride / 4u + x;
    let dst_pixel_idx = y * uniforms.width + x;

    // Bounds check for buffer access
    if (src_pixel_idx >= arrayLength(&input_buffer) ||
        dst_pixel_idx >= arrayLength(&output_buffer)) {
        return;
    }

    // Load BGRA pixel as single u32
    // Layout in memory: [B][G][R][A] -> 0xAARRGGBB in little-endian
    let bgra = input_buffer[src_pixel_idx];

    // Extract components using bit shifts
    let b = (bgra >> 0u) & 0xFFu;
    let g = (bgra >> 8u) & 0xFFu;
    let r = (bgra >> 16u) & 0xFFu;
    let a = (bgra >> 24u) & 0xFFu;

    // Repack as RGBA: [R][G][B][A] -> 0xAABBGGRR in little-endian
    let rgba = (r << 0u) | (g << 8u) | (b << 16u) | (a << 24u);

    output_buffer[dst_pixel_idx] = rgba;
}

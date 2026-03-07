# Syphon wgpu Examples

This directory contains examples demonstrating the wgpu-based Syphon integration with GPU-accelerated BGRA to RGBA conversion.

## Available Examples

### `simple_test.rs` - Basic Output Test

Renders an animated colored texture and publishes it to Syphon.

```bash
cargo run --example simple_test --package syphon-wgpu
```

**Features:**
- Creates a wgpu device and queue
- Renders an animated gradient
- Publishes frames to Syphon using zero-copy IOSurface sharing
- Prints client count and FPS stats

### `input_test.rs` - Standard GPU-Accelerated Input

Connects to a Syphon server and receives frames using GPU-accelerated BGRA to RGBA conversion.

```bash
# First, run a Syphon server (e.g., the simple_test example)
cargo run --example simple_test --package syphon-wgpu

# Then in another terminal, run the input test
cargo run --example input_test --package syphon-wgpu
```

**Features:**
- Connects to a Syphon server
- Receives frames as wgpu textures
- Uses GPU compute shader for BGRA→RGBA conversion (no CPU overhead)
- Prints received frame stats

### `compare_input.rs` - Performance Comparison

Compares the standard and fast input implementations side-by-side.

```bash
cargo run --example compare_input --package syphon-wgpu --release
```

**Features:**
- Tests both `SyphonWgpuInput` and `SyphonWgpuInputFast`
- Measures FPS and frame processing time
- Shows performance improvement percentage

## How It Works

### Output (SyphonWgpuOutput)

The output example uses `SyphonWgpuOutput` to publish wgpu-rendered frames:

1. Create a wgpu texture with `Bgra8Unorm` format
2. Render your content to this texture
3. Call `syphon_output.publish(&texture, &device, &queue)`
4. The crate handles Y-flip and IOSurface sharing automatically

### Input (SyphonWgpuInput)

The input example uses `SyphonWgpuInput` to receive frames:

1. Create a `SyphonWgpuInput` with your wgpu device/queue
2. Connect to a server: `input.connect("Server Name")`
3. Each frame, call `input.receive_texture(&device, &queue)`
4. Returns `Option<wgpu::Texture>` in RGBA8Unorm format ready for use

### Fast Input (SyphonWgpuInputFast)

For better performance, use `SyphonWgpuInputFast` which includes:

- **Buffer pooling**: Reuses GPU buffers across frames (eliminates allocation overhead)
- **Direct texture compute**: Shader writes directly to texture (no buffer→texture copy)
- **Integrated Y-flip**: Combined with BGRA→RGBA in one compute pass
- **Apple Silicon optimized**: Uses 8x8 workgroups for better GPU occupancy

```rust
use syphon_wgpu::SyphonWgpuInputFast;

let mut input = SyphonWgpuInputFast::new(&device, &queue);
input.connect("Server Name").unwrap();

if let Some(texture) = input.receive_texture(&device, &queue) {
    // Use texture in your render pipeline
}
```

## Performance Comparison

| Implementation | Buffer Allocations | Copies | Y-Flip | Best For |
|----------------|-------------------|--------|--------|----------|
| `SyphonWgpuInput` | Per-frame | Buffer→Texture | Separate pass | General use |
| `SyphonWgpuInputFast` | Pooled | Direct to texture | Combined in compute | High FPS |

### Expected Performance

On Apple Silicon (M1/M2/M3):
- **Standard**: ~120-180 FPS at 1080p
- **Fast**: ~180-240 FPS at 1080p (20-30% improvement)

## Implementation Details

### BGRA→RGBA Conversion

Both implementations use GPU compute shaders for format conversion:

```wgsl
// Packed pixel processing (u32 per pixel)
let bgra = input_buffer[src_idx];
let r = (bgra >> 16u) & 0xFFu;
let g = (bgra >> 8u) & 0xFFu;
let b = (bgra >> 0u) & 0xFFu;
let a = (bgra >> 24u) & 0xFFu;
```

This is significantly faster than byte-by-byte processing on the CPU.

### Zero-Copy IOSurface Sharing

For output, the crate uses IOSurface-backed Metal textures that are shared
directly between wgpu and Syphon without CPU memory copies.

For input, the current implementation requires a CPU copy from IOSurface
(`frame.to_vec()`) due to Syphon API limitations. Future optimizations
may use direct IOSurface→Metal texture creation to eliminate this copy.

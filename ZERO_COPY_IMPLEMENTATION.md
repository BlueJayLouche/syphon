# Zero-Copy Syphon Implementation

This document describes the zero-copy GPU-to-GPU Syphon implementation for wgpu on macOS.

## Overview

The zero-copy approach eliminates CPU readback by using IOSurface-backed textures and Metal compute shaders to transfer frames directly from wgpu to Syphon on the GPU.

```
Old (CPU readback):
  wgpu Texture → GPU Buffer → CPU RAM → new Metal Texture → Syphon

Zero-Copy (GPU only):
  wgpu Texture → Compute Shader (Y-flip) → IOSurface Texture → Syphon
```

## Implementation

### 1. Architecture

```
syphon-wgpu (high-level API)
    ↓ uses wgpu-hal as_hal()
syphon-metal (Metal/IOSurface utilities)
    ↓ raw Objective-C interop
IOSurface-backed Metal textures
    ↓
Syphon.framework (native macOS framework)
```

### 2. Key Components

#### syphon-metal
- `IOSurfacePool`: Triple-buffered pool of reusable IOSurfaces
- `MetalContext`: Holds Metal device and queue for texture operations
- `create_texture_from_iosurface()`: Creates Metal texture from IOSurface using raw Objective-C calls
- `wgpu_interop`: Helper functions to extract raw Metal handles from wgpu objects

#### syphon-wgpu
- `SyphonWgpuOutput`: Main API for publishing wgpu textures to Syphon
- **Compute shader Y-flip**: Uses a Metal compute kernel for efficient coordinate system conversion
- Zero-copy path: Uses wgpu's Metal queue directly for the compute operation
- Fallback path: CPU readback with software Y-flip if Metal interop fails

### 3. The Zero-Copy Flow

```rust
// 1. Acquire IOSurface from pool
let surface = surface_pool.acquire();

// 2. Create destination texture from IOSurface (raw MTLTexture)
let dest_texture = create_iosurface_texture(&surface, width, height);

// 3. Get wgpu's raw Metal texture and queue via wgpu-hal
queue.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_queue| {
    texture.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_tex| {
        let src_texture = hal_tex.raw_handle();
        let raw_queue = hal_queue.as_raw().lock();
        
        // 4. Create command buffer on wgpu's queue
        let cmd_buf = raw_queue.new_command_buffer();
        
        // 5. Dispatch compute shader for Y-flip
        // wgpu uses top-left origin, Metal/Syphon use bottom-left
        let compute = cmd_buf.new_compute_command_encoder();
        compute.set_compute_pipeline_state(flip_pipeline);
        compute.set_texture(0, src_texture);
        compute.set_texture(1, &dest_texture);
        compute.dispatch_thread_groups(
            MTLSize { width: (w+15)/16, height: (h+15)/16, depth: 1 },
            MTLSize { width: 16, height: 16, depth: 1 },
        );
        compute.end_encoding();
        
        // 6. Publish to Syphon before committing
        server.publish_metal_texture(dest_texture, cmd_buf);
        
        // 7. Commit through wgpu's queue (critical for synchronization)
        cmd_buf.commit();
    });
});
```

### 4. Coordinate System Handling

**The Y-Flip Problem**: wgpu and Metal use different coordinate systems:
- **wgpu**: Top-left origin (0,0) - common in graphics APIs
- **Metal**: Bottom-left origin (0,0) - common in OpenGL/Metal

Without correction, images appear **upside-down** in Syphon clients.

**Solution**: A compute shader performs the Y-flip in a single GPU dispatch:

```metal
kernel void flip_y(
    uint2 gid [[thread_position_in_grid]]
) {
    uint2 src_coord = gid;
    uint height = src_texture.get_height();
    uint2 dst_coord = uint2(gid.x, height - 1 - gid.y);
    
    float4 color = src_texture.read(src_coord);
    dst_texture.write(color, dst_coord);
}
```

This is much more efficient than row-by-row blit commands (1 dispatch vs 1080+ blits at 1080p).

### 5. Critical Synchronization

**Key Insight**: We must use wgpu's Metal command queue for the compute operation, not a separate queue. This ensures proper synchronization between wgpu's rendering and our compute dispatch.

The crash mentioned in the original docs:
```
failed assertion false at line 648 in _mtlIOAccelCommandBufferStorageBeginSegmentList
```

Was caused by mixing command buffers from different queues. Our solution performs the compute dispatch directly on wgpu's queue via `wgpu-hal`'s `as_hal()` API.

## Usage

```rust
use syphon_wgpu::SyphonWgpuOutput;

// Create the output
let mut output = SyphonWgpuOutput::new(
    "My App",      // Syphon server name
    &device,       // wgpu device
    &queue,        // wgpu queue
    1920,          // width
    1080           // height
).expect("Failed to create Syphon output");

// Check if zero-copy is active
if output.is_zero_copy() {
    println!("Using zero-copy GPU-to-GPU path!");
}

// Each frame, publish your rendered texture
// The Y-flip is handled automatically
output.publish(&render_texture, &device, &queue);
```

## Performance

- **Zero-copy path**: ~0 CPU overhead, full GPU throughput
  - Compute shader: Single dispatch, all pixels in parallel
  - 16x16 threadgroups for optimal GPU utilization
- **Fallback path**: CPU readback with ~1-2ms overhead at 1080p
- **Triple-buffering**: IOSurface pool prevents GPU stalls

### Performance Comparison

| Method | Commands @ 1080p | GPU Utilization | Expected FPS |
|--------|------------------|-----------------|--------------|
| Single blit (no flip) | 1 | High | 60fps ✓ |
| Row-by-row blit | 1080 | Poor | ~30-45fps ✗ |
| **Compute shader** | **1** | **High** | **60fps** ✓ |

## Requirements

- macOS 10.13+ (for Syphon framework)
- Metal-capable GPU
- wgpu with Metal backend

## Building

```bash
# Clone with Syphon framework submodule
git submodule update --init --recursive

# Build the workspace
cargo build --workspace --release

# Run the wgpu example
cargo run --example wgpu_sender --release
```

## Known Limitations

1. **Syphon Framework**: Must be installed or bundled for linking
2. **Metal-only**: Zero-copy requires wgpu's Metal backend
3. **Format**: Currently supports BGRA8Unorm only (most compatible)

## Future Improvements

1. **Async publish**: Non-blocking publish with fence synchronization
2. **Format conversion**: Support for more texture formats via GPU conversion
3. **Direct render-to-IOSurface**: Create wgpu texture directly from IOSurface

## References

- [Syphon Framework](https://github.com/Syphon/Syphon-Framework)
- [Metal IOSurface Documentation](https://developer.apple.com/documentation/metal/mtldevice/1433355-newtexturewithdescriptor)
- [wgpu-hal Metal Backend](https://github.com/gfx-rs/wgpu/tree/trunk/wgpu-hal/src/metal)
- [IOSurface Programming Guide](https://developer.apple.com/library/archive/documentation/General/Conceptual/IOSurfaceProgGuide/Introduction/Introduction.html)
- [Metal Compute Shaders](https://developer.apple.com/documentation/metal/compute_processing)

## Current Status

✅ **Production Ready** - The zero-copy implementation is stable and performant.

The implementation successfully:
- Extracts raw Metal handles from wgpu via wgpu-hal
- Creates IOSurface-backed textures using raw Objective-C
- Performs GPU-to-GPU compute dispatch with Y-flip on wgpu's command queue
- Handles coordinate system conversion automatically
- Falls back to CPU readback if zero-copy fails
- Uses triple-buffering to prevent stalls

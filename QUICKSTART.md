# Quick Start Guide

Get Syphon running in 5 minutes.

## 1. Install Framework

```bash
# Clone with submodules (includes Syphon framework)
git clone --recursive https://github.com/yourusername/syphon.git
cd syphon

# Or if already cloned
git submodule update --init --recursive
```

## 2. Add to Your Project

**Cargo.toml:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
syphon-wgpu = { path = "../crates/syphon/syphon-wgpu" }
```

**build.rs:**
```rust
fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-search=framework=../crates/syphon/syphon-lib/Syphon-Framework");
        println!("cargo:rustc-link-arg=-Wl,-rpath,../crates/syphon/syphon-lib/Syphon-Framework");
    }
}
```

## 3. Send Video from wgpu (Server)

```rust
use syphon_wgpu::SyphonWgpuOutput;

// Create the output (BGRA8Unorm format)
let mut output = SyphonWgpuOutput::new(
    "My App",      // Server name visible to clients
    &wgpu_device,  // Your wgpu device
    &wgpu_queue,   // Your wgpu queue
    1920,          // Width
    1080           // Height
).expect("Failed to create Syphon output");

// Each frame, publish your rendered Bgra8Unorm texture
output.publish(&render_texture, &wgpu_device, &wgpu_queue);
```

## 4. Receive Video (Client)

### Zero-Copy with Metal (Recommended)

```rust
use syphon_core::SyphonClient;
use syphon_metal::MetalContext;

// Create a Metal context
let metal_ctx = MetalContext::system_default()
    .expect("Metal not available");

// Connect to a Syphon server
let client = SyphonClient::connect("My App")
    .expect("Failed to connect");

// Receive frames
loop {
    if let Ok(Some(mut frame)) = client.try_receive() {
        // ZERO-COPY: Create Metal texture directly from IOSurface
        let surface = frame.iosurface();
        let texture = metal_ctx.create_texture_from_iosurface(
            surface,
            frame.width,
            frame.height
        ).expect("Failed to create texture");
        
        // Use texture in your Metal render pipeline
        // Format is BGRA8Unorm
        
        // Texture and IOSurface are released when dropped
    }
}
```

### With wgpu

```rust
use syphon_wgpu::SyphonWgpuInput;

// Create input handler
let mut input = SyphonWgpuInput::new(&device, &queue);

// Connect to a server
input.connect("My App").unwrap();

// Receive frames as wgpu textures (BGRA8Unorm format)
if let Some(texture) = input.receive_texture(&device, &queue) {
    // Use texture in your wgpu render pipeline
}
```

## 5. Test It

```bash
# Build the workspace
cargo build --workspace --release

# Run the wgpu sender example
cargo run --example wgpu_sender --release

# In another terminal, run the Metal client
cargo run --example metal_client --release -- "WGPU Zero-Copy Test"
```

You should see the client receiving frames from the wgpu sender!

## Format

This crate uses **native macOS BGRA8Unorm format** throughout:

- **Output**: Render to `Bgra8Unorm` textures and publish directly
- **Input**: Received textures are `Bgra8Unorm` (no conversion)

This eliminates all format conversion overhead.

## Next Steps

- [Full README](README.md) - Complete documentation
- [Examples](syphon-examples/examples/) - Example code
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Fix common issues

## One-Liners

**Check if Syphon works:**
```bash
cd ../crates/syphon && cargo run --example wgpu_sender --package syphon-examples
```

**Check available servers:**
```rust
let servers = syphon_wgpu::list_servers();
println!("Found {} servers", servers.len());
```

**Get GPU info:**
```rust
use syphon_core::available_devices;
for gpu in available_devices() {
    println!("{}", gpu.name);
}
```

## Common Errors

| Error | Solution |
|-------|----------|
| `FrameworkNotFound` | Run `git submodule update --init --recursive` |
| `ServerNotFound` | Wait 2s for server to announce |
| `segmentation fault` | Wrap thread in `autoreleasepool` |

## Need Help?

1. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. Review [examples/](syphon-examples/examples/)
3. File an issue with error output

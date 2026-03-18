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
        println!("cargo:rustc-link-search=framework=../crates/syphon/syphon-lib");
        println!("cargo:rustc-link-arg=-Wl,-rpath,../crates/syphon/syphon-lib");
    }
}
```

## 3. Send Video from wgpu (Server)

```rust
use syphon_wgpu::{SyphonWgpuOutput, PublishStatus};

// Create the output (Bgra8Unorm format)
let mut output = SyphonWgpuOutput::new(
    "My App",      // Server name visible to clients
    &wgpu_device,  // Your wgpu device
    &wgpu_queue,   // Your wgpu queue
    1920,          // Width
    1080           // Height
).expect("Failed to create Syphon output");

// Each frame, publish your rendered Bgra8Unorm texture.
// publish() returns a status so you can detect fallbacks.
match output.publish(&render_texture, &wgpu_device, &wgpu_queue) {
    PublishStatus::ZeroCopy    => {}   // GPU-to-GPU, ~0% CPU
    PublishStatus::CpuFallback => log::warn!("CPU fallback — check Metal setup"),
    PublishStatus::NoClients   => {}   // no receivers yet
    PublishStatus::PoolExhausted => log::warn!("increase pool_size"),
}
```

## 4. Receive Video (Client)

### Push-Based with wgpu (Recommended — No Polling)

```rust
use syphon_wgpu::SyphonWgpuInput;
use std::thread;

let mut input = SyphonWgpuInput::new(&device, &queue);

// connect_with_channel() returns a Receiver<()> that fires on every new frame
let rx = input.connect_with_channel("My App")?;

thread::spawn(move || {
    while rx.recv().is_ok() {
        if let Some(texture) = input.receive_texture(&device, &queue) {
            // texture is Bgra8Unorm — zero CPU copies on Metal
        }
    }
});
```

### Zero-Copy with Metal (Fastest — Direct IOSurface Alias)

```rust
use syphon_core::SyphonClient;
use syphon_metal::MetalContext;

let metal_ctx = MetalContext::system_default().expect("Metal not available");
let client = SyphonClient::connect("My App").expect("Failed to connect");

loop {
    if let Ok(Some(frame)) = client.try_receive() {
        // ZERO-COPY: Metal texture aliasing the same GPU memory as the IOSurface
        let texture = metal_ctx.create_texture_from_iosurface(
            frame.iosurface(), frame.width, frame.height
        ).expect("Failed to create texture");
        // Format is BGRA8Unorm
    }
}
```

### Unambiguous Connection by UUID

If multiple servers might share a name, use `connect_by_info()`:

```rust
use syphon_core::SyphonServerDirectory;

let servers = SyphonServerDirectory::servers(); // fast — no 1.5s sleep
let info = servers.iter().find(|s| s.app_name == "My App").unwrap();
let client = SyphonClient::connect_by_info(info)?; // matched by UUID
```

## 5. Test It

```bash
# Build the workspace
cargo build --workspace --release

# Terminal 1 — start the wgpu sender
cargo run --example wgpu_sender --release

# Terminal 2 — connect the Metal zero-copy client
cargo run --example metal_client --release -- "WGPU Zero-Copy Test"
```

## Format

This crate uses **native macOS BGRA8Unorm format** throughout:

- **Output**: Render to `Bgra8Unorm` textures and publish directly
- **Input**: Received textures are `Bgra8Unorm` (no conversion)

## Next Steps

- [Full README](README.md) — Complete documentation
- [CHANGES.md](CHANGES.md) — What's new
- [Examples](syphon-examples/examples/) — Example code
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) — Fix common issues

## Common Errors

| Error | Solution |
|-------|----------|
| `FrameworkNotFound` | Run `git submodule update --init --recursive` |
| `ServerNotFound` | No server with that name is running |
| `AmbiguousServerName` | Use `connect_by_info()` with a UUID |
| `PublishStatus::CpuFallback` | Ensure you're on the Metal backend with `Bgra8Unorm` texture |

## Need Help?

1. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. Review [examples/](syphon-examples/examples/)
3. Enable logging: `RUST_LOG=info cargo run`

# Syphon Crate Documentation Index

Welcome! Here's how to navigate the documentation.

## 🚀 Getting Started

| Document | Purpose | Read If... |
|----------|---------|------------|
| [QUICKSTART.md](QUICKSTART.md) | Get running in 5 minutes | You want to try it now |
| [README.md](README.md) | Complete overview with examples | You want full documentation |
| [ZERO_COPY_IMPLEMENTATION.md](ZERO_COPY_IMPLEMENTATION.md) | Technical implementation details | You're integrating zero-copy |

## 📚 Core Documentation

### Using the Crate

| Document | Purpose | Read If... |
|----------|---------|------------|
| [README.md](README.md) | API reference, examples, patterns | You're using this crate |
| [ZERO_COPY_IMPLEMENTATION.md](ZERO_COPY_IMPLEMENTATION.md) | Deep dive on zero-copy architecture | You need to understand the internals |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Fix common issues | Something isn't working |

### Project Information

| Document | Purpose | Read If... |
|----------|---------|------------|
| [CHANGES.md](CHANGES.md) | Version history & changelog | You want to know what changed |
| [DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md) | This file | You're lost in the docs |

## 🏗️ Project Structure

```
crates/syphon/
├── README.md                  # Main documentation
├── QUICKSTART.md              # 5-minute quick start
├── ZERO_COPY_IMPLEMENTATION.md # Technical deep-dive
├── TROUBLESHOOTING.md         # Problem solving
├── CHANGES.md                 # Version history
├── DOCUMENTATION_INDEX.md     # This file
│
├── syphon-core/               # Core bindings
│   └── src/
│       ├── lib.rs             # Re-exports
│       ├── server.rs          # SyphonServer
│       ├── client.rs          # SyphonClient
│       ├── directory.rs       # Server discovery
│       ├── metal_device.rs    # GPU utilities
│       └── iosurface_ext.rs   # IOSurface helpers
│
├── syphon-wgpu/               # wgpu integration
│   └── src/
│       ├── lib.rs             # SyphonWgpuOutput
│       └── input.rs           # SyphonWgpuInput
│
├── syphon-metal/              # Metal utilities
│   └── src/
│       └── lib.rs             # MetalContext, IOSurfacePool
│
└── syphon-examples/           # Example code
    └── examples/
        ├── simple_client.rs   # Basic client (CPU readback)
        ├── metal_client.rs    # Zero-copy Metal client ⭐
        └── wgpu_sender.rs     # wgpu integration (server)
```

## 🎯 Use Cases

### "I want to SEND video from my Rust app"

| Approach | Documentation | Example |
|----------|--------------|---------|
| From wgpu (zero-copy) | [README.md](README.md) "Server: Publishing from wgpu" | `examples/wgpu_sender.rs` |
| From Metal (native) | [ZERO_COPY_IMPLEMENTATION.md](ZERO_COPY_IMPLEMENTATION.md) | Create Metal texture, publish |

### "I want to RECEIVE video in my Rust app"

| Approach | Documentation | Example | Performance |
|----------|--------------|---------|-------------|
| Direct Metal (zero-copy) ⭐ | [ZERO_COPY_IMPLEMENTATION.md](ZERO_COPY_IMPLEMENTATION.md) "Client: Receiving" | `examples/metal_client.rs` | **~0% CPU, ~1ms latency** |
| Via wgpu | [README.md](README.md) "Client: Receiving to wgpu" | `examples/wgpu_sender.rs` (client section) | ~5-10% CPU, ~5ms latency |
| Simple/CPU readback | [syphon-core/src/client.rs](syphon-core/src/client.rs) | `examples/simple_client.rs` | ~5-10% CPU, ~5ms latency |

### "I want ZERO-COPY integration"

Read these in order:
1. [ZERO_COPY_IMPLEMENTATION.md](ZERO_COPY_IMPLEMENTATION.md) - Complete technical guide
2. [examples/metal_client.rs](syphon-examples/examples/metal_client.rs) - Working zero-copy client
3. [syphon-metal/src/lib.rs](syphon-metal/src/lib.rs) - Metal interop utilities

Key APIs:
- **Server**: `syphon_wgpu::SyphonWgpuOutput` or `syphon_metal::MetalContext`
- **Client**: `syphon_metal::MetalContext::create_texture_from_iosurface()`

### "Something is crashing"

1. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. Verify framework installation
3. Check for missing `autoreleasepool`

## 📖 By Topic

### Server (Publishing)

| Method | Example | API |
|--------|---------|-----|
| From wgpu | [examples/wgpu_sender.rs](syphon-examples/examples/wgpu_sender.rs) | `syphon_wgpu::SyphonWgpuOutput` |
| From Metal | [ZERO_COPY_IMPLEMENTATION.md](ZERO_COPY_IMPLEMENTATION.md) | `syphon_core::SyphonServer` |

### Client (Receiving)

| Method | Example | API | Zero-Copy? |
|--------|---------|-----|------------|
| Direct Metal ⭐ | [examples/metal_client.rs](syphon-examples/examples/metal_client.rs) | `syphon_metal::MetalContext` | **Yes** |
| To wgpu | [README.md](README.md) | `syphon_wgpu::SyphonWgpuInput` | No (CPU readback) |
| Simple | [examples/simple_client.rs](syphon-examples/examples/simple_client.rs) | `syphon_core::SyphonClient` | No (CPU readback) |

### Discovery

- **Basic**: [examples/simple_client.rs](syphon-examples/examples/simple_client.rs) (first part)
- **API**: [syphon-core/src/directory.rs](syphon-core/src/directory.rs)

### GPU/Device Selection

- **API**: [syphon-core/src/metal_device.rs](syphon-core/src/metal_device.rs)
- **Examples**: [README.md](README.md) "Checking GPU Compatibility"

## 🔗 External Resources

- [Syphon Framework](https://github.com/Syphon/Syphon-Framework) - Official framework
- [Syphon Website](http://syphon.v002.info/) - Project website

## 💡 Quick Tips

### Always wrap background threads in autoreleasepool:

```rust
use objc::rc::autoreleasepool;

thread::spawn(move || {
    autoreleasepool(|| {
        // Syphon code here
    });
});
```

### Use local framework, not /Library/Frameworks:

```bash
cp -R ~/Downloads/Syphon.framework ../crates/syphon/syphon-lib/
```

### Check framework is working:

```rust
if !syphon_core::is_available() {
    panic!("Syphon not available!");
}
```

### For zero-copy client, use MetalContext:

```rust
let metal_ctx = syphon_metal::MetalContext::system_default()?;
let client = syphon_core::SyphonClient::connect("Server")?;

if let Ok(Some(frame)) = client.try_receive() {
    let texture = metal_ctx.create_texture_from_iosurface(
        frame.iosurface(),
        frame.width,
        frame.height
    )?;
    // Use texture directly - NO CPU COPIES!
}
```

## 🐛 Reporting Issues

Before filing an issue:

1. ✅ Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. ✅ Test with `metal_client` example
3. ✅ Enable debug logging: `RUST_LOG=debug cargo run`
4. ✅ Include:
   - macOS version
   - GPU model
   - Full error output
   - Minimal reproduction code

## 📝 License

MIT License - See individual crate directories for LICENSE files.

# Syphon Crate Documentation Index

Welcome! Here's how to navigate the documentation.

## 🚀 Getting Started

| Document | Purpose | Read If... |
|----------|---------|------------|
| [QUICKSTART.md](QUICKSTART.md) | Get running in 5 minutes | You want to try it now |
| [README.md](README.md) | Complete overview | You want full documentation |

## 📚 Core Documentation

### Using the Crate

| Document | Purpose | Read If... |
|----------|---------|------------|
| [README.md](README.md) | API reference, examples, patterns | You're using this crate |
| [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) | Upgrading from 0.1.0 | You used the old version |
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
├── MIGRATION_GUIDE.md         # Upgrading guide
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
│       └── metal_device.rs    # GPU utilities
│
├── syphon-wgpu/               # wgpu integration
│   └── src/
│       └── lib.rs             # SyphonWgpuOutput
│
├── syphon-metal/              # Metal utilities
│   └── src/
│       └── lib.rs             # IOSurfacePool
│
└── syphon-examples/           # Example code
    └── examples/
        ├── simple_server.rs   # Basic server
        ├── simple_client.rs   # Basic client
        ├── wgpu_sender.rs     # wgpu integration
        └── metal_sender.rs    # Metal integration
```

## 🎯 Use Cases

### "I want to send video from my Rust app"

1. Read [QUICKSTART.md](QUICKSTART.md) section 3
2. See [examples/simple_server.rs](syphon-examples/examples/simple_server.rs)
3. Check [README.md](README.md) "Publishing from wgpu"

### "I want to receive video in my Rust app"

1. Read [QUICKSTART.md](QUICKSTART.md) section 4
2. See [examples/simple_client.rs](syphon-examples/examples/simple_client.rs)
3. Check [README.md](README.md) "Receiving in a Background Thread"

### "I'm upgrading from 0.1.0"

1. Read [CHANGES.md](CHANGES.md) for summary
2. Follow [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md)
3. Add `autoreleasepool` to background threads

### "Something is crashing"

1. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. Verify framework installation
3. Check for missing `autoreleasepool`

## 📖 By Topic

### Server (Publishing)

- **Basic:** [examples/simple_server.rs](syphon-examples/examples/simple_server.rs)
- **wgpu:** [examples/wgpu_sender.rs](syphon-examples/examples/wgpu_sender.rs)
- **API:** [syphon-core/src/server.rs](syphon-core/src/server.rs)

### Client (Receiving)

- **Basic:** [examples/simple_client.rs](syphon-examples/examples/simple_client.rs)
- **API:** [syphon-core/src/client.rs](syphon-core/src/client.rs)

### Discovery

- **Basic:** [examples/simple_client.rs](syphon-examples/examples/simple_client.rs) (first part)
- **API:** [syphon-core/src/directory.rs](syphon-core/src/directory.rs)

### GPU/Device Selection

- **API:** [syphon-core/src/metal_device.rs](syphon-core/src/metal_device.rs)
- **Examples:** [README.md](README.md) "Checking GPU Compatibility"

## 🔗 External Resources

- [Syphon Framework](https://github.com/Syphon/Syphon-Framework) - Official framework
- [Syphon Website](http://syphon.v002.info/) - Project website
- [Rusty-404](../../../rusty-404) - Example application
- [RustJay Waaaves](../../../rustjay_waaaves) - Example application

## 💡 Quick Tips

**Always wrap background threads in autoreleasepool:**
```rust
use objc::rc::autoreleasepool;

thread::spawn(move || {
    autoreleasepool(|| {
        // Syphon code here
    });
});
```

**Use local framework, not /Library/Frameworks:**
```bash
cp -R ~/Downloads/Syphon.framework ../crates/syphon/syphon-lib/
```

**Check framework is working:**
```rust
if !syphon_core::is_available() {
    panic!("Syphon not available!");
}
```

## 🐛 Reporting Issues

Before filing an issue:

1. ✅ Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. ✅ Test with `simple_server`/`simple_client` examples
3. ✅ Enable debug logging: `RUST_LOG=debug cargo run`
4. ✅ Include:
   - macOS version
   - GPU model
   - Full error output
   - Minimal reproduction code

## 📝 License

MIT License - See individual crate directories for LICENSE files.

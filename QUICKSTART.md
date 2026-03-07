# Quick Start Guide

Get Syphon running in 5 minutes.

## 1. Install Framework

```bash
# Download Syphon
open https://github.com/Syphon/Syphon-Framework/releases

# Copy to workspace
cp -R ~/Downloads/Syphon.framework ../crates/syphon/syphon-lib/
```

## 2. Add to Your Project

**Cargo.toml:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
syphon-core = { path = "../crates/syphon/syphon-core" }
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

## 3. Send Video (Server)

```rust
use syphon_core::SyphonServer;

fn main() -> anyhow::Result<()> {
    // Create server
    let server = SyphonServer::new("My App", 1920, 1080)?;
    
    println!("Server running! Connect from Resolume/OBS.");
    println!("Press Ctrl+C to stop.");
    
    // Keep running
    std::thread::park();
    Ok(())
}
```

## 4. Receive Video (Client)

```rust
use syphon_core::{SyphonClient, SyphonServerDirectory};
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    // Wait for server
    println!("Looking for servers...");
    thread::sleep(Duration::from_secs(2));
    
    // List servers
    let servers = SyphonServerDirectory::servers();
    if servers.is_empty() {
        println!("No servers found!");
        return Ok(());
    }
    
    // Connect to first server
    let target = &servers[0];
    println!("Connecting to '{}'...", target.name);
    
    let client = SyphonClient::connect(&target.name)?;
    
    // Receive frames
    loop {
        if let Some(frame) = client.try_receive()? {
            let data = frame.to_vec()?;
            println!("Got frame: {}x{} ({} bytes)", 
                frame.width, frame.height, data.len());
        }
        thread::sleep(Duration::from_millis(16));
    }
}
```

## 5. Test It

```bash
# Terminal 1: Start server
cargo run --example simple_server

# Terminal 2: Connect client
cargo run --example simple_client
```

You should see the client receiving frames from the server!

## Next Steps

- [Full README](README.md) - Complete documentation
- [Examples](syphon-examples/examples/) - More code samples
- [Troubleshooting](TROUBLESHOOTING.md) - Fix common issues

## One-Liners

**Check if Syphon works:**
```bash
cd ../crates/syphon/syphon-examples && cargo run --example simple_server
```

**Check available servers:**
```rust
let servers = SyphonServerDirectory::servers();
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
| `FrameworkNotFound` | Copy Syphon.framework to syphon-lib/ |
| `ServerNotFound` | Wait 2s for server to announce |
| `segmentation fault` | Wrap thread in `autoreleasepool` |

## Need Help?

1. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. Review [examples/](syphon-examples/examples/)
3. File an issue with error output

# Migration Guide

## Migrating to syphon 0.2.0

### Critical: Fix Background Thread Crashes

The most important change in 0.2.0 is the addition of `autoreleasepool` handling. If you're using Syphon from background threads, you **must** update your code or it will crash.

#### Before (0.1.0) - Will Crash

```rust
use syphon_core::SyphonClient;
use std::thread;

// This will segfault in 0.2.0!
thread::spawn(move || {
    let client = SyphonClient::connect("Server").unwrap();
    
    loop {
        if let Some(frame) = client.try_receive().unwrap() {
            let data = frame.to_vec().unwrap();
            // Process...
        }
        thread::sleep(Duration::from_millis(1));
    }
});
```

#### After (0.2.0) - Safe

```rust
use syphon_core::SyphonClient;
use objc::rc::autoreleasepool;  // Add this import
use std::thread;

// Wrap the entire thread in autoreleasepool
thread::spawn(move || {
    autoreleasepool(|| {
        let client = SyphonClient::connect("Server").unwrap();
        
        loop {
            if let Some(frame) = client.try_receive().unwrap() {
                let data = frame.to_vec().unwrap();
                // Process...
            }
            thread::sleep(Duration::from_millis(1));
        }
    });
});
```

### Dependency Updates

#### Cargo.toml

No breaking changes to dependencies, but you may want to add `objc` for autoreleasepool:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
syphon-core = { path = "../crates/syphon/syphon-core" }
syphon-wgpu = { path = "../crates/syphon/syphon-wgpu" }

# Add this for autoreleasepool support
objc = "0.2"
```

### API Changes

#### New Methods

```rust
// New: Use framework's internal device creation
let server = SyphonServer::new_with_framework_device(
    "My Server", 1920, 1080
)?;

// New: Check GPU compatibility
use syphon_core::check_device_compatibility;
check_device_compatibility(metal_device)?;

// New: List available GPUs
use syphon_core::available_devices;
let gpus = available_devices();
```

#### Improved Error Messages

Errors now include helpful context:

```rust
// Before
Error: FrameworkNotFound

// After  
Error: SyphonMetalServer class not found. 
       Ensure Syphon.framework is installed at /Library/Frameworks/
       and has the correct install name.
       Try: install_name_tool -id /Library/Frameworks/Syphon.framework/Versions/A/Syphon \
            /Library/Frameworks/Syphon.framework/Syphon
```

### Build Configuration

#### Recommended: Use Local Framework

Instead of modifying `/Library/Frameworks/`, copy the framework to your project:

```bash
mkdir -p ../crates/syphon/syphon-lib
cp -R /path/to/Syphon.framework ../crates/syphon/syphon-lib/
```

Then in `build.rs`:

```rust
fn main() {
    #[cfg(target_os = "macos")]
    {
        let framework_path = "../crates/syphon/syphon-lib";
        println!("cargo:rustc-link-search=framework={}", framework_path);
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", framework_path);
    }
}
```

### Common Patterns

#### Pattern 1: Background Frame Receiver

```rust
use syphon_core::SyphonClient;
use objc::rc::autoreleasepool;
use crossbeam::channel::Sender;

pub fn start_receiver(
    server_name: String, 
    frame_tx: Sender<FrameData>
) {
    thread::spawn(move || {
        autoreleasepool(|| {
            let client = match SyphonClient::connect(&server_name) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to connect: {}", e);
                    return;
                }
            };
            
            while running.load(Ordering::SeqCst) {
                match client.try_receive() {
                    Ok(Some(mut frame)) => {
                        match frame.to_vec() {
                            Ok(data) => {
                                let _ = frame_tx.try_send(FrameData {
                                    width: frame.width,
                                    height: frame.height,
                                    data,
                                });
                            }
                            Err(e) => log::warn!("Frame error: {}", e),
                        }
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(1)),
                    Err(e) => log::warn!("Receive error: {}", e),
                }
            }
        });
    });
}
```

#### Pattern 2: Multi-GPU System

```rust
use syphon_core::{
    available_devices,
    recommended_high_performance_device,
    validate_device_match
};

// On multi-GPU systems, ensure rendering and Syphon use same GPU
fn setup_syphon(device: &wgpu::Device) -> Result<SyphonOutput> {
    // List GPUs
    let gpus = available_devices();
    for gpu in &gpus {
        log::info!("GPU: {} (high-performance: {})", 
            gpu.name, gpu.is_high_performance());
    }
    
    // Get recommended GPU
    let recommended = recommended_high_performance_device()
        .ok_or("No GPU found")?;
    
    log::info!("Using GPU: {}", recommended.name);
    
    // Create Syphon output
    SyphonOutput::new("My App", device, queue, 1920, 1080)
}
```

#### Pattern 3: Graceful Fallback

```rust
use syphon_wgpu::SyphonWgpuOutput;
use syphon_core::SyphonError;

fn create_syphon_output(
    name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
) -> Option<SyphonWgpuOutput> {
    match SyphonWgpuOutput::new(name, device, queue, width, height) {
        Ok(output) => {
            log::info!("Syphon output created (zero-copy: {})", 
                output.is_zero_copy());
            Some(output)
        }
        Err(SyphonError::FrameworkNotFound(msg)) => {
            log::warn!("Syphon not available: {}", msg);
            log::warn!("Install from: https://github.com/Syphon/Syphon-Framework/releases");
            None
        }
        Err(e) => {
            log::error!("Failed to create Syphon output: {}", e);
            None
        }
    }
}
```

### Troubleshooting

#### Issue: Still crashing after migration

**Check:** Are you using any other Objective-C libraries?

If you're using other crates that call Objective-C (like `metal`, `cocoa`), they might also need autoreleasepool wrapping.

#### Issue: "Cannot find autoreleasepool"

**Solution:** Add `objc` to your dependencies:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc = "0.2"
```

Then import:

```rust
use objc::rc::autoreleasepool;
```

#### Issue: High CPU usage

**Solution:** Add sleep in your receive loop:

```rust
while running.load(Ordering::SeqCst) {
    if let Some(frame) = client.try_receive()? {
        // Process...
    }
    // Essential: prevent busy-waiting
    thread::sleep(Duration::from_millis(1));
}
```

### Testing

After migration, test these scenarios:

1. ✅ Server creation
2. ✅ Client connection
3. ✅ Frame publishing
4. ✅ Frame receiving
5. ✅ Background thread receiving
6. ✅ Multiple connections
7. ✅ Disconnection/reconnection

### Getting Help

- Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- Review [examples/](syphon-examples/examples/)
- File an issue with:
  - macOS version
  - GPU model
  - Full error output
  - Minimal reproduction code

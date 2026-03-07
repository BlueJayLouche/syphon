# Troubleshooting Guide

## Common Issues

### 1. "Library not loaded" / "image not found"

**Error:**
```
dyld[PID]: Library not loaded: @rpath/Syphon.framework/Versions/A/Syphon
  Referenced from: /path/to/your/app
  Reason: image not found
```

**Causes & Solutions:**

#### A. Framework not in search path

**Solution 1:** Use local framework (recommended)
```bash
# Copy framework to shared location
cp -R ~/Downloads/Syphon.framework ../crates/syphon/syphon-lib/

# Build with rpath
cargo build
```

**Solution 2:** Set environment variable
```bash
export DYLD_FRAMEWORK_PATH=/Library/Frameworks
cargo run
```

**Solution 3:** Modify system framework (not recommended)
```bash
sudo install_name_tool -id \
    /Library/Frameworks/Syphon.framework/Versions/A/Syphon \
    /Library/Frameworks/Syphon.framework/Syphon
```

#### B. Framework has wrong install name

Check:
```bash
otool -D ../crates/syphon/syphon-lib/Syphon.framework/Syphon
```

Should show:
```
@rpath/Syphon.framework/Versions/A/Syphon
```

Not:
```
@loader_path/../Frameworks/...
```

### 2. "Attempt to use unknown class" / Segmentation Fault

**Error:**
```
objc[PID]: Attempt to use unknown class 0x...
zsh: segmentation fault
```

**Cause:** Missing `autoreleasepool` around Objective-C code.

**Solution:**

Wrap all Syphon usage in autoreleasepool, especially in background threads:

```rust
use objc::rc::autoreleasepool;

// Main thread - usually OK
let client = SyphonClient::connect("Server")?;

// Background thread - REQUIRED!
thread::spawn(move || {
    autoreleasepool(|| {
        let client = SyphonClient::connect("Server").unwrap();
        // ... use client
    });
});
```

### 3. Server not found / No servers discovered

**Error:**
```
No Syphon servers found. Make sure you have a server running.
```

**Causes & Solutions:**

#### A. Timing issue

The client checks before server announces:

```rust
// Retry with delay
for attempt in 0..10 {
    let servers = SyphonServerDirectory::servers();
    if !servers.is_empty() {
        break;
    }
    thread::sleep(Duration::from_millis(200));
}
```

#### B. Different framework instances

Server and client using different Syphon.framework copies:

```bash
# Check both use same framework
otool -L server_binary | grep Syphon
otool -L client_binary | grep Syphon

# Should show same path
```

#### C. macOS privacy/security

Check System Preferences > Security & Privacy:
- Add both apps to Screen Recording if needed
- No permission dialogs blocked

### 4. High CPU Usage

**Problem:** Receive loop consuming 100% CPU.

**Solution:** Add sleep to the loop:

```rust
while running.load(Ordering::SeqCst) {
    match client.try_receive() {
        Ok(Some(frame)) => { /* process */ }
        Ok(None) => {
            // ESSENTIAL: Prevent busy-waiting
            thread::sleep(Duration::from_millis(1));
        }
        Err(e) => log::warn!("Error: {}", e),
    }
}
```

### 5. Black Frames / No Video

**Causes:**

#### A. Wrong pixel format

Syphon uses BGRA on macOS. Check your texture format:

```rust
// Correct
desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);

// Wrong
desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
```

#### B. Coordinate system mismatch

wgpu uses top-left origin, Metal uses bottom-left. The syphon-wgpu crate handles this automatically.

#### C. Texture not committed

Ensure Metal command buffer is committed:

```rust
cmd_buf.commit();
```

### 6. Zero-Copy Not Working

**Problem:** `is_zero_copy()` returns false.

**Check:**

1. **Same GPU:** Rendering and Syphon must use same Metal device
2. **Shared storage mode:** Texture must be `MTLStorageMode::Shared`
3. **Framework issue:** Try `new_with_framework_device()` fallback

### 7. Crash in `to_vec()` / Frame Lock

**Error:**
```
Failed to lock IOSurface
```

**Cause:** Double-locking or frame dropped while locked.

**Solution:**

```rust
if let Some(mut frame) = client.try_receive()? {
    // Lock, copy, unlock immediately
    let data = frame.to_vec()?;
    // Now safe to use data
}
```

### 8. Build Errors

#### "SyphonClient class not found"

Framework not linked correctly. Check `build.rs`:

```rust
println!("cargo:rustc-link-search=framework=../crates/syphon/syphon-lib");
```

#### "Cannot find objc crate"

Add to Cargo.toml:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc = "0.2"
```

## Debugging Tips

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run
```

### Check Framework Loading

```bash
# Verify binary links to framework
otool -L target/debug/your_app | grep Syphon

# Check rpath settings
otool -l target/debug/your_app | grep -A2 LC_RPATH
```

### Verify Framework Content

```bash
# Check framework exists and has correct structure
ls -la ../crates/syphon/syphon-lib/Syphon.framework/

# Check install name
otool -D ../crates/syphon/syphon-lib/Syphon.framework/Syphon
```

### Test with Simple Examples

```bash
cd crates/syphon/syphon-examples

# Terminal 1: Start server
cargo run --example simple_server

# Terminal 2: Connect client  
cargo run --example simple_client
```

### Check GPU Availability

```rust
use syphon_core::available_devices;

let gpus = available_devices();
for gpu in &gpus {
    println!("GPU: {}", gpu.name);
    println!("  Low power: {}", gpu.is_low_power);
    println!("  Unified memory: {}", gpu.has_unified_memory);
}
```

## Platform-Specific Issues

### macOS 14+ (Sonoma)

No known issues, but ensure:
- Latest Syphon framework (3.0+)
- Xcode Command Line Tools installed

### Apple Silicon (M1/M2/M3)

Should work natively. If issues:
- Build for arm64: `cargo build --target aarch64-apple-darwin`
- Check Rosetta not interfering

### Intel Mac

Should work. If issues:
- Build for x86_64: `cargo build --target x86_64-apple-darwin`

### Multi-GPU Systems (MacBook Pro with dGPU)

Ensure rendering and Syphon use same GPU:

```rust
use syphon_core::validate_device_match;

validate_device_match(render_device, syphon_device)?;
```

## Getting Help

Before filing an issue:

1. ✅ Test with `simple_server`/`simple_client` examples
2. ✅ Enable debug logging
3. ✅ Check framework installation
4. ✅ Verify autoreleasepool usage

Include in bug report:
- macOS version (`sw_vers`)
- GPU model (About This Mac)
- Full error output
- Minimal reproduction code
- Framework version

## Quick Diagnostic Checklist

```bash
# 1. Framework exists
ls ../crates/syphon/syphon-lib/Syphon.framework/Syphon

# 2. Correct install name
otool -D ../crates/syphon/syphon-lib/Syphon.framework/Syphon

# 3. Binary links correctly
otool -L target/debug/your_app | grep Syphon

# 4. Syphon available at runtime
cargo run --example simple_server  # Should print "Syphon is available!"

# 5. Discovery works
cargo run --example simple_client  # Should find server

# 6. Connection works
# (Run both examples simultaneously)
```

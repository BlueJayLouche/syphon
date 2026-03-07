# Syphon Crate Changelog

## Version 0.2.0 (2024-03-07)

### Bug Fixes

#### Critical: Fixed Segmentation Faults and "Unknown Class" Crashes

**Problem:** Applications were crashing with:
- `objc[PID]: Attempt to use unknown class 0x...`
- `zsh: segmentation fault`
- `zsh: abort`

**Root Cause:** Missing `autoreleasepool` blocks around Objective-C calls. When temporary Objective-C objects are created (like Syphon server lists, frame images), they're added to an autorelease pool. Without a pool, these objects become invalid and accessing them causes crashes.

**Solution:** Added `autoreleasepool` wrappers to all Objective-C interop code:

- `directory.rs` - Server discovery
- `client.rs` - Client creation, frame receiving, connection checking
- `server.rs` - Texture publishing, client counting, server stopping

**Migration:**
If you have background threads using Syphon, wrap them:

```rust
use objc::rc::autoreleasepool;

thread::spawn(move || {
    autoreleasepool(|| {
        let client = SyphonClient::connect("Server").unwrap();
        // ... use client
    });
});
```

### Improvements

#### Better Error Messages

Added detailed error messages for common issues:
- Framework not found with installation instructions
- Incorrect install name detection with fix command
- GPU device compatibility warnings

#### GPU Device Utilities (New)

Added `metal_device.rs` module with:
- `available_devices()` - List all Metal GPUs
- `recommended_high_performance_device()` - Select best GPU
- `check_device_compatibility()` - Validate device support
- `validate_device_match()` - Check render/Syphon GPU match

#### Server Creation Fallback

Added `new_with_framework_device()` method that uses Syphon's internal device creation. This provides a fallback when framework loading issues occur.

### API Changes

#### Added

```rust
// Server
impl SyphonServer {
    pub fn new_with_framework_device(name, width, height) -> Result<Self>;
}

// Device utilities
pub fn available_devices() -> Vec<MetalDeviceInfo>;
pub fn recommended_high_performance_device() -> Option<MetalDeviceInfo>;
pub fn check_device_compatibility(device) -> Result<()>;
pub fn validate_device_match(render_device, syphon_device) -> Result<()>;
```

#### Deprecated

```rust
// No deprecations in this release
```

## Version 0.1.0 (2024-03-01)

### Initial Release

#### Features

- **syphon-core**: Core Objective-C bindings
  - `SyphonServer` - Publish frames
  - `SyphonClient` - Receive frames  
  - `SyphonServerDirectory` - Server discovery
  
- **syphon-wgpu**: wgpu integration
  - `SyphonWgpuOutput` - Zero-copy wgpu output
  - Automatic Y-flip handling
  - IOSurface-backed textures

- **syphon-metal**: Metal utilities
  - `IOSurfacePool` - Triple-buffering surface pool
  - Metal device helpers

#### Known Issues

- Requires manual framework installation
- Background threads may crash without autoreleasepool (fixed in 0.2.0)
- Multi-GPU systems need careful device selection

## Migration Guides

### Upgrading from 0.1.0 to 0.2.0

1. **Update dependencies** - No changes needed

2. **Add autoreleasepool to background threads:**
   ```rust
   // Before (may crash)
   thread::spawn(|| {
       let client = SyphonClient::connect("Server").unwrap();
   });
   
   // After (safe)
   use objc::rc::autoreleasepool;
   thread::spawn(|| {
       autoreleasepool(|| {
           let client = SyphonClient::connect("Server").unwrap();
       });
   });
   ```

3. **Consider using framework fallback if needed:**
   ```rust
   // Try standard method first
   let result = SyphonServer::new(name, width, height);
   
   // Fall back to framework device
   let server = match result {
       Ok(s) => s,
       Err(SyphonError::FrameworkNotFound(_)) => {
           SyphonServer::new_with_framework_device(name, width, height)?
       }
       Err(e) => return Err(e),
   };
   ```

## Future Plans

### Version 0.3.0 (Planned)

- [ ] Spout support for Windows
- [ ] v4l2loopback support for Linux
- [ ] Async/await API
- [ ] Better error recovery
- [ ] Performance metrics

### Long Term

- [ ] Unified cross-platform API
- [ ] Vulkan support
- [ ] DirectX 12 support

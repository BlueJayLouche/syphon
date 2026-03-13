# Syphon Crate Changelog

## Version 0.3.0 (2024-03-13)

### API Cleanup and Simplification

This release streamlines the API by removing redundant components and focusing on the native macOS BGRA format for maximum performance.

#### Removed

- **Y-flip compute shader** - Removed the Metal compute shader for Y-flip. Users should now render directly to BGRA8Unorm textures in the correct orientation.
- **Input format variants** - Removed `input_fast.rs` and `input_optimized.rs`. Now only `input.rs` with native BGRA support.
- **BGRA to RGBA conversion** - Removed GPU conversion. The API now uses native BGRA8Unorm throughout.
- **Redundant examples** - Removed 11 example files, keeping only the essential 3:
  - `wgpu_sender.rs` - wgpu output example
  - `metal_client.rs` - Zero-copy Metal client
  - `simple_client.rs` - Basic client example

#### Simplified API

```rust
// Before: Multiple input types with format conversion
use syphon_wgpu::{SyphonWgpuInput, InputFormat};
let mut input = SyphonWgpuInput::new(&device, &queue);
input.set_format(InputFormat::Bgra);  // No longer needed

// After: Single input type, always BGRA
use syphon_wgpu::SyphonWgpuInput;
let mut input = SyphonWgpuInput::new(&device, &queue);
// Textures are always Bgra8Unorm
```

#### Migration

1. **Update input usage:**
   ```rust
   // Remove format configuration
   // input.set_format(...);  // No longer exists
   
   // Textures are always Bgra8Unorm
   let texture = input.receive_texture(&device, &queue);
   ```

2. **Update rendering:**
   ```rust
   // Ensure your render target uses Bgra8Unorm
   let texture = device.create_texture(&wgpu::TextureDescriptor {
       format: wgpu::TextureFormat::Bgra8Unorm,
       // ...
   });
   ```

3. **Check examples:**
   Many example files were removed. Update your references:
   - `wgpu_sender.rs` (still available)
   - `metal_client.rs` (still available)
   - `simple_client.rs` (still available)

### Documentation Updates

- Updated README with simplified API
- Updated ZERO_COPY_IMPLEMENTATION.md
- Removed MIGRATION_GUIDE.md (no longer needed for new API)
- Simplified DOCUMENTATION_INDEX.md

---

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

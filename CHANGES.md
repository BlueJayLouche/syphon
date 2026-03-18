# Syphon Crate Changelog

## Version 0.4.0 (2026-03-18)

### Performance and API Improvements

A comprehensive overhaul focused on performance, correctness, and ergonomics. All changes are
backward-compatible unless noted.

#### New: Push-Based Frame Delivery

`newFrameHandler` callbacks eliminate polling entirely:

```rust
// syphon-core
let (client, rx) = SyphonClient::connect_with_channel("My App")?;
let (client, rx) = SyphonClient::connect_by_info_with_channel(&info)?;

// syphon-wgpu
let rx = input.connect_with_channel("My App")?;
let rx = input.connect_by_info_with_channel(&info)?;

// rx.recv() wakes the thread exactly when a new frame is published
while rx.recv().is_ok() {
    if let Some(texture) = input.receive_texture(&device, &queue) { ... }
}
```

The channel is a bounded `mpsc::sync_channel(1)` — signals are coalesced if the consumer
is slower than the producer, so there is no unbounded backpressure.

#### New: `PublishStatus` — No More Silent CPU Fallbacks

`publish()` now returns an enum instead of `()`:

```rust
match output.publish(&texture, &device, &queue) {
    PublishStatus::ZeroCopy    => { /* GPU-to-GPU blit, ~0% CPU */ }
    PublishStatus::CpuFallback => log::warn!("falling back to CPU"),
    PublishStatus::NoClients   => { /* no receivers connected */ }
    PublishStatus::PoolExhausted => { /* increase SyphonOutputConfig::pool_size */ }
}
```

#### New: UUID-Based Server Connection

`connect()` now returns `SyphonError::AmbiguousServerName` when multiple servers share a
name, instead of silently picking one. Use `connect_by_info()` for precision:

```rust
let servers = SyphonServerDirectory::servers();
let info = servers.iter().find(|s| s.app_name == "Resolume").unwrap();
SyphonClient::connect_by_info(info)?;   // matched by UUID
```

`SyphonServerDirectory::find_by_uuid(uuid)` is also available.

#### New: `SyphonOutputConfig` and `ServerOptions`

```rust
let config = SyphonOutputConfig {
    pool_size: 4,
    server_options: ServerOptions { is_private: true },
};
SyphonWgpuOutput::new_with_config("Hidden", &device, &queue, w, h, config)?;
```

#### wgpu Input: Now Truly Zero-Copy

`SyphonWgpuInput::receive_texture()` previously fell back to CPU readback.
It now performs a GPU-to-GPU Metal blit (IOSurface → wgpu texture) with zero CPU
involvement when the wgpu device is Metal-backed:

```
Before: IOSurface → CPU lock → write_texture (2 copies, ~5ms)
After:  IOSurface ← Metal texture alias → blit to wgpu output (~0 CPU, ~1ms)
```

CPU fallback is retained for non-Metal backends and logged as a warning.

#### Server Discovery: `requestServerAnnounce` Replaces Polling

`SyphonServerDirectory::servers()` no longer sleeps for 1.5 seconds:

- If servers are already registered, returns immediately
- Otherwise sends `requestServerAnnounce` and spins the run loop for 200ms

#### `metal_interop.rs`: wgpu-hal Isolation

All `wgpu-hal` version-specific code is now in `syphon-wgpu/src/metal_interop.rs`.
When upgrading wgpu, edit only that file and update the version banner.
Current version: wgpu 25.0.

#### Safety: Internalized Autorelease Pools

All ObjC call sites now manage their own autorelease pools internally.
Users no longer need to wrap Syphon calls in `objc::rc::autoreleasepool`.

#### `// SAFETY:` Documentation

All `unsafe impl Send/Sync` bounds now carry explicit `SAFETY:` comments.

### Migration from 0.3.x

- `publish()` return type changed from `()` to `PublishStatus`. Existing call sites
  compile with a dead-code warning; add `let _ =` or match on the result.
- `connect()` may now return `AmbiguousServerName` where it previously succeeded silently.
  Switch to `connect_by_info()` in production code.
- User-level `autoreleasepool` wrappers around Syphon calls are no longer required
  (they remain harmless if present).

---

## Version 0.3.0 (2024-03-13)

### API Cleanup and Simplification

This release streamlines the API by removing redundant components and focusing on the native macOS BGRA format for maximum performance.

#### Removed

- **Y-flip compute shader** — Removed the Metal compute shader for Y-flip. Users should now render directly to BGRA8Unorm textures in the correct orientation.
- **Input format variants** — Removed `input_fast.rs` and `input_optimized.rs`. Now only `input.rs` with native BGRA support.
- **BGRA to RGBA conversion** — Removed GPU conversion. The API now uses native BGRA8Unorm throughout.
- **Redundant examples** — Reduced to 3 essential examples:
  - `wgpu_sender.rs` — wgpu output example
  - `metal_client.rs` — Zero-copy Metal client
  - `simple_client.rs` — Basic client example

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

---

## Version 0.2.0 (2024-03-07)

### Bug Fixes

#### Critical: Fixed Segmentation Faults and "Unknown Class" Crashes

**Problem:** Applications were crashing with:
- `objc[PID]: Attempt to use unknown class 0x...`
- `zsh: segmentation fault`

**Root Cause:** Missing `autoreleasepool` blocks around Objective-C calls.

**Solution:** Added `autoreleasepool` wrappers to all Objective-C interop code in
`directory.rs`, `client.rs`, and `server.rs`.

### Improvements

- Better error messages for framework-not-found and GPU compatibility issues
- `metal_device.rs`: `available_devices()`, `recommended_high_performance_device()`,
  `check_device_compatibility()`, `validate_device_match()`

---

## Version 0.1.0 (2024-03-01)

### Initial Release

- `syphon-core`: `SyphonServer`, `SyphonClient`, `SyphonServerDirectory`
- `syphon-wgpu`: `SyphonWgpuOutput`, zero-copy send path, IOSurface-backed textures
- `syphon-metal`: `IOSurfacePool`, Metal device helpers

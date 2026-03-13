# Syphon Examples

Minimal examples demonstrating the core Syphon integration patterns.

## Examples

### `wgpu_sender.rs` - wgpu Output

Zero-copy GPU-to-GPU Syphon output using wgpu.

```bash
cargo run --example wgpu_sender --package syphon-examples
```

This is the recommended approach for publishing from wgpu applications.

### `metal_client.rs` - Direct Metal Client (Zero-Copy)

Receives frames from a Syphon server as Metal textures without any CPU copies.

```bash
# List available servers
cargo run --example metal_client --package syphon-examples

# Connect to specific server
cargo run --example metal_client --package syphon-examples -- "Server Name"
```

This is the fastest way to receive frames - directly as Metal textures from IOSurface.

### `simple_client.rs` - Basic Client

Simple client demonstrating frame reception.

```bash
cargo run --example simple_client --package syphon-examples
```

## Performance Notes

- **Zero-copy path**: Both `wgpu_sender.rs` and `metal_client.rs` use IOSurface for zero-copy GPU sharing
- **Native BGRA**: All examples use native BGRA8Unorm format without conversion

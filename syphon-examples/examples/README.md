# Syphon Examples

This directory contains examples demonstrating various Syphon integration patterns.

## Core Examples (syphon-core)

### `test_discovery.rs` - Server Discovery
Simple test for discovering Syphon servers.

```bash
cargo run --example test_discovery --package syphon-examples
```

### `simple_client.rs` - Basic Client
Connects to a Syphon server and receives frames as raw data.

```bash
cargo run --example simple_client --package syphon-examples -- "Server Name"
```

### `simple_server.rs` - Basic Server
Creates a simple Syphon server (Metal-based).

```bash
cargo run --example simple_server --package syphon-examples
```

## wgpu Integration Examples (syphon-wgpu)

### `wgpu_sender.rs` - wgpu Output
Zero-copy GPU-to-GPU Syphon output using wgpu.

```bash
cargo run --example wgpu_sender --package syphon-examples
```

Features:
- Creates wgpu context
- Renders animated content
- Publishes to Syphon with zero-copy IOSurface sharing

### `simple_test.rs` (in syphon-wgpu crate)
Simpler version of wgpu_sender.

```bash
cargo run --example simple_test --package syphon-wgpu
```

### `input_test.rs` (in syphon-wgpu crate)
GPU-accelerated Syphon input with BGRA→RGBA conversion.

```bash
cargo run --example input_test --package syphon-wgpu
```

## Metal Examples

### `metal_syphon.rs` - Raw Metal Integration
Low-level Metal texture sharing with Syphon.

```bash
cargo run --example metal_syphon --package syphon-examples
```

### `simple_metal_sender.rs` - Simple Metal Sender
Basic Metal-based Syphon server.

```bash
cargo run --example simple_metal_sender --package syphon-examples
```

### `full_metal_sender.rs` - Complete Metal Example
Full-featured Metal sender with IOSurface handling.

```bash
cargo run --example full_metal_sender --package syphon-examples
```

## Application Integration

### `connect_simple.rs` - Simple Connection Test
Tests connecting to the Simple Server app.

```bash
cargo run --example connect_simple --package syphon-examples
```

### `connect_rusty404.rs` - Rusty-404 Integration
Connects to the rusty-404 application.

```bash
cargo run --example connect_rusty404 --package syphon-examples
```

## Performance Notes

- **Zero-copy path**: `wgpu_sender.rs` and `simple_test.rs` use IOSurface-backed textures for zero-copy GPU sharing
- **GPU compute**: `input_test.rs` uses wgpu compute shaders for BGRA→RGBA conversion
- **CPU fallback**: Core examples can fall back to CPU readback if needed

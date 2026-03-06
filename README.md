# Syphon

Rust bindings and utilities for [Syphon](https://syphon.v002.info/) - an open source macOS framework for sharing video between applications in real-time.

## Overview

This workspace provides Rust crates for integrating Syphon frame sharing into your applications.

## Crates

| Crate | Description |
|-------|-------------|
| [`syphon-core`](./syphon-core/) | Core Objective-C bindings and FFI layer |
| [`syphon-metal`](./syphon-metal/) | Metal/IOSurface utilities for texture sharing |
| [`syphon-wgpu`](./syphon-wgpu/) | High-level wgpu integration for cross-platform apps |
| [`syphon-examples`](./syphon-examples/) | Example applications demonstrating usage |

## Requirements

- macOS 10.13+ (required for Syphon framework)
- Xcode Command Line Tools

## Quick Start

```bash
# Build the entire workspace
cargo build --workspace

# Run examples
cargo run --package syphon-examples --example simple_sender
cargo run --package syphon-examples --example simple_client
cargo run --package syphon-examples --example metal_sender
```

## Architecture

```
syphon-wgpu (high-level API)
    ↓
syphon-metal (Metal/IOSurface utilities)
    ↓
syphon-core (Objective-C FFI bindings)
    ↓
Syphon.framework (native macOS framework)
```

## Syphon Framework

This repository includes the Syphon framework as a Git submodule:

```bash
# Clone with submodules
git clone --recursive https://github.com/BlueJayLouche/syphon.git

# Or if already cloned
git submodule update --init --recursive
```

See [syphon-lib/Syphon-Framework](./syphon-lib/Syphon-Framework/) for the upstream source.

## License

Licensed under the MIT License - see [LICENSE](./LICENSE) for details.

The bundled Syphon.framework is licensed under the BSD 3-Clause License.

## Links

- [Syphon Official Website](https://syphon.v002.info/)
- [Syphon Framework GitHub](https://github.com/Syphon/Syphon-Framework)
- [vvvv.org - Syphon Documentation](https://vvvv.org/documentation/syphon)

## Note

Syphon is macOS only. For cross-platform video sharing, consider [Spout](https://spout.zeal.co/) for Windows or [NDI](https://ndi.tv/).

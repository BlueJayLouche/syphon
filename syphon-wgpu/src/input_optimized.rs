//! Optimized Syphon Input with Zero-Copy Metal Integration (Experimental)
//!
//! This module is a placeholder for future zero-copy IOSurface optimization.
//!
//! ## Future Work
//!
//! The ultimate optimization would:
//! 1. Get raw IOSurface from syphon-core::Frame
//! 2. Create Metal texture directly from IOSurface (zero copy)
//! 3. Use Metal compute for BGRA→RGBA conversion
//! 4. Export as wgpu texture
//!
//! This requires syphon-core changes to expose IOSurface access.

use syphon_core::{SyphonClient, SyphonError, Result};
use std::sync::Arc;

/// Placeholder for optimized input
///
/// Currently just wraps the standard input. Future implementation will
/// provide zero-copy IOSurface→Metal→wgpu path.
pub struct SyphonWgpuInputOptimized {
    inner: super::SyphonWgpuInput,
}

impl SyphonWgpuInputOptimized {
    /// Create a new optimized Syphon input
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            inner: super::SyphonWgpuInput::new(device, queue),
        }
    }

    /// Connect to a Syphon server
    pub fn connect(&mut self, server_name: &str) -> Result<()> {
        self.inner.connect(server_name)
    }

    /// Disconnect from current server
    pub fn disconnect(&mut self) {
        self.inner.disconnect()
    }

    /// Check if connected to a server
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    /// Try to receive a frame as wgpu texture
    pub fn receive_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::Texture> {
        self.inner.receive_texture(device, queue)
    }

    /// Get connected server name
    pub fn server_name(&self) -> Option<&str> {
        self.inner.server_name()
    }

    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        0 // Placeholder
    }
}

impl Drop for SyphonWgpuInputOptimized {
    fn drop(&mut self) {
        self.disconnect();
    }
}

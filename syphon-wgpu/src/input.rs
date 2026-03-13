//! Syphon Input with wgpu Integration
//!
//! Provides zero-copy reception of Syphon frames as wgpu textures.
//! 
//! ## Native BGRA Format
//!
//! Syphon uses native macOS BGRA8Unorm format. This module returns textures
//! in that format directly without any conversion.
//!
//! ## Example
//!
//! ```no_run
//! use syphon_wgpu::SyphonWgpuInput;
//!
//! let mut input = SyphonWgpuInput::new(&device, &queue);
//! input.connect("Simple Server").unwrap();
//!
//! if let Some(texture) = input.receive_texture(&device, &queue) {
//!     // Texture is Bgra8Unorm (native Syphon format)
//! }
//! ```

use syphon_core::{SyphonClient, Result};

/// Syphon input receiver that outputs wgpu textures
///
/// This struct handles:
/// - Connecting to Syphon servers
/// - Receiving frames via IOSurface
/// - Output as wgpu textures (BGRA8Unorm native format)
pub struct SyphonWgpuInput {
    client: Option<SyphonClient>,
    connected_server: Option<String>,
    // Pooled texture for efficient reuse
    pool_texture: Option<wgpu::Texture>,
    pool_width: u32,
    pool_height: u32,
}

impl SyphonWgpuInput {
    /// Create a new Syphon wgpu input
    pub fn new(_device: &wgpu::Device, _queue: &wgpu::Queue) -> Self {
        Self {
            client: None,
            connected_server: None,
            pool_texture: None,
            pool_width: 0,
            pool_height: 0,
        }
    }

    /// Connect to a Syphon server
    pub fn connect(&mut self, server_name: &str) -> Result<()> {
        log::info!("[SyphonWgpuInput] Connecting to: {}", server_name);

        let client = SyphonClient::connect(server_name)?;

        self.client = Some(client);
        self.connected_server = Some(server_name.to_string());

        log::info!("[SyphonWgpuInput] Connected successfully");
        Ok(())
    }

    /// Disconnect from current server
    pub fn disconnect(&mut self) {
        self.client = None;
        self.connected_server = None;
        self.pool_texture = None;
        log::info!("[SyphonWgpuInput] Disconnected");
    }

    /// Check if connected to a server
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Try to receive a frame as wgpu texture
    ///
    /// Returns None if no new frame is available.
    /// The returned texture is always in Bgra8Unorm format (native Syphon format).
    pub fn receive_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::Texture> {
        let client = self.client.as_mut()?;

        // Try to receive frame from Syphon
        let mut frame = match client.try_receive() {
            Ok(Some(frame)) => frame,
            _ => return None,
        };

        let width = frame.width;
        let height = frame.height;

        // Get BGRA data from IOSurface
        let bgra_data = match frame.to_vec() {
            Ok(data) => data,
            Err(e) => {
                log::warn!("[SyphonWgpuInput] Failed to read frame: {}", e);
                return None;
            }
        };

        let stride = bgra_data.len() as u32 / height;

        // Create or reuse pooled texture
        let needs_recreate = self.pool_texture.is_none()
            || self.pool_width != width
            || self.pool_height != height;

        if needs_recreate {
            self.pool_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Syphon Input Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }));
            self.pool_width = width;
            self.pool_height = height;
        }

        let texture = self.pool_texture.as_ref()?;

        // Upload data to texture
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bgra_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // Return the texture (taking it from the pool)
        self.pool_texture.take()
    }

    /// Get connected server name
    pub fn server_name(&self) -> Option<&str> {
        self.connected_server.as_deref()
    }
}

impl Drop for SyphonWgpuInput {
    fn drop(&mut self) {
        self.disconnect();
    }
}

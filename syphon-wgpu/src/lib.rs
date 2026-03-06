//! # Syphon wgpu Integration

use syphon_core::{SyphonServer, SyphonError, Result};

#[cfg(target_os = "macos")]
use metal::*;
#[cfg(target_os = "macos")]
use objc::runtime::Object;

/// High-level wgpu-to-Syphon output
pub struct SyphonWgpuOutput {
    server: SyphonServer,
    width: u32,
    height: u32,
    #[cfg(target_os = "macos")]
    metal_device: Device,
    #[cfg(target_os = "macos")]
    command_queue: CommandQueue,
    /// Staging buffer for async readback
    #[cfg(target_os = "macos")]
    staging_buffer: Option<wgpu::Buffer>,
    /// Whether a frame is currently being read back
    #[cfg(target_os = "macos")]
    pending: bool,
}

#[cfg(target_os = "macos")]
unsafe impl Send for SyphonWgpuOutput {}
#[cfg(target_os = "macos")]
unsafe impl Sync for SyphonWgpuOutput {}

impl SyphonWgpuOutput {
    /// Create a new Syphon output for wgpu
    pub fn new(name: &str, _wgpu_device: &wgpu::Device, width: u32, height: u32) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Self::new_macos(name, _wgpu_device, width, height)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(SyphonError::NotAvailable)
        }
    }
    
    #[cfg(target_os = "macos")]
    fn new_macos(name: &str, _wgpu_device: &wgpu::Device, width: u32, height: u32) -> Result<Self> {
        let metal_device = Device::system_default()
            .ok_or_else(|| SyphonError::CreateFailed(
                "Failed to get Metal device".to_string()
            ))?;
        
        let command_queue = metal_device.new_command_queue();
        
        let server = unsafe {
            let device_ptr = metal_device.as_ref() as *const DeviceRef as *mut Object;
            SyphonServer::new_with_name_and_device(name, device_ptr, width, height)?
        };
        
        Ok(Self {
            server,
            width,
            height,
            metal_device,
            command_queue,
            staging_buffer: None,
            pending: false,
        })
    }
    
    /// Publish a wgpu texture to Syphon
    /// 
    /// This uses a non-blocking approach - if a previous frame is still being
    /// processed, this frame is skipped.
    pub fn publish(&mut self, texture: &wgpu::Texture, device: &wgpu::Device, queue: &wgpu::Queue) {
        #[cfg(target_os = "macos")]
        {
            if self.server.client_count() == 0 {
                return;
            }
            
            // If we have a pending frame, check if it's done
            if self.pending {
                // Try to complete the previous frame
                if let Some(ref buffer) = self.staging_buffer {
                    // Check if we can map it (non-blocking)
                    // For now, just skip this frame to avoid blocking
                    // In a production implementation, you'd use proper async
                    return;
                }
            }
            
            // Create staging buffer if needed
            if self.staging_buffer.is_none() {
                let buffer_size = (self.width * self.height * 4) as u64;
                self.staging_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Syphon Staging"),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }));
            }
            
            let buffer = self.staging_buffer.as_ref().unwrap();
            
            // Copy texture to buffer
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syphon Copy"),
            });
            
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(self.width * 4),
                        rows_per_image: Some(self.height),
                    },
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
            
            queue.submit(std::iter::once(encoder.finish()));
            
            // Readback and publish (blocking with timeout)
            self.readback_and_publish_blocking(buffer, device);
        }
    }
    
    /// Blocking readback - waits for GPU and uploads to Syphon
    #[cfg(target_os = "macos")]
    fn readback_and_publish_blocking(&self, buffer: &wgpu::Buffer, device: &wgpu::Device) {
        use std::time::{Duration, Instant};
        
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        
        // Poll device and wait for mapping with timeout
        let start = Instant::now();
        let timeout = Duration::from_millis(100); // 100ms timeout
        
        while start.elapsed() < timeout {
            device.poll(wgpu::PollType::Poll);
            
            if let Ok(result) = rx.try_recv() {
                if result.is_ok() {
                    let data = buffer_slice.get_mapped_range();
                    self.upload_to_metal(&data);
                    drop(data);
                    buffer.unmap();
                    return;
                }
                break; // Error during mapping
            }
            
            std::thread::sleep(Duration::from_micros(100));
        }
        
        // Timeout or error - unmap
        buffer.unmap();
    }
    
    /// Upload pixel data to Metal and publish to Syphon
    #[cfg(target_os = "macos")]
    fn upload_to_metal(&self, data: &[u8]) {
        // Create Metal texture
        let desc = TextureDescriptor::new();
        desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        desc.set_width(self.width as u64);
        desc.set_height(self.height as u64);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        
        let texture = self.metal_device.new_texture(&desc);
        
        // Upload data to texture
        let region = MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width: self.width as u64,
                height: self.height as u64,
                depth: 1,
            },
        };
        texture.replace_region(
            region,
            0, // mipmap level
            data.as_ptr() as *const _,
            (self.width * 4) as u64, // bytes per row
        );
        
        // Create command buffer for publishing
        let cmd_buf = self.command_queue.new_command_buffer();
        
        // Publish to Syphon
        unsafe {
            let texture_ptr = &*texture as *const _ as *mut Object;
            let cmd_buf_ptr = &*cmd_buf as *const _ as *mut Object;
            self.server.publish_metal_texture(texture_ptr, cmd_buf_ptr);
        }
        
        cmd_buf.commit();
    }
    
    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.server.client_count()
    }
    
    /// Check if any clients are connected
    pub fn has_clients(&self) -> bool {
        self.server.client_count() > 0
    }
    
    /// Get the server name
    pub fn name(&self) -> &str {
        self.server.name()
    }
    
    /// Get dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Utility to list available Syphon servers
pub fn list_servers() -> Vec<String> {
    syphon_core::SyphonServerDirectory::servers()
        .into_iter()
        .map(|info| info.name)
        .collect()
}

/// Check if Syphon is available on this system
pub fn is_available() -> bool {
    syphon_core::is_available()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_availability() {
        println!("Syphon available: {}", is_available());
    }
}

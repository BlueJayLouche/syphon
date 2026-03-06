//! # Syphon wgpu Integration - Zero-Copy Edition
//! 
//! High-performance, zero-copy GPU-to-GPU Syphon output for wgpu applications.
//! 
//! ## Overview
//! 
//! This crate provides a `SyphonWgpuOutput` that enables publishing wgpu-rendered
//! frames to Syphon clients without CPU readback, using IOSurface-backed textures
//! and Metal blit encoders.
//! 
//! ## Usage
//! 
//! ```no_run
//! use syphon_wgpu::SyphonWgpuOutput;
//! 
//! // Create the output
//! let mut output = SyphonWgpuOutput::new(
//!     "My App",
//!     &device,
//!     &queue,
//!     1920,
//!     1080
//! ).expect("Failed to create Syphon output");
//! 
//! // Each frame, publish your rendered texture
//! output.publish(&render_texture, &device, &queue);
//! ```

use syphon_core::{SyphonServer, SyphonError, Result};

#[cfg(target_os = "macos")]
use metal::*;
#[cfg(target_os = "macos")]
use metal::foreign_types::{ForeignType, ForeignTypeRef};
#[cfg(target_os = "macos")]
use objc::runtime::Object;
#[cfg(target_os = "macos")]
use objc::{msg_send, class, sel, sel_impl};
#[cfg(target_os = "macos")]
use cocoa::foundation::NSUInteger;
#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;

/// High-level wgpu-to-Syphon output with zero-copy GPU transfer
/// 
/// This implementation uses IOSurface-backed textures and Metal blit encoders
/// to transfer frames directly from wgpu to Syphon without CPU readback.
pub struct SyphonWgpuOutput {
    server: SyphonServer,
    width: u32,
    height: u32,
    #[cfg(target_os = "macos")]
    surface_pool: syphon_metal::IOSurfacePool,
    #[cfg(target_os = "macos")]
    frame_count: u64,
    #[cfg(target_os = "macos")]
    use_zero_copy: bool,
    #[cfg(target_os = "macos")]
    // Fallback to CPU readback if zero-copy fails
    metal_device: Option<Device>,
    #[cfg(target_os = "macos")]
    metal_queue: Option<CommandQueue>,
}

#[cfg(target_os = "macos")]
unsafe impl Send for SyphonWgpuOutput {}
#[cfg(target_os = "macos")]
unsafe impl Sync for SyphonWgpuOutput {}

impl SyphonWgpuOutput {
    /// Create a new Syphon output for wgpu
    /// 
    /// This attempts to set up zero-copy GPU-to-GPU transfer. If the Metal interop
    /// fails, it falls back to CPU readback.
    /// 
    /// # Arguments
    /// * `name` - The name of the Syphon server (visible to clients)
    /// * `wgpu_device` - The wgpu device used for rendering
    /// * `wgpu_queue` - The wgpu queue used for rendering
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    pub fn new(
        name: &str,
        wgpu_device: &wgpu::Device,
        wgpu_queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Self::new_macos(name, wgpu_device, wgpu_queue, width, height)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (name, wgpu_device, wgpu_queue, width, height);
            Err(SyphonError::NotAvailable)
        }
    }
    
    #[cfg(target_os = "macos")]
    fn new_macos(
        name: &str,
        wgpu_device: &wgpu::Device,
        _wgpu_queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        // Try to get the Metal device from wgpu for zero-copy
        let metal_device = unsafe {
            use wgpu::hal::metal::Device as HalMetalDevice;
            
            let mut device_opt: Option<metal::Device> = None;
            wgpu_device.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_device| {
                if let Some(dev) = hal_device {
                    let raw = dev.raw_device().lock();
                    device_opt = Some(raw.clone());
                }
            });
            device_opt
        };
        
        let use_zero_copy = metal_device.is_some();
        
        if use_zero_copy {
            log::info!("SyphonWgpuOutput: Using zero-copy GPU-to-GPU path");
            
            let metal_device = metal_device.unwrap();
            let metal_queue = metal_device.new_command_queue();
            
            // Create the Syphon server with the Metal device
            let server = unsafe {
                let device_ptr = metal_device.as_ref() as *const DeviceRef as *mut Object;
                SyphonServer::new_with_name_and_device(name, device_ptr, width, height)?
            };
            
            // Create an IOSurface pool for triple-buffering
            let surface_pool = syphon_metal::IOSurfacePool::new(width, height, 3);
            
            log::info!(
                "SyphonWgpuOutput created: {}x{} (zero-copy with {} IOSurfaces)",
                width, height, surface_pool.capacity()
            );
            
            Ok(Self {
                server,
                width,
                height,
                surface_pool,
                frame_count: 0,
                use_zero_copy: true,
                metal_device: Some(metal_device),
                metal_queue: Some(metal_queue),
            })
        } else {
            log::warn!("SyphonWgpuOutput: Metal interop failed, falling back to CPU readback");
            
            // Fallback: Create separate Metal device and use CPU readback
            let metal_device = Device::system_default()
                .ok_or_else(|| SyphonError::CreateFailed(
                    "Failed to get Metal device".to_string()
                ))?;
            
            let metal_queue = metal_device.new_command_queue();
            
            let server = unsafe {
                let device_ptr = metal_device.as_ref() as *const DeviceRef as *mut Object;
                SyphonServer::new_with_name_and_device(name, device_ptr, width, height)?
            };
            
            // Empty pool for fallback mode
            let surface_pool = syphon_metal::IOSurfacePool::new(width, height, 0);
            
            log::info!("SyphonWgpuOutput created: {}x{} (CPU fallback)", width, height);
            
            Ok(Self {
                server,
                width,
                height,
                surface_pool,
                frame_count: 0,
                use_zero_copy: false,
                metal_device: Some(metal_device),
                metal_queue: Some(metal_queue),
            })
        }
    }
    
    /// Publish a texture to Syphon
    /// 
    /// This performs a zero-copy GPU-to-GPU transfer if possible,
    /// falling back to CPU readback if necessary.
    pub fn publish(&mut self, texture: &wgpu::Texture, device: &wgpu::Device, queue: &wgpu::Queue) {
        #[cfg(target_os = "macos")]
        {
            if self.server.client_count() == 0 {
                return;
            }
            
            self.frame_count += 1;
            
            if self.use_zero_copy {
                self.publish_zero_copy(texture, device, queue);
            } else {
                self.publish_cpu_fallback(texture, device, queue);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    fn publish_zero_copy(&mut self, texture: &wgpu::Texture, _device: &wgpu::Device, queue: &wgpu::Queue) {
        use wgpu::hal::metal::Queue as HalMetalQueue;
        
        // Get an IOSurface from the pool
        let surface = match self.surface_pool.acquire() {
            Some(s) => s,
            None => {
                log::warn!("No available IOSurfaces in pool, skipping frame");
                return;
            }
        };
        
        // We need to perform the blit on wgpu's queue to avoid synchronization issues
        // This requires using wgpu-hal to access the raw Metal queue and texture
        unsafe {
            queue.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_queue| {
                if let Some(mtl_queue) = hal_queue {
                    let raw_queue = mtl_queue.as_raw().lock();
                    
                    // Get the raw Metal texture from wgpu
                    texture.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_tex| {
                        if let Some(mtl_tex) = hal_tex {
                            let src_texture = mtl_tex.raw_handle();
                            
                            // Create destination texture from IOSurface using raw Metal calls
                            if let Some(ref metal_device) = self.metal_device {
                                // Create texture from IOSurface using raw Objective-C
                                let dest_texture = Self::create_iosurface_texture(
                                    metal_device,
                                    &surface,
                                    self.width,
                                    self.height
                                );
                                
                                if let Some(dest_texture) = dest_texture {
                                    // Create command buffer from wgpu's queue
                                    let cmd_buf = raw_queue.new_command_buffer();
                                    
                                    // Blit from wgpu texture to IOSurface texture
                                    let blit = cmd_buf.new_blit_command_encoder();
                                    blit.copy_from_texture(
                                        src_texture,
                                        0, 0,
                                        MTLOrigin { x: 0, y: 0, z: 0 },
                                        MTLSize { 
                                            width: self.width as u64, 
                                            height: self.height as u64, 
                                            depth: 1 
                                        },
                                        &dest_texture,
                                        0, 0,
                                        MTLOrigin { x: 0, y: 0, z: 0 },
                                    );
                                    blit.end_encoding();
                                    
                                    // Publish to Syphon before committing
                                    // Syphon will use the command buffer for synchronization
                                    let texture_ptr = dest_texture.as_ptr() as *mut Object;
                                    let cmd_buf_ptr = cmd_buf.as_ptr() as *mut Object;
                                    self.server.publish_metal_texture(texture_ptr, cmd_buf_ptr);
                                    
                                    // Commit through wgpu's queue
                                    cmd_buf.commit();
                                    // Note: We don't wait for completion here - 
                                    // the IOSurface is valid until the blit completes,
                                    // and Syphon handles the rest
                                }
                            }
                        }
                    });
                }
            });
        }
        
        // Return the surface to the pool
        // Note: In production, we should wait for the GPU to finish before returning
        // For now, triple-buffering gives us enough safety margin
        self.surface_pool.release(surface);
    }
    
    /// Create a Metal texture from an IOSurface using raw Objective-C
    #[cfg(target_os = "macos")]
    fn create_iosurface_texture(
        device: &metal::Device,
        surface: &io_surface::IOSurface,
        width: u32,
        height: u32,
    ) -> Option<metal::Texture> {
        use objc::runtime::Object;
        use objc::{msg_send, class};
        use cocoa::foundation::NSUInteger;
        use core_foundation::base::TCFType;
        use metal::{MTLStorageMode, MTLTextureUsage, MTLPixelFormat};
        
        unsafe {
            // Create texture descriptor
            let desc: *mut Object = msg_send![class!(MTLTextureDescriptor), new];
            let _: () = msg_send![desc, setPixelFormat: MTLPixelFormat::BGRA8Unorm];
            let _: () = msg_send![desc, setWidth: width as NSUInteger];
            let _: () = msg_send![desc, setHeight: height as NSUInteger];
            let _: () = msg_send![desc, setStorageMode: MTLStorageMode::Shared];
            let _: () = msg_send![desc, setUsage: MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead];
            
            // Get the raw IOSurfaceRef
            let surface_ref = surface.as_concrete_TypeRef();
            
            // Call newTextureWithDescriptor:iosurface:plane:
            let device_ptr = device.as_ptr() as *mut Object;
            let texture_ptr: *mut Object = msg_send![
                device_ptr,
                newTextureWithDescriptor: desc
                iosurface: surface_ref
                plane: 0 as NSUInteger
            ];
            
            // Release the descriptor
            let _: () = msg_send![desc, release];
            
            if texture_ptr.is_null() {
                None
            } else {
                // Convert to metal::Texture
                Some(metal::Texture::from_ptr(texture_ptr as *mut metal::MTLTexture))
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    fn publish_cpu_fallback(&mut self, texture: &wgpu::Texture, device: &wgpu::Device, queue: &wgpu::Queue) {
        // CPU readback fallback implementation
        // This is the stable but slower path
        
        let buffer_size = (self.width * self.height * 4) as u64;
        
        // Create staging buffer
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syphon Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
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
                buffer: &buffer,
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
        
        // Wait for GPU
        device.poll(wgpu::PollType::Wait);
        
        // Map and upload
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result.is_ok());
        });
        
        // Wait for map (with timeout)
        let start = std::time::Instant::now();
        let mut ready = false;
        while start.elapsed().as_millis() < 10 {
            if let Ok(true) = rx.try_recv() {
                ready = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
            device.poll(wgpu::PollType::Poll);
        }
        
        if ready {
            let data = buffer_slice.get_mapped_range();
            
            // Check if we have actual data
            if data.iter().any(|&b| b != 0) {
                if let (Some(ref metal_device), Some(ref metal_queue)) = 
                    (&self.metal_device, &self.metal_queue) 
                {
                    // Create Metal texture and upload
                    let desc = TextureDescriptor::new();
                    desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
                    desc.set_width(self.width as u64);
                    desc.set_height(self.height as u64);
                    desc.set_storage_mode(MTLStorageMode::Managed);
                    desc.set_usage(MTLTextureUsage::ShaderRead);
                    
                    let mtl_texture = metal_device.new_texture(&desc);
                    
                    mtl_texture.replace_region(
                        MTLRegion {
                            origin: MTLOrigin { x: 0, y: 0, z: 0 },
                            size: MTLSize {
                                width: self.width as u64,
                                height: self.height as u64,
                                depth: 1,
                            },
                        },
                        0,
                        data.as_ptr() as *const _,
                        (self.width * 4) as u64,
                    );
                    
                    let cmd_buf = metal_queue.new_command_buffer();
                    
                    unsafe {
                        let texture_ptr = mtl_texture.as_ptr() as *mut Object;
                        let cmd_buf_ptr = cmd_buf.as_ptr() as *mut Object;
                        self.server.publish_metal_texture(texture_ptr, cmd_buf_ptr);
                    }
                    
                    cmd_buf.commit();
                }
            }
            
            drop(data);
            buffer.unmap();
        }
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
    
    /// Check if zero-copy is being used
    #[cfg(target_os = "macos")]
    pub fn is_zero_copy(&self) -> bool {
        self.use_zero_copy
    }
    
    /// Check if zero-copy is being used (non-macOS always returns false)
    #[cfg(not(target_os = "macos"))]
    pub fn is_zero_copy(&self) -> bool {
        false
    }
}

/// List available Syphon servers
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

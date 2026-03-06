//! # Syphon Metal
//! 
//! Metal-specific utilities for Syphon, including IOSurface creation
//! and Metal texture interop.
//! 
//! ## Usage
//! 
//! ```ignore
//! use syphon_metal::{IOSurfacePool, MetalInterop};
//! 
//! // Create an IOSurface pool for efficient reuse
//! let pool = IOSurfacePool::new(1920, 1080, 3);
//! 
//! // Get a surface for rendering
//! let surface = pool.acquire();
//! 
//! // Create a Metal texture from the IOSurface
//! let texture = MetalInterop::create_texture(&device, &surface);
//! ```

use std::sync::Arc;

/// A pool of reusable IOSurfaces for efficient frame publishing
pub struct IOSurfacePool {
    width: u32,
    height: u32,
    pixel_format: u32, // BGRA, etc.
    #[cfg(target_os = "macos")]
    surfaces: Vec<io_surface::IOSurface>,
}

impl IOSurfacePool {
    /// Create a new pool with the specified capacity
    pub fn new(width: u32, height: u32, capacity: usize) -> Self {
        #[cfg(target_os = "macos")]
        {
            let mut surfaces = Vec::with_capacity(capacity);
            
            for _ in 0..capacity {
                if let Some(surface) = create_iosurface(width, height) {
                    surfaces.push(surface);
                }
            }
            
            Self {
                width,
                height,
                pixel_format: 0x42475241, // BGRA
                surfaces,
            }
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                width,
                height,
                pixel_format: 0x42475241,
            }
        }
    }
    
    /// Acquire an IOSurface from the pool
    /// Returns None if all surfaces are in use
    #[cfg(target_os = "macos")]
    pub fn acquire(&mut self) -> Option<io_surface::IOSurface> {
        self.surfaces.pop()
    }
    
    /// Return an IOSurface to the pool
    #[cfg(target_os = "macos")]
    pub fn release(&mut self, surface: io_surface::IOSurface) {
        self.surfaces.push(surface);
    }
    
    /// Get pool capacity
    pub fn capacity(&self) -> usize {
        #[cfg(target_os = "macos")]
        {
            self.surfaces.capacity()
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            0
        }
    }
    
    /// Get available surface count
    pub fn available(&self) -> usize {
        #[cfg(target_os = "macos")]
        {
            self.surfaces.len()
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            0
        }
    }
}

/// Create an IOSurface with the specified dimensions
#[cfg(target_os = "macos")]
fn create_iosurface(width: u32, height: u32) -> Option<io_surface::IOSurface> {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    
    let width_num = CFNumber::from(width as i64);
    let height_num = CFNumber::from(height as i64);
    let bytes_per_elem = CFNumber::from(4i64); // RGBA8 = 4 bytes
    let pixel_format = CFNumber::from(0x42475241i64); // 'BGRA'
    
    let keys: Vec<CFString> = vec![
        CFString::from_static_string("IOSurfaceWidth"),
        CFString::from_static_string("IOSurfaceHeight"),
        CFString::from_static_string("IOSurfaceBytesPerElement"),
        CFString::from_static_string("IOSurfacePixelFormat"),
    ];
    
    // Create slices of references for the pairs
    let pairs: Vec<(CFString, CFType)> = vec![
        (keys[0].clone(), width_num.as_CFType().clone()),
        (keys[1].clone(), height_num.as_CFType().clone()),
        (keys[2].clone(), bytes_per_elem.as_CFType().clone()),
        (keys[3].clone(), pixel_format.as_CFType().clone()),
    ];
    
    let props = CFDictionary::from_CFType_pairs(&pairs);
    
    Some(io_surface::new(&props))
}

/// Metal interop utilities
#[cfg(target_os = "macos")]
pub struct MetalInterop;

#[cfg(target_os = "macos")]
impl MetalInterop {
    /// Create a Metal texture from an IOSurface
    /// 
    /// # Safety
    /// The IOSurface must remain valid for the lifetime of the texture
    pub fn create_texture(
        _device: &metal::Device,
        _surface: &io_surface::IOSurface,
    ) -> metal::Texture {
        // This would use MTLDevice.makeTexture(descriptor:iosurface:plane:)
        // which is available in the metal crate but might have different API
        unimplemented!("Metal IOSurface texture creation not yet implemented")
    }
    
    /// Get the underlying MTLDevice from a wgpu device
    /// 
    /// This requires wgpu's Metal backend
    #[cfg(feature = "wgpu")]
    pub fn get_metal_device(_wgpu_device: &wgpu::Device) -> Option<metal::Device> {
        // This requires wgpu-hal interop which is still TODO
        unimplemented!("wgpu-hal Metal interop not yet implemented")
    }
}

/// Blit utility for copying between textures and IOSurfaces
pub struct BlitHelper;

impl BlitHelper {
    /// Copy from a wgpu texture to an IOSurface
    /// 
    /// This requires:
    /// 1. Creating a Metal texture view from the wgpu texture
    /// 2. Using a blit encoder to copy to the IOSurface-backed texture
    #[cfg(feature = "wgpu")]
    pub fn copy_to_iosurface(
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _source: &wgpu::Texture,
        _dest: &io_surface::IOSurface,
    ) {
        unimplemented!("wgpu-hal Metal interop not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_pool() {
        let mut pool = IOSurfacePool::new(640, 480, 3);
        assert_eq!(pool.capacity(), 3);
        
        #[cfg(target_os = "macos")]
        {
            assert_eq!(pool.available(), 3);
            
            let surface = pool.acquire();
            assert!(surface.is_some());
            assert_eq!(pool.available(), 2);
            
            pool.release(surface.unwrap());
            assert_eq!(pool.available(), 3);
        }
    }
}

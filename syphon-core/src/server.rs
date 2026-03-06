//! Syphon Server - Publishes frames for other apps to receive
//!
//! This wraps the Objective-C SyphonMetalServer class

use crate::{Result, SyphonError};

// Objective-C imports
#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object};
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use objc_id::ShareId;

/// A Syphon server that publishes frames
///
/// # Example
///
/// ```no_run
/// use syphon_core::SyphonServer;
///
/// let server = SyphonServer::new("My Rust App", 1920, 1080).unwrap();
/// ```
pub struct SyphonServer {
    #[cfg(target_os = "macos")]
    inner: ShareId<Object>,
    
    name: String,
    width: u32,
    height: u32,
}

#[cfg(target_os = "macos")]
unsafe impl Send for SyphonServer {}
#[cfg(target_os = "macos")]
unsafe impl Sync for SyphonServer {}

impl SyphonServer {
    /// Create a new Syphon server with the given name
    ///
    /// Creates a default Metal device internally
    pub fn new(name: &str, width: u32, height: u32) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            // Create default Metal device
            let device = Self::create_default_metal_device()
                .ok_or_else(|| SyphonError::CreateFailed(
                    "Failed to create Metal device".to_string()
                ))?;
            
            Self::new_with_name_and_device(name, device, width, height)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(SyphonError::NotAvailable)
        }
    }
    
    /// Create a new Syphon server with a specific Metal device
    pub fn new_with_name_and_device(
        name: &str,
        metal_device: *mut Object,
        width: u32,
        height: u32
    ) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Self::new_macos(name, metal_device, width, height)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(SyphonError::NotAvailable)
        }
    }
    
    #[cfg(target_os = "macos")]
    fn create_default_metal_device() -> Option<*mut Object> {
        unsafe {
            extern "C" {
                fn MTLCreateSystemDefaultDevice() -> *mut Object;
            }
            
            let device = MTLCreateSystemDefaultDevice();
            if device.is_null() {
                None
            } else {
                Some(device)
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    fn new_macos(name: &str, metal_device: *mut Object, width: u32, height: u32) -> Result<Self> {
        use crate::utils::to_nsstring;
        
        unsafe {
            // Try SyphonMetalServer first, fall back to SyphonServer
            let cls = Class::get("SyphonMetalServer")
                .or_else(|| Class::get("SyphonServer"))
                .ok_or_else(|| SyphonError::FrameworkNotFound(
                    "SyphonMetalServer class not found".to_string()
                ))?;
            
            let ns_name = to_nsstring(name)?;
            
            let obj: *mut Object = msg_send![cls, alloc];
            let obj: *mut Object = msg_send![
                obj,
                initWithName: ns_name
                device: metal_device
                options: std::ptr::null_mut::<Object>()
            ];
            
            let _: () = msg_send![ns_name, release];
            
            if obj.is_null() {
                return Err(SyphonError::CreateFailed(
                    "Failed to create SyphonServer".to_string()
                ));
            }
            
            let inner = ShareId::from_ptr(obj);
            
            Ok(Self {
                inner,
                name: name.to_string(),
                width,
                height,
            })
        }
    }
    
    /// Get the server name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    /// Publish a Metal texture to the server
    /// 
    /// # Safety
    /// The texture must be valid and from the same Metal device as the server
    #[cfg(target_os = "macos")]
    pub unsafe fn publish_metal_texture(
        &self,
        texture: *mut Object,  // id<MTLTexture>
        command_buffer: *mut Object,  // id<MTLCommandBuffer>
    ) {
        use cocoa::foundation::{NSRect, NSPoint, NSSize};
        
        let region = NSRect {
            origin: NSPoint::new(0.0, 0.0),
            size: NSSize::new(self.width as f64, self.height as f64),
        };
        
        let _: () = msg_send![
            &*self.inner,
            publishFrameTexture:texture
            onCommandBuffer:command_buffer
            imageRegion:region
            flipped:false
        ];
    }
    
    /// Get the number of connected clients
    #[cfg(target_os = "macos")]
    pub fn client_count(&self) -> usize {
        unsafe {
            let has_clients: bool = msg_send![&*self.inner, hasClients];
            if has_clients { 1 } else { 0 }
        }
    }
    
    /// Check if any clients are connected
    pub fn has_clients(&self) -> bool {
        self.client_count() > 0
    }
    
    /// Stop the server
    pub fn stop(&self) {
        #[cfg(target_os = "macos")]
        unsafe {
            let _: () = msg_send![&*self.inner, stop];
        }
    }
}

impl Drop for SyphonServer {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        log::debug!("SyphonServer '{}' dropped", self.name);
    }
}

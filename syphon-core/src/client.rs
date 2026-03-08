//! Syphon Client - Receives frames from a Syphon server
//!
//! This wraps the Objective-C SyphonClient class

use crate::{Result, SyphonError};

#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;

#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object};
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use objc_id::ShareId;

/// A frame received from a Syphon server
pub struct Frame {
    /// The IOSurface containing the frame data
    #[cfg(target_os = "macos")]
    pub(crate) surface: io_surface::IOSurface,
    /// Frame dimensions
    pub width: u32,
    pub height: u32,
}

impl Frame {
    /// Get the IOSurface ID
    #[cfg(target_os = "macos")]
    pub fn iosurface_id(&self) -> io_surface::IOSurfaceID {
        self.surface.get_id()
    }
    
    /// Get a reference to the underlying IOSurface
    /// 
    /// This allows zero-copy access to the frame data by creating a Metal texture
    /// directly from the IOSurface.
    #[cfg(target_os = "macos")]
    pub fn iosurface(&self) -> &io_surface::IOSurface {
        &self.surface
    }
    
    /// Get the raw IOSurface reference
    /// 
    /// # Safety
    /// The returned pointer is only valid as long as this Frame exists.
    /// The caller must not release or modify the surface.
    #[cfg(target_os = "macos")]
    pub fn iosurface_ref(&self) -> io_surface::IOSurfaceRef {
        self.surface.as_concrete_TypeRef()
    }
    
    /// Lock the surface for reading (returns base address and seed)
    /// 
    /// Don't forget to unlock when done! Use the returned seed for unlock.
    #[cfg(target_os = "macos")]
    pub fn lock(&mut self) -> Result<(*mut u8, u32)> {
        use crate::iosurface_ext::{IOSurfaceLock, kIOSurfaceLockReadOnly, kIOSurfaceLockAvoidSync};
        
        unsafe {
            let surface_ref = self.surface.as_CFTypeRef() as io_surface::IOSurfaceRef;
            let mut seed = 0u32;
            
            // Try with read-only flag first
            let result = IOSurfaceLock(surface_ref, kIOSurfaceLockReadOnly, &mut seed);
            
            if result != 0 {
                log::debug!("[Syphon Frame] IOSurfaceLock failed with error code: {}. Retrying with avoid sync...", result);
                
                // Try with avoid sync flag as fallback
                let result2 = IOSurfaceLock(surface_ref, kIOSurfaceLockReadOnly | kIOSurfaceLockAvoidSync, &mut seed);
                if result2 != 0 {
                    log::warn!("[Syphon Frame] IOSurfaceLock retry failed with error code: {}", result2);
                    return Err(SyphonError::LockFailed);
                }
            }
            
            let addr = crate::iosurface_ext::IOSurfaceGetBaseAddress(surface_ref);
            
            if addr.is_null() {
                log::error!("[Syphon Frame] IOSurfaceGetBaseAddress returned null");
                let _ = self.unlock(seed);
                return Err(SyphonError::LockFailed);
            }
            
            Ok((addr as *mut u8, seed))
        }
    }
    
    /// Unlock the surface with the seed from lock
    #[cfg(target_os = "macos")]
    pub fn unlock(&mut self, seed: u32) -> Result<()> {
        use crate::iosurface_ext::IOSurfaceUnlock;
        
        unsafe {
            let surface_ref = self.surface.as_CFTypeRef() as io_surface::IOSurfaceRef;
            let mut seed_copy = seed;
            
            let result = IOSurfaceUnlock(surface_ref, 0, &mut seed_copy);
            
            if result != 0 {
                // Log at trace level only - this happens frequently with some servers
                // and doesn't affect functionality since we've already copied the data
                log::trace!("[Syphon Frame] IOSurfaceUnlock failed with error code: {} (ignoring)", result);
                return Err(SyphonError::LockFailed);
            }
            
            Ok(())
        }
    }
    
    /// Get the bytes per row (stride)
    #[cfg(target_os = "macos")]
    pub fn bytes_per_row(&self) -> usize {
        use crate::iosurface_ext::IOSurfaceGetBytesPerRow;
        
        unsafe {
            IOSurfaceGetBytesPerRow(self.surface.as_CFTypeRef() as io_surface::IOSurfaceRef)
        }
    }
    
    /// Copy frame data to a Vec<u8>
    #[cfg(target_os = "macos")]
    pub fn to_vec(&mut self) -> Result<Vec<u8>> {
        use std::slice;
        
        // Try to lock the surface and get the seed
        let (addr, seed) = match self.lock() {
            Ok(result) => result,
            Err(e) => {
                log::error!("[Syphon Frame] Failed to lock IOSurface: {:?}", e);
                return Err(e);
            }
        };
        
        let height = self.height as usize;
        let stride = self.bytes_per_row();
        let size = height * stride;
        
        log::trace!("[Syphon Frame] Locked surface (seed={}), copying {} bytes ({}x{}, stride={})", 
            seed, size, self.width, self.height, stride);
        
        unsafe {
            let data = slice::from_raw_parts(addr, size);
            let result = data.to_vec();
            
            // Try to unlock the surface with the same seed
            // Note: Unlock can fail if the surface was already unlocked or
            // if there's a synchronization issue, but we've already copied the data
            if let Err(_) = self.unlock(seed) {
                // Silently ignore unlock errors - the data is already copied
                // This happens frequently with some Syphon servers and doesn't affect functionality
            }
            
            log::trace!("[Syphon Frame] Successfully copied frame data");
            Ok(result)
        }
    }
}

/// A Syphon client that receives frames from a server
pub struct SyphonClient {
    #[cfg(target_os = "macos")]
    inner: ShareId<Object>,
    
    server_name: String,
    server_app: String,
}

#[cfg(target_os = "macos")]
unsafe impl Send for SyphonClient {}
#[cfg(target_os = "macos")]
unsafe impl Sync for SyphonClient {}

impl SyphonClient {
    /// Connect to a Syphon server by name
    ///
    /// Matches against both the server's `name` and `app_name` fields. This handles
    /// servers with empty names (like the official "Simple Server" app) that only
    /// have an `app_name`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use syphon_core::SyphonClient;
    ///
    /// // These both work:
    /// // - "Resolume Arena" (matches name)
    /// // - "Simple Server" (matches app_name, since name is empty)
    /// let client = SyphonClient::connect("Simple Server")?;
    /// if let Some(frame) = client.receive_frame()? {
    ///     // Use frame...
    /// }
    /// ```
    pub fn connect(server_name: &str) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Self::connect_macos(server_name)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(SyphonError::NotAvailable)
        }
    }
    
    #[cfg(target_os = "macos")]
    fn connect_macos(server_name: &str) -> Result<Self> {
        use crate::utils::{to_nsstring, from_nsstring};
        use objc::rc::autoreleasepool;
        
        unsafe {
            autoreleasepool(|| {
                // Get SyphonServerDirectory
                let dir_cls = Class::get("SyphonServerDirectory")
                    .ok_or_else(|| SyphonError::FrameworkNotFound(
                        "SyphonServerDirectory not found".to_string()
                    ))?;
                let dir: *mut Object = msg_send![dir_cls, sharedDirectory];
                
                // Request server announcements
                let _: () = msg_send![dir, requestServerAnnounce];
                
                // Poll for servers with run loop processing
                // Note: serversMatchingName:appName: doesn't always work, so we use servers
                let mut server_desc: *mut Object = std::ptr::null_mut();
                
                for attempt in 0..30 {
                    // Process run loop to receive distributed notifications
                    let run_loop: *mut Object = msg_send![Class::get("NSRunLoop").unwrap(), currentRunLoop];
                    let date: *mut Object = msg_send![Class::get("NSDate").unwrap(), dateWithTimeIntervalSinceNow:0.05];
                    let _: () = msg_send![run_loop, runUntilDate:date];
                    
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    
                    // Get all servers and look for match by name OR app name
                    let servers: *mut Object = msg_send![dir, servers];
                    let count: usize = msg_send![servers, count];
                    
                    log::debug!("Attempt {}: found {} servers", attempt + 1, count);
                    
                    for i in 0..count {
                        let desc: *mut Object = msg_send![servers, objectAtIndex:i];
                        let name = Self::get_string_from_desc(desc, "SyphonServerDescriptionNameKey");
                        let app = Self::get_string_from_desc(desc, "SyphonServerDescriptionAppNameKey");
                        
                        log::debug!("  Server {}: name='{}', app='{}'", i, name, app);
                        
                        // Match by name OR by app name (Simple Server has empty name)
                        if name == server_name || app == server_name {
                            server_desc = desc;
                            let _: () = msg_send![server_desc, retain];
                            break;
                        }
                    }
                    
                    if !server_desc.is_null() {
                        break;
                    }
                }
                
                if server_desc.is_null() {
                    return Err(SyphonError::ServerNotFound(
                        server_name.to_string()
                    ));
                }
                
                // Get server info before we potentially move the description
                let name = Self::get_string_from_desc(server_desc, "SyphonServerDescriptionNameKey");
                let app = Self::get_string_from_desc(server_desc, "SyphonServerDescriptionAppNameKey");
                
                log::info!("Connecting to server: '{}' from '{}'", name, app);
                
                // Get the default Metal device
                let device = crate::metal_device::default_device()
                    .map(|info| info.raw_device)
                    .ok_or_else(|| SyphonError::FrameworkNotFound(
                        "Metal not available - cannot create Syphon client".to_string()
                    ))?;
                
                // Get SyphonMetalClient class (Metal client is easier to set up than OpenGL)
                let cls = Class::get("SyphonMetalClient")
                    .ok_or_else(|| SyphonError::FrameworkNotFound(
                        "SyphonMetalClient class not found".to_string()
                    ))?;
                
                // Create the client with Rust-level panic catching
                let create_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let obj: *mut Object = msg_send![cls, alloc];
                    let obj: *mut Object = msg_send![
                        obj,
                        initWithServerDescription: server_desc
                        device: device
                        options: std::ptr::null_mut::<Object>()
                        newFrameHandler: std::ptr::null_mut::<Object>()
                    ];
                    obj
                }));
                
                // Release the server description (we retained it when we found it)
                let _: () = msg_send![server_desc, release];
                
                let obj = match create_result {
                    Ok(obj) => obj,
                    Err(_) => {
                        log::error!("SyphonMetalClient init panicked (likely threw Objective-C exception)");
                        return Err(SyphonError::CreateFailed(
                            "SyphonMetalClient initialization failed - server may be invalid".to_string()
                        ));
                    }
                };
                
                if obj.is_null() {
                    return Err(SyphonError::CreateFailed(
                        "Failed to create SyphonClient".to_string()
                    ));
                }
                
                // Check if client is valid
                let is_valid: bool = msg_send![obj, isValid];
                if !is_valid {
                    return Err(SyphonError::CreateFailed(
                        "Client is not valid - server may have stopped".to_string()
                    ));
                }
                
                let inner = ShareId::from_ptr(obj);
                
                Ok(Self {
                    inner,
                    server_name: name,
                    server_app: app,
                })
            })
        }
    }
    
    /// Helper to get string from server description dictionary
    #[cfg(target_os = "macos")]
    unsafe fn get_string_from_desc(desc: *mut Object, key: &str) -> String {
        use crate::utils::{to_nsstring, from_nsstring};
        
        let key_obj = match to_nsstring(key) {
            Ok(k) => k,
            Err(_) => return String::new(),
        };
        
        // Use objectForKey: with the actual constant key string
        let value: *mut Object = msg_send![desc, objectForKey:key_obj];
        
        if value.is_null() {
            String::new()
        } else {
            from_nsstring(value)
        }
    }
    
    /// Try to receive a frame (non-blocking)
    ///
    /// Returns None if no new frame is available
    #[cfg(target_os = "macos")]
    pub fn try_receive(&self) -> Result<Option<Frame>> {
        unsafe {
            // Check if we have a new frame
            let has_new_frame: bool = msg_send![
                &*self.inner,
                hasNewFrame
            ];
            
            if !has_new_frame {
                return Ok(None);
            }
            
            // Get IOSurface directly using newSurface (private but available)
            // This avoids needing to deal with SyphonOpenGLImage
            let surface: *mut Object = msg_send![&*self.inner, newSurface];
            
            if surface.is_null() {
                return Ok(None);
            }
            
            // Get dimensions from IOSurface
            use crate::iosurface_ext::{IOSurfaceGetHeight, IOSurfaceGetBytesPerRow, IOSurfaceGetWidth};
            let height = IOSurfaceGetHeight(surface as io_surface::IOSurfaceRef);
            let bytes_per_row = IOSurfaceGetBytesPerRow(surface as io_surface::IOSurfaceRef);
            // Try to get actual width, fallback to bytes_per_row/4
            let width = IOSurfaceGetWidth(surface as io_surface::IOSurfaceRef).max(bytes_per_row / 4);
            
            log::trace!("Got IOSurface: {}x{} (stride={} bytes)", width, height, bytes_per_row);
            
            // Retain the surface (we'll own it)
            let _: () = msg_send![surface, retain];
            
            // Wrap in IOSurface struct
            let surface = io_surface::IOSurface::wrap_under_get_rule(
                surface as io_surface::IOSurfaceRef
            );
            
            Ok(Some(Frame {
                surface,
                width: width as u32,
                height: height as u32,
            }))
        }
    }
    
    /// Receive a frame, blocking until one is available
    #[cfg(target_os = "macos")]
    pub fn receive(&self) -> Result<Frame> {
        loop {
            if let Some(frame) = self.try_receive()? {
                return Ok(frame);
            }
            
            // Small yield to prevent busy-waiting
            std::thread::yield_now();
        }
    }
    
    /// Check if the server is still available
    #[cfg(target_os = "macos")]
    pub fn is_connected(&self) -> bool {
        unsafe {
            let is_valid: bool = msg_send![&*self.inner, isValid];
            is_valid
        }
    }
    
    /// Get the server name
    pub fn server_name(&self) -> &str {
        &self.server_name
    }
    
    /// Get the server application name
    pub fn server_app(&self) -> &str {
        &self.server_app
    }
    
    /// Stop the client
    pub fn stop(&self) {
        #[cfg(target_os = "macos")]
        unsafe {
            let _: () = msg_send![&*self.inner, stop];
        }
    }
}

impl Drop for SyphonClient {
    fn drop(&mut self) {
        self.stop();
        #[cfg(target_os = "macos")]
        log::debug!("SyphonClient for '{}' dropped", self.server_name);
    }
}

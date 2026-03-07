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
    
    /// Lock the surface for reading (returns base address)
    /// 
    /// Don't forget to unlock when done!
    #[cfg(target_os = "macos")]
    pub fn lock(&mut self) -> Result<*mut u8> {
        use crate::iosurface_ext::{IOSurfaceLock, kIOSurfaceLockReadOnly};
        
        unsafe {
            let surface_ref = self.surface.as_CFTypeRef() as io_surface::IOSurfaceRef;
            let mut seed = 0u32;
            
            let result = IOSurfaceLock(surface_ref, kIOSurfaceLockReadOnly, &mut seed);
            
            if result != 0 {
                return Err(SyphonError::LockFailed);
            }
            
            let addr = crate::iosurface_ext::IOSurfaceGetBaseAddress(surface_ref);
            
            Ok(addr as *mut u8)
        }
    }
    
    /// Unlock the surface
    #[cfg(target_os = "macos")]
    pub fn unlock(&mut self) -> Result<()> {
        use crate::iosurface_ext::IOSurfaceUnlock;
        
        unsafe {
            let surface_ref = self.surface.as_CFTypeRef() as io_surface::IOSurfaceRef;
            let mut seed = 0u32;
            
            let result = IOSurfaceUnlock(surface_ref, 0, &mut seed);
            
            if result != 0 {
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
        
        let addr = self.lock()?;
        let height = self.height as usize;
        let stride = self.bytes_per_row();
        let size = height * stride;
        
        unsafe {
            let data = slice::from_raw_parts(addr, size);
            let result = data.to_vec();
            self.unlock()?;
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
    /// # Example
    ///
    /// ```no_run
    /// use syphon_core::SyphonClient;
    ///
    /// let client = SyphonClient::connect("Resolume Arena")?;
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
                let _: () = msg_send![server_desc, retain];
                
                // Get server info
                let name = Self::get_string_from_desc(server_desc, "SyphonServerDescriptionNameKey");
                let app = Self::get_string_from_desc(server_desc, "SyphonServerDescriptionAppNameKey");
                
                log::info!("Connecting to server: '{}' from '{}'", name, app);
                
                // Get SyphonClient class
                let cls = Class::get("SyphonClient")
                    .ok_or_else(|| SyphonError::FrameworkNotFound(
                        "SyphonClient class not found".to_string()
                    ))?;
                
                // Create the client
                let obj: *mut Object = msg_send![cls, alloc];
                let obj: *mut Object = msg_send![
                    obj,
                    initWithServerDescription: server_desc
                    options: std::ptr::null_mut::<Object>()
                    newFrameHandler: std::ptr::null_mut::<Object>()
                ];
                
                let _: () = msg_send![server_desc, release];
                
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
            use crate::iosurface_ext::{IOSurfaceGetHeight, IOSurfaceGetBytesPerRow};
            let height = IOSurfaceGetHeight(surface as io_surface::IOSurfaceRef);
            let bytes_per_row = IOSurfaceGetBytesPerRow(surface as io_surface::IOSurfaceRef);
            // Assume 4 bytes per pixel (BGRA)
            let width = (bytes_per_row / 4) as usize;
            
            log::debug!("Got IOSurface: {}x{}", width, height);
            
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

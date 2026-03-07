//! Syphon Server Directory - Lists available Syphon servers
//!
//! # Important Note: Empty Server Names
//!
//! Some Syphon servers (like the official "Simple Server" example app) may have
//! an empty `name` field. This is valid behavior - servers are not required to
//! have a display name. Always check if `name` is empty and fall back to
//! `app_name` when displaying server lists or matching servers.
//!
//! ```rust,no_run
//! use syphon_core::SyphonServerDirectory;
//!
//! let servers = SyphonServerDirectory::servers();
//! for server in servers {
//!     // Use app_name as fallback when name is empty
//!     let display_name = if server.name.is_empty() {
//!         &server.app_name
//!     } else {
//!         &server.name
//!     };
//!     println!("Server: {}", display_name);
//! }
//! ```

use crate::{Result, SyphonError};

#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object};
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

/// Information about a Syphon server
///
/// # Note on Empty Names
///
/// The `name` field may be an empty string for some servers (e.g., the official
/// "Simple Server" app). This is valid - servers are not required to have a
/// display name. Use the [`display_name()`](ServerInfo::display_name) method
/// to get a user-friendly name that handles this automatically.
///
/// # Example
///
/// ```rust,no_run
/// use syphon_core::SyphonServerDirectory;
///
/// let servers = SyphonServerDirectory::servers();
/// for server in servers {
///     // Use display_name() for UI - handles empty names automatically
///     println!("Found: {}", server.display_name());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// The server name (what users see)
    /// 
    /// **May be empty!** Some servers don't set a display name. Use 
    /// [`display_name()`](ServerInfo::display_name) for UI instead.
    pub name: String,
    /// The server UUID (unique identifier)
    pub uuid: String,
    /// The application that owns the server
    /// 
    /// Examples: "Simple Server", "rusty-404", "Resolume Arena"
    pub app_name: String,
    /// The application bundle identifier
    pub bundle_id: String,
}

impl ServerInfo {
    /// Get the display name for this server
    ///
    /// Returns `name` if it's not empty, otherwise falls back to `app_name`.
    /// This handles servers like the official "Simple Server" app which has
    /// an empty `name` but a valid `app_name`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use syphon_core::SyphonServerDirectory;
    ///
    /// let servers = SyphonServerDirectory::servers();
    /// for server in servers {
    ///     // Safe for UI - never returns empty string
    ///     let display = server.display_name();
    ///     println!("Server: {}", display);
    /// }
    /// ```
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            &self.app_name
        } else {
            &self.name
        }
    }
}

/// The Syphon server directory - lists all available servers
pub struct SyphonServerDirectory;

impl SyphonServerDirectory {
    /// Get the shared directory instance
    #[cfg(target_os = "macos")]
    fn shared_directory() -> *mut Object {
        unsafe {
            let cls = Class::get("SyphonServerDirectory").unwrap();
            msg_send![cls, sharedDirectory]
        }
    }
    
    /// List all available Syphon servers
    pub fn servers() -> Vec<ServerInfo> {
        #[cfg(target_os = "macos")]
        {
            Self::servers_macos()
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }
    
    #[cfg(target_os = "macos")]
    fn servers_macos() -> Vec<ServerInfo> {
        use crate::utils::from_nsstring;
        use objc::rc::autoreleasepool;
        use std::thread;
        use std::time::Duration;
        
        unsafe {
            autoreleasepool(|| {
                // Get the shared directory
                let dir = Self::shared_directory();
                
                // Request servers to announce themselves
                let _: () = msg_send![dir, requestServerAnnounce];
                
                // Wait for announcements with run loop processing
                let mut count = 0;
                for attempt in 0..30 {
                    // Process run loop to receive distributed notifications
                    let run_loop: *mut Object = msg_send![Class::get("NSRunLoop").unwrap(), currentRunLoop];
                    let date: *mut Object = msg_send![Class::get("NSDate").unwrap(), dateWithTimeIntervalSinceNow:0.05];
                    let _: () = msg_send![run_loop, runUntilDate:date];
                    
                    thread::sleep(Duration::from_millis(50));
                    
                    let servers: *mut Object = msg_send![dir, servers];
                    count = msg_send![servers, count];
                    
                    println!("Attempt {}: {} servers", attempt, count);
                    
                    if count > 0 {
                        break;
                    }
                }
                
                // Get the final servers array
                let servers: *mut Object = msg_send![dir, servers];
                count = msg_send![servers, count];
                
                let mut result = Vec::with_capacity(count);
                
                for i in 0..count {
                    let server_desc: *mut Object = msg_send![servers, objectAtIndex:i];
                    
                    // Extract values using valueForKey: (KVC)
                    let name = Self::value_for_key(server_desc, "name");
                    let uuid = Self::value_for_key(server_desc, "uuid");
                    let app = Self::value_for_key(server_desc, "appName");
                    let bundle = Self::value_for_key(server_desc, "bundleIdentifier");
                    
                    result.push(ServerInfo {
                        name,
                        uuid,
                        app_name: app,
                        bundle_id: bundle,
                    });
                }
                
                result
            })
        }
    }
    
    /// Get a value using KVC valueForKey:
    #[cfg(target_os = "macos")]
    unsafe fn value_for_key(dict: *mut Object, key: &str) -> String {
        use crate::utils::{to_nsstring, from_nsstring};
        
        let key_obj = match to_nsstring(key) {
            Ok(k) => k,
            Err(_) => return String::new(),
        };
        
        let value: *mut Object = msg_send![dict, valueForKey:key_obj];
        
        // key_obj is autoreleased, don't release it
        
        if value.is_null() {
            String::new()
        } else {
            from_nsstring(value)
        }
    }
    
    /// Check if a server with the given name exists
    pub fn server_exists(name: &str) -> bool {
        Self::servers().iter().any(|s| s.name == name)
    }
    
    /// Find a server by name
    pub fn find_server(name: &str) -> Option<ServerInfo> {
        Self::servers().into_iter().find(|s| s.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_servers() {
        let servers = SyphonServerDirectory::servers();
        println!("Found {} Syphon servers", servers.len());
        
        for server in &servers {
            println!("  - {} ({} from {})", 
                server.name, 
                server.uuid,
                server.app_name
            );
        }
    }
}



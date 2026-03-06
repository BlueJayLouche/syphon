//! Syphon Server Directory - Lists available Syphon servers
//!
//! This wraps the Objective-C SyphonServerDirectory class

use crate::{Result, SyphonError};

#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object};
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

/// Information about a Syphon server
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// The server name (what users see)
    pub name: String,
    /// The server UUID (unique identifier)
    pub uuid: String,
    /// The application that owns the server
    pub app_name: String,
    /// The application bundle identifier
    pub bundle_id: String,
}

/// The Syphon server directory - lists all available servers
///
/// # Example
///
/// ```no_run
/// use syphon_core::SyphonServerDirectory;
///
/// let servers = SyphonServerDirectory::servers();
/// for server in servers {
///     println!("{} from {}", server.name, server.app_name);
/// }
/// ```
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
        
        unsafe {
            let dir = Self::shared_directory();
            
            // Get the servers array
            let servers: *mut Object = msg_send![dir, servers];
            let count: usize = msg_send![servers, count];
            
            let mut result = Vec::with_capacity(count);
            
            for i in 0..count {
                let server_desc: *mut Object = msg_send![servers, objectAtIndex:i];
                
                // Extract values from the dictionary
                // Keys are defined in Syphon.h as:
                // - SyphonServerDescriptionNameKey
                // - SyphonServerDescriptionUUIDKey  
                // - SyphonServerDescriptionAppNameKey
                
                let name = Self::string_for_key(server_desc, "SyphonServerDescriptionNameKey");
                let uuid = Self::string_for_key(server_desc, "SyphonServerDescriptionUUIDKey");
                let app = Self::string_for_key(server_desc, "SyphonServerDescriptionAppNameKey");
                let bundle = Self::string_for_key(server_desc, "SyphonServerDescriptionAppBundleIdentifierKey");
                
                result.push(ServerInfo {
                    name,
                    uuid,
                    app_name: app,
                    bundle_id: bundle,
                });
            }
            
            result
        }
    }
    
    /// Helper to get a string value from the server description dictionary
    #[cfg(target_os = "macos")]
    unsafe fn string_for_key(dict: *mut Object, key: &str) -> String {
        use crate::utils::{to_nsstring, from_nsstring};
        
        let key_obj = to_nsstring(key).unwrap_or(std::ptr::null_mut());
        let value: *mut Object = msg_send![dict, objectForKey:key_obj];
        
        if !key_obj.is_null() {
            let _: () = msg_send![key_obj, release];
        }
        
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
    
    /// Get all servers from a specific application
    pub fn servers_from_app(app_name: &str) -> Vec<ServerInfo> {
        Self::servers()
            .into_iter()
            .filter(|s| s.app_name == app_name)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_servers() {
        let servers = SyphonServerDirectory::servers();
        println!("Found {} Syphon servers", servers.len());
        
        for server in servers {
            println!("  - {} ({} from {})", 
                server.name, 
                server.uuid,
                server.app_name
            );
        }
    }
}

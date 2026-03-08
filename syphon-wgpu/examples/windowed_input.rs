//! Simple Windowed Syphon Input Test
//!
//! This example creates a window and displays the received Syphon texture directly,
//! making it easy to debug input issues.

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    env_logger::init();
    
    println!("=== Syphon Windowed Input Test ===\n");
    
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
    use syphon_wgpu::{SyphonWgpuInput as SyphonInput, InputFormat};
    
    // Check if Syphon is available
    if !syphon_wgpu::is_available() {
        eprintln!("Error: Syphon is not available");
        std::process::exit(1);
    }
    println!("✓ Syphon available");
    
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
    println!("\nConnecting to '{}'...", server_name);
    
    // Create window
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .with_title("Syphon Input Test")
        .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32))
        .build(&event_loop)
        .expect("Failed to create window");
    
    // Create wgpu instance
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..Default::default()
    });
    
    let surface = instance.create_surface(&window).expect("Failed to create surface");
    
    // Create adapter and device
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    })).expect("Failed to find adapter");
    
    println!("  Adapter: {}", adapter.get_info().name);
    
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Syphon Test Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        },
    )).expect("Failed to create device");
    
    println!("✓ wgpu initialized\n");
    
    // Configure surface
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats.iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);
    
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);
    
    // Create Syphon input
    let mut syphon_input = SyphonInput::new(&device, &queue);
    
    // Try both formats
    println!("Testing with RGBA format (with BGRA→RGBA conversion)...");
    syphon_input.set_format(InputFormat::Rgba);
    
    if let Err(e) = syphon_input.connect(server_name) {
        eprintln!("✗ Failed to connect: {}", e);
        std::process::exit(1);
    }
    println!("✓ Connected to '{}'\n", server_name);
    
    // Create a simple shader to display the texture
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Display Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("display_shader.wgsl").into()),
    });
    
    // Create bind group layout for texture
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });
    
    // Create sampler
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    
    // State for texture display
    let mut current_texture: Option<wgpu::Texture> = None;
    let mut current_bind_group: Option<wgpu::BindGroup> = None;
    let mut frame_count = 0u64;
    let start_time = std::time::Instant::now();
    
    println!("Starting render loop...");
    println!("Press 'Q' to quit, 'B' to toggle BGRA/RGBA format\n");
    
    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Resized(physical_size) => {
                        config.width = physical_size.width;
                        config.height = physical_size.height;
                        surface.configure(&device, &config);
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state == winit::event::ElementState::Pressed {
                            match event.logical_key {
                                winit::keyboard::Key::Character(c) if c == "q" || c == "Q" => {
                                    target.exit();
                                }
                                winit::keyboard::Key::Character(c) if c == "b" || c == "B" => {
                                    // Toggle format
                                    let new_format = match syphon_input.format() {
                                        InputFormat::Rgba => {
                                            println!("Switching to BGRA format...");
                                            InputFormat::Bgra
                                        }
                                        InputFormat::Bgra => {
                                            println!("Switching to RGBA format...");
                                            InputFormat::Rgba
                                        }
                                    };
                                    syphon_input.set_format(new_format);
                                    // Need to reconnect for format change to take effect
                                    let _ = syphon_input.disconnect();
                                    if let Err(e) = syphon_input.connect(server_name) {
                                        println!("Failed to reconnect: {}", e);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                // Try to receive frame
                if let Some(texture) = syphon_input.receive_texture(&device, &queue) {
                    frame_count += 1;
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let fps = frame_count as f64 / elapsed;
                    
                    if frame_count % 60 == 0 {
                        println!("Frames: {} | FPS: {:.1} | Format: {:?}", 
                            frame_count, fps, syphon_input.format());
                    }
                    
                    // Create view and bind group for this texture
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Texture Bind Group"),
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                        ],
                    });
                    
                    current_texture = Some(texture);
                    current_bind_group = Some(bind_group);
                }
                
                // Render
                if let Some(ref bind_group) = current_bind_group {
                    let output = surface.get_current_texture().expect("Failed to get surface");
                    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                    
                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });
                    
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                        
                        render_pass.set_pipeline(&render_pipeline);
                        render_pass.set_bind_group(0, bind_group, &[]);
                        render_pass.draw(0..6, 0..1);
                    }
                    
                    queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }
            }
            _ => {}
        }
    }).expect("Event loop failed");
}

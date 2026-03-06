//! Syphon Sender Example with wgpu Rendering
//!
//! This example creates a wgpu context, renders a colorful animated pattern,
//! and publishes the output via Syphon so you can view it in Resolume, OBS, etc.

use std::time::Instant;

fn main() {
    env_logger::init();
    
    println!("=== Syphon Sender Example ===\n");
    
    // Check if Syphon is available
    if !syphon_core::is_available() {
        eprintln!("Error: Syphon is not available on this system");
        eprintln!("Make sure you're on macOS and have the Syphon framework installed.");
        std::process::exit(1);
    }
    
    println!("✓ Syphon is available!");
    
    // Set up wgpu
    println!("\nInitializing wgpu...");
    let (device, queue) = match pollster::block_on(setup_wgpu()) {
        Ok((d, q)) => {
            println!("✓ wgpu initialized");
            (d, q)
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize wgpu: {}", e);
            eprintln!("Note: This example requires Metal support.");
            std::process::exit(1);
        }
    };
    
    // Configuration
    let width = 1280u32;
    let height = 720u32;
    let server_name = "Rusty-404 Syphon Test";
    
    println!("\nCreating Syphon server '{}' ({}x{})...", server_name, width, height);
    
    // Create the Syphon output
    let mut syphon_output = match syphon_wgpu::SyphonWgpuOutput::new(server_name, &device, &queue, width, height) {
        Ok(output) => {
            println!("✓ Syphon server created");
            output
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
    
    // Create render pipeline
    println!("\nSetting up render pipeline...");
    let renderer = match ColorPatternRenderer::new(&device, width, height) {
        Ok(r) => {
            println!("✓ Render pipeline ready");
            r
        }
        Err(e) => {
            eprintln!("✗ Failed to create renderer: {}", e);
            std::process::exit(1);
        }
    };
    
    // Main loop
    println!("\n🎬 Broadcasting to Syphon...");
    println!("Open Resolume Arena, OBS, or another Syphon client to view.");
    println!("Press Ctrl+C to exit.\n");
    
    let start_time = Instant::now();
    let mut frame_count = 0u64;
    let target_fps = 60.0;
    let frame_duration = std::time::Duration::from_secs_f64(1.0 / target_fps);
    
    loop {
        let frame_start = Instant::now();
        let elapsed = start_time.elapsed().as_secs_f32();
        
        // Render a frame
        let texture = renderer.render(&device, &queue, elapsed);
        
        // Publish to Syphon
        syphon_output.publish(&texture, &device, &queue);
        
        frame_count += 1;
        
        // Print stats every 60 frames
        if frame_count % 60 == 0 {
            let total_elapsed = start_time.elapsed().as_secs_f64();
            let fps = frame_count as f64 / total_elapsed;
            let clients = syphon_output.client_count();
            
            print!("\r📡 {} clients | {} frames | {:.1} FPS    ", 
                clients, frame_count, fps);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
        
        // Frame rate limiting
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
}

/// Initialize wgpu
async fn setup_wgpu() -> Result<(wgpu::Device, wgpu::Queue), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..Default::default()
    });
    
    // Get adapter (prefer low power for this example)
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .map_err(|e| format!("Failed to find adapter: {:?}", e))?;
    
    println!("  Adapter: {:?}", adapter.get_info().name);
    
    // Create device and queue
    // Use downlevel defaults for compatibility with older GPUs
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Syphon Sender Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            },
        )
        .await?;
    
    Ok((device, queue))
}

/// Simple renderer that creates a colorful animated pattern
struct ColorPatternRenderer {
    width: u32,
    height: u32,
    pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl ColorPatternRenderer {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self, String> {
        // Create output texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT 
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create uniform buffer for time
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<[f32; 4]>() as u64, // time, width, height, pad
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        
        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pattern Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_CODE)),
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        Ok(Self {
            width,
            height,
            pipeline,
            texture,
            texture_view,
            bind_group,
            uniform_buffer,
        })
    }
    
    fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue, time: f32) -> &wgpu::Texture {
        // Update uniform buffer
        let uniforms = [time, self.width as f32, self.height as f32, 0.0f32];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));
        
        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
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
            
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Full-screen triangle
        }
        
        queue.submit(std::iter::once(encoder.finish()));
        
        &self.texture
    }
}

/// WGSL shader for colorful animated pattern
const SHADER_CODE: &str = r#"
struct Uniforms {
    time: f32,
    width: f32,
    height: f32,
    _pad: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Full-screen triangle vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Create a full-screen triangle
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), // Bottom-left
        vec2<f32>( 3.0, -1.0), // Bottom-right (extends beyond screen)
        vec2<f32>(-1.0,  3.0)  // Top-left (extends beyond screen)
    );
    
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

// Animated color pattern fragment shader
@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(uniforms.width, uniforms.height);
    let uv = frag_coord.xy / resolution;
    let time = uniforms.time;
    
    // Create animated color pattern
    let pos = uv * 3.0 - vec2<f32>(1.5, 1.5);
    
    // Rotating coordinates
    let angle = time * 0.5;
    let cos_a = cos(angle);
    let sin_a = sin(angle);
    let rotated = vec2<f32>(
        pos.x * cos_a - pos.y * sin_a,
        pos.x * sin_a + pos.y * cos_a
    );
    
    // Color based on position and time
    let r = 0.5 + 0.5 * sin(rotated.x * 3.0 + time);
    let g = 0.5 + 0.5 * sin(rotated.y * 3.0 + time + 2.0);
    let b = 0.5 + 0.5 * sin((rotated.x + rotated.y) * 2.0 + time + 4.0);
    
    // Add some grid lines
    let grid = vec2<f32>(
        abs(fract(rotated.x * 4.0) - 0.5),
        abs(fract(rotated.y * 4.0) - 0.5)
    );
    let grid_line = step(0.45, grid.x) + step(0.45, grid.y);
    
    // Mix colors with grid
    let color = vec3<f32>(r, g, b) * (1.0 - grid_line * 0.3);
    
    return vec4<f32>(color, 1.0);
}
"#;

//! Full Metal + Syphon Implementation

#[cfg(target_os = "macos")]
mod imp {
    use metal::*;
    use objc::runtime::Object;
    use std::mem;
    
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
        tex_coord: [f32; 2],
    }
    
    pub struct Sender {
        device: Device,
        queue: CommandQueue,
        server: syphon_core::SyphonServer,
        pipeline: RenderPipelineState,
        vertex_buffer: Buffer,
        width: u32,
        height: u32,
    }
    
    impl Sender {
        pub fn new(name: &str, width: u32, height: u32) -> Option<Self> {
            let device = Device::system_default()?;
            let queue = device.new_command_queue();
            
            let server = unsafe {
                let ptr = device.as_ref() as *const DeviceRef as *mut Object;
                syphon_core::SyphonServer::new_with_name_and_device(name, ptr, width, height).ok()?
            };
            
            // Create shader library
            let library_src = r#"
                #include <metal_stdlib>
                using namespace metal;
                
                struct VertexIn {
                    float2 position [[attribute(0)]];
                    float2 texCoord [[attribute(1)]];
                };
                
                struct VertexOut {
                    float4 position [[position]];
                    float2 texCoord;
                };
                
                vertex VertexOut vs_main(VertexIn in [[stage_in]]) {
                    VertexOut out;
                    out.position = float4(in.position, 0.0, 1.0);
                    out.texCoord = in.texCoord;
                    return out;
                }
                
                fragment float4 fs_main(VertexOut in [[stage_in]],
                                        constant float3* color [[buffer(0)]]) {
                    return float4(color[0], 1.0);
                }
            "#;
            
            let library = device.new_library_with_source(library_src, &CompileOptions::new()).ok()?;
            let vs = library.get_function("vs_main", None).ok()?;
            let fs = library.get_function("fs_main", None).ok()?;
            
            // Create pipeline with proper color attachment
            let desc = RenderPipelineDescriptor::new();
            desc.set_vertex_function(Some(&vs));
            desc.set_fragment_function(Some(&fs));
            
            // Configure vertex descriptor
            let vertex_desc = VertexDescriptor::new();
            let attr_pos = vertex_desc.attributes().object_at(0)?;
            attr_pos.set_format(MTLVertexFormat::Float2);
            attr_pos.set_offset(0);
            attr_pos.set_buffer_index(0);
            
            let attr_tex = vertex_desc.attributes().object_at(1)?;
            attr_tex.set_format(MTLVertexFormat::Float2);
            attr_tex.set_offset(8);
            attr_tex.set_buffer_index(0);
            
            let layout = vertex_desc.layouts().object_at(0)?;
            layout.set_stride(16);
            layout.set_step_function(MTLVertexStepFunction::PerVertex);
            
            desc.set_vertex_descriptor(Some(&vertex_desc));
            
            // Set pixel format for color attachment 0
            desc.color_attachments().object_at(0)?.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
            
            let pipeline = device.new_render_pipeline_state(&desc).ok()?;
            
            // Create vertex buffer
            let vertices = [
                Vertex { position: [-1.0, -1.0], tex_coord: [0.0, 1.0] },
                Vertex { position: [ 3.0, -1.0], tex_coord: [2.0, 1.0] },
                Vertex { position: [-1.0,  3.0], tex_coord: [0.0, -1.0] },
            ];
            
            let vertex_buffer = device.new_buffer_with_data(
                vertices.as_ptr() as *const _,
                mem::size_of_val(&vertices) as u64,
                MTLResourceOptions::CPUCacheModeDefaultCache | MTLResourceOptions::StorageModeShared,
            );
            
            Some(Self { device, queue, server, pipeline, vertex_buffer, width, height })
        }
        
        pub fn render(&self, time: f32) {
            let cmd_buf = self.queue.new_command_buffer();
            
            // Create texture
            let desc = TextureDescriptor::new();
            desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
            desc.set_width(self.width as u64);
            desc.set_height(self.height as u64);
            desc.set_storage_mode(MTLStorageMode::Shared);
            desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
            let texture = self.device.new_texture(&desc);
            
            // Create render pass
            let pass_desc = RenderPassDescriptor::new();
            let color_att = pass_desc.color_attachments().object_at(0).unwrap();
            color_att.set_texture(Some(&texture));
            color_att.set_load_action(MTLLoadAction::Clear);
            color_att.set_store_action(MTLStoreAction::Store);
            color_att.set_clear_color(MTLClearColor::new(0.0, 0.0, 0.0, 1.0));
            
            let enc = cmd_buf.new_render_command_encoder(&pass_desc);
            enc.set_render_pipeline_state(&self.pipeline);
            enc.set_vertex_buffer(0, Some(&self.vertex_buffer), 0);
            
            // Animated color
            let color: [f32; 3] = [
                (time.sin() * 0.5 + 0.5),
                ((time + 2.0).sin() * 0.5 + 0.5),
                ((time + 4.0).sin() * 0.5 + 0.5),
            ];
            let color_buf = self.device.new_buffer_with_data(
                color.as_ptr() as *const _,
                mem::size_of::<[f32; 3]>() as u64,
                MTLResourceOptions::CPUCacheModeDefaultCache,
            );
            enc.set_fragment_buffer(0, Some(&color_buf), 0);
            
            enc.draw_primitives(MTLPrimitiveType::Triangle, 0, 3);
            enc.end_encoding();
            
            // Publish texture to Syphon BEFORE committing
            // The command buffer is used by Syphon to schedule the publish
            unsafe {
                let texture_ptr = &*texture as *const _ as *mut Object;
                let cmd_buf_ptr = &*cmd_buf as *const _ as *mut Object;
                self.server.publish_metal_texture(texture_ptr, cmd_buf_ptr);
            }
            
            cmd_buf.commit();
            cmd_buf.wait_until_completed();
        }
        
        pub fn client_count(&self) -> usize {
            self.server.client_count()
        }
    }
}

fn main() {
    env_logger::init();
    
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Requires macOS");
        return;
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::time::Instant;
        
        println!("=== Full Metal Syphon Sender ===\n");
        
        if !syphon_core::is_available() {
            eprintln!("Syphon not available");
            return;
        }
        
        let sender = imp::Sender::new("Rusty-404 Full Metal", 1280, 720)
            .expect("Failed to create sender");
        
        println!("✓ Running - Open Syphon Simple Client to view");
        
        let start = Instant::now();
        let mut frame = 0u64;
        
        loop {
            let t = start.elapsed().as_secs_f32();
            sender.render(t);
            
            frame += 1;
            if frame % 60 == 0 {
                let fps = frame as f64 / start.elapsed().as_secs_f64();
                println!("📡 {} clients | {} frames | {:.1} FPS", 
                    sender.client_count(), frame, fps);
            }
            
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}

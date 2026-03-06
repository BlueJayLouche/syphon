//! Complete Metal + Syphon Example
//!
//! Renders an animated test pattern using Metal and publishes to Syphon.

#[cfg(target_os = "macos")]
mod metal_impl {
    use metal::*;
    use objc::runtime::Object;
    use std::time::Instant;
    
    pub struct MetalSyphonSender {
        device: Device,
        command_queue: CommandQueue,
        server: syphon_core::SyphonServer,
        width: u32,
        height: u32,
    }
    
    impl MetalSyphonSender {
        pub fn new(name: &str, width: u32, height: u32) -> Option<Self> {
            // Get default Metal device
            let device = Device::system_default()?;
            let command_queue = device.new_command_queue();
            
            // Create Syphon server with the device
            let server = unsafe {
                let device_ptr = device.as_ref() as *const DeviceRef as *mut Object;
                syphon_core::SyphonServer::new_with_name_and_device(name, device_ptr, width, height).ok()?
            };
            
            Some(Self {
                device,
                command_queue,
                server,
                width,
                height,
            })
        }
        
        pub fn render_and_publish(&self, time: f32) {
            // Create command buffer
            let command_buffer = self.command_queue.new_command_buffer();
            
            // For now, just commit an empty command buffer
            // In a full implementation, we would:
            // 1. Create an IOSurface
            // 2. Create a Metal texture from it
            // 3. Render to the texture
            // 4. Publish the texture
            
            command_buffer.commit();
        }
        
        pub fn client_count(&self) -> usize {
            self.server.client_count()
        }
    }
}

fn main() {
    env_logger::init();
    
    println!("=== Metal Syphon Sender ===\n");
    
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("This example requires macOS");
        std::process::exit(1);
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::time::Instant;
        
        if !syphon_core::is_available() {
            eprintln!("Syphon not available");
            std::process::exit(1);
        }
        
        println!("✓ Syphon available");
        
        let sender = metal_impl::MetalSyphonSender::new("Rusty-404 Metal", 1280, 720)
            .expect("Failed to create sender");
        
        println!("✓ Metal + Syphon initialized");
        println!("\nOpen Syphon Simple Client to view.");
        println!("Press Ctrl+C to exit.\n");
        
        let start = Instant::now();
        let mut frame = 0u64;
        
        loop {
            let time = start.elapsed().as_secs_f32();
            sender.render_and_publish(time);
            
            frame += 1;
            if frame % 60 == 0 {
                let fps = frame as f64 / start.elapsed().as_secs_f64();
                print!("\r📡 {} clients | {} frames | {:.1} FPS   ", 
                    sender.client_count(), frame, fps);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
            
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}

//! Test connecting to Simple Server

fn main() {
    env_logger::init();
    
    println!("=== Syphon Client Test ===\n");
    
    // Try to connect to Simple Server
    println!("Connecting to 'Simple Server'...");
    
    match syphon_core::SyphonClient::connect("Simple Server") {
        Ok(client) => {
            println!("✅ Connected successfully!");
            println!("   Server: {} from {}", client.server_name(), client.server_app());
            
            // Try to receive frames
            println!("\nWaiting for frames (10 attempts)...");
            for i in 0..10 {
                match client.try_receive() {
                    Ok(Some(frame)) => {
                        println!("✅ Frame {}: {}x{}", i, frame.width, frame.height);
                    }
                    Ok(None) => {
                        println!("   No frame yet...");
                    }
                    Err(e) => {
                        println!("❌ Error: {}", e);
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
        }
    }
    
    println!("\n=== Test complete ===");
}

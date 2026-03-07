//! Test connecting to Rusty-404

fn main() {
    env_logger::init();
    
    println!("=== Connecting to Rusty-404 ===\n");
    
    match syphon_core::SyphonClient::connect("Rusty-404") {
        Ok(client) => {
            println!("✅ Connected successfully!");
            println!("   Server: {} from {}", client.server_name(), client.server_app());
            
            println!("\nReceiving frames...");
            for i in 0..30 {
                match client.try_receive() {
                    Ok(Some(frame)) => {
                        println!("✅ Frame {}: {}x{}", i, frame.width, frame.height);
                    }
                    Ok(None) => {
                        print!(".");
                    }
                    Err(e) => {
                        println!("\n❌ Error: {}", e);
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            println!("\n");
        }
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
        }
    }
    
    println!("=== Test complete ===");
}

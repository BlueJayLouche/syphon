//! Simple test for Syphon server discovery

fn main() {
    env_logger::init();
    
    println!("=== Syphon Discovery Test ===\n");
    
    // Check if Syphon is available
    println!("Checking if Syphon is available...");
    if !syphon_core::is_available() {
        println!("❌ Syphon framework not available!");
        return;
    }
    println!("✅ Syphon is available!\n");
    
    // List servers
    println!("Discovering servers...");
    let servers = syphon_core::SyphonServerDirectory::servers();
    
    println!("Found {} servers:", servers.len());
    for server in &servers {
        println!("  - '{}' (from {})", server.name, server.app_name);
    }
    
    println!("\n=== Test completed successfully ===");
}

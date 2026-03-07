//! Test Syphon server discovery

fn main() {
    env_logger::init();
    
    println!("=== Testing Syphon Discovery ===\n");
    
    let servers = syphon_core::SyphonServerDirectory::servers();
    println!("Found {} servers:", servers.len());
    
    for server in &servers {
        println!("  Server:");
        println!("    name:    '{}'", server.name);
        println!("    app:     '{}'", server.app_name);
        println!("    display: '{}'", server.display_name());
        println!("    uuid:    '{}'", server.uuid);
    }
    
    println!("\n=== Complete ===");
}

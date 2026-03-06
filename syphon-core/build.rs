//! Build script for syphon-core
//! 
//! Links the required Apple frameworks on macOS

fn main() {
    #[cfg(target_os = "macos")]
    {
        // Check for local framework first (for development without system install)
        let local_framework = std::path::PathBuf::from("../lib/Syphon.framework");
        if local_framework.exists() {
            let framework_path = local_framework.canonicalize().unwrap();
            println!("cargo:rustc-link-search=framework={}", framework_path.parent().unwrap().display());
        }
        
        println!("cargo:rustc-link-lib=framework=Syphon");
        println!("cargo:rustc-link-lib=framework=IOSurface");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        println!("cargo:rustc-link-lib=framework=OpenGL");
        
        // Tell cargo to rerun if this file changes
        println!("cargo:rerun-if-changed=build.rs");
    }
}

//! Build script for syphon-examples

fn main() {
    #[cfg(target_os = "macos")]
    {
        // Path to the local Syphon framework (relative to this crate)
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let framework_path = std::path::Path::new(&manifest_dir).join("../syphon-lib");
        
        if framework_path.join("Syphon.framework").exists() {
            let framework_full = framework_path.canonicalize().unwrap();
            println!("cargo:rustc-link-search=framework={}", framework_full.display());
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", framework_full.display());
            println!("cargo:rustc-link-lib=framework=Syphon");
        } else {
            // Try to find it elsewhere
            println!("cargo:warning=Syphon.framework not found at {:?}", framework_path);
        }
        
        // Link required frameworks
        println!("cargo:rustc-link-lib=framework=IOSurface");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Foundation");
        
        println!("cargo:rerun-if-changed=build.rs");
    }
}

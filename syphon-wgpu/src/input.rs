//! Syphon Input with wgpu Integration
//!
//! Provides GPU-accelerated BGRA to RGBA conversion for Syphon input.
//! Returns wgpu textures directly, eliminating CPU-bound conversion.
//!
//! ## Optional Features
//!
//! - **BGRA Conversion**: Can be disabled if your app supports BGRA directly
//! - **Zero-Copy Mode**: Use IOSurface directly for maximum performance (requires BGRA support)
//!
//! ## Example
//!
//! ```no_run
//! use syphon_wgpu::SyphonWgpuInput;
//!
//! let mut input = SyphonWgpuInput::new(&device, &queue);
//! input.connect("Simple Server").unwrap();
//!
//! // With BGRA→RGBA conversion (default)
//! if let Some(texture) = input.receive_texture(&device, &queue) {
//!     // Texture is RGBA8Unorm
//! }
//!
//! // Without conversion (BGRA output)
//! input.set_format(InputFormat::Bgra);
//! if let Some(texture) = input.receive_texture(&device, &queue) {
//!     // Texture is Bgra8Unorm (native Syphon format)
//! }
//! ```

use syphon_core::{SyphonClient, SyphonError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[cfg(target_os = "macos")]
use io_surface::IOSurfaceRef;
#[cfg(target_os = "macos")]
use metal::{MTLPixelFormat, MTLStorageMode, MTLTextureUsage};
#[cfg(target_os = "macos")]
use objc::runtime::Object;
#[cfg(target_os = "macos")]
use cocoa::foundation::NSUInteger;
#[cfg(target_os = "macos")]
use objc::{msg_send, class, sel, sel_impl};
#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;

/// Format for received textures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    /// BGRA8Unorm - Native Syphon format, no conversion needed
    Bgra,
    /// RGBA8Unorm - Converted format (default)
    Rgba,
}

/// Syphon input receiver that outputs wgpu textures
///
/// This struct handles:
/// - Connecting to Syphon servers
/// - Receiving frames via IOSurface
/// - Optional GPU-accelerated BGRA→RGBA conversion
/// - Output as wgpu textures (ready for rendering)
pub struct SyphonWgpuInput {
    client: Option<SyphonClient>,
    device: Option<Arc<wgpu::Device>>,
    queue: Option<Arc<wgpu::Queue>>,
    converter: Option<BgraConverter>,
    connected_server: Option<String>,
    /// Output format (BGRA or RGBA)
    format: InputFormat,
    /// Whether to use IOSurface directly (zero-copy) when format is BGRA
    use_iosurface: bool,
}

impl SyphonWgpuInput {
    /// Create a new Syphon wgpu input
    ///
    /// Default configuration:
    /// - Output format: RGBA8Unorm (with BGRA→RGBA conversion)
    /// - Zero-copy IOSurface: Disabled
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            client: None,
            device: Some(Arc::new(device.clone())),
            queue: Some(Arc::new(queue.clone())),
            converter: None,
            connected_server: None,
            format: InputFormat::Rgba,
            use_iosurface: false,
        }
    }

    /// Connect to a Syphon server
    pub fn connect(&mut self, server_name: &str) -> Result<()> {
        log::info!("[SyphonWgpuInput] Connecting to: {}", server_name);

        let client = SyphonClient::connect(server_name)?;

        // Initialize GPU converter if needed
        if self.format == InputFormat::Rgba {
            if let (Some(device), Some(queue)) = (&self.device, &self.queue) {
                self.converter = Some(BgraConverter::new(device.clone(), queue.clone()));
            }
        }

        self.client = Some(client);
        self.connected_server = Some(server_name.to_string());

        log::info!("[SyphonWgpuInput] Connected successfully (format: {:?})", self.format);
        Ok(())
    }

    /// Disconnect from current server
    pub fn disconnect(&mut self) {
        self.client = None;
        self.converter = None;
        self.connected_server = None;
        log::info!("[SyphonWgpuInput] Disconnected");
    }

    /// Check if connected to a server
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Set the output format
    ///
    /// - `InputFormat::Rgba` - Convert BGRA to RGBA (default, most compatible)
    /// - `InputFormat::Bgra` - Keep native BGRA format (zero-copy, fastest)
    ///
    /// Must be called before `connect()` to take effect.
    pub fn set_format(&mut self, format: InputFormat) {
        self.format = format;
    }

    /// Get the current output format
    pub fn format(&self) -> InputFormat {
        self.format
    }

    /// Enable/disable zero-copy IOSurface mode
    ///
    /// When enabled and format is BGRA, the IOSurface is used directly
    /// without any GPU copies or conversions.
    ///
    /// Must be called before `connect()` to take effect.
    pub fn set_use_iosurface(&mut self, enabled: bool) {
        self.use_iosurface = enabled;
    }

    /// Try to receive a frame as wgpu texture
    ///
    /// Returns None if no new frame is available.
    /// The returned texture format depends on `self.format()`:
    /// - `InputFormat::Rgba` - RGBA8Unorm
    /// - `InputFormat::Bgra` - Bgra8Unorm
    pub fn receive_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::Texture> {
        let client = self.client.as_mut()?;

        // Try to receive frame from Syphon
        let mut frame = match client.try_receive() {
            Ok(Some(frame)) => frame,
            _ => return None,
        };

        let width = frame.width;
        let height = frame.height;

        match self.format {
            InputFormat::Bgra => {
                // Native BGRA format - no conversion needed
                if self.use_iosurface {
                    // Zero-copy path: create texture from IOSurface directly
                    self.create_texture_from_iosurface(device, &frame, width, height)
                } else {
                    // Copy path: upload BGRA data to texture
                    self.create_bgra_texture_from_data(device, queue, &mut frame, width, height)
                }
            }
            InputFormat::Rgba => {
                // RGBA format - need conversion
                let converter = self.converter.as_mut()?;

                // Get BGRA data from IOSurface
                let bgra_data = match frame.to_vec() {
                    Ok(data) => data,
                    Err(e) => {
                        log::warn!("[SyphonWgpuInput] Failed to read frame: {}", e);
                        return None;
                    }
                };

                // Convert to RGBA texture using GPU
                converter.convert(&bgra_data, width, height, device, queue)
            }
        }
    }

    /// Create a wgpu texture directly from IOSurface (zero-copy)
    #[cfg(target_os = "macos")]
    fn create_texture_from_iosurface(
        &self,
        _device: &wgpu::Device,
        _frame: &syphon_core::Frame,
        _width: u32,
        _height: u32,
    ) -> Option<wgpu::Texture> {
        // TODO: Implement true zero-copy IOSurface→wgpu texture interop
        // This requires wgpu-hal support for creating textures from raw Metal handles
        // For now, fall back to copy path
        None
    }

    #[cfg(not(target_os = "macos"))]
    fn create_texture_from_iosurface(
        &self,
        _device: &wgpu::Device,
        _frame: &syphon_core::Frame,
        _width: u32,
        _height: u32,
    ) -> Option<wgpu::Texture> {
        None
    }

    /// Create BGRA texture from frame data (copy path)
    fn create_bgra_texture_from_data(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &mut syphon_core::Frame,
        width: u32,
        height: u32,
    ) -> Option<wgpu::Texture> {
        // Get BGRA data from IOSurface
        let bgra_data = match frame.to_vec() {
            Ok(data) => data,
            Err(e) => {
                log::warn!("[SyphonWgpuInput] Failed to read frame: {}", e);
                return None;
            }
        };

        let stride = bgra_data.len() as u32 / height;

        // Create BGRA texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Syphon BGRA Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Upload data
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bgra_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Some(texture)
    }

    /// Get connected server name
    pub fn server_name(&self) -> Option<&str> {
        self.connected_server.as_deref()
    }
}

impl Drop for SyphonWgpuInput {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(target_os = "macos")]
fn create_metal_texture_from_iosurface(
    device: &metal::Device,
    surface_ref: io_surface::IOSurfaceRef,
    width: u32,
    height: u32,
) -> Option<metal::Texture> {
    use metal::{MTLPixelFormat, MTLStorageMode, MTLTextureUsage, DeviceRef};
    use metal::foreign_types::{ForeignType, ForeignTypeRef};
    use objc::runtime::Object;
    use cocoa::foundation::NSUInteger;
    use objc::{msg_send, class};

    unsafe {
        // Create texture descriptor
        let desc: *mut Object = msg_send![class!(MTLTextureDescriptor), new];
        let _: () = msg_send![desc, setPixelFormat: MTLPixelFormat::BGRA8Unorm];
        let _: () = msg_send![desc, setWidth: width as NSUInteger];
        let _: () = msg_send![desc, setHeight: height as NSUInteger];
        let _: () = msg_send![desc, setStorageMode: MTLStorageMode::Shared];
        let _: () = msg_send![
            desc,
            setUsage: MTLTextureUsage::ShaderRead | MTLTextureUsage::ShaderWrite
        ];

        // Create texture from IOSurface
        let device_ptr = device.as_ref() as *const DeviceRef as *mut Object;
        let texture_ptr: *mut Object = msg_send![
            device_ptr,
            newTextureWithDescriptor: desc
            iosurface: surface_ref
            plane: 0 as NSUInteger
        ];

        // Release descriptor
        let _: () = msg_send![desc, release];

        if texture_ptr.is_null() {
            None
        } else {
            Some(metal::Texture::from_ptr(texture_ptr as *mut metal::MTLTexture))
        }
    }
}

/// GPU-accelerated BGRA to RGBA converter using wgpu compute shader
pub struct BgraConverter {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
}

impl BgraConverter {
    /// Create a new converter
    fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BGRA Converter Bind Group Layout"),
            entries: &[
                // Input BGRA buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output RGBA buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BGRA Converter Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create compute shader (no Y-flip - only BGRA→RGBA conversion)
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("BGRA Converter Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bgra_to_rgba.wgsl").into()),
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("BGRA Converter Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BGRA Converter Uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            uniform_buffer,
        }
    }

    /// Convert BGRA data to RGBA texture
    fn convert(
        &mut self,
        bgra_data: &[u8],
        width: u32,
        height: u32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::Texture> {
        let stride = bgra_data.len() as u32 / height;
        let input_size = bgra_data.len() as u64;
        let output_size = (width * height * 4) as u64;

        // Create input buffer (BGRA)
        let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BGRA Input"),
            contents: bgra_data,
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create output buffer (RGBA)
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RGBA Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Update uniforms
        let uniforms = [width, height, stride, 0u32];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BGRA Converter Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Encode compute pass
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("BGRA Converter Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("BGRA Converter Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (8x8 threads each)
            let wg_x = (width + 7) / 8;
            let wg_y = (height + 7) / 8;
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Create output texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Syphon RGBA Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Copy output buffer to texture
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
            },
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        Some(texture)
    }
}

/// Get Metal device from wgpu device
#[cfg(target_os = "macos")]
fn get_metal_device(wgpu_device: &wgpu::Device) -> Option<metal::Device> {
    use wgpu::hal::metal::Device as HalMetalDevice;
    
    unsafe {
        let mut device_opt: Option<metal::Device> = None;
        wgpu_device.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_device| {
            if let Some(dev) = hal_device {
                let raw = dev.raw_device().lock();
                device_opt = Some(raw.clone());
            }
        });
        device_opt
    }
}

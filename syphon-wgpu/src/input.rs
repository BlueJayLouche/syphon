//! Syphon wgpu input receiver
//!
//! ## Zero-copy path (default on Metal)
//!
//! When the wgpu device is backed by Metal, frames are transferred via a GPU
//! blit from the IOSurface-backed Metal texture directly into the output wgpu
//! texture — no CPU involvement at all.
//!
//! ## CPU fallback
//!
//! If the Metal HAL is unavailable (e.g. wgpu Vulkan/DX12), the frame is
//! locked on the CPU and uploaded via `queue.write_texture`.

use syphon_core::{SyphonClient, SyphonError, Result, ServerInfo};
#[cfg(target_os = "macos")]
use crate::metal_interop;

pub struct SyphonWgpuInput {
    client: Option<SyphonClient>,
    connected_server: Option<String>,
    pool_texture: Option<wgpu::Texture>,
    pool_width: u32,
    pool_height: u32,
    /// Metal context created from wgpu's underlying Metal device.
    /// Present only when wgpu is backed by Metal.
    #[cfg(target_os = "macos")]
    metal_ctx: Option<syphon_metal::MetalContext>,
}

impl SyphonWgpuInput {
    /// Create a new input receiver.
    ///
    /// Extracts the underlying Metal device from `device` (if Metal-backed) so
    /// the zero-copy blit path is available immediately.
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue) -> Self {
        #[cfg(target_os = "macos")]
        let metal_ctx = Self::build_metal_ctx(device);

        Self {
            client: None,
            connected_server: None,
            pool_texture: None,
            pool_width: 0,
            pool_height: 0,
            #[cfg(target_os = "macos")]
            metal_ctx,
        }
    }

    #[cfg(target_os = "macos")]
    fn build_metal_ctx(device: &wgpu::Device) -> Option<syphon_metal::MetalContext> {
        let ctx = metal_interop::extract_metal_device(device)
            .map(|raw| unsafe { syphon_metal::MetalContext::from_raw_device(raw) });
        if ctx.is_none() {
            log::warn!("[SyphonWgpuInput] wgpu device is not Metal-backed; will use CPU fallback");
        }
        ctx
    }

    /// Connect to a Syphon server by display name.
    ///
    /// Returns [`SyphonError::AmbiguousServerName`] when multiple servers share
    /// the same name. In that case use [`connect_by_info`](Self::connect_by_info).
    pub fn connect(&mut self, server_name: &str) -> Result<()> {
        log::info!("[SyphonWgpuInput] Connecting to '{}'", server_name);
        let client = SyphonClient::connect(server_name)?;
        self.client = Some(client);
        self.connected_server = Some(server_name.to_string());
        log::info!("[SyphonWgpuInput] Connected");
        Ok(())
    }

    /// Connect using a [`ServerInfo`] obtained from `SyphonServerDirectory`.
    /// Matches by UUID — unambiguous even when names collide.
    pub fn connect_by_info(&mut self, info: &ServerInfo) -> Result<()> {
        log::info!("[SyphonWgpuInput] Connecting to '{}' (uuid={})", info.display_name(), info.uuid);
        let client = SyphonClient::connect_by_info(info)?;
        self.connected_server = Some(info.display_name().to_string());
        self.client = Some(client);
        log::info!("[SyphonWgpuInput] Connected");
        Ok(())
    }

    /// Connect with push-based delivery via a channel.
    ///
    /// Returns `((), receiver)`. The receiver yields `()` each time the server
    /// publishes a new frame — no polling needed. Call [`receive_texture`](Self::receive_texture)
    /// after waking on the channel.
    pub fn connect_with_channel(
        &mut self,
        server_name: &str,
    ) -> Result<std::sync::mpsc::Receiver<()>> {
        log::info!("[SyphonWgpuInput] Connecting to '{}' (push mode)", server_name);
        let (client, rx) = SyphonClient::connect_with_channel(server_name)?;
        self.connected_server = Some(server_name.to_string());
        self.client = Some(client);
        log::info!("[SyphonWgpuInput] Connected (push mode)");
        Ok(rx)
    }

    /// Connect by [`ServerInfo`] with push-based delivery.
    ///
    /// UUID-based — unambiguous even when names collide.
    pub fn connect_by_info_with_channel(
        &mut self,
        info: &ServerInfo,
    ) -> Result<std::sync::mpsc::Receiver<()>> {
        log::info!("[SyphonWgpuInput] Connecting to '{}' (uuid={}, push mode)", info.display_name(), info.uuid);
        let (client, rx) = SyphonClient::connect_by_info_with_channel(info)?;
        self.connected_server = Some(info.display_name().to_string());
        self.client = Some(client);
        log::info!("[SyphonWgpuInput] Connected (push mode)");
        Ok(rx)
    }

    pub fn disconnect(&mut self) {
        self.client = None;
        self.connected_server = None;
        self.pool_texture = None;
        log::info!("[SyphonWgpuInput] Disconnected");
    }

    pub fn is_connected(&self) -> bool {
        self.client.as_ref().map_or(false, |c| {
            #[cfg(target_os = "macos")]
            { c.is_connected() }
            #[cfg(not(target_os = "macos"))]
            { true }
        })
    }

    /// Try to receive a frame as a wgpu texture (Bgra8Unorm).
    ///
    /// Returns `None` when no new frame is available.
    ///
    /// On Metal, performs a GPU-to-GPU blit with zero CPU copies.
    /// On other backends, falls back to CPU upload.
    pub fn receive_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::Texture> {
        let client = self.client.as_ref()?;

        #[cfg(target_os = "macos")]
        {
            // Fast-path guard: avoid syscall if no new frame.
            if !client.has_new_frame() { return None; }

            let mut frame = match client.try_receive() {
                Ok(Some(f)) => f,
                _ => return None,
            };

            let w = frame.width;
            let h = frame.height;

            // Create or reuse the output texture.
            if self.pool_texture.is_none() || self.pool_width != w || self.pool_height != h {
                self.pool_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("syphon_input"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                }));
                self.pool_width = w;
                self.pool_height = h;
            }

            let output = self.pool_texture.as_ref()?;

            // Attempt zero-copy GPU blit; fall back to CPU on failure.
            let used_gpu = self.metal_ctx.as_ref().map_or(false, |ctx| {
                Self::gpu_blit(ctx, &frame, output, queue)
            });

            if !used_gpu {
                log::warn!("[SyphonWgpuInput] GPU blit unavailable, using CPU fallback");
                let stride = frame.bytes_per_row() as u32;
                let data = match frame.to_vec() {
                    Ok(d) => d,
                    Err(e) => { log::warn!("[SyphonWgpuInput] CPU read failed: {}", e); return None; }
                };
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: output,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(h),
                    },
                    wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                );
            }

            self.pool_texture.take()
        }

        #[cfg(not(target_os = "macos"))]
        { None }
    }

    /// GPU-to-GPU blit: IOSurface → output wgpu texture, zero CPU copies.
    ///
    /// Uses wgpu's underlying Metal command queue so ordering with subsequent
    /// wgpu render commands is preserved by Metal's queue semantics.
    #[cfg(target_os = "macos")]
    fn gpu_blit(
        ctx: &syphon_metal::MetalContext,
        frame: &syphon_core::Frame,
        output: &wgpu::Texture,
        queue: &wgpu::Queue,
    ) -> bool {
        use metal::foreign_types::ForeignType;

        // Create an IOSurface-backed Metal texture on the same device as wgpu.
        // This is zero-copy: the texture shares GPU memory with the IOSurface.
        let src = match ctx.create_texture_from_iosurface(
            frame.iosurface(), frame.width, frame.height,
        ) {
            Some(t) => t,
            None => {
                log::warn!("[SyphonWgpuInput] create_texture_from_iosurface failed");
                return false;
            }
        };

        let mut ok = false;

        unsafe {
            objc::rc::autoreleasepool(|| {
                // Submit the blit on wgpu's own Metal queue so Metal's command-queue
                // ordering guarantees the blit completes before any subsequent wgpu
                // commands that read `output`.
                metal_interop::with_metal_queue_and_texture(queue, output, |raw_q, dst| {
                    let cmd = raw_q.new_command_buffer();
                    let enc = cmd.new_blit_command_encoder();
                    enc.copy_from_texture(
                        &src,
                        0, 0,
                        metal::MTLOrigin { x: 0, y: 0, z: 0 },
                        metal::MTLSize {
                            width:  frame.width  as u64,
                            height: frame.height as u64,
                            depth:  1,
                        },
                        dst,
                        0, 0,
                        metal::MTLOrigin { x: 0, y: 0, z: 0 },
                    );
                    enc.end_encoding();
                    cmd.commit();
                    ok = true;
                });
            });
        }

        ok
    }

    pub fn server_name(&self) -> Option<&str> {
        self.connected_server.as_deref()
    }
}

impl Drop for SyphonWgpuInput {
    fn drop(&mut self) {
        self.disconnect();
    }
}

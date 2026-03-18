//! wgpu ↔ Metal interop helpers
//!
//! **All `wgpu-hal` version-specific code lives here.**
//! When upgrading wgpu, edit only this module.
//!
//! Current wgpu version: 25.0

#[cfg(target_os = "macos")]
use metal::foreign_types::ForeignType;

/// Extract the underlying `metal::Device` from a wgpu device.
///
/// Returns `None` when the wgpu device is not backed by Metal (e.g. Vulkan).
#[cfg(target_os = "macos")]
pub fn extract_metal_device(device: &wgpu::Device) -> Option<metal::Device> {
    let mut result = None;
    unsafe {
        device.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_device| {
            if let Some(dev) = hal_device {
                result = Some(dev.raw_device().lock().clone());
            }
        });
    }
    result
}

/// Call `f` with the raw `MTLCommandQueue` backing a wgpu queue.
///
/// `f` is not called if the wgpu queue is not Metal-backed.
#[cfg(target_os = "macos")]
pub fn with_metal_queue<F>(queue: &wgpu::Queue, f: F)
where
    F: FnOnce(&metal::CommandQueueRef),
{
    unsafe {
        queue.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_queue| {
            if let Some(q) = hal_queue {
                let raw = q.as_raw().lock();
                f(&*raw);
            }
        });
    }
}

/// Call `f` with both the raw queue and raw texture extracted from wgpu.
///
/// `f` is not called unless both handles are Metal-backed.
#[cfg(target_os = "macos")]
pub fn with_metal_queue_and_texture<F>(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    f: F,
)
where
    F: FnOnce(&metal::CommandQueueRef, &metal::TextureRef),
{
    unsafe {
        queue.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_queue| {
            let Some(q) = hal_queue else { return };
            let raw_queue = q.as_raw().lock();

            texture.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_tex| {
                let Some(t) = hal_tex else { return };
                f(&*raw_queue, t.raw_handle());
            });
        });
    }
}

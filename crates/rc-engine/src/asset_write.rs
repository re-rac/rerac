//! **Port-only: writes to GPU-backed assets that only count when something changed.** Every `Assets::get_mut` marks the
//! asset modified, and Bevy then uploads it again (a `ShaderBuffer` through `queue.write_buffer`, which allocates a
//! staging buffer per write; a `Mesh` through the mesh allocator, its vertices and its indices): the systems that
//! rebuild a buffer every frame would upload it every frame even when the bytes are the same. [`set_buffer`] compares
//! first and copies in place.

use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;

/// Sets `h`'s contents to `bytes` unless they already are (no change, no upload); true when it wrote.
pub fn set_buffer(buffers: &mut Assets<ShaderBuffer>, h: &Handle<ShaderBuffer>, bytes: &[u8]) -> bool {
    if buffers.get(h).and_then(|b| b.data.as_deref()).is_some_and(|d| d == bytes) { return false; }
    let Some(mut b) = buffers.get_mut(h) else { return false };
    match b.data.as_mut() {
        Some(d) if d.len() == bytes.len() => d.copy_from_slice(bytes),
        _ => b.data = Some(bytes.to_vec()),
    }
    true
}

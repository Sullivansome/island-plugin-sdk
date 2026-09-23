//! Island plugin protocol — shared by host, SDK, and CLI.
//!
//! Wire format (length-prefixed):
//! ```text
//! u32 LE body_len | u8 kind | payload
//! kind 1 = JSON UTF-8 message
//! kind 2 = binary frame: u32 w | u32 h | u64 frame_id | BGRA8 pixels
//! ```

mod codec;
mod manifest;
mod messages;
mod paths;
mod permissions;

pub use codec::{FramePacket, MessageKind, read_message, write_frame, write_json};
pub use manifest::{
    CollapsedViewMode, EntryKind, ExpandedViewMode, MarketMetadata, PluginEntry, PluginManifest,
    PluginMeta, PreferredSize, ViewConfig, validate_plugin_id,
};
pub use messages::*;
pub use paths::{
    Endpoint, default_index_url, plugins_dir, read_endpoint, runtime_dir, socket_path,
    write_endpoint_file,
};
pub use permissions::{Permission, PermissionSet};

/// Host/plugin protocol major version.
pub const API_VERSION: u32 = 2;

/// The only application deep link accepted by the shared URL launcher.
/// A concrete thread UUID cannot carry prompts, query parameters, or commands.
pub fn is_codex_thread_url(url: &str) -> bool {
    let Some(id) = url.strip_prefix("codex://threads/") else {
        return false;
    };
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

/// Shared compact lyric-strip contract used by the host and stock media plugins.
pub use island_design::{
    COMPACT_HEIGHT as COMPACT_LYRIC_HEIGHT, COMPACT_WIDTH as COMPACT_LYRIC_WIDTH,
};

/// Soft caps for custom view frames (physical pixels — allow 3× Retina).
pub const MAX_FRAME_WIDTH: u32 = 2048;
pub const MAX_FRAME_HEIGHT: u32 = 1024;
pub const MAX_FRAME_BYTES: usize = (MAX_FRAME_WIDTH * MAX_FRAME_HEIGHT * 4) as usize;
pub mod website;

use serde::{Deserialize, Serialize};

use crate::API_VERSION;

/// Messages on the control channel (JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostToPlugin {
    Hello {
        api_version: u32,
        host_version: String,
        instance_id: String,
    },
    Configure {
        permissions: Vec<String>,
        data_dir: String,
    },
    SurfaceReady {
        surface_id: u64,
        w: u32,
        h: u32,
        scale: f32,
    },
    SurfaceResize {
        surface_id: u64,
        w: u32,
        h: u32,
        scale: f32,
    },
    ExpandState {
        expand: f32,
        phase: ExpandPhase,
    },
    Input {
        event: InputEvent,
    },
    Focus {
        focused: bool,
    },
    /// Reply only when OpenUrl included a request_id (API 2 opt-in extension).
    OpenUrlResult {
        request_id: u64,
        error: Option<String>,
    },
    Tick {
        time_ms: u64,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginToHost {
    HelloOk {
        api_version: u32,
        name: String,
        token: String,
    },
    Present {
        priority: u8,
        /// Older plugins keep the existing expanded behavior.
        #[serde(default)]
        mode: PresentationMode,
        #[serde(default)]
        ttl_ms: Option<u64>,
        #[serde(default)]
        sticky: bool,
        #[serde(default)]
        reason: Option<String>,
    },
    Dismiss,
    SetCollapsedMeta {
        #[serde(default)]
        leading: Option<String>,
        #[serde(default)]
        trailing: Option<String>,
        #[serde(default)]
        dot: Option<String>,
        /// Optional bounded BGRA icon, rendered beside the hardware notch.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<SurfaceFrame>,
    },
    PreferredSize {
        w: u32,
        h: u32,
    },
    SetCursor {
        name: String,
    },
    OpenUrl {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<u64>,
    },
    Log {
        level: LogLevel,
        msg: String,
    },
    /// Optional JSON-only frame (base64 BGRA) — prefer binary Frame packets.
    FrameJson {
        surface_id: u64,
        frame_id: u64,
        width: u32,
        height: u32,
        data_b64: String,
    },
}

/// Expansion intent is separate from queue lifetime and priority.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    #[default]
    Expanded,
    Compact,
    /// Expand only while the pointer is over the island; clicks never pin it open.
    Hover,
    /// Briefly expand, then retain a collapsed activity until dismissed/expired.
    Peek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpandPhase {
    Collapsed,
    Expanding,
    Expanded,
    Collapsing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputEvent {
    KeyDown {
        key: String,
    },
    KeyUp {
        key: String,
    },
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerDown {
        x: f32,
        y: f32,
        button: u8,
    },
    PointerUp {
        x: f32,
        y: f32,
        button: u8,
    },
    /// Logical-point viewport deltas: positive dx/dy scroll right/down.
    Scroll {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl HostToPlugin {
    pub fn hello(host_version: impl Into<String>, instance_id: impl Into<String>) -> Self {
        Self::Hello {
            api_version: API_VERSION,
            host_version: host_version.into(),
            instance_id: instance_id.into(),
        }
    }
}

/// Latest pixels for a plugin surface (host-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceFrame {
    pub width: u32,
    pub height: u32,
    pub frame_id: u64,
    /// BGRA8 tightly packed.
    #[serde(with = "pixel_base64")]
    pub pixels: Vec<u8>,
}

// JSON metadata icons use base64 so the maximum 96px icon fits the 64KiB
// control-message limit. Binary surface frames retain their existing encoding.
mod pixel_base64 {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer, de::Error};
    pub fn serialize<S: Serializer>(pixels: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(pixels))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(D::Error::custom)
    }
}

impl SurfaceFrame {
    pub fn validate_icon(&self) -> Result<(), String> {
        if self.width > 96 || self.height > 96 {
            return Err("collapsed icon exceeds 96 pixels".into());
        }
        self.validate()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("frame has zero size".into());
        }
        if self.width > crate::MAX_FRAME_WIDTH || self.height > crate::MAX_FRAME_HEIGHT {
            return Err("frame exceeds max dimensions".into());
        }
        let expected = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        if self.pixels.len() != expected {
            return Err(format!(
                "frame byte length {} != {}x{}x4",
                self.pixels.len(),
                self.width,
                self.height
            ));
        }
        if self.pixels.len() > crate::MAX_FRAME_BYTES {
            return Err("frame exceeds max bytes".into());
        }
        Ok(())
    }
}

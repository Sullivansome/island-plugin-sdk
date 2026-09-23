use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// `unix:/path` or `tcp:127.0.0.1:port`
    pub addr: String,
}

pub fn runtime_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("ISLAND_RUNTIME_DIR") {
        return PathBuf::from(path);
    }
    if let Some(dir) = dirs_next_runtime() {
        let p = dir.join("island");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    let p = std::env::temp_dir().join("island");
    let _ = fs::create_dir_all(&p);
    p
}

fn dirs_next_runtime() -> Option<PathBuf> {
    // Prefer XDG_RUNTIME_DIR / macOS equivalent via dirs crate patterns
    if let Ok(x) = std::env::var("XDG_RUNTIME_DIR") {
        return Some(PathBuf::from(x));
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home).join("Library/Caches"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(l) = std::env::var("LOCALAPPDATA") {
            return Some(PathBuf::from(l));
        }
    }
    None
}

pub fn plugins_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("ISLAND_PLUGINS_DIR") {
        return PathBuf::from(path);
    }
    let base = if let Some(d) = data_dir() {
        d.join("Island").join("plugins")
    } else {
        std::env::temp_dir().join("Island").join("plugins")
    };
    let _ = fs::create_dir_all(&base);
    base
}

fn data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(x) = std::env::var("XDG_DATA_HOME") {
            return Some(PathBuf::from(x));
        }
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

pub fn socket_path() -> PathBuf {
    runtime_dir().join("island.sock")
}

pub fn endpoint_file() -> PathBuf {
    runtime_dir().join("endpoint.json")
}

pub fn write_endpoint_file(endpoint: &Endpoint) -> std::io::Result<()> {
    let path = endpoint_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(endpoint).unwrap())
}

pub fn read_endpoint() -> std::io::Result<Endpoint> {
    let bytes = fs::read(endpoint_file())?;
    serde_json::from_slice(&bytes).map_err(std::io::Error::other)
}

pub fn default_index_url() -> String {
    std::env::var("ISLAND_INDEX_URL").unwrap_or_else(|_| {
        "https://raw.githubusercontent.com/Sullivansome/island-plugin-sdk/main/registry/index/v1.json".into()
    })
}

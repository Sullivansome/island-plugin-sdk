use serde::{Deserialize, Serialize};

use crate::permissions::PermissionSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub api_version: u32,
    pub plugin: PluginMeta,
    pub entry: PluginEntry,
    #[serde(default)]
    pub view: ViewConfig,
    #[serde(default)]
    pub permissions: ManifestPermissions,
    #[serde(default)]
    pub market: MarketMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketMetadata {
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    /// Empty means the package does not restrict its host OS.
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub kind: EntryKind,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub wasm_path: Option<String>,
    #[serde(default)]
    pub builtin_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Ipc,
    Wasm,
    Builtin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewConfig {
    #[serde(default = "default_expanded")]
    pub expanded: ExpandedViewMode,
    #[serde(default = "default_collapsed")]
    pub collapsed: CollapsedViewMode,
    #[serde(default)]
    pub preferred_expanded: Option<PreferredSize>,
}

fn default_expanded() -> ExpandedViewMode {
    ExpandedViewMode::Custom
}
fn default_collapsed() -> CollapsedViewMode {
    CollapsedViewMode::Metadata
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            expanded: ExpandedViewMode::Custom,
            collapsed: CollapsedViewMode::Metadata,
            preferred_expanded: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpandedViewMode {
    Custom,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollapsedViewMode {
    Custom,
    Metadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PreferredSize {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManifestPermissions {
    #[serde(default)]
    pub grant: Vec<String>,
}

impl PluginManifest {
    pub fn parse(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    pub fn permission_set(&self) -> Result<PermissionSet, String> {
        PermissionSet::from_strings(&self.permissions.grant)
    }
}

/// IDs are portable directory names, never paths or reserved builtin identities.
pub fn validate_plugin_id(id: &str) -> Result<(), String> {
    if id.len() > 255
        || id.starts_with("builtin.")
        || !id.contains('.')
        || !id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        })
    {
        return Err(format!("invalid plugin id: {id:?}"));
    }
    Ok(())
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        validate_plugin_id(&self.plugin.id)?;
        if self.permission_set()?.allows(&crate::Permission::Builtin) {
            return Err("external plugins cannot grant builtin authority".into());
        }
        let path = match self.entry.kind {
            EntryKind::Ipc => self.entry.command.as_deref(),
            EntryKind::Wasm => self.entry.wasm_path.as_deref(),
            EntryKind::Builtin => return Err("external builtin packages are unsupported".into()),
        }
        .ok_or("missing plugin entry path")?;
        // Check both separators on every OS so packages remain portable.
        if path.is_empty()
            || path.contains(':')
            || path.contains('\\')
            || !path
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != "..")
        {
            return Err("entry path must stay inside the package".into());
        }
        Ok(())
    }
}

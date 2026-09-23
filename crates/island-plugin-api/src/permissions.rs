use std::collections::BTreeSet;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Request visibility in the island queue.
    Present,
    /// Sticky present until dismiss.
    PresentSticky,
    /// Submit framebuffer custom views.
    SurfaceFramebuffer,
    /// Embed a child window (advanced).
    SurfaceEmbed,
    /// Unrestricted network.
    Net,
    /// Read plugin data dir only (always implied for ipc data path).
    FsRead,
    ClipboardRead,
    Media,
    ShellOpenUrl,
    /// Host-local only.
    Builtin,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::PresentSticky => "present.sticky",
            Self::SurfaceFramebuffer => "surface.framebuffer",
            Self::SurfaceEmbed => "surface.embed",
            Self::Net => "net",
            Self::FsRead => "fs:read",
            Self::ClipboardRead => "clipboard:read",
            Self::Media => "media",
            Self::ShellOpenUrl => "shell:open-url",
            Self::Builtin => "builtin",
        }
    }
}

impl FromStr for Permission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // net:host is treated as Net for v1
        if s.starts_with("net:") || s == "net" {
            return Ok(Self::Net);
        }
        Ok(match s {
            "present" => Self::Present,
            "present.sticky" => Self::PresentSticky,
            "surface.framebuffer" => Self::SurfaceFramebuffer,
            "surface.embed" => Self::SurfaceEmbed,
            "fs:read" => Self::FsRead,
            "clipboard:read" => Self::ClipboardRead,
            "media" => Self::Media,
            "shell:open-url" => Self::ShellOpenUrl,
            "builtin" => Self::Builtin,
            other => return Err(format!("unknown permission: {other}")),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSet {
    grants: BTreeSet<Permission>,
}

impl PermissionSet {
    pub fn new(perms: impl IntoIterator<Item = Permission>) -> Self {
        Self {
            grants: perms.into_iter().collect(),
        }
    }

    pub fn from_strings(list: &[String]) -> Result<Self, String> {
        let mut grants = BTreeSet::new();
        for s in list {
            grants.insert(s.parse()?);
        }
        // present is always required to show; auto-add if surface is requested
        if grants.contains(&Permission::SurfaceFramebuffer)
            || grants.contains(&Permission::SurfaceEmbed)
        {
            grants.insert(Permission::Present);
        }
        Ok(Self { grants })
    }

    pub fn allows(&self, p: &Permission) -> bool {
        self.grants.contains(p) || self.grants.contains(&Permission::Builtin)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Permission> {
        self.grants.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }
}

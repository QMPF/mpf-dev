use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// SDK root directory (~/.mpf-sdk)
pub fn sdk_root() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".mpf-sdk")
}

/// Path to dev.json configuration
pub fn dev_config_path() -> PathBuf {
    sdk_root().join("dev.json")
}

/// Get the current SDK directory path (junction/symlink at ~/.mpf-sdk/current)
pub fn current_link() -> PathBuf {
    sdk_root().join("current")
}

/// Path to a specific version directory
pub fn version_dir(version: &str) -> PathBuf {
    sdk_root().join(version)
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DevConfig {
    #[serde(default)]
    pub sdk_version: Option<String>,
    
    #[serde(default)]
    pub components: HashMap<String, ComponentConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentConfig {
    pub mode: ComponentMode,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qml: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<String>,
    
    /// Path to executable binary directory (for host component)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<String>,

    /// Project source root directory (contains CMakeLists.txt)
    /// Set by link (inferred) or init (from CWD). Used by reinit_all
    /// to regenerate CMakeUserPresets.json when dev.json changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentMode {
    Binary,
    Source,
}

impl DevConfig {
    pub fn load() -> Result<Self> {
        let path = dev_config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            serde_json::from_str(&content)
                .with_context(|| "Failed to parse dev.json")
        } else {
            Ok(Self::default())
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let path = dev_config_path();
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }
}

/// Get the current SDK version by reading the junction/symlink target
pub fn current_version() -> Option<String> {
    let link = current_link();
    // Try read_link first (works for both Unix symlinks and Windows junctions)
    if let Ok(target) = fs::read_link(&link) {
        return target
            .file_name()
            .map(|s| s.to_string_lossy().to_string());
    }
    // Fallback: if the directory exists but read_link fails, check if it's a real directory
    // (shouldn't normally happen, but be resilient)
    None
}

/// Set the current SDK version
pub fn set_current_version(version: &str) -> Result<()> {
    let root = sdk_root();
    fs::create_dir_all(&root)?;

    // Create "current" junction/symlink so CMAKE_PREFIX_PATH can resolve ~/.mpf-sdk/current
    let link = root.join("current");
    let target = version_dir(version);

    #[cfg(unix)]
    {
        if link.exists() || link.is_symlink() {
            let _ = fs::remove_file(&link);
        }
        let _ = std::os::unix::fs::symlink(&target, &link);
    }

    #[cfg(windows)]
    {
        // Remove existing junction/directory
        if link.exists() {
            let _ = fs::remove_dir(&link);
        }
        // Create directory junction (no admin privileges required)
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if let Err(e) = status {
            eprintln!("Warning: failed to create junction for 'current': {}", e);
        }
    }

    Ok(())
}

/// List all installed SDK versions
pub fn installed_versions() -> Vec<String> {
    let root = sdk_root();
    if !root.exists() {
        return vec![];
    }
    
    fs::read_dir(&root)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    // Filter out non-version directories
                    name.starts_with('v') || name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                })
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Known MPF components
pub const KNOWN_COMPONENTS: &[&str] = &[
    "sdk",
    "http-client",
    "ui-components",
    "host",
    "plugin-orders",
    "plugin-rules",
];

pub fn is_known_component(name: &str) -> bool {
    KNOWN_COMPONENTS.contains(&name)
}

mod setup;
mod link;
mod init;
mod run;
mod workspace;

// Re-export public command functions
pub use setup::{setup, versions, use_version};
pub use link::{link_action, unlink};
pub use init::init;
pub use run::{run, env_vars, status};
pub use workspace::{workspace_init, workspace_build, workspace_run, workspace_status};

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};
use colored::*;

use crate::config::{self, ComponentMode, DevConfig};

// ─── Shared constants ────────────────────────────────────────────────────────

const GITHUB_REPO: &str = "QMPF/mpf-release";

// ─── Path utilities ──────────────────────────────────────────────────────────

/// Normalize a path by removing .\ and .. components
fn normalize_path(p: PathBuf) -> String {
    // Try to canonicalize, fall back to string cleanup
    let result = if let Ok(canonical) = p.canonicalize() {
        canonical.to_string_lossy().to_string()
    } else {
        // Path doesn't exist yet, just clean up the string
        let s = p.to_string_lossy().to_string();
        s.replace("\\.\\", "\\").replace("/./", "/")
    };

    // Remove Windows extended path prefix (\\?\)
    if result.starts_with(r"\\?\") {
        result[4..].to_string()
    } else {
        result
    }
}

/// Infer project source root from a build output path.
///
/// Tries:
/// 1. Parent of build_path (convention: <source>/build -> <source>)
/// 2. Current working directory
///
/// Returns Some(normalized_path) if a CMakeLists.txt is found, None otherwise.
fn infer_project_root(build_path: &std::path::Path) -> Option<String> {
    // Walk up to 3 levels to find a directory containing CMakeLists.txt
    // Handles both <source>/build and <source>/build/install conventions
    let mut current = Some(build_path);
    for _ in 0..3 {
        current = current.and_then(|p| p.parent());
        if let Some(dir) = current {
            if dir.join("CMakeLists.txt").exists() {
                return Some(normalize_path(dir.to_path_buf()));
            }
        }
    }
    // Try CWD
    if let Ok(cwd) = env::current_dir() {
        if cwd.join("CMakeLists.txt").exists() {
            return Some(normalize_path(cwd));
        }
    }
    None
}

// ─── Tool detection ──────────────────────────────────────────────────────────

/// Try to detect Qt installation path
fn detect_qt_path() -> Option<String> {
    // Check environment first
    if let Ok(qt_dir) = std::env::var("QT_DIR") {
        return Some(qt_dir);
    }
    if let Ok(qt_dir) = std::env::var("Qt6_DIR") {
        return Some(qt_dir);
    }

    // Check common paths
    #[cfg(windows)]
    {
        let common_paths = [
            "C:\\Qt\\6.8.3\\mingw_64",
            "C:\\Qt\\6.8.2\\mingw_64",
            "C:\\Qt\\6.8.1\\mingw_64",
            "C:\\Qt\\6.8.0\\mingw_64",
        ];
        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }
    }

    #[cfg(unix)]
    {
        let common_paths = ["/opt/qt6", "/usr/local/Qt-6.8.3", "/usr/lib/qt6"];
        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }
    }

    None
}

/// Try to detect MinGW compiler paths from Qt installation
/// Navigates from Qt path (e.g. C:/Qt/6.8.3/mingw_64) up to C:/Qt/Tools/,
/// finds the newest mingw* directory with gcc.exe and g++.exe
fn detect_mingw_path(qt_path: &str) -> Option<(String, String)> {
    let qt = std::path::Path::new(qt_path);
    // Navigate up to Qt root: C:/Qt/6.8.3/mingw_64 -> C:/Qt
    let qt_root = qt.parent()?.parent()?;
    let tools_dir = qt_root.join("Tools");
    if !tools_dir.exists() {
        return None;
    }

    let mut candidates: Vec<_> = fs::read_dir(&tools_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("mingw") && e.path().is_dir()
        })
        .collect();

    // Sort descending so newest version is first
    candidates.sort_by(|a, b| {
        b.file_name()
            .to_string_lossy()
            .cmp(&a.file_name().to_string_lossy())
    });

    for entry in candidates {
        let bin = entry.path().join("bin");
        let gcc = bin.join("gcc.exe");
        let gpp = bin.join("g++.exe");
        if gcc.exists() && gpp.exists() {
            let gcc_str = gcc.to_string_lossy().replace('\\', "/");
            let gpp_str = gpp.to_string_lossy().replace('\\', "/");
            return Some((gcc_str, gpp_str));
        }
    }
    None
}

/// Map component name to CMake package directory variable name
fn component_cmake_dir_var(component_name: &str) -> Option<&'static str> {
    match component_name {
        "ui-components" => Some("MPFUIComponents_DIR"),
        "http-client" => Some("MPFHttpClient_DIR"),
        _ => None,
    }
}

// ─── Environment path builder ────────────────────────────────────────────────

/// Build environment path strings
/// Returns: (sdk_root, lib_path, qml_path, qt_plugin_path, mpf_plugin_path, host_path, host_qml_path)
fn build_env_paths() -> Result<(String, String, String, String, String, PathBuf, Option<String>)> {
    let dev_config = DevConfig::load().unwrap_or_default();
    let sdk = config::current_link();

    if !sdk.exists() {
        bail!("No SDK version set. Run 'mpf-dev setup' first.");
    }

    // SDK root path (used by mpf-host to find default paths)
    let sdk_root = sdk.to_string_lossy().to_string();

    let mut lib_paths: Vec<String> = Vec::new();
    let mut qml_paths: Vec<String> = Vec::new();
    let mut plugin_paths: Vec<String> = Vec::new();
    let mut mpf_plugin_paths: Vec<String> = Vec::new();
    let mut host_bin_override: Option<String> = None;
    let mut host_qml_override: Option<String> = None;

    // Source components first (higher priority)
    for (name, comp) in &dev_config.components {
        if comp.mode == ComponentMode::Source {
            if let Some(lib) = &comp.lib {
                lib_paths.push(lib.clone());

                // On Windows, DLLs may be in a sibling bin/ directory
                // (MinGW installs: RUNTIME→bin/, ARCHIVE→lib/)
                let lib_path = std::path::Path::new(lib.as_str());
                if let Some(parent) = lib_path.parent() {
                    let sibling_bin = parent.join("bin");
                    if sibling_bin.is_dir() {
                        let bin_str = sibling_bin.to_string_lossy().replace('\\', "/");
                        if !lib_paths.contains(&bin_str) {
                            lib_paths.push(bin_str);
                        }
                    }
                }

                // For plugin components (not host/sdk), also add to MPF_PLUGIN_PATH
                if name != "host" && name != "sdk" {
                    mpf_plugin_paths.push(lib.clone());
                }
            }
            if let Some(qml) = &comp.qml {
                qml_paths.push(qml.clone());
            }
            if let Some(plugin) = &comp.plugin {
                plugin_paths.push(plugin.clone());
            }

            // Check for host component bin/qml override
            if name == "host" {
                if let Some(bin) = &comp.bin {
                    host_bin_override = Some(bin.clone());
                }
                if let Some(qml) = &comp.qml {
                    host_qml_override = Some(qml.clone());
                }
            }

            // Debug: show which components are in source mode
            eprintln!("{} Using source: {}", "->".cyan(), name);
        }
    }

    // SDK paths as fallback (include both lib/ and bin/ for Windows DLL discovery)
    lib_paths.push(sdk.join("lib").to_string_lossy().to_string());
    lib_paths.push(sdk.join("bin").to_string_lossy().to_string());
    qml_paths.push(sdk.join("qml").to_string_lossy().to_string());
    plugin_paths.push(sdk.join("plugins").to_string_lossy().to_string());

    let sep = if cfg!(windows) { ";" } else { ":" };

    // Use linked host bin if available, otherwise use SDK's mpf-host
    let host_exe_name = if cfg!(windows) {
        "mpf-host.exe"
    } else {
        "mpf-host"
    };
    let host_path = if let Some(bin_dir) = host_bin_override {
        let linked_host = PathBuf::from(&bin_dir).join(host_exe_name);
        eprintln!(
            "{} Using linked host: {}",
            "->".cyan(),
            linked_host.display()
        );
        linked_host
    } else {
        sdk.join("bin").join(host_exe_name)
    };

    Ok((
        sdk_root,
        lib_paths.join(sep),
        qml_paths.join(sep),
        plugin_paths.join(sep),
        mpf_plugin_paths.join(sep),
        host_path,
        host_qml_override,
    ))
}

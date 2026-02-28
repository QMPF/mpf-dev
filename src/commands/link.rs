use anyhow::{bail, Result};
use colored::*;
use std::env;
use std::path::PathBuf;

use crate::config::{
    self, ComponentConfig, ComponentMode, DevConfig, KNOWN_COMPONENTS,
};
use crate::LinkAction;

use super::{normalize_path, infer_project_root};
use super::init::reinit_all;

/// New link action handler - dispatches to appropriate link function
pub fn link_action(action: LinkAction) -> Result<()> {
    match action {
        LinkAction::Plugin { name, path } => link_plugin(&name, &path),
        LinkAction::Host { path } => link_host(&path),
        LinkAction::Component { name, path } => link_component(&name, &path),
        LinkAction::Sdk { path } => link_sdk(&path),
        LinkAction::Manual {
            name,
            lib,
            qml,
            plugin,
            headers,
            bin,
        } => link(&name, lib, qml, plugin, headers, bin, None),
    }
}

/// Resolve a path argument to an absolute, normalized PathBuf
fn resolve_abs(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    let abs = if p.is_absolute() {
        p
    } else {
        env::current_dir().unwrap_or_default().join(p)
    };
    PathBuf::from(normalize_path(abs))
}

/// Link a plugin - auto-derives lib, qml, plugin paths from build directory
fn link_plugin(name: &str, path: &str) -> Result<()> {
    let abs_path = resolve_abs(path);

    // Auto-derive paths from plugin build output
    let lib_path = normalize_path(abs_path.join("plugins"));
    let qml_path = normalize_path(abs_path.join("qml"));
    let plugin_path = normalize_path(abs_path.clone());

    println!("{} Linking plugin '{}'", "->".cyan(), name);
    println!("  Build root: {}", abs_path.display());
    println!("  lib (plugins): {}", lib_path);
    println!("  qml: {}", qml_path);

    let mut dev_config = DevConfig::load().unwrap_or_default();

    // Store as "plugin-<name>" for clarity
    let component_name = if name.starts_with("plugin-") {
        name.to_string()
    } else {
        format!("plugin-{}", name)
    };

    dev_config.components.insert(
        component_name.clone(),
        ComponentConfig {
            mode: ComponentMode::Source,
            lib: Some(lib_path),
            qml: Some(qml_path),
            plugin: Some(plugin_path),
            headers: None,
            bin: None,
            root: infer_project_root(&abs_path),
        },
    );
    dev_config.save()?;
    reinit_all(&dev_config)?;

    println!("{} Plugin '{}' linked", "✓".green(), component_name);
    Ok(())
}

/// Link host - auto-derives bin, qml paths from build directory
fn link_host(path: &str) -> Result<()> {
    let abs_path = resolve_abs(path);

    let host_exe = if cfg!(windows) {
        "mpf-host.exe"
    } else {
        "mpf-host"
    };

    // Auto-derive bin path
    let bin_path = if abs_path.join("bin").join(host_exe).exists() {
        normalize_path(abs_path.join("bin"))
    } else if abs_path.join(host_exe).exists() {
        normalize_path(abs_path.clone())
    } else {
        normalize_path(abs_path.join("bin"))
    };

    // Auto-derive qml path
    let qml_path = if abs_path.join("qml").exists() {
        normalize_path(abs_path.join("qml"))
    } else {
        normalize_path(abs_path.clone())
    };

    println!("{} Linking host", "->".cyan());
    println!("  Build root: {}", abs_path.display());
    println!("  bin: {}", bin_path);
    println!("  qml: {}", qml_path);

    let mut dev_config = DevConfig::load().unwrap_or_default();
    dev_config.components.insert(
        "host".to_string(),
        ComponentConfig {
            mode: ComponentMode::Source,
            lib: None,
            qml: Some(qml_path),
            plugin: None,
            headers: None,
            bin: Some(bin_path),
            root: infer_project_root(&abs_path),
        },
    );
    dev_config.save()?;
    reinit_all(&dev_config)?;

    println!("{} Host linked", "✓".green());
    Ok(())
}

/// Link a local SDK install directory for development
///
/// The path should point to the cmake install prefix of a locally built SDK,
/// which must contain lib/cmake/MPF/MPFConfig.cmake and include/mpf/.
/// This overrides ~/.mpf-sdk/current when generating CMakeUserPresets.json.
fn link_sdk(path: &str) -> Result<()> {
    let abs_path = resolve_abs(path);

    // Validate: must contain lib/cmake/MPF/MPFConfig.cmake
    let cmake_config = abs_path
        .join("lib")
        .join("cmake")
        .join("MPF")
        .join("MPFConfig.cmake");
    if !cmake_config.exists() {
        bail!(
            "Invalid SDK install path: {} not found.\n\
             Make sure to run 'cmake --install build --prefix <path>' first.",
            cmake_config.display()
        );
    }

    // Validate: must contain include/mpf/
    let include_dir = abs_path.join("include").join("mpf");
    if !include_dir.exists() {
        bail!(
            "Invalid SDK install path: {} not found.",
            include_dir.display()
        );
    }

    let lib_path = normalize_path(abs_path.join("lib"));
    let headers_path = normalize_path(abs_path.join("include"));

    println!("{} Linking SDK", "->".cyan());
    println!("  Install root: {}", abs_path.display());
    println!("  lib (cmake configs): {}", lib_path);
    println!("  headers: {}", headers_path);

    let mut dev_config = DevConfig::load().unwrap_or_default();
    dev_config.components.insert(
        "sdk".to_string(),
        ComponentConfig {
            mode: ComponentMode::Source,
            lib: Some(lib_path),
            qml: None,
            plugin: None,
            headers: Some(headers_path),
            bin: None,
            root: None, // SDK has no project root to re-init
        },
    );
    dev_config.save()?;
    reinit_all(&dev_config)?;

    println!("{} SDK linked for local development", "✓".green());
    Ok(())
}

/// Link a library component (ui-components, http-client, etc.)
fn link_component(name: &str, path: &str) -> Result<()> {
    let abs_path = resolve_abs(path);

    // Auto-derive paths based on component type
    let lib_path = if abs_path.join("lib").exists() {
        Some(normalize_path(abs_path.join("lib")))
    } else if abs_path.join("bin").exists() {
        // Windows DLLs often go in bin/
        Some(normalize_path(abs_path.join("bin")))
    } else {
        Some(normalize_path(abs_path.clone()))
    };

    let qml_path = if abs_path.join("qml").exists() {
        Some(normalize_path(abs_path.join("qml")))
    } else {
        None
    };

    let headers_path = if abs_path.join("include").exists() {
        Some(normalize_path(abs_path.join("include")))
    } else {
        None
    };

    println!("{} Linking component '{}'", "->".cyan(), name);
    println!("  Build root: {}", abs_path.display());
    if let Some(ref p) = lib_path {
        println!("  lib: {}", p);
    }
    if let Some(ref p) = qml_path {
        println!("  qml: {}", p);
    }
    if let Some(ref p) = headers_path {
        println!("  headers: {}", p);
    }

    let mut dev_config = DevConfig::load().unwrap_or_default();
    dev_config.components.insert(
        name.to_string(),
        ComponentConfig {
            mode: ComponentMode::Source,
            lib: lib_path,
            qml: qml_path,
            plugin: None,
            headers: headers_path,
            bin: None,
            root: infer_project_root(&abs_path),
        },
    );
    dev_config.save()?;
    reinit_all(&dev_config)?;

    println!("{} Component '{}' linked", "✓".green(), name);
    Ok(())
}

/// Link command: register component for source development (legacy interface)
fn link(
    component: &str,
    lib: Option<String>,
    qml: Option<String>,
    plugin: Option<String>,
    headers: Option<String>,
    bin: Option<String>,
    host: Option<String>,
) -> Result<()> {
    // Warn if unknown component
    if !config::is_known_component(component) {
        println!(
            "{} Unknown component '{}'. Known components: {}",
            "Warning:".yellow(),
            component,
            KNOWN_COMPONENTS.join(", ")
        );
    }

    // Warn if bin is used for non-host component
    if bin.is_some() && component != "host" {
        println!(
            "{} --bin option is typically used for 'host' component only",
            "Note:".yellow()
        );
    }

    let mut dev_config = DevConfig::load().unwrap_or_default();

    // Resolve paths to absolute and normalize (remove .\ and ..)
    let cwd = env::current_dir()?;
    let resolve = |p: Option<String>| -> Option<String> {
        p.map(|s| {
            let path = PathBuf::from(&s);
            if path.is_absolute() {
                normalize_path(path)
            } else {
                normalize_path(cwd.join(path))
            }
        })
    };

    // If --plugin is specified, automatically derive lib and qml paths
    let (derived_lib, derived_qml) = if let Some(ref plugin_root) = plugin {
        let plugin_path = PathBuf::from(plugin_root);
        let abs_plugin_root = PathBuf::from(normalize_path(if plugin_path.is_absolute() {
            plugin_path
        } else {
            cwd.join(plugin_path)
        }));

        let lib_path = normalize_path(abs_plugin_root.join("plugins"));
        let qml_path = normalize_path(abs_plugin_root.join("qml"));

        println!(
            "{} --plugin specified, auto-deriving paths from build root:",
            "Info:".cyan()
        );
        println!("  -> lib (plugins): {}", lib_path);
        println!("  -> qml: {}", qml_path);

        (Some(lib_path), Some(qml_path))
    } else {
        (None, None)
    };

    // If --host is specified, automatically derive bin and qml paths
    let (derived_bin, derived_host_qml) = if let Some(ref host_root) = host {
        let host_path = PathBuf::from(host_root);
        let abs_host_root = PathBuf::from(normalize_path(if host_path.is_absolute() {
            host_path
        } else {
            cwd.join(host_path)
        }));

        let host_exe = if cfg!(windows) {
            "mpf-host.exe"
        } else {
            "mpf-host"
        };

        let bin_path = if abs_host_root.join("bin").join(host_exe).exists() {
            normalize_path(abs_host_root.join("bin"))
        } else if abs_host_root.join(host_exe).exists() {
            normalize_path(abs_host_root.clone())
        } else {
            normalize_path(abs_host_root.join("bin"))
        };

        let qml_path = if abs_host_root.join("qml").exists() {
            normalize_path(abs_host_root.join("qml"))
        } else {
            normalize_path(abs_host_root.clone())
        };

        println!(
            "{} --host specified, auto-deriving paths from build root:",
            "Info:".cyan()
        );
        println!("  -> bin: {}", bin_path);
        println!("  -> qml: {}", qml_path);

        (Some(bin_path), Some(qml_path))
    } else {
        (None, None)
    };

    // Use explicit options if provided, otherwise use derived paths
    let final_lib = resolve(lib).or(derived_lib);
    let final_qml = resolve(qml).or(derived_host_qml).or(derived_qml);
    let final_bin = resolve(bin).or(derived_bin);

    let comp_config = ComponentConfig {
        mode: ComponentMode::Source,
        lib: final_lib,
        qml: final_qml,
        plugin: resolve(plugin),
        headers: resolve(headers),
        bin: final_bin,
        root: None, // Manual link — user can run init to set root
    };

    dev_config
        .components
        .insert(component.to_string(), comp_config.clone());
    dev_config.save()?;
    reinit_all(&dev_config)?;

    println!(
        "{} Component '{}' linked for source development",
        "✓".green(),
        component
    );

    if let Some(bin) = &comp_config.bin {
        println!("  bin: {}", bin);
    }
    if let Some(lib) = &comp_config.lib {
        println!("  lib: {}", lib);
    }
    if let Some(qml) = &comp_config.qml {
        println!("  qml: {}", qml);
    }
    if let Some(plugin) = &comp_config.plugin {
        println!("  plugin (build root): {}", plugin);
    }
    if let Some(headers) = &comp_config.headers {
        println!("  headers: {}", headers);
    }

    Ok(())
}

/// Unlink command: remove component from source development
pub fn unlink(component: &str) -> Result<()> {
    let mut dev_config = DevConfig::load()?;

    if component == "all" {
        let count = dev_config.components.len();
        dev_config.components.clear();
        dev_config.save()?;
        reinit_all(&dev_config)?;
        println!("{} Unlinked {} component(s)", "✓".green(), count);
        return Ok(());
    }

    // Try exact match first
    if dev_config.components.remove(component).is_some() {
        dev_config.save()?;
        reinit_all(&dev_config)?;
        println!("{} Component '{}' unlinked", "✓".green(), component);
        return Ok(());
    }

    // Try with plugin- prefix
    let with_prefix = format!("plugin-{}", component);
    if dev_config.components.remove(&with_prefix).is_some() {
        dev_config.save()?;
        reinit_all(&dev_config)?;
        println!("{} Plugin '{}' unlinked", "✓".green(), component);
        return Ok(());
    }

    println!(
        "{} Component '{}' was not linked",
        "Note:".yellow(),
        component
    );
    Ok(())
}

use anyhow::{bail, Result};
use colored::*;
use std::env;
use std::process::Command;

use crate::config::{self, ComponentConfig, ComponentMode, DevConfig};

use super::{build_env_paths, detect_qt_path};

/// Status command: show current configuration
pub fn status() -> Result<()> {
    let dev_config = DevConfig::load().unwrap_or_default();
    let current = config::current_version();
    let sdk_root = config::sdk_root();

    println!("{}", "MPF Development Environment Status".bold().cyan());
    println!();

    // SDK info
    println!("{}", "📦 SDK".bold());
    println!("  Root: {}", sdk_root.display());
    if let Some(v) = &current {
        println!("  Version: {}", v.green());
    } else {
        println!("  Version: {}", "not set".red());
    }
    if let Some(sdk_comp) = dev_config.components.get("sdk") {
        if sdk_comp.mode == ComponentMode::Source {
            if let Some(lib) = &sdk_comp.lib {
                let install_root = std::path::Path::new(lib.as_str())
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                println!(
                    "  Local: {} {}",
                    install_root.green(),
                    "(overrides current)".dimmed()
                );
            }
        }
    }
    println!();

    // Group components by type
    let mut host: Option<(&String, &ComponentConfig)> = None;
    let mut plugins: Vec<(&String, &ComponentConfig)> = Vec::new();
    let mut libs: Vec<(&String, &ComponentConfig)> = Vec::new();

    for (name, comp) in &dev_config.components {
        if name == "host" {
            host = Some((name, comp));
        } else if name.starts_with("plugin-") || name.contains("plugin") {
            plugins.push((name, comp));
        } else {
            libs.push((name, comp));
        }
    }

    // Host section
    println!("{}", "🖥️  Host".bold());
    if let Some((_, comp)) = host {
        if let Some(bin) = &comp.bin {
            println!("  {} bin: {}", "✓".green(), bin);
        }
        if let Some(qml) = &comp.qml {
            println!("    qml: {}", qml);
        }
    } else {
        println!("  {} Not linked", "○".dimmed());
        println!("  {}", "mpf-dev link host <build-path>".dimmed());
    }
    println!();

    // Plugins section
    println!("{}", "🔌 Plugins".bold());
    if plugins.is_empty() {
        println!("  {} None linked", "○".dimmed());
        println!(
            "  {}",
            "mpf-dev link plugin <name> <build-path>".dimmed()
        );
    } else {
        for (name, comp) in &plugins {
            let display_name = name.strip_prefix("plugin-").unwrap_or(name);
            println!("  {} {}", "✓".green(), display_name.bold());
            if let Some(lib) = &comp.lib {
                println!("    lib: {}", lib);
            }
            if let Some(qml) = &comp.qml {
                println!("    qml: {}", qml);
            }
        }
    }
    println!();

    // Libraries section
    println!("{}", "📚 Libraries".bold());
    if libs.is_empty() {
        println!("  {} None linked", "○".dimmed());
        println!(
            "  {}",
            "mpf-dev link component <name> <build-path>".dimmed()
        );
    } else {
        for (name, comp) in &libs {
            println!("  {} {}", "✓".green(), name.bold());
            if let Some(lib) = &comp.lib {
                println!("    lib: {}", lib);
            }
            if let Some(qml) = &comp.qml {
                println!("    qml: {}", qml);
            }
            if let Some(headers) = &comp.headers {
                println!("    headers: {}", headers);
            }
        }
    }
    println!();

    // Config file location
    println!("{}", "📝 Config".bold());
    println!("  {}", config::dev_config_path().display());

    Ok(())
}

/// Env command: print environment variables
pub fn env_vars() -> Result<()> {
    let (sdk_root, lib_path, qml_path, plugin_path, mpf_plugin_path, _host_path, host_qml_path) =
        build_env_paths()?;

    println!("{}", "# MPF Development Environment".bold().cyan());
    println!("{}", "# Add these to your shell or IDE:".dimmed());
    println!();

    // Detect Qt path from common locations
    let qt_hint = detect_qt_path();

    #[cfg(unix)]
    {
        println!("{}", "# === Unix/Linux/macOS ===".green());
        println!("export MPF_SDK_ROOT=\"{}\"", sdk_root);
        if let Some(ref qt) = qt_hint {
            println!("export CMAKE_PREFIX_PATH=\"{};{}\"", qt, sdk_root);
        } else {
            println!(
                "export CMAKE_PREFIX_PATH=\"$QT_DIR;{}\"  # Set QT_DIR to your Qt path",
                sdk_root
            );
        }
        println!("export QML_IMPORT_PATH=\"{}\"", qml_path);
        println!("export LD_LIBRARY_PATH=\"{}\"", lib_path);
        println!("export QT_PLUGIN_PATH=\"{}\"", plugin_path);
        if !mpf_plugin_path.is_empty() {
            println!("export MPF_PLUGIN_PATH=\"{}\"", mpf_plugin_path);
        }
        if let Some(ref hqp) = host_qml_path {
            println!("export MPF_QML_PATH=\"{}\"", hqp);
        }
    }

    #[cfg(windows)]
    {
        println!("{}", "# === Windows (CMD) ===".green());
        println!("set MPF_SDK_ROOT={}", sdk_root);
        if let Some(ref qt) = qt_hint {
            println!("set CMAKE_PREFIX_PATH={};{}", qt, sdk_root);
        } else {
            println!(
                "set CMAKE_PREFIX_PATH=C:\\Qt\\6.8.3\\mingw_64;{}",
                sdk_root
            );
        }
        println!("set QML_IMPORT_PATH={}", qml_path);
        println!("set PATH={};%PATH%", lib_path);
        println!("set QT_PLUGIN_PATH={}", plugin_path);
        if !mpf_plugin_path.is_empty() {
            println!("set MPF_PLUGIN_PATH={}", mpf_plugin_path);
        }
        if let Some(ref hqp) = host_qml_path {
            println!("set MPF_QML_PATH={}", hqp);
        }

        println!();
        println!("{}", "# === Windows (PowerShell) ===".green());
        println!("$env:MPF_SDK_ROOT=\"{}\"", sdk_root);
        if let Some(ref qt) = qt_hint {
            println!("$env:CMAKE_PREFIX_PATH=\"{};{}\"", qt, sdk_root);
        } else {
            println!(
                "$env:CMAKE_PREFIX_PATH=\"C:\\Qt\\6.8.3\\mingw_64;{}\"",
                sdk_root
            );
        }
        println!("$env:QML_IMPORT_PATH=\"{}\"", qml_path);
        println!("$env:PATH=\"{};$env:PATH\"", lib_path);
        println!("$env:QT_PLUGIN_PATH=\"{}\"", plugin_path);
        if !mpf_plugin_path.is_empty() {
            println!("$env:MPF_PLUGIN_PATH=\"{}\"", mpf_plugin_path);
        }
        if let Some(ref hqp) = host_qml_path {
            println!("$env:MPF_QML_PATH=\"{}\"", hqp);
        }
    }

    println!();
    println!("{}", "# Then configure CMake:".dimmed());
    println!(
        "{}",
        "#   cmake -B build -G \"MinGW Makefiles\"  # Windows".dimmed()
    );
    println!(
        "{}",
        "#   cmake -B build -G Ninja                # Linux".dimmed()
    );

    Ok(())
}

/// Run command: execute mpf-host with development overrides
pub fn run(debug: bool, args: Vec<String>) -> Result<()> {
    let current = config::current_link();
    if !current.exists() {
        bail!("No SDK version set. Run `mpf-dev setup` first.");
    }

    let (sdk_root, lib_path, qml_path, plugin_path, mpf_plugin_path, host_path, host_qml_path) =
        build_env_paths()?;

    if !host_path.exists() {
        bail!("mpf-host not found at: {}", host_path.display());
    }

    if debug {
        println!("{}", "Running with development overrides:".dimmed());
        println!("  MPF_SDK_ROOT={}", sdk_root);
        #[cfg(unix)]
        println!("  LD_LIBRARY_PATH={}", lib_path);
        #[cfg(windows)]
        println!("  PATH={}", lib_path);
        println!("  QML_IMPORT_PATH={}", qml_path);
        println!("  QT_PLUGIN_PATH={}", plugin_path);
        if !mpf_plugin_path.is_empty() {
            println!("  MPF_PLUGIN_PATH={}", mpf_plugin_path);
        }
        if let Some(ref hqp) = host_qml_path {
            println!("  MPF_QML_PATH={}", hqp);
        }
        println!();
    }

    let mut cmd = Command::new(&host_path);
    cmd.args(&args);

    // MPF_SDK_ROOT tells mpf-host where the SDK is installed
    cmd.env("MPF_SDK_ROOT", &sdk_root);

    #[cfg(unix)]
    {
        cmd.env("LD_LIBRARY_PATH", &lib_path);
    }

    #[cfg(windows)]
    {
        let current_path = env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("{};{}", lib_path, current_path));
    }

    cmd.env("QML_IMPORT_PATH", &qml_path);
    cmd.env("QT_PLUGIN_PATH", &plugin_path);

    // Set MPF_PLUGIN_PATH for mpf-host to discover linked plugins
    if !mpf_plugin_path.is_empty() {
        cmd.env("MPF_PLUGIN_PATH", &mpf_plugin_path);
    }

    // Set MPF_QML_PATH to override host's QML base path when host is linked
    if let Some(ref hqp) = host_qml_path {
        cmd.env("MPF_QML_PATH", hqp);
    }

    let status = cmd.status()?;

    std::process::exit(status.code().unwrap_or(1));
}

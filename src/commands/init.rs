use anyhow::{bail, Context, Result};
use colored::*;
use std::env;
use std::fs;

use crate::config::{self, ComponentMode, DevConfig};

use super::{
    component_cmake_dir_var, detect_mingw_path, detect_qt_path, normalize_path,
};

/// Generate CMakeUserPresets.json for a project directory.
///
/// Pure logic — no interactive output. Always wipes the entire build/
/// directory so stale artifacts and cmake cache never linger.
///
/// Returns Ok(true) if generated, Ok(false) if skipped (no CMakeLists.txt).
fn generate_user_presets(
    project_dir: &std::path::Path,
    dev_config: &DevConfig,
    qt_path_fwd: &str,
    gcc: &str,
    gpp: &str,
) -> Result<bool> {
    // Skip if not a CMake project
    if !project_dir.join("CMakeLists.txt").exists() {
        return Ok(false);
    }

    // Clean entire build directory — removes stale artifacts and cmake cache
    let build_dir = project_dir.join("build");
    if build_dir.exists() {
        let _ = fs::remove_dir_all(&build_dir);
    }

    // SDK current path
    let sdk_current = config::current_link();
    let sdk_current_str = sdk_current.to_string_lossy().replace('\\', "/");

    // Build CMAKE_PREFIX_PATH — if SDK is linked locally, prepend it
    let mut prefix_parts: Vec<String> = Vec::new();

    if let Some(sdk_comp) = dev_config.components.get("sdk") {
        if sdk_comp.mode == ComponentMode::Source {
            if let Some(lib_path) = &sdk_comp.lib {
                let sdk_local = std::path::Path::new(lib_path)
                    .parent()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                if !sdk_local.is_empty() {
                    prefix_parts.push(sdk_local);
                }
            }
        }
    }

    prefix_parts.push(qt_path_fwd.to_string());
    prefix_parts.push(sdk_current_str.clone());

    // Append linked library component install paths (not plugins, not host)
    for (name, comp) in &dev_config.components {
        if comp.mode != ComponentMode::Source {
            continue;
        }
        // Skip special components and those with dedicated CMake _DIR variables
        if name == "sdk" || name == "host" || name.starts_with("plugin-") {
            continue;
        }
        if component_cmake_dir_var(name).is_some() {
            continue;
        }
        // Add lib parent as cmake prefix path (the install root)
        if let Some(lib_path) = &comp.lib {
            let lib_parent = std::path::Path::new(lib_path)
                .parent()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if !lib_parent.is_empty() && !prefix_parts.contains(&lib_parent) {
                prefix_parts.push(lib_parent);
            }
        }
    }

    let cmake_prefix_path = prefix_parts.join(";");

    // Build QML_IMPORT_PATH parts and package dir variables
    let mut qml_parts: Vec<String> = Vec::new();
    let mut extra_cache_vars: Vec<(String, String)> = Vec::new();

    for (name, comp) in &dev_config.components {
        if comp.mode != ComponentMode::Source {
            continue;
        }
        if let Some(qml) = &comp.qml {
            let qml_fwd = qml.replace('\\', "/");
            if !qml_parts.contains(&qml_fwd) {
                qml_parts.push(qml_fwd);
            }
        }
        if let Some(var_name) = component_cmake_dir_var(name) {
            let build_root = comp
                .lib
                .as_ref()
                .and_then(|p| {
                    std::path::Path::new(p)
                        .parent()
                        .map(|pp| pp.to_string_lossy().replace('\\', "/"))
                })
                .or_else(|| {
                    comp.headers.as_ref().and_then(|p| {
                        std::path::Path::new(p)
                            .parent()
                            .map(|pp| pp.to_string_lossy().replace('\\', "/"))
                    })
                });
            if let Some(root) = build_root {
                extra_cache_vars.push((var_name.to_string(), root));
            }
        }
    }

    qml_parts.push(format!("{}/qml", sdk_current_str));
    qml_parts.push(format!("{}/qml", qt_path_fwd));
    let qml_import_path = qml_parts.join(";");

    // Build JSON
    let mut dev_cache = serde_json::Map::new();
    dev_cache.insert(
        "CMAKE_BUILD_TYPE".into(),
        serde_json::Value::String("Debug".into()),
    );
    dev_cache.insert(
        "CMAKE_C_COMPILER".into(),
        serde_json::Value::String(gcc.to_string()),
    );
    dev_cache.insert(
        "CMAKE_CXX_COMPILER".into(),
        serde_json::Value::String(gpp.to_string()),
    );
    dev_cache.insert(
        "CMAKE_PREFIX_PATH".into(),
        serde_json::Value::String(cmake_prefix_path.clone()),
    );
    dev_cache.insert(
        "CMAKE_EXPORT_COMPILE_COMMANDS".into(),
        serde_json::Value::String("ON".into()),
    );
    dev_cache.insert(
        "QML_IMPORT_PATH".into(),
        serde_json::Value::String(qml_import_path.clone()),
    );
    for (var_name, dir_path) in &extra_cache_vars {
        dev_cache.insert(
            var_name.clone(),
            serde_json::Value::String(dir_path.clone()),
        );
    }

    let mut release_cache = serde_json::Map::new();
    release_cache.insert(
        "CMAKE_BUILD_TYPE".into(),
        serde_json::Value::String("Release".into()),
    );
    release_cache.insert(
        "CMAKE_C_COMPILER".into(),
        serde_json::Value::String(gcc.to_string()),
    );
    release_cache.insert(
        "CMAKE_CXX_COMPILER".into(),
        serde_json::Value::String(gpp.to_string()),
    );
    release_cache.insert(
        "CMAKE_PREFIX_PATH".into(),
        serde_json::Value::String(cmake_prefix_path),
    );
    release_cache.insert(
        "CMAKE_EXPORT_COMPILE_COMMANDS".into(),
        serde_json::Value::String("ON".into()),
    );
    release_cache.insert(
        "QML_IMPORT_PATH".into(),
        serde_json::Value::String(qml_import_path),
    );
    for (var_name, dir_path) in &extra_cache_vars {
        release_cache.insert(
            var_name.clone(),
            serde_json::Value::String(dir_path.clone()),
        );
    }

    let presets = serde_json::json!({
        "version": 6,
        "configurePresets": [
            {
                "name": "dev",
                "inherits": "base",
                "displayName": "MPF Dev",
                "cacheVariables": serde_json::Value::Object(dev_cache)
            },
            {
                "name": "release",
                "inherits": "base",
                "displayName": "MPF Release",
                "binaryDir": "${sourceDir}/build-release",
                "cacheVariables": serde_json::Value::Object(release_cache)
            }
        ],
        "buildPresets": [
            {"name": "dev", "configurePreset": "dev"},
            {"name": "release", "configurePreset": "release"}
        ]
    });

    let output_path = project_dir.join("CMakeUserPresets.json");
    let content = serde_json::to_string_pretty(&presets)?;
    fs::write(&output_path, &content)
        .with_context(|| format!("写入 {} 失败", output_path.display()))?;

    Ok(true)
}

/// Re-init all projects that have a known root directory.
///
/// Called after link/unlink to propagate dev.json changes to all
/// CMakeUserPresets.json files. Silently skips projects whose root
/// no longer exists.
pub(super) fn reinit_all(dev_config: &DevConfig) -> Result<()> {
    // Collect roots, deduplicate by normalized path (multiple components can
    // share the same project root, e.g. plugin-lib and plugin-lib-qml).
    let mut seen = std::collections::HashSet::new();
    let roots: Vec<&str> = dev_config
        .components
        .values()
        .filter_map(|c| c.root.as_deref())
        .filter(|r| seen.insert(r.to_lowercase().replace('\\', "/")))
        .collect();

    if roots.is_empty() {
        return Ok(());
    }

    // Detect Qt/MinGW once for all projects
    let qt_path = match detect_qt_path() {
        Some(p) => p,
        None => return Ok(()), // Can't detect Qt — skip silently
    };
    let qt_path_fwd = qt_path.replace('\\', "/");
    let (gcc, gpp) = match detect_mingw_path(&qt_path) {
        Some(p) => p,
        None => return Ok(()),
    };

    println!("{} 正在重新初始化 {} 个项目...", "→".cyan(), roots.len());

    let mut updated = 0u32;
    for root in &roots {
        let path = std::path::Path::new(root);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string());

        if !path.exists() {
            println!("  {} {} （目录不存在）", "⚠".yellow(), name);
            continue;
        }
        match generate_user_presets(path, dev_config, &qt_path_fwd, &gcc, &gpp) {
            Ok(true) => {
                println!("  {} {}", "✓".green(), name);
                updated += 1;
            }
            Ok(false) => {
                println!("  {} {} （无 CMakeLists.txt）", "⚠".yellow(), name);
            }
            Err(e) => {
                println!("  {} {} — {}", "✗".red(), name, e);
            }
        }
    }

    if updated > 0 {
        println!(
            "\n{} {} 个项目已重新初始化，构建目录已清空。",
            "✓".green(),
            updated,
        );
        println!(
            "  {} 请关闭 Qt Creator 并重新打开，然后重新执行构建。",
            "!".yellow().bold()
        );
    }

    Ok(())
}

/// Init command: generate CMakeUserPresets.json for the current project
pub fn init(_clean: bool) -> Result<()> {
    println!("{}", "MPF 项目初始化".bold().cyan());

    let cwd = env::current_dir()?;

    // Verify CMakeLists.txt exists
    if !cwd.join("CMakeLists.txt").exists() {
        bail!("当前目录未找到 CMakeLists.txt，请在 CMake 项目根目录执行此命令。");
    }

    // Load dev.json
    let mut dev_config = DevConfig::load().unwrap_or_default();

    // Detect Qt path
    let qt_path = detect_qt_path().context(
        "未检测到 Qt 安装路径，请设置 QT_DIR 或 Qt6_DIR 环境变量。",
    )?;
    let qt_path_fwd = qt_path.replace('\\', "/");

    // Detect MinGW compilers
    let (gcc, gpp) = detect_mingw_path(&qt_path)
        .context("未在 Qt Tools 目录下检测到 MinGW 编译器。")?;

    // Check if CMakePresets.json exists; if not, generate a base one
    let base_presets_path = cwd.join("CMakePresets.json");
    if !base_presets_path.exists() {
        let base_presets = serde_json::json!({
            "version": 6,
            "configurePresets": [
                {
                    "name": "base",
                    "hidden": true,
                    "generator": "Ninja",
                    "binaryDir": "${sourceDir}/build",
                    "cacheVariables": {
                        "CMAKE_EXPORT_COMPILE_COMMANDS": "ON"
                    }
                }
            ]
        });
        let base_content = serde_json::to_string_pretty(&base_presets)?;
        fs::write(&base_presets_path, &base_content)
            .with_context(|| format!("写入 {} 失败", base_presets_path.display()))?;
        println!(
            "{} 已生成 {}",
            "✓".green(),
            base_presets_path.display()
        );
        println!("  {} 请将此文件提交到代码仓库", "→".cyan());
    }

    // Generate CMakeUserPresets.json (also clears CMake cache)
    generate_user_presets(&cwd, &dev_config, &qt_path_fwd, &gcc, &gpp)?;

    // Register this project's root in dev.json so reinit_all can find it.
    let cwd_normalized = normalize_path(cwd.clone());
    let cwd_build_prefix = format!("{}/build", cwd_normalized.replace('\\', "/"));
    for (_name, comp) in dev_config.components.iter_mut() {
        if comp.root.is_some() {
            continue;
        }
        let paths = [&comp.lib, &comp.qml, &comp.plugin, &comp.headers, &comp.bin];
        let matches = paths.iter().any(|p| {
            p.as_ref()
                .map(|s| s.replace('\\', "/").starts_with(&cwd_build_prefix))
                .unwrap_or(false)
        });
        if matches {
            comp.root = Some(cwd_normalized.clone());
        }
    }
    dev_config.save()?;

    let output_path = cwd.join("CMakeUserPresets.json");
    println!("{} 已生成 {}", "✓".green(), output_path.display());
    println!("{} 构建目录已清空", "✓".green());
    println!();
    println!("  {} 请关闭 Qt Creator 并重新打开，然后重新执行构建。", "!".yellow().bold());
    println!();
    println!("  预设：{}, {}", "dev".green(), "release".green());
    println!();
    println!("使用方式：");
    println!("  # 根据预设生成构建系统（配置 CMake）");
    println!("  cmake --preset dev");
    println!("  # 编译项目");
    println!("  cmake --build build");

    Ok(())
}

use anyhow::{bail, Context, Result};
use colored::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const WORKSPACE_REPOS: &[(&str, &str)] = &[
    ("mpf-sdk", "https://github.com/QMPF/mpf-sdk.git"),
    (
        "mpf-ui-components",
        "https://github.com/QMPF/mpf-ui-components.git",
    ),
    (
        "mpf-http-client",
        "https://github.com/QMPF/mpf-http-client.git",
    ),
    ("mpf-host", "https://github.com/QMPF/mpf-host.git"),
    (
        "mpf-plugin-orders",
        "https://github.com/QMPF/mpf-plugin-orders.git",
    ),
    (
        "mpf-plugin-rules",
        "https://github.com/QMPF/mpf-plugin-rules.git",
    ),
];

/// Find workspace root by looking for .mpf-workspace marker
fn find_workspace_root() -> Option<PathBuf> {
    let mut current = env::current_dir().ok()?;
    loop {
        if current.join(".mpf-workspace").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Workspace init: create workspace and clone all components
pub fn workspace_init(path: Option<String>) -> Result<()> {
    let workspace_dir = path
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap());

    println!("{}", "MPF Workspace Initialization".bold().cyan());
    println!("Directory: {}", workspace_dir.display());
    println!();

    fs::create_dir_all(&workspace_dir)?;

    // Create workspace marker
    let marker_path = workspace_dir.join(".mpf-workspace");
    fs::write(&marker_path, "# MPF Workspace\n")?;

    // Clone all repos
    for (name, url) in WORKSPACE_REPOS {
        let repo_dir = workspace_dir.join(name);

        if repo_dir.exists() {
            println!("{} {} (already exists)", "->".yellow(), name);
            continue;
        }

        println!("{} Cloning {}...", "->".cyan(), name);
        let status = Command::new("git")
            .args(["clone", url, &repo_dir.to_string_lossy()])
            .status()
            .context("Failed to run git clone")?;

        if !status.success() {
            bail!("Failed to clone {}", name);
        }
    }

    // Create top-level CMakeLists.txt
    let cmake_content = generate_workspace_cmake();
    fs::write(workspace_dir.join("CMakeLists.txt"), cmake_content)?;

    // Create CMakePresets.json for easy Qt Creator integration
    let presets_content = generate_cmake_presets();
    fs::write(workspace_dir.join("CMakePresets.json"), presets_content)?;

    println!();
    println!("{} Workspace initialized!", "[OK]".green());
    println!();
    println!("Next steps:");
    println!(
        "  1. Open {} in Qt Creator",
        workspace_dir.join("CMakeLists.txt").display()
    );
    println!("  2. Configure with MinGW kit");
    println!("  3. Build and run");
    println!();
    println!("Or use CLI:");
    println!("  cd {}", workspace_dir.display());
    println!("  mpf-dev workspace build");
    println!("  mpf-dev workspace run");

    Ok(())
}

/// Workspace build: build all components
pub fn workspace_build(config: &str) -> Result<()> {
    let workspace = find_workspace_root()
        .context("Not in an MPF workspace. Run 'mpf-dev workspace init' first.")?;

    println!("{}", "Building MPF Workspace".bold().cyan());
    println!("Directory: {}", workspace.display());
    println!("Configuration: {}", config);
    println!();

    let build_dir = workspace.join("build");

    // Configure if needed
    if !build_dir.join("CMakeCache.txt").exists() {
        println!("{} Configuring CMake...", "->".cyan());

        let status = Command::new("cmake")
            .current_dir(&workspace)
            .args([
                "-B",
                "build",
                "-G",
                "Ninja",
                &format!("-DCMAKE_BUILD_TYPE={}", config),
            ])
            .status()
            .context("Failed to run cmake configure")?;

        if !status.success() {
            bail!("CMake configuration failed");
        }
    }

    // Build
    println!("{} Building...", "->".cyan());

    let status = Command::new("cmake")
        .current_dir(&workspace)
        .args(["--build", "build", "-j"])
        .status()
        .context("Failed to run cmake build")?;

    if !status.success() {
        bail!("Build failed");
    }

    println!();
    println!("{} Build complete!", "[OK]".green());
    println!();
    println!("Output:");
    let host_name = if cfg!(windows) {
        "mpf-host.exe"
    } else {
        "mpf-host"
    };
    println!(
        "  Host: {}",
        build_dir.join("bin").join(host_name).display()
    );
    println!("  Plugins: {}", build_dir.join("plugins").display());
    println!("  QML: {}", build_dir.join("qml").display());

    Ok(())
}

/// Workspace run: run mpf-host from workspace
pub fn workspace_run(args: Vec<String>) -> Result<()> {
    let workspace = find_workspace_root()
        .context("Not in an MPF workspace. Run 'mpf-dev workspace init' first.")?;

    let build_dir = workspace.join("build");
    let host_exe = if cfg!(windows) {
        build_dir.join("bin").join("mpf-host.exe")
    } else {
        build_dir.join("bin").join("mpf-host")
    };

    if !host_exe.exists() {
        bail!("mpf-host not found. Run 'mpf-dev workspace build' first.");
    }

    println!("{} Running mpf-host from workspace...", "->".cyan());

    let mut cmd = Command::new(&host_exe);
    cmd.current_dir(&workspace);
    cmd.args(&args);

    // Set library paths
    #[cfg(windows)]
    {
        let current_path = env::var("PATH").unwrap_or_default();
        let lib_path = format!(
            "{};{};{}",
            build_dir.join("bin").display(),
            build_dir.join("plugins").display(),
            current_path
        );
        cmd.env("PATH", lib_path);
    }

    #[cfg(unix)]
    {
        let lib_path = format!(
            "{}:{}",
            build_dir.join("bin").display(),
            build_dir.join("plugins").display()
        );
        cmd.env("LD_LIBRARY_PATH", lib_path);
    }

    cmd.env(
        "QML_IMPORT_PATH",
        build_dir.join("qml").to_string_lossy().to_string(),
    );

    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}

/// Workspace status: show workspace info
pub fn workspace_status() -> Result<()> {
    let workspace = find_workspace_root();

    println!("{}", "MPF Workspace Status".bold().cyan());
    println!();

    if let Some(ws) = workspace {
        println!("{} Workspace: {}", "[OK]".green(), ws.display());

        // Check each component
        for (name, _) in WORKSPACE_REPOS {
            let repo_dir = ws.join(name);
            if repo_dir.exists() {
                let output = Command::new("git")
                    .current_dir(&repo_dir)
                    .args(["log", "-1", "--oneline"])
                    .output();

                let commit = output
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                println!("  {} {}: {}", "[OK]".green(), name, commit.dimmed());
            } else {
                println!("  {} {}: {}", "[X]".red(), name, "missing".red());
            }
        }

        // Check build
        let build_dir = ws.join("build");
        if build_dir.exists() {
            println!();
            let host_exe = if cfg!(windows) {
                build_dir.join("bin").join("mpf-host.exe")
            } else {
                build_dir.join("bin").join("mpf-host")
            };

            if host_exe.exists() {
                println!("{} Built: {}", "[OK]".green(), "yes".green());
            } else {
                println!("{} Built: {}", "->".yellow(), "not yet".yellow());
            }
        } else {
            println!();
            println!("{} Built: {}", "[X]".red(), "no".red());
        }
    } else {
        println!("{} Not in an MPF workspace", "[X]".red());
        println!();
        println!("Run {} to create one.", "mpf-dev workspace init".cyan());
    }

    Ok(())
}

// ─── CMake template generators ───────────────────────────────────────────────

fn generate_workspace_cmake() -> String {
    String::from(
        r##"cmake_minimum_required(VERSION 3.21)
project(mpf-workspace VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

if(COMMAND qt_policy)
    qt_policy(SET QTP0001 NEW)
    qt_policy(SET QTP0004 NEW)
endif()

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 Network)

# SDK (header-only)
add_library(mpf-sdk INTERFACE)
add_library(MPF::sdk ALIAS mpf-sdk)
target_include_directories(mpf-sdk INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/mpf-sdk/include>
)
target_link_libraries(mpf-sdk INTERFACE Qt6::Core Qt6::Gui Qt6::Qml)

# HTTP Client (static)
add_library(mpf-http-client STATIC
    mpf-http-client/src/http_client.cpp
    mpf-http-client/include/mpf/http/http_client.h
)
add_library(MPF::http-client ALIAS mpf-http-client)
target_include_directories(mpf-http-client PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/mpf-http-client/include>
)
target_compile_definitions(mpf-http-client PUBLIC MPF_HTTP_CLIENT_STATIC)
target_link_libraries(mpf-http-client PUBLIC Qt6::Core Qt6::Network)

# UI Components
add_compile_definitions(MPF_UI_COMPONENTS_EXPORTS)

set(UI_QML_FILES
    mpf-ui-components/qml/MPFCard.qml
    mpf-ui-components/qml/MPFButton.qml
    mpf-ui-components/qml/MPFIconButton.qml
    mpf-ui-components/qml/StatusBadge.qml
    mpf-ui-components/qml/MPFDialog.qml
    mpf-ui-components/qml/MPFTextField.qml
    mpf-ui-components/qml/MPFLoadingIndicator.qml
)

foreach(file ${UI_QML_FILES})
    string(REGEX REPLACE "^mpf-ui-components/qml/" "" alias "${file}")
    set_source_files_properties(${file} PROPERTIES QT_RESOURCE_ALIAS ${alias})
endforeach()

qt_add_qml_module(mpf-ui-components
    URI MPF.Components
    VERSION 1.0
    RESOURCE_PREFIX /
    SOURCES
        mpf-ui-components/src/ui_components_global.h
        mpf-ui-components/src/color_helper.h
        mpf-ui-components/src/color_helper.cpp
        mpf-ui-components/src/input_validator.h
        mpf-ui-components/src/input_validator.cpp
    QML_FILES ${UI_QML_FILES}
    OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/qml/MPF/Components
)
add_library(MPF::ui-components ALIAS mpf-ui-components)
target_include_directories(mpf-ui-components PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/mpf-ui-components/src>
)
target_link_libraries(mpf-ui-components PUBLIC Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick)

# Host Application
add_executable(mpf-host
    mpf-host/src/main.cpp
    mpf-host/src/application.cpp
    mpf-host/src/service_registry.cpp
    mpf-host/src/logger.cpp
    mpf-host/src/plugin_metadata.cpp
    mpf-host/src/plugin_manager.cpp
    mpf-host/src/plugin_loader.cpp
    mpf-host/src/navigation_service.cpp
    mpf-host/src/settings_service.cpp
    mpf-host/src/theme_service.cpp
    mpf-host/src/menu_service.cpp
    mpf-host/src/event_bus_service.cpp
    mpf-host/src/qml_context.cpp
)
target_include_directories(mpf-host PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/mpf-host/include
    ${CMAKE_CURRENT_BINARY_DIR}/host
)
target_link_libraries(mpf-host PRIVATE
    Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2
    MPF::sdk MPF::ui-components
)

# Generate version header
file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/host/mpf/version.h [=[
#pragma once
#define MPF_VERSION_MAJOR 1
#define MPF_VERSION_MINOR 0
#define MPF_VERSION_PATCH 0
#define MPF_VERSION_STRING "1.0.0-workspace"
]=])

# Generate sdk_paths header
file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/host/mpf/sdk_paths.h [=[
#pragma once
#define MPF_SDK_HAS_QML_PATH 0
#define MPF_PREFIX ""
#define MPF_QML_PATH ""
]=])

# Host QML
set(HOST_QML_FILES
    mpf-host/qml/Main.qml
    mpf-host/qml/SideMenu.qml
    mpf-host/qml/MenuItemCustom.qml
    mpf-host/qml/ErrorDialog.qml
)
set(HOST_RESOURCES mpf-host/qml/images/logo.svg)

foreach(file ${HOST_QML_FILES} ${HOST_RESOURCES})
    string(REGEX REPLACE "^mpf-host/qml/" "" alias "${file}")
    set_source_files_properties(${file} PROPERTIES QT_RESOURCE_ALIAS ${alias})
endforeach()

qt_add_qml_module(mpf-host
    URI MPF.Host
    VERSION 1.0
    RESOURCE_PREFIX /
    QML_FILES ${HOST_QML_FILES}
    RESOURCES ${HOST_RESOURCES}
    OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/qml/MPF/Host
)

# Orders Plugin
add_library(orders-plugin SHARED
    mpf-plugin-orders/src/orders_plugin.cpp
    mpf-plugin-orders/src/orders_service.cpp
    mpf-plugin-orders/src/order_model.cpp
)
target_include_directories(orders-plugin PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/mpf-plugin-orders/include
)
target_link_libraries(orders-plugin PRIVATE
    Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::Network
    MPF::sdk MPF::http-client
)

set(ORDERS_QML_FILES
    mpf-plugin-orders/qml/OrdersPage.qml
    mpf-plugin-orders/qml/OrderCard.qml
    mpf-plugin-orders/qml/CreateOrderDialog.qml
)
foreach(file ${ORDERS_QML_FILES})
    string(REGEX REPLACE "^mpf-plugin-orders/qml/" "" alias "${file}")
    set_source_files_properties(${file} PROPERTIES QT_RESOURCE_ALIAS ${alias})
endforeach()

qt_add_qml_module(orders-plugin
    URI YourCo.Orders
    VERSION 1.0
    RESOURCE_PREFIX /
    QML_FILES ${ORDERS_QML_FILES}
    OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/qml/YourCo/Orders
    NO_PLUGIN
)

# Rules Plugin
add_library(rules-plugin SHARED
    mpf-plugin-rules/src/rules_plugin.cpp
    mpf-plugin-rules/src/orders_service.cpp
    mpf-plugin-rules/src/order_model.cpp
)
target_include_directories(rules-plugin PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/mpf-plugin-rules/include
)
target_link_libraries(rules-plugin PRIVATE
    Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick
    MPF::sdk
)

set(RULES_QML_FILES
    mpf-plugin-rules/qml/OrdersPage.qml
    mpf-plugin-rules/qml/OrderCard.qml
    mpf-plugin-rules/qml/CreateOrderDialog.qml
    mpf-plugin-rules/qml/TestCard.qml
)
foreach(file ${RULES_QML_FILES})
    string(REGEX REPLACE "^mpf-plugin-rules/qml/" "" alias "${file}")
    set_source_files_properties(${file} PROPERTIES QT_RESOURCE_ALIAS ${alias})
endforeach()

qt_add_qml_module(rules-plugin
    URI Biiz.Rules
    VERSION 1.0
    RESOURCE_PREFIX /
    QML_FILES ${RULES_QML_FILES}
    OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/qml/Biiz/Rules
    NO_PLUGIN
)

# Output directories
set_target_properties(mpf-host PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
)
set_target_properties(orders-plugin rules-plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/plugins
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/plugins
)

file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/plugins)
file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/qml)
"##,
    )
}

fn generate_cmake_presets() -> String {
    r##"{
  "version": 6,
  "configurePresets": [
    {
      "name": "debug",
      "displayName": "Debug",
      "generator": "Ninja",
      "binaryDir": "${sourceDir}/build",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Debug"
      }
    },
    {
      "name": "release",
      "displayName": "Release",
      "generator": "Ninja",
      "binaryDir": "${sourceDir}/build",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Release"
      }
    }
  ],
  "buildPresets": [
    {"name": "debug", "configurePreset": "debug"},
    {"name": "release", "configurePreset": "release"}
  ]
}
"##
    .to_string()
}

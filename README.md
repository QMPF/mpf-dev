# mpf-dev

MPF Development Environment CLI Tool - Manage MPF SDK and component development.

## Installation

### From Releases (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/dyzdyz010/mpf-dev/releases).

**Linux:**
```bash
curl -LO https://github.com/dyzdyz010/mpf-dev/releases/latest/download/mpf-dev-linux-x86_64.tar.gz
tar xzf mpf-dev-linux-x86_64.tar.gz
sudo mv mpf-dev /usr/local/bin/
```

**Windows:**
Download `mpf-dev-windows-x86_64.zip`, extract, and add to PATH.

### From Source

```bash
cargo install --git https://github.com/dyzdyz010/mpf-dev
```

## Quick Start

```bash
# 1. Install the SDK
mpf-dev setup

# 2. Clone your component
git clone https://github.com/dyzdyz010/mpf-http-client
cd mpf-http-client

# 3. Register for source development
mpf-dev link http-client --lib ./build/lib --qml ./qml

# 4. Build your component
mkdir build && cd build
cmake .. -DMPF_SDK_ROOT=~/.mpf-sdk/current
make

# 5. Run with your changes
mpf-dev run
```

## Commands

### `mpf-dev setup [--version X.Y.Z]`

Download and install the MPF SDK. Downloads the latest version by default.

```bash
mpf-dev setup                    # Install latest
mpf-dev setup --version 1.0.0    # Install specific version
```

### `mpf-dev versions`

List installed SDK versions.

```bash
$ mpf-dev versions
Installed SDK versions:
  * v1.0.0 (current)
    v0.9.0
```

### `mpf-dev use <version>`

Switch to a different installed SDK version.

```bash
mpf-dev use 0.9.0
```

### `mpf-dev link <component> [options]`

Register a component for source development. Paths are relative to current directory.

**Options:**
- `--lib <path>` - Path to built library directory
- `--qml <path>` - Path to QML modules directory
- `--plugin <path>` - Path to plugin directory
- `--headers <path>` - Path to include directory

```bash
# In your component directory
mpf-dev link http-client --lib ./build/lib --qml ./qml
mpf-dev link plugin-orders --lib ./build --plugin ./build
```

### `mpf-dev unlink <component>`

Remove a component from source development mode.

```bash
mpf-dev unlink http-client
```

### `mpf-dev status`

Show current development environment status.

```bash
$ mpf-dev status
MPF Development Environment Status

SDK:
  Root: /home/user/.mpf-sdk
  Current version: v1.0.0
  Config: /home/user/.mpf-sdk/dev.json

Components:
  http-client [source]
    lib: /home/user/mpf-http-client/build/lib
    qml: /home/user/mpf-http-client/qml
  ui-components [binary]
```

### `mpf-dev env`

Print environment variables for manual shell setup.

```bash
$ mpf-dev env
# Add these to your shell:
export LD_LIBRARY_PATH="/home/user/mpf-http-client/build/lib:/home/user/.mpf-sdk/current/lib"
export QML_IMPORT_PATH="/home/user/mpf-http-client/qml:/home/user/.mpf-sdk/current/qml"
export QT_PLUGIN_PATH="/home/user/.mpf-sdk/current/plugins"
```

### `mpf-dev run [--debug] [-- args]`

Run mpf-host with development environment overrides applied.

```bash
mpf-dev run                  # Normal run
mpf-dev run --debug          # Show env vars being used
mpf-dev run -- --help        # Pass args to mpf-host
```

## How It Works

1. **SDK as baseline**: The SDK provides a complete, working application with all components as pre-built binaries.

2. **Source overrides**: When you `link` a component, its build output paths are added to the environment with higher priority than the SDK.

3. **Runtime resolution**: When running via `mpf-dev run`, environment variables (`LD_LIBRARY_PATH`, `QML_IMPORT_PATH`, etc.) are set so that your source-built components are loaded instead of the SDK's binaries.

This means:
- You only need source code for the component(s) you're working on
- All other components come from the SDK as pre-built binaries
- Multiple developers can work on different components without needing each other's source

## Directory Structure

```
~/.mpf-sdk/
├── v1.0.0/                 # Versioned SDK installation
│   ├── bin/mpf-host
│   ├── lib/
│   ├── include/
│   ├── plugins/
│   └── qml/
├── current -> v1.0.0       # Symlink to active version
└── dev.json                # Development configuration
```

## Configuration File

`~/.mpf-sdk/dev.json`:

```json
{
  "sdk_version": "v1.0.0",
  "components": {
    "http-client": {
      "mode": "source",
      "lib": "/home/user/mpf-http-client/build/lib",
      "qml": "/home/user/mpf-http-client/qml"
    }
  }
}
```

## Documentation

📖 **[完整开发流程指南](docs/DEVELOPMENT-WORKFLOW.md)** - 从环境准备到发布的完整流程

## License

MIT

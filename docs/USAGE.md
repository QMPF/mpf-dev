# MPF 开发工具使用手册

## 目录

- [一、概述与架构](#一概述与架构)
- [二、安装](#二安装)
- [三、命令参考](#三命令参考)
- [四、开发流程](#四开发流程)
- [五、IDE 配置](#五ide-配置)
- [六、dev.json 与运行时发现机制](#六devjson-与运行时发现机制)
- [七、CMake 集成](#七cmake-集成)
- [八、常见问题](#八常见问题)

---

## 一、概述与架构

MPF (Modular Plugin Framework) 是一个基于 Qt 6 的模块化插件框架。`mpf-dev` 是配套的 CLI 开发工具，负责管理 SDK 版本、组件注册、项目初始化和运行调试。

### 核心理念

**SDK 作为基线 + 源码覆盖**：开发者只需拥有自己负责的组件源码，其余依赖以 SDK 二进制形式提供。构建和运行时，源码构建产物优先于 SDK 二进制。

### 架构图

```
┌─────────────────────────────────────────────────────┐
│                  mpf-release (SDK)                   │
│  ┌────────┬──────────┬──────────────┬────────────┐  │
│  │  bin/  │   lib/   │   include/   │    qml/    │  │
│  │mpf-host│ 所有组件库│ 所有组件头文件 │  QML模块   │  │
│  └────────┴──────────┴──────────────┴────────────┘  │
└─────────────────────────────────────────────────────┘

依赖关系:
┌──────────────────┐
│ foundation-sdk   │ (核心接口: IPlugin, ServiceRegistry, INavigation...)
└────────┬─────────┘
         │
         ├──────────────┬────────────────┐
         v              v                v
┌──────────────┐ ┌──────────────┐ ┌──────────┐
│ http-client  │ │ui-components │ │   其他   │
└──────┬───────┘ └──────┬───────┘ └────┬─────┘
       │                │              │
       └────────────────┼──────────────┘
                        v
                 ┌───────────┐
                 │  mpf-host │ (宿主程序)
                 └─────┬─────┘
                       │
         ┌─────────────┼─────────────┐
         v             v             v
  ┌─────────────┐ ┌─────────────┐ ┌─────────┐
  │plugin-orders│ │plugin-rules │ │ 你的插件 │
  └─────────────┘ └─────────────┘ └─────────┘
```

### 目录结构

```
~/.mpf-sdk/
├── v1.0.26/                 # SDK 版本目录
│   ├── bin/mpf-host         # 宿主可执行文件
│   ├── lib/                 # 所有组件库
│   ├── include/             # 所有组件头文件
│   │   └── mpf/             # foundation-sdk 头文件
│   ├── plugins/             # 预编译插件
│   └── qml/                 # QML 模块
│       ├── MPF/Components/  # ui-components
│       └── MPF/Host/        # host QML
├── current -> v1.0.26       # 指向当前版本的 junction/symlink
└── dev.json                 # 开发配置（已注册的源码组件）
```

---

## 二、安装

### 2.1 前置条件

- **Qt 6.8+**（MinGW 版本，Windows）
- **Ninja** 构建工具
- **Git**

### 2.2 安装 mpf-dev

**Windows:**
```powershell
# 从 GitHub Releases 下载
Invoke-WebRequest -Uri "https://github.com/dyzdyz010/mpf-dev/releases/latest/download/mpf-dev-windows-x86_64.zip" -OutFile mpf-dev.zip
Expand-Archive mpf-dev.zip -DestinationPath C:\Tools\mpf-dev

# 添加到 PATH（永久）
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Tools\mpf-dev", "User")
```

**Linux:**
```bash
curl -LO https://github.com/dyzdyz010/mpf-dev/releases/latest/download/mpf-dev-linux-x86_64.tar.gz
tar xzf mpf-dev-linux-x86_64.tar.gz
sudo mv mpf-dev /usr/local/bin/
```

**从源码编译:**
```bash
cargo install --git https://github.com/dyzdyz010/mpf-dev
```

### 2.3 安装 SDK

```bash
# 安装最新版本
mpf-dev setup

# 安装指定版本
mpf-dev setup --version v1.0.26

# 验证
mpf-dev status
```

---

## 三、命令参考

### `mpf-dev setup [--version VERSION]`

下载并安装 MPF SDK。

```bash
mpf-dev setup                      # 安装最新版本
mpf-dev setup --version v1.0.26    # 安装指定版本
```

SDK 安装到 `~/.mpf-sdk/<version>/`，并自动设为当前版本。

### `mpf-dev versions`

列出所有已安装的 SDK 版本。

```bash
$ mpf-dev versions
Installed SDK versions:
  * v1.0.26 (current)
    v1.0.25
```

### `mpf-dev use <version>`

切换当前使用的 SDK 版本。

```bash
mpf-dev use v1.0.25
```

实现方式：更新 `~/.mpf-sdk/current` junction（Windows）或 symlink（Unix）指向目标版本目录。

### `mpf-dev link` — 注册源码组件

将本地构建产物注册到 `dev.json`，使 host 在运行时优先加载这些路径。

#### `mpf-dev link plugin <name> <build-path>`

注册一个插件的构建输出。自动推导 `lib`（DLL/so）和 `qml` 路径。

```bash
# 在插件项目目录中
cd mpf-plugin-orders
mpf-dev link plugin orders ./build
```

自动推导：
- `lib` → `<build-path>/plugins`
- `qml` → `<build-path>/qml`

#### `mpf-dev link host <build-path>`

注册 host 的构建输出。自动推导 `bin` 和 `qml` 路径。

```bash
cd mpf-host
mpf-dev link host ./build
```

自动推导：
- `bin` → `<build-path>/bin`
- `qml` → `<build-path>/qml`

#### `mpf-dev link component <name> <build-path>`

注册一个库组件的构建输出。自动推导 `lib`、`headers`、`qml` 路径。

```bash
cd mpf-ui-components
mpf-dev link component ui-components ./build
```

#### `mpf-dev link manual <name> [--lib PATH] [--qml PATH] [--plugin PATH] [--headers PATH] [--bin PATH]`

手动指定各路径，用于非标准目录结构。

```bash
mpf-dev link manual my-lib --lib ./build/lib --qml ./build/qml --headers ./include
```

### `mpf-dev unlink <component>`

取消组件的源码注册。

```bash
mpf-dev unlink orders          # 取消单个组件
mpf-dev unlink all             # 取消所有组件
```

### `mpf-dev init [--clean]`

在当前项目目录生成 `CMakeUserPresets.json`，自动检测 Qt、MinGW、SDK 路径。

```bash
cd mpf-plugin-orders
mpf-dev init                   # 生成 preset，清除 CMake 缓存
mpf-dev init --clean           # 生成 preset，删除整个 build/ 目录
```

**默认行为：**
- 生成包含 `dev` 和 `release` 两个配置的 `CMakeUserPresets.json`
- 删除 `build/CMakeCache.txt` 和 `build/CMakeFiles/`（确保新 preset 干净生效）
- 读取 `dev.json` 中的已注册组件，自动添加 QML 路径和 CMake 包路径

**`--clean` 行为：**
- 删除整个 `build/` 目录
- 然后生成 `CMakeUserPresets.json`

**生成的 preset 内容：**
- `CMAKE_C_COMPILER` / `CMAKE_CXX_COMPILER`：自动检测的 MinGW 路径
- `CMAKE_PREFIX_PATH`：Qt + SDK current
- `QML_IMPORT_PATH`：已注册组件 QML 路径 + SDK QML + Qt QML
- 已注册库组件的 `<PackageName>_DIR` 变量

### `mpf-dev status`

显示当前开发环境状态。

```bash
$ mpf-dev status
MPF Development Environment Status

SDK:
  Root: C:\Users\dyz\.mpf-sdk
  Current version: v1.0.26
  Config: C:\Users\dyz\.mpf-sdk\dev.json

Components:
  host [source]
    bin: C:\Users\dyz\...\mpf-host\build\bin
    qml: C:\Users\dyz\...\mpf-host\build\qml
  orders [source]
    lib: C:\Users\dyz\...\mpf-plugin-orders\build\plugins
    qml: C:\Users\dyz\...\mpf-plugin-orders\build\qml
```

### `mpf-dev env`

输出当前环境变量配置，可用于手动设置 shell 环境。

```bash
mpf-dev env
```

### `mpf-dev run [--debug] [-- args]`

使用 dev.json 中的开发覆盖路径运行 mpf-host。

```bash
mpf-dev run                    # 正常运行
mpf-dev run --debug            # 显示使用的环境变量
mpf-dev run -- --help          # 传递参数给 mpf-host
```

运行时自动设置的环境变量：
- `MPF_SDK_ROOT`：SDK 路径
- `MPF_PLUGIN_PATH`：源码构建的插件路径
- `QML_IMPORT_PATH`：源码构建的 QML 路径
- `PATH`（Windows）/ `LD_LIBRARY_PATH`（Linux）：DLL/so 搜索路径

### `mpf-dev workspace` — 全源码工作区

用于同时从源码构建所有 MPF 组件的场景。

#### `mpf-dev workspace init [--path DIR]`

初始化工作区，从 GitHub 克隆所有 MPF 组件仓库。

```bash
mkdir mpf-workspace && cd mpf-workspace
mpf-dev workspace init
```

#### `mpf-dev workspace build [--config Debug|Release]`

构建工作区中所有组件。

```bash
mpf-dev workspace build                    # Debug 构建
mpf-dev workspace build --config Release   # Release 构建
```

#### `mpf-dev workspace run [-- args]`

从工作区运行 mpf-host。

```bash
mpf-dev workspace run
```

#### `mpf-dev workspace status`

显示工作区状态。

---

## 四、开发流程

### 4.1 插件开发（最常见场景）

适用于：开发一个新插件或修改现有插件，Host 和库使用 SDK 预编译版本。

```bash
# 1. 确保 SDK 已安装
mpf-dev setup

# 2. 克隆或创建插件项目
git clone https://github.com/dyzdyz010/mpf-plugin-orders
cd mpf-plugin-orders

# 3. 初始化项目（生成 CMakeUserPresets.json）
mpf-dev init

# 4. 配置和构建
cmake --preset dev
cmake --build build

# 5. 注册插件到 dev.json
mpf-dev link plugin orders ./build

# 6. 运行测试
mpf-dev run

# 7. 日常开发循环
#    修改代码 → cmake --build build → mpf-dev run
```

### 4.2 库组件开发

适用于：开发 http-client、ui-components 等共享库。

```bash
cd mpf-ui-components
mpf-dev init
cmake --preset dev
cmake --build build

# 注册库组件
mpf-dev link component ui-components ./build

# 在其他依赖此库的项目中重新 init 以更新路径
cd ../mpf-plugin-orders
mpf-dev init
cmake --preset dev
cmake --build build
mpf-dev run
```

### 4.3 Host + 插件联合开发

适用于：同时修改 Host 和插件。

```bash
# 注册 Host
cd mpf-host
mpf-dev init
cmake --preset dev
cmake --build build
mpf-dev link host ./build

# 注册插件
cd ../mpf-plugin-orders
mpf-dev init
cmake --preset dev
cmake --build build
mpf-dev link plugin orders ./build

# 运行（使用源码构建的 Host）
mpf-dev run
```

### 4.4 全源码工作区

适用于：需要同时修改所有组件，或初次搭建完整开发环境。

```bash
mkdir mpf-workspace && cd mpf-workspace
mpf-dev workspace init         # 克隆所有仓库
mpf-dev workspace build        # 构建全部
mpf-dev workspace run          # 运行
```

### 4.5 Qt Creator 调试

mpf-host 在启动时自动读取 `~/.mpf-sdk/dev.json`，发现已注册的源码组件路径。因此在 Qt Creator 中直接调试时**无需手动配置环境变量**。

```bash
# 1. 注册组件
mpf-dev link host ./build
mpf-dev link plugin orders ../mpf-plugin-orders/build

# 2. 初始化项目
mpf-dev init

# 3. 在 Qt Creator 中：
#    - 打开 CMakeLists.txt
#    - 选择 "dev" preset
#    - 构建并运行/调试
```

Host 会自动从 dev.json 中读取：
- 插件 DLL 搜索路径（`lib` 字段）
- QML 模块搜索路径（`qml` 字段）
- Host 可执行文件路径（`bin` 字段）

---

## 五、IDE 配置

### 5.1 Qt Creator

**推荐方式（几乎零配置）：**

1. 在项目目录执行 `mpf-dev init`
2. Qt Creator 打开 `CMakeLists.txt`，选择 `dev` preset
3. 构建并运行

`CMakeUserPresets.json` 中已包含所有必要路径（编译器、SDK、QML），Qt Creator 会自动读取。

**QML 智能提示：**

`mpf-dev init` 生成的 preset 中包含 `QML_IMPORT_PATH`，Qt Creator 据此提供 QML 补全。如果仍有 QML import 报错，检查 preset 中的路径是否正确：

```bash
mpf-dev init    # 重新生成
```

### 5.2 VS Code

1. 安装 CMake Tools 扩展
2. 执行 `mpf-dev init`
3. `Ctrl+Shift+P` → `CMake: Select Configure Preset` → 选择 `dev`
4. `CMake: Build`

**调试配置 (launch.json):**
```json
{
  "version": "0.2.0",
  "configurations": [{
    "name": "Debug MPF Host",
    "type": "cppdbg",
    "request": "launch",
    "program": "${env:USERPROFILE}/.mpf-sdk/current/bin/mpf-host",
    "cwd": "${workspaceFolder}"
  }]
}
```

> 注意：VS Code 调试不会自动读取 dev.json，建议使用 `mpf-dev run --debug` 查看需要设置的环境变量，然后添加到 launch.json 的 `environment` 中。

---

## 六、dev.json 与运行时发现机制

### dev.json 文件

`~/.mpf-sdk/dev.json` 是 mpf-dev 管理的开发配置文件，记录所有已注册的源码组件及其构建路径。

**示例：**
```json
{
  "sdk_version": "v1.0.26",
  "components": {
    "host": {
      "mode": "source",
      "qml": "C:\\Users\\dyz\\...\\mpf-host\\build\\qml",
      "bin": "C:\\Users\\dyz\\...\\mpf-host\\build\\bin"
    },
    "orders": {
      "mode": "source",
      "lib": "C:\\Users\\dyz\\...\\mpf-plugin-orders\\build\\plugins",
      "qml": "C:\\Users\\dyz\\...\\mpf-plugin-orders\\build\\qml"
    }
  }
}
```

### 运行时发现

mpf-host 在启动时（`Application::setupPaths()`）自动读取 dev.json：

1. 遍历 `components`，筛选 `mode == "source"` 的条目
2. 将 `lib` 路径添加到插件搜索路径
3. 将 `qml` 路径添加到 QML import 路径
4. 在 Windows 上将 `lib` 路径添加到 `PATH`（DLL 加载依赖）
5. 将 QML 路径发布到 `QML_IMPORT_PATH` 环境变量（供插件使用）

**这意味着：**
- 从 Qt Creator 直接运行/调试时，Host 自动发现源码组件
- 无需手动配置 `LD_LIBRARY_PATH`、`PATH`、`QML_IMPORT_PATH` 等环境变量
- 与 `mpf-dev run` 等价的行为，但不需要通过 CLI 启动

### 插件侧 QML 发现

插件在注册路由时，按以下优先级搜索 QML 文件：

1. `QML_IMPORT_PATH` 环境变量（dev.json 路径，最高优先）
2. `MPF_SDK_ROOT/qml`（SDK 兜底）
3. 应用程序相对路径

---

## 七、CMake 集成

### find_package(MPF)

所有 MPF 组件项目使用 `find_package` 发现 SDK：

```cmake
find_package(MPF REQUIRED)                    # 核心 SDK
find_package(MPFHttpClient REQUIRED)           # HTTP 客户端库（可选）
# MPFUIComponents 由 Host 加载，插件不要链接！

target_link_libraries(my-plugin PRIVATE
    MPF::foundation-sdk
    MPF::mpf-http-client
)
```

`CMAKE_PREFIX_PATH` 由 `mpf-dev init` 自动配置，包含 Qt 和 SDK 路径。

### CMakeUserPresets.json

`mpf-dev init` 生成的 preset 结构：

```json
{
  "version": 6,
  "configurePresets": [
    {
      "name": "dev",
      "displayName": "MPF Dev",
      "generator": "Ninja",
      "binaryDir": "${sourceDir}/build",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Debug",
        "CMAKE_C_COMPILER": "C:/Qt/Tools/mingw1310_64/bin/gcc.exe",
        "CMAKE_CXX_COMPILER": "C:/Qt/Tools/mingw1310_64/bin/g++.exe",
        "CMAKE_PREFIX_PATH": "C:/Qt/6.8.3/mingw_64;C:/Users/dyz/.mpf-sdk/current",
        "CMAKE_EXPORT_COMPILE_COMMANDS": "ON",
        "QML_IMPORT_PATH": "...linked_qml...;.../current/qml;.../Qt/qml"
      }
    },
    {
      "name": "release",
      "displayName": "MPF Release",
      "generator": "Ninja",
      "binaryDir": "${sourceDir}/build-release",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Release",
        "...": "同 dev，但 Release 模式"
      }
    }
  ],
  "buildPresets": [
    { "name": "dev", "configurePreset": "dev" },
    { "name": "release", "configurePreset": "release" }
  ]
}
```

**已注册库组件的处理：**

如果 dev.json 中注册了库组件（如 ui-components），`mpf-dev init` 会额外添加对应的 CMake 包路径变量：

- `ui-components` → `MPFUIComponents_DIR`
- `http-client` → `MPFHttpClient_DIR`

这使得 `find_package()` 能找到源码构建的库，而非 SDK 中的预编译版本。

### 标准构建流程

```bash
mpf-dev init                # 生成/更新 preset
cmake --preset dev          # 配置
cmake --build build         # 构建
```

---

## 八、常见问题

### Q: `mpf-dev init` 报 "Could not detect Qt installation"

设置环境变量 `QT_DIR` 或 `Qt6_DIR` 指向 Qt 安装路径：
```bash
# Windows
set QT_DIR=C:\Qt\6.8.3\mingw_64

# Linux
export QT_DIR=/opt/Qt/6.8.3/gcc_64
```

### Q: `cmake --preset dev` 报编译器找不到

`mpf-dev init` 自动在 Qt 的 `Tools/` 目录下查找 MinGW。确保 Qt 安装时选择了 MinGW 组件，且路径类似 `C:/Qt/Tools/mingw1310_64/bin/gcc.exe`。

### Q: 运行时找不到插件 DLL

1. 确保已注册：`mpf-dev link plugin <name> ./build`
2. 确保已构建：`cmake --build build`
3. 使用 `mpf-dev run --debug` 查看实际加载路径
4. 或在 Qt Creator 中调试时查看控制台输出

### Q: QML import 报错（找不到 MPF.Components）

1. 执行 `mpf-dev init` 更新 QML 路径
2. 确认 SDK 中有 `qml/MPF/Components/` 目录
3. Qt Creator 中：重新加载 CMake 配置

### Q: 如何切换 SDK 版本？

```bash
mpf-dev setup --version v1.0.25   # 安装新版本（如未安装）
mpf-dev use v1.0.25               # 切换
mpf-dev init                      # 重新生成 preset（路径已变）
```

### Q: 如何完全重置项目构建？

```bash
mpf-dev init --clean               # 删除 build/ + 重新生成 preset
cmake --preset dev                  # 重新配置
cmake --build build                 # 重新构建
```

### Q: Windows 上 ui-components 链接导致崩溃

**这是已知设计约束。** 插件**不得**链接 `MPF::mpf-ui-components`，因为跨 DLL 堆分配会导致崩溃。ui-components 由 Host 加载，插件通过 `QML_IMPORT_PATH` 在运行时访问。

### Q: 从 Qt Creator 调试 Host，插件从 SDK 加载而非源码

确保已执行 `mpf-dev link plugin <name> ./build`。Host 启动时读取 dev.json，自动发现源码构建的插件路径。使用 Qt Creator 的 "Application Output" 窗口查看 Host 的日志输出，确认加载路径。

# MPF 组件开发全流程指南

## 概述

MPF (Modular Plugin Framework) 采用组件化架构，每个开发者只需关注自己负责的组件源码，其他依赖以二进制形式提供。

## 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      mpf-release (SDK)                       │
│  ┌─────────┬──────────────┬────────────────┬─────────────┐  │
│  │   bin/  │    lib/      │    include/    │    qml/     │  │
│  │mpf-host │ 所有组件库    │  所有组件头文件  │  QML模块    │  │
│  └─────────┴──────────────┴────────────────┴─────────────┘  │
└─────────────────────────────────────────────────────────────┘

依赖关系:
┌─────────┐
│ mpf-sdk │ (核心接口)
└────┬────┘
     │
     ├──────────────┬────────────────┐
     ▼              ▼                ▼
┌──────────┐  ┌─────────────┐  ┌──────────┐
│http-client│  │ui-components│  │   其他   │
└─────┬────┘  └──────┬──────┘  └─────┬────┘
      │              │               │
      └──────────────┼───────────────┘
                     ▼
              ┌───────────┐
              │  mpf-host │ (宿主程序)
              └─────┬─────┘
                    │
      ┌─────────────┼─────────────┐
      ▼             ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐
│plugin-orders│ │plugin-rules│ │ 你的插件  │
└───────────┘ └───────────┘ └───────────┘
```

---

## 一、环境准备

### 1.1 安装 mpf-dev 工具

**Windows:**
```powershell
# 下载
Invoke-WebRequest -Uri "https://github.com/dyzdyz010/mpf-dev/releases/latest/download/mpf-dev-windows-x86_64.zip" -OutFile mpf-dev.zip

# 解压
Expand-Archive mpf-dev.zip -DestinationPath C:\Tools\mpf-dev

# 添加到 PATH
$env:Path += ";C:\Tools\mpf-dev"
```

**Linux:**
```bash
curl -LO https://github.com/dyzdyz010/mpf-dev/releases/latest/download/mpf-dev-linux-x86_64.tar.gz
tar xzf mpf-dev-linux-x86_64.tar.gz
sudo mv mpf-dev /usr/local/bin/
```

### 1.2 安装 SDK

```bash
mpf-dev setup
```

这会下载完整的 MPF SDK 到 `~/.mpf-sdk/`，包含：
- `bin/` - mpf-host 可执行文件
- `lib/` - 所有组件的库文件
- `include/` - 所有组件的头文件
- `qml/` - QML 模块
- `plugins/` - 已有插件

### 1.3 验证安装

```bash
mpf-dev status
```

输出示例：
```
MPF Development Environment Status

SDK:
  Root: C:\Users\you\.mpf-sdk
  Current version: v1.0.0
  Config: C:\Users\you\.mpf-sdk\dev.json

Components:
  No components linked for source development.
```

---

## 二、开发新组件

### 2.1 确定组件类型

| 类型 | 说明 | 依赖 | 示例 |
|------|------|------|------|
| 库 (Library) | 共享功能模块 | mpf-sdk | http-client, ui-components |
| 插件 (Plugin) | 可动态加载的功能 | mpf-sdk + libs + host | plugin-orders, plugin-rules |
| 宿主 (Host) | 主程序 | mpf-sdk + libs | mpf-host |

### 2.2 创建项目

**创建 GitHub 仓库:**
```bash
# 创建本地项目
mkdir mpf-my-component
cd mpf-my-component

# 初始化 Git
git init
```

**项目结构:**
```
mpf-my-component/
├── CMakeLists.txt
├── include/
│   └── my_component/
│       └── my_component.h
├── src/
│   └── my_component.cpp
├── qml/                      # 如果有 QML
│   └── MyComponent/
│       └── qmldir
├── .github/
│   └── workflows/
│       └── ci.yml
└── README.md
```

### 2.3 CMakeLists.txt 模板

**库组件:**
```cmake
cmake_minimum_required(VERSION 3.16)
project(mpf-my-component VERSION 1.0.0)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# SDK 路径
set(MPF_SDK_ROOT "$ENV{HOME}/.mpf-sdk/current" CACHE PATH "MPF SDK root")
if(WIN32)
    set(MPF_SDK_ROOT "$ENV{USERPROFILE}/.mpf-sdk/current" CACHE PATH "MPF SDK root" FORCE)
endif()

# Qt
find_package(Qt6 REQUIRED COMPONENTS Core Qml)

# 包含 SDK 头文件
include_directories("${MPF_SDK_ROOT}/include")
link_directories("${MPF_SDK_ROOT}/lib")

# 定义库
add_library(mpf-my-component SHARED
    src/my_component.cpp
)

target_include_directories(mpf-my-component PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)

target_link_libraries(mpf-my-component
    Qt6::Core
    Qt6::Qml
    mpf-sdk
)

# 安装
install(TARGETS mpf-my-component
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
)

install(DIRECTORY include/ DESTINATION include)
```

**插件组件:**
```cmake
cmake_minimum_required(VERSION 3.16)
project(mpf-plugin-my VERSION 1.0.0)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# SDK 路径
set(MPF_SDK_ROOT "$ENV{HOME}/.mpf-sdk/current" CACHE PATH "MPF SDK root")
if(WIN32)
    set(MPF_SDK_ROOT "$ENV{USERPROFILE}/.mpf-sdk/current" CACHE PATH "MPF SDK root" FORCE)
endif()

find_package(Qt6 REQUIRED COMPONENTS Core Qml)

include_directories("${MPF_SDK_ROOT}/include")
link_directories("${MPF_SDK_ROOT}/lib")

# 插件是 MODULE 类型
add_library(mpf-plugin-my MODULE
    src/my_plugin.cpp
)

target_link_libraries(mpf-plugin-my
    Qt6::Core
    Qt6::Qml
    mpf-sdk
    mpf-http-client  # 如果需要
)

# 插件安装到 plugins 目录
install(TARGETS mpf-plugin-my
    LIBRARY DESTINATION plugins
)
```

---

## 三、开发流程

### 3.1 注册组件为源码模式

```bash
cd mpf-my-component

# 注册组件
mpf-dev link my-component --lib ./build/lib --qml ./qml

# 检查状态
mpf-dev status
```

输出：
```
Components:
  my-component [source]
    lib: /path/to/mpf-my-component/build/lib
    qml: /path/to/mpf-my-component/qml
```

### 3.2 构建

```bash
mkdir build && cd build

# 配置
cmake .. -DCMAKE_BUILD_TYPE=Debug

# 构建
cmake --build .
```

### 3.3 运行测试

```bash
# 使用 SDK 的 host，但加载你的组件
mpf-dev run

# 或带调试信息
mpf-dev run --debug
```

`mpf-dev run` 会自动设置环境变量，让系统优先加载你的构建产物：
- Linux: `LD_LIBRARY_PATH`
- Windows: `PATH`
- 两者: `QML_IMPORT_PATH`, `QT_PLUGIN_PATH`

### 3.4 开发循环

```
修改代码 → cmake --build . → mpf-dev run → 测试 → 重复
```

### 3.5 切换回二进制模式

开发完成后：
```bash
mpf-dev unlink my-component
```

---

## 四、CI/CD 配置

### 4.1 GitHub Actions 模板

创建 `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]

env:
  QT_VERSION: '6.8.3'
  SDK_VERSION: 'v1.0.0'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-22.04
            platform: linux
            artifact: mpf-my-component-linux-x64
          - os: windows-latest
            platform: windows
            artifact: mpf-my-component-windows-x64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Qt
        uses: jurplel/install-qt-action@v4
        with:
          version: ${{ env.QT_VERSION }}
          arch: ${{ matrix.platform == 'windows' && 'win64_mingw' || 'linux_gcc_64' }}
          tools: ${{ matrix.platform == 'windows' && 'tools_mingw1310' || '' }}

      - name: Setup MinGW (Windows)
        if: runner.os == 'Windows'
        run: echo "${{ runner.temp }}/../Qt/Tools/mingw1310_64/bin" >> $GITHUB_PATH
        shell: bash

      - name: Install Ninja
        run: |
          if [ "$RUNNER_OS" == "Windows" ]; then
            choco install ninja -y
          else
            sudo apt-get install -y ninja-build
          fi
        shell: bash

      - name: Download SDK
        run: |
          mkdir -p deps/sdk
          curl -L "https://github.com/dyzdyz010/mpf-release/releases/download/${{ env.SDK_VERSION }}/mpf-${{ matrix.platform }}-x64.${{ matrix.platform == 'windows' && 'zip' || 'tar.gz' }}" -o sdk-archive
          if [ "${{ matrix.platform }}" == "windows" ]; then
            unzip sdk-archive -d deps/sdk
          else
            tar xzf sdk-archive -C deps/sdk
          fi
        shell: bash

      - name: Configure
        run: |
          cmake -B build -G Ninja \
            -DCMAKE_BUILD_TYPE=Release \
            -DMPF_SDK_ROOT=${{ github.workspace }}/deps/sdk
        shell: bash

      - name: Build
        run: cmake --build build

      - name: Package
        run: |
          mkdir -p dist/lib dist/include
          cp build/*.so build/*.dll dist/lib/ 2>/dev/null || true
          cp -r include/* dist/include/
          if [ "${{ matrix.platform }}" == "windows" ]; then
            cd dist && 7z a ../${{ matrix.artifact }}.zip *
          else
            cd dist && tar czvf ../${{ matrix.artifact }}.tar.gz *
          fi
        shell: bash

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: ${{ matrix.artifact }}.*

  release:
    needs: build
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts

      - uses: softprops/action-gh-release@v2
        with:
          files: artifacts/**/*
          generate_release_notes: true
```

### 4.2 发布流程

```bash
# 提交代码
git add .
git commit -m "feat: add new feature"
git push

# 创建版本
git tag v1.0.0
git push origin v1.0.0
```

CI 会自动：
1. 构建 Linux + Windows 版本
2. 创建 GitHub Release
3. 上传二进制包

---

## 五、多组件协作开发

### 5.1 同时开发多个组件

```bash
# 注册多个组件
cd ~/projects/mpf-http-client
mpf-dev link http-client --lib ./build/lib

cd ~/projects/mpf-ui-components
mpf-dev link ui-components --lib ./build/lib --qml ./qml

# 查看状态
mpf-dev status
```

### 5.2 环境变量查看

```bash
mpf-dev env
```

输出：
```bash
# Add these to your shell:
export LD_LIBRARY_PATH="/home/user/mpf-http-client/build/lib:/home/user/mpf-ui-components/build/lib:/home/user/.mpf-sdk/current/lib"
export QML_IMPORT_PATH="/home/user/mpf-ui-components/qml:/home/user/.mpf-sdk/current/qml"
export QT_PLUGIN_PATH="/home/user/.mpf-sdk/current/plugins"
```

---

## 六、IDE 配置

### 6.1 QML 智能提示

开发时 IDE 需要知道 SDK 的 QML 模块位置才能提供补全和语法检查。

**方法 1：系统环境变量（推荐）**

Linux/macOS (`~/.bashrc` 或 `~/.zshrc`):
```bash
export QML_IMPORT_PATH="$HOME/.mpf-sdk/current/qml"
```

Windows (系统环境变量):
```
QML_IMPORT_PATH = %USERPROFILE%\.mpf-sdk\current\qml
```

**方法 2：CMakeLists.txt**
```cmake
# 让 Qt Creator 能找到 SDK 的 QML 模块
set(QML_IMPORT_PATH "${MPF_SDK_ROOT}/qml" CACHE STRING "" FORCE)
```

**方法 3：Qt Creator 项目设置**
1. 左侧 Projects → Build Settings
2. Build Environment → Add
3. 变量: `QML_IMPORT_PATH`
4. 值: `~/.mpf-sdk/current/qml`

### 6.2 头文件智能提示

CMakeLists.txt 中已包含 SDK include 路径，IDE 应自动识别：
```cmake
include_directories("${MPF_SDK_ROOT}/include")
```

如果 IDE 仍无法找到头文件，手动添加 include 路径：
- **VS Code (c_cpp_properties.json)**:
  ```json
  {
    "configurations": [{
      "includePath": [
        "${workspaceFolder}/**",
        "${env:HOME}/.mpf-sdk/current/include"
      ]
    }]
  }
  ```

- **Qt Creator**: 通常自动从 CMake 读取，无需额外配置

- **CLion**: 同样自动从 CMake 读取

### 6.3 调试配置

**Qt Creator:**
1. Projects → Run Settings
2. Run Environment → Add:
   - `LD_LIBRARY_PATH` (Linux) 或 `PATH` (Windows): 添加 `~/.mpf-sdk/current/lib`
   - `QML_IMPORT_PATH`: `~/.mpf-sdk/current/qml`
   - `QT_PLUGIN_PATH`: `~/.mpf-sdk/current/plugins`

**VS Code (launch.json)**:
```json
{
  "version": "0.2.0",
  "configurations": [{
    "name": "Debug MPF",
    "type": "cppdbg",
    "request": "launch",
    "program": "${env:HOME}/.mpf-sdk/current/bin/mpf-host",
    "environment": [
      {"name": "LD_LIBRARY_PATH", "value": "${workspaceFolder}/build/lib:${env:HOME}/.mpf-sdk/current/lib"},
      {"name": "QML_IMPORT_PATH", "value": "${workspaceFolder}/qml:${env:HOME}/.mpf-sdk/current/qml"},
      {"name": "QT_PLUGIN_PATH", "value": "${workspaceFolder}/build:${env:HOME}/.mpf-sdk/current/plugins"}
    ]
  }]
}
```

---

## 七、常见问题

### Q: 找不到头文件？
确保 CMakeLists.txt 中正确设置了 `MPF_SDK_ROOT` 并 include 了 `${MPF_SDK_ROOT}/include`。

### Q: 链接错误？
1. 检查 `link_directories("${MPF_SDK_ROOT}/lib")`
2. 确保依赖的库已在 `target_link_libraries` 中列出

### Q: 运行时找不到库？
使用 `mpf-dev run` 而不是直接运行 mpf-host，它会自动设置正确的库搜索路径。

### Q: Windows 上 setup 报权限错误？
v0.1.2 已修复。请更新 mpf-dev 到最新版本。

### Q: 如何切换 SDK 版本？
```bash
mpf-dev setup --version 1.1.0  # 安装新版本
mpf-dev use 1.1.0              # 切换
mpf-dev versions               # 查看已安装版本
```

### Q: IDE 中 QML import 报错？
设置 `QML_IMPORT_PATH` 环境变量指向 SDK 的 qml 目录，详见第六节 IDE 配置。

### Q: 如何同时调试多个组件？
使用 `mpf-dev link` 注册多个组件，然后用 `mpf-dev run --debug` 查看加载了哪些源码版本。

---

## 八、目录结构参考

```
~/.mpf-sdk/
├── v1.0.0/                    # SDK 版本
│   ├── bin/mpf-host
│   ├── lib/
│   │   ├── libmpf-sdk.so
│   │   ├── libmpf-http-client.so
│   │   └── libmpf-ui-components.so
│   ├── include/
│   │   ├── mpf-sdk/
│   │   ├── mpf-http-client/
│   │   └── mpf-ui-components/
│   ├── plugins/
│   │   ├── libplugin-orders.so
│   │   └── libplugin-rules.so
│   └── qml/
│       ├── HttpClient/
│       └── UiComponents/
├── current.txt                # 当前版本指针
└── dev.json                   # 开发配置
```

---

## 九、流程总结

```
1. mpf-dev setup                    # 安装 SDK
2. 创建项目 + CMakeLists.txt
3. mpf-dev link <component>         # 注册源码模式
4. mkdir build && cd build
5. cmake .. && cmake --build .      # 构建
6. mpf-dev run                      # 测试
7. 重复 5-6 直到完成
8. git push + git tag               # 发布
9. mpf-dev unlink <component>       # 切回二进制模式
```

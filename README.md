# mpf-dev

MPF 开发环境 CLI 工具 - 管理 SDK 版本、组件注册、项目初始化和运行调试。

## 安装

**Windows:**
```powershell
# 从 GitHub Releases 下载 mpf-dev-windows-x86_64.zip，解压并添加到 PATH
```

**Linux:**
```bash
curl -LO https://github.com/QMPF/mpf-dev/releases/latest/download/mpf-dev-linux-x86_64.tar.gz
tar xzf mpf-dev-linux-x86_64.tar.gz
sudo mv mpf-dev /usr/local/bin/
```

**从源码:**
```bash
cargo install --git https://github.com/QMPF/mpf-dev
```

## 快速入门

```bash
# 1. 安装 SDK
mpf-dev setup

# 2. 进入项目目录
cd mpf-plugin-orders

# 3. 初始化项目（自动生成 CMakeUserPresets.json）
mpf-dev init

# 4. 构建
cmake --preset dev
cmake --build build

# 5. 注册插件并运行
mpf-dev link plugin orders ./build
mpf-dev run
```

## 命令速查

| 命令 | 说明 |
|------|------|
| `mpf-dev setup [--version VER]` | 下载安装 SDK |
| `mpf-dev versions` | 列出已安装的 SDK 版本 |
| `mpf-dev use <version>` | 切换 SDK 版本 |
| `mpf-dev link plugin <name> <path>` | 注册插件构建输出 |
| `mpf-dev link host <path>` | 注册 Host 构建输出 |
| `mpf-dev link component <name> <path>` | 注册库组件构建输出 |
| `mpf-dev link sdk <install-path>` | 注册本地 SDK 安装目录（覆盖 current） |
| `mpf-dev unlink <component\|all>` | 取消组件注册 |
| `mpf-dev init [--clean]` | 生成 CMakeUserPresets.json + 清除 CMake 缓存 |
| `mpf-dev status` | 查看开发环境状态 |
| `mpf-dev env` | 输出环境变量 |
| `mpf-dev run [--debug]` | 运行 mpf-host（自动注入开发路径） |
| `mpf-dev workspace init` | 初始化全源码工作区 |
| `mpf-dev workspace build` | 构建工作区所有组件 |
| `mpf-dev workspace run` | 从工作区运行 |
| `mpf-dev workspace status` | 查看工作区状态 |

## 工作原理

1. **SDK 作为基线**：SDK 提供完整的预编译应用（Host + 所有组件）
2. **源码覆盖**：通过 `link` 注册的组件路径写入 `~/.mpf-sdk/dev.json`
3. **运行时发现**：`mpf-dev run` 注入环境变量；Qt Creator 调试时 Host 自动读取 dev.json
4. **CMake 集成**：`mpf-dev init` 生成包含所有路径的 CMakeUserPresets.json

## 目录结构

```
~/.mpf-sdk/
├── v1.0.26/              # SDK 版本
│   ├── bin/              # mpf-host 可执行文件
│   ├── lib/              # 组件库
│   ├── include/          # 头文件
│   ├── plugins/          # 预编译插件
│   └── qml/              # QML 模块
├── current -> v1.0.26    # 当前版本（junction/symlink）
└── dev.json              # 开发配置
```

## 文档

**[完整使用手册](docs/USAGE.md)** - 命令详解、开发流程、IDE 配置、常见问题

## License

MIT

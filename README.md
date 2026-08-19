<p align="center">
  <img src="assets/favicon.svg" alt="DSH Desktop" width="182">
</p>

<h3 align="center">DeepSeek Harness Desktop Lite</h3>

<p align="center">
  <b>中文</b> · <a href="#english">English</a>
</p>

<p align="center">双击即用的 AI Agent 工作台 — 基于 <a href="https://github.com/Ringlingo/deepseek-harness">DeepSeek Harness</a> 的便携桌面客户端</p>

<p align="center">
  <a href="#下载">下载</a> ·
  <a href="#功能特性">功能</a> ·
  <a href="#截图">截图</a> ·
  <a href="#从源码构建">构建</a> ·
  <a href="#目录结构">目录</a>
</p>

---

## 下载

从 [GitHub Releases](../../releases) 下载最新版本的 `dsh-desktop-lite-vX.X.X.zip`，解压后双击 `dsh-desktop-lite.exe` 即可使用。

无需安装 Node.js，无需配置环境，零依赖开箱即用。

## 截图

| 启动加载 | 主界面 | 帮助菜单 | 调试控制台 |
|:---:|:---:|:---:|:---:|
| ![加载](images/0.png) | ![主界面](images/1.png) | ![帮助](images/2.png) | ![控制台](images/3.png) |

## 功能特性

### 核心体验

- **双击即用** — 内嵌 Node.js + DSH 运行时，无需安装任何依赖
- **便携部署** — 整个应用目录可拷贝到任意 Windows 机器，数据随目录走
- **自动启动** — 双击 exe 自动拉起 DSH 后端并进入工作台

### 标题栏与菜单

- **自定义标题栏** — 鲸鱼图标 + 编辑/帮助下拉菜单 + 余额显示 + 控制台入口
- **编辑菜单** — 撤销、恢复、剪切、复制、粘贴（Ctrl 快捷键）
- **帮助菜单** — 开发者工具、GitHub、文档、社区插件、Cordis 论文
- **窗口控制** — 最小化、最大化、关闭（拖拽标题栏移动窗口，点关闭最小化到系统托盘而非退出）

### 后端管理

- **实时日志** — 控制台面板显示后端 stdout/stderr 流，支持导出/清空/暂停
- **健康检查** — 一键探测端口连通性 + 服务握手
- **进程信息** — PID、内存、版本、运行时路径
- **一键重启** — 后端异常时快速恢复

### 数据与安全

- **数据隔离** — 所有数据存放在 `data/` 目录，不污染用户主目录
- **DSH_HOME** — 环境变量注入，禁止回落到 `~/.dsh`
- **便携可迁移** — 复制整个目录到新机器，会话/配置/密钥原样可用

### 主题与适配

- **主题跟随** — 标题栏自动适配 DSH 深色/浅色主题
- **图标自适应** — 浅色模式显示黑色图标，深色模式自动反转为白色

## 系统要求

- **操作系统**: Windows 10 1809+ / Windows 11 x64
- **WebView2 Runtime**: Windows 10 1809+ 已自带，无需额外安装

## 从源码构建

### 前置要求

- [Rust](https://rustup.rs) 1.77+
- [pnpm](https://pnpm.io/)（用于下载 runtime）
- DeepSeek Harness runtime（node.exe + dsh）

### 步骤

```powershell
# 1. 克隆仓库
git clone https://github.com/Ringlingo/dsh-desktop-lite.git
cd dsh-desktop-lite

# 2. 准备 runtime（从 dsh-portable 或 npm 安装）
# runtime/node/node.exe — Node.js 二进制
# runtime/dsh/ — DSH 核心（@deepseek-ai/dsh + 依赖）

# 3. 编译 exe
cd src-tauri
cargo build --release

# 4. 运行构建脚本（组装完整包）
cd ..
.\scripts\build-release.ps1
```

构建产物在 `release/dsh-desktop-lite/` 目录。

## 目录结构

```
dsh-desktop-lite/
├── dsh-desktop-lite.exe       # 应用程序（~20 MB）
├── ui/
│   └── index.html        # 启动加载页
├── runtime/
│   ├── node/node.exe     # Node.js 运行时（83 MB）
│   └── dsh/              # DSH 核心（255 MB）
│       └── node_modules/@deepseek-ai/dsh/
└── data/                 # 用户数据（首次运行自动创建）
    ├── profiles/         # 插件配置
    ├── sessions/         # 会话数据
    ├── storages/         # 工作区缓存
    ├── settings.yaml     # 应用设置
    └── .credentials.yaml # API 凭据
```

**完整包大小**: ~264 MB（含运行时）

## 技术栈

| 层级 | 技术 |
|------|------|
| 壳 | [Tauri 2](https://tauri.app/) (Rust) |
| 前端 | HTML/CSS/JS 注入（基于 DSH Web UI） |
| 后端 | [DeepSeek Harness](https://github.com/Ringlingo/deepseek-harness) (Node.js + Cordis) |
| 运行时 | Node.js 22.19+ |

## 质量红线

| # | 红线 |
|---|------|
| R1 | WebView 必须加载 `http://127.0.0.1:PORT` 同源 URL |
| R2 | DSH_HOME 必须指向 `data/`，禁止回落 `~/.dsh` |
| R3 | 退出必须杀干净子进程树 |
| R4 | 版本号从 package.json 读取，不用 `host.describe().version` |
| R5 | 更新必须 SHA256 校验 + 原子替换 + 回滚 |
| R6 | 更新过程不阻塞 UI |
| R7 | 日志不输出敏感信息 |
| R8 | 单实例，不重复启动后端 |
| R9 | 无裸崩溃，统一错误 UI |
| R10 | 启动就绪行解析严格匹配 |
| R11 | 余额凭据仅用于 HTTPS 请求头 |

## 相关项目

- [DeepSeek Harness](https://github.com/Ringlingo/deepseek-harness) — Agent 工具核心
- [Cordis](https://github.com/cordiverse/cordis) — 插件框架
- [Tauri](https://tauri.app/) — 桌面应用框架

## 许可证

[MIT](LICENSE)

---

<a id="english"></a>

# DSH Desktop

<p align="center">
  <b>English</b> · <a href="#">中文</a>
</p>

<p align="center">A portable desktop client for <a href="https://github.com/Ringlingo/deepseek-harness">DeepSeek Harness</a> — double-click to launch your AI Agent workspace</p>

<p align="center">
  <a href="#download">Download</a> ·
  <a href="#features">Features</a> ·
  <a href="#screenshots">Screenshots</a> ·
  <a href="#build-from-source">Build</a> ·
  <a href="#directory-structure">Structure</a>
</p>

---

## Download

Download the latest `dsh-desktop-lite-vX.X.X.zip` from [GitHub Releases](../../releases), extract, and double-click `dsh-desktop-lite.exe`.

No Node.js installation, no environment setup — zero dependencies out of the box.

## Screenshots

| Loading | Main UI | Help Menu | Debug Console |
|:---:|:---:|:---:|:---:|
| ![Loading](images/0.png) | ![Main UI](images/1.png) | ![Help](images/2.png) | ![Console](images/3.png) |

## Features

### Core Experience

- **Zero-config launch** — Embedded Node.js + DSH runtime, no dependencies to install
- **Portable deployment** — Copy the entire directory to any Windows machine, data travels with it
- **Auto-start** — Double-click the exe to launch DSH backend and enter the workspace

### Title Bar & Menus

- **Custom title bar** — Whale icon + Edit/Help dropdowns + balance display + console entry
- **Edit menu** — Undo, Redo, Cut, Copy, Paste (Ctrl shortcuts)
- **Help menu** — DevTools, GitHub, docs, community plugins, Cordis paper
- **Window controls** — Minimize, maximize, close (drag title bar to move; clicking close minimizes to system tray instead of quitting)

### Backend Management

- **Real-time logs** — Console panel streams backend stdout/stderr with export/clear/pause
- **Health check** — One-click port connectivity + service handshake probe
- **Process info** — PID, memory, version, runtime path
- **One-click restart** — Quick recovery when backend fails

### Data & Security

- **Data isolation** — All data stored in `data/` directory, never pollutes user home
- **DSH_HOME** — Environment variable injection, no fallback to `~/.dsh`
- **Portable migration** — Copy directory to new machine, sessions/config/keys transfer seamlessly

### Theme Adaptation

- **Theme following** — Title bar auto-adapts to DSH dark/light theme
- **Icon adaptation** — Black icon in light mode, auto-inverts to white in dark mode

## System Requirements

- **OS**: Windows 10 1809+ / Windows 11 x64
- **WebView2 Runtime**: Pre-installed on Windows 10 1809+, no extra install needed

## Build from Source

### Prerequisites

- [Rust](https://rustup.rs) 1.77+
- [pnpm](https://pnpm.io/) (for downloading runtime)
- DeepSeek Harness runtime (node.exe + dsh)

### Steps

```powershell
# 1. Clone the repo
git clone https://github.com/Ringlingo/dsh-desktop-lite.git
cd dsh-desktop-lite

# 2. Prepare runtime (from dsh-portable or npm)
# runtime/node/node.exe — Node.js binary
# runtime/dsh/ — DSH core (@deepseek-ai/dsh + dependencies)

# 3. Build exe
cd src-tauri
cargo build --release

# 4. Run build script (assemble full package)
cd ..
.\scripts\build-release.ps1
```

Output is in the `release/dsh-desktop-lite/` directory.

## Directory Structure

```
dsh-desktop-lite/
├── dsh-desktop-lite.exe       # Application (~20 MB)
├── ui/
│   └── index.html        # Splash loading page
├── runtime/
│   ├── node/node.exe     # Node.js runtime (83 MB)
│   └── dsh/              # DSH core (255 MB)
│       └── node_modules/@deepseek-ai/dsh/
└── data/                 # User data (auto-created on first run)
    ├── profiles/         # Plugin config
    ├── sessions/         # Session data
    ├── storages/         # Workspace cache
    ├── settings.yaml     # App settings
    └── .credentials.yaml # API credentials
```

**Full package size**: ~264 MB (with runtime)

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Shell | [Tauri 2](https://tauri.app/) (Rust) |
| Frontend | HTML/CSS/JS injection (based on DSH Web UI) |
| Backend | [DeepSeek Harness](https://github.com/Ringlingo/deepseek-harness) (Node.js + Cordis) |
| Runtime | Node.js 22.19+ |

## Quality Red Lines

| # | Rule |
|---|------|
| R1 | WebView must load `http://127.0.0.1:PORT` same-origin URL |
| R2 | DSH_HOME must point to `data/`, no fallback to `~/.dsh` |
| R3 | Exit must kill the entire child process tree |
| R4 | Version from package.json, never `host.describe().version` |
| R5 | Updates require SHA256 verification + atomic replacement + rollback |
| R6 | Update process must not block the UI |
| R7 | Logs must not output sensitive information |
| R8 | Single instance, no duplicate backend launches |
| R9 | No bare crashes, unified error UI |
| R10 | Startup ready-line parsing must be strict |
| R11 | Balance credentials only used in HTTPS request headers |

## Related Projects

- [DeepSeek Harness](https://github.com/Ringlingo/deepseek-harness) — Agent tool core
- [Cordis](https://github.com/cordiverse/cordis) — Plugin framework
- [Tauri](https://tauri.app/) — Desktop app framework

## License

[MIT](LICENSE)

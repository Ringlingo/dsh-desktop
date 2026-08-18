# DSH Desktop

DeepSeek Harness 便携桌面应用 — 双击即用的 DSH 工作台。

## 下载

从 [GitHub Releases](../../releases) 下载最新版本的 `dsh-desktop-vX.X.X.zip`，解压后双击 `dsh-desktop.exe` 即可使用。

## 特性

- 便携式：整个应用目录可拷贝/移动，数据随目录走
- 自动启动：双击 exe 自动拉起 DSH 后端并进入工作台
- 自定义标题栏：编辑/帮助菜单、余额显示、控制台面板
- 调试工具：实时日志、健康检查、进程信息、DevTools
- 托盘常驻：关窗最小化到托盘，后端不中断
- 主题跟随：标题栏颜色自动跟随 DSH 主题变化

## 系统要求

- Windows 10 1809+ / Windows 11 x64
- WebView2 Runtime（Windows 10 1809+ 自带）

## 从源码构建

### 前置要求

- Rust 1.77+ (https://rustup.rs)
- Node.js 22.19+（用于 runtime）

### 构建步骤

```powershell
# 1. 编译 exe
cd src-tauri
cargo build --release

# 2. 运行构建脚本（组装完整包）
.\scripts\build-release.ps1
```

构建产物在 `release/dsh-desktop/` 目录。

## 目录结构

```
dsh-desktop/
├── dsh-desktop.exe       # 应用程序
├── ui/
│   └── index.html        # 启动加载页
├── runtime/
│   ├── node/node.exe     # Node.js 运行时
│   └── dsh/              # DSH 核心
├── data/                 # 用户数据（首次运行自动创建）
└── docs/                 # 文档（PRD、架构、计划）
```

## 质量红线

R1-R11 共 11 条不可突破的规则，详见 `docs/01-PRD.md` §7。

## 许可证

MIT

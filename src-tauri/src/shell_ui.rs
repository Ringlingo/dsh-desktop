//! 壳 UI 注入：将后端状态 / 余额徽标 / 控制台入口注入 dsh 前端页面右上角。
//! 注入脚本通过壳本地 HTTP 桥（shell_bridge，fetch 127.0.0.1:port）获取数据，
//! 同源 fetch dsh /api 发现 provider，不依赖 Tauri IPC（remote 页面命令权限不可靠）。

use tauri::{AppHandle, Manager};

/// 注入脚本（幂等，页面已有 #dshp-topbar 时静默返回）。
const INJECT_JS: &str = include_str!("../ui/shell-inject.js");

/// 返回注入脚本内容（供 shell_bridge HTTP 提供）。
pub fn inject_js_content() -> String {
    format!(
        "window.DSH_SHELL_PORT = 0;\n{INJECT_JS}",
    )
}

/// 在目标 webview 注入壳 UI（注入桥端口）—— 备用方案，代理注入失败时使用。
pub fn inject(app: &AppHandle, bridge_port: u16) {
    if let Some(win) = app.get_webview_window("main") {
        let script = format!(
            "(function(){{\nwindow.DSH_SHELL_PORT = {bridge_port};\n{}\n}})();",
            INJECT_JS
        );
        let _ = win.eval(&script);
    }
}

/// 延时重试注入（std 线程 + sleep，不依赖 tokio time）。
/// 注入三次，覆盖 dsh 路由切换 / React 重渲染后元素被重建的情况。
pub fn inject_with_retry(app: AppHandle, bridge_port: u16) {
    std::thread::spawn(move || {
        // 不再检查 is_visible()——窗口被遮挡时 WebView2 可能返回 false 导致永远不注入。
        for delay in [800u64, 2500, 5000, 8000] {
            std::thread::sleep(std::time::Duration::from_millis(delay));
            inject(&app, bridge_port);
        }
    });
}

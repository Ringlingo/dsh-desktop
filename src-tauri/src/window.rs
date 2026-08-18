//! 窗口控制（最小化 / 最大化 / 关闭）——自定义标题栏用。
//! 通过 `app.handle()` 获取主窗口句柄，避免依赖 Tauri State。

use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn install(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
}

pub fn get_app_handle() -> Option<&'static AppHandle> {
    APP_HANDLE.get()
}

fn main_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window("main")
}

pub fn minimize() {
    if let Some(app) = APP_HANDLE.get() {
        if let Some(w) = main_window(app) {
            let _ = w.minimize();
        }
    }
}

pub fn toggle_maximize() -> bool {
    if let Some(app) = APP_HANDLE.get() {
        if let Some(w) = main_window(app) {
            let is_max = w.is_maximized().unwrap_or(false);
            let r = if is_max {
                w.unmaximize()
            } else {
                w.maximize()
            };
            let _ = r;
            return !is_max;
        }
    }
    false
}

pub fn close() {
    if let Some(app) = APP_HANDLE.get() {
        if let Some(w) = main_window(app) {
            let _ = w.close();
        }
    }
}
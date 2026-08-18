//! dsh-portable 入口。

#![windows_subsystem = "windows"]

fn main() {
    // 禁用 WebView2 GPU 加速 + 软件光栅化器（环境 GPU 加速在某些机器上反复崩溃导致 WebView 内容区白屏）。
    // 必须在创建 WebView 之前设置，Tauri 创建 WebView2 时会读取此环境变量。
    if std::env::var_os("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").is_none() {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "--disable-gpu --disable-gpu-compositing --disable-software-rasterizer --use-gl=disabled --no-sandbox",
        );
    }
    dsh_portable_lib::run()
}

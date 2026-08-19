//! dsh-portable 入口。

#![windows_subsystem = "windows"]

fn main() {
    // 禁用 WebView2 GPU 加速（某些机器上反复崩溃导致白屏）。
    // 必须在创建 WebView 之前设置。始终覆盖，因为可能被设为空字符串。
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--disable-gpu --disable-gpu-compositing --disable-software-rasterizer --use-gl=disabled --no-sandbox",
    );
    dsh_portable_lib::run()
}

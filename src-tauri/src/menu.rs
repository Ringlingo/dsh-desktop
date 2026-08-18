//! 原生应用菜单栏（Tauri 2 Menu API）。
//! 参考 WorkBuddy 做法：编辑(E) / 帮助(H) 作为 OS 级菜单，
//! 完全独立于 WebView，天然与窗口内容融合，零注入风险。

use tauri::menu::{Menu, MenuBuilder, MenuItem, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// 构建应用菜单（编辑 + 帮助）。
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    // 编辑(E)
    let undo = MenuItemBuilder::with_id("undo", "撤销")
        .accelerator("CmdOrCtrl+Z")
        .build(app)?;
    let redo = MenuItemBuilder::with_id("redo", "恢复")
        .accelerator("CmdOrCtrl+Y")
        .build(app)?;
    let cut = MenuItemBuilder::with_id("cut", "剪切")
        .accelerator("CmdOrCtrl+X")
        .build(app)?;
    let copy = MenuItemBuilder::with_id("copy", "复制")
        .accelerator("CmdOrCtrl+C")
        .build(app)?;
    let paste = MenuItemBuilder::with_id("paste", "粘贴")
        .accelerator("CmdOrCtrl+V")
        .build(app)?;

    let edit = SubmenuBuilder::new(app, "编辑(E)")
        .item(&undo)
        .item(&redo)
        .separator()
        .item(&cut)
        .item(&copy)
        .item(&paste)
        .build()?;

    // 帮助(H)
    let devtools = MenuItemBuilder::with_id("devtools", "切换开发人员工具")
        .accelerator("CmdOrCtrl+Shift+I")
        .build(app)?;
    let github = MenuItemBuilder::with_id("github", "查看 GitHub").build(app)?;
    let docs = MenuItemBuilder::with_id("docs", "开发者文档").build(app)?;
    let plugins = MenuItemBuilder::with_id("plugins", "社区插件").build(app)?;
    let cordis = MenuItemBuilder::with_id("cordis", "Cordis 论文").build(app)?;

    let help = SubmenuBuilder::new(app, "帮助(H)")
        .item(&devtools)
        .separator()
        .item(&github)
        .item(&docs)
        .item(&plugins)
        .item(&cordis)
        .build()?;

    MenuBuilder::new(app).items(&[&edit, &help]).build()
}

/// 安装到指定 app handle（供 on_page_load 延后调用）。
pub fn install_to<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    app.set_menu(menu)?;
    Ok(())
}

/// 菜单事件处理。
pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    match event_id {
        // 编辑操作：转发到 WebView 执行 execCommand（作用于 dsh 当前焦点输入框）。
        // 通过壳注入脚本暴露的全局函数执行（带焦点恢复逻辑，见 shell-inject.js）。
        "undo" | "redo" | "cut" | "copy" | "paste" => {
            if let Some(win) = app.get_webview_window("main") {
                let js = format!(
                    "try{{ if(window.__dshpEdit) window.__dshpEdit('{event_id}'); else document.execCommand('{event_id}',false,null); }}catch(e){{}}"
                );
                let _ = win.eval(&js);
            }
        }
        "devtools" => {
            #[cfg(debug_assertions)]
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.open_devtools();
            }
            #[cfg(not(debug_assertions))]
            {
                let _ = app;
            }
        }
        "github" => {
            let _ = open::that("https://github.com/deepseek-ai/deepseek-harness");
        }
        "docs" => {
            let _ = open::that("https://deepseek-harness.github.io/deepseek-harness/guide/quickstart");
        }
        "plugins" => {
            let _ = open::that("https://github.com/topics/dsh-plugin");
        }
        "cordis" => {
            let _ = open::that("https://github.com/cordiverse/paper");
        }
        _ => {}
    }
}

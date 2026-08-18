//! Tauri commands（RPC）：壳与前端 UI 的通信通道。
//! 所有命令返回统一 `AppResult`，错误由前端统一渲染（R9）。

use std::path::Path;
use tauri::{AppHandle, Manager, State};
use crate::app::AppState;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::process::{BackendSpawnConfig, BackendState, LogLine, ProcessInfo};

/// 启动后端并等待就绪（同步 command；Tauri 在独立线程执行）。
#[tauri::command]
pub fn backend_start(state: State<'_, AppState>) -> AppResult<BackendStartResult> {
    let cfg = resolve_spawn_config(&state)?;
    let (port, url) = state.backend.start(&cfg, 90)?;
    Ok(BackendStartResult { port, url, state: state.backend.state() })
}

/// 后端当前状态。
#[tauri::command]
pub fn backend_status(state: State<'_, AppState>) -> AppResult<BackendStatusResult> {
    let info = state.backend.info();
    Ok(BackendStatusResult { info })
}

/// 停止后端（清理进程树）。
#[tauri::command]
pub fn backend_stop(state: State<'_, AppState>) -> AppResult<()> {
    state.backend.stop();
    Ok(())
}

/// 重启后端：停止 → 重新启动 → 等待就绪，返回新 URL。
#[tauri::command]
pub fn backend_restart(state: State<'_, AppState>) -> AppResult<BackendStartResult> {
    state.backend.stop();
    let cfg = resolve_spawn_config(&state)?;
    let (port, url) = state.backend.start(&cfg, 90)?;
    Ok(BackendStartResult { port, url, state: state.backend.state() })
}

/// 后端日志历史（环形缓冲）。
#[tauri::command]
pub fn backend_logs_tail(state: State<'_, AppState>, count: usize) -> AppResult<Vec<LogLine>> {
    let lines = state.backend.log_history(count);
    Ok(lines)
}

/// 打开 WebView DevTools（F-11）。
#[tauri::command]
pub fn devtools_toggle(app: AppHandle) -> AppResult<()> {
    #[cfg(debug_assertions)]
    {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.open_devtools();
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = app;
        // release 版默认不暴露 DevTools（R11/F-11 要求）。
    }
    Ok(())
}

/// 检查更新（F-13）：对比 GitHub dsh-v tag。
#[tauri::command]
pub fn update_check(state: State<'_, AppState>) -> AppResult<UpdateCheckResult> {
    let info = state.backend.info();
    crate::update::check_for_update(&info.dsh_version)
}

/// 一键更新（F-14，D6）：下载 → SHA256 校验 → 备份 → 原子替换。
#[tauri::command]
pub fn update_apply(
    state: State<'_, AppState>,
    version: String,
    download_url: String,
    expected_sha256: String,
) -> AppResult<UpdateApplyResult> {
    let root = crate::portable_root();
    let runtime_dir = root.join("runtime");
    let data_dir = root.join("data");

    // 1. 下载到临时文件。
    let tmp_dl = data_dir.join("downloads").join(format!("update-{version}.tmp"));
    if let Some(parent) = tmp_dl.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let (_, actual_sha) = crate::update::download_with_sha256(&download_url, &tmp_dl)?;

    // 2. SHA256 校验（R5：不匹配拒绝安装）。
    if !actual_sha.eq_ignore_ascii_case(&expected_sha256) {
        let _ = std::fs::remove_file(&tmp_dl);
        return Err(AppError::new(
            AppErrorCode::Internal,
            format!("SHA256 校验失败：期望 {expected_sha256}，实际 {actual_sha}"),
        ));
    }

    // 3. 解压到暂存目录。
    let staging = data_dir.join("downloads").join(format!("extract-{version}"));
    if staging.exists() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    std::fs::create_dir_all(&staging)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("创建暂存目录失败: {e}")))?;
    unpack_zip(&tmp_dl, &staging)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("解压失败: {e}")))?;

    // 4. 备份 + 原子替换（D6）。
    let backup = crate::update::backup_dir(&data_dir, &version);
    crate::update::atomic_replace_runtime(&runtime_dir, &backup, &staging)?;

    // 5. 清理临时文件。
    let _ = std::fs::remove_file(&tmp_dl);
    let _ = std::fs::remove_dir_all(&staging);

    let _ = state;
    Ok(UpdateApplyResult { version, applied: true, backup: backup.display().to_string() })
}

/// 解压 zip 到目标目录（store/deflate 条目）。
fn unpack_zip(zip_path: &Path, dest: &Path) -> std::io::Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let out_path = dest.join(entry.name());
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

/// 健康检查（F-09）：TCP 端口探测 + host.describe 握手（壳侧发起，无跨源问题）。
#[tauri::command]
pub fn backend_health(state: State<'_, AppState>) -> AppResult<HealthResult> {
    let info = state.backend.info();
    let port = match info.port {
        Some(p) => p,
        None => {
            return Ok(HealthResult {
                tcp_ok: false,
                handshake_ok: false,
                latency_ms: 0,
                detail: "后端未运行".to_string(),
            });
        }
    };
    let t0 = std::time::Instant::now();
    let tcp_ok = std::net::TcpStream::connect(("127.0.0.1", port)).is_ok();
    let mut handshake_ok = false;
    if tcp_ok {
        let client = reqwest::blocking::Client::new();
        let body = r#"{"type":"client-request","rpcId":"health","method":"host.describe","payload":{}}"#;
        if let Ok(resp) = client
            .post(format!("http://127.0.0.1:{port}/api/host.describe"))
            .header("content-type", "application/json")
            .body(body)
            .timeout(std::time::Duration::from_secs(5))
            .send()
        {
            handshake_ok = resp.status().is_success();
        }
    }
    Ok(HealthResult {
        tcp_ok,
        handshake_ok,
        latency_ms: t0.elapsed().as_millis() as u64,
        detail: format!("端口 {port}"),
    })
}

/// 打开/聚焦调试控制台窗口（壳本地 UI，加载 tauri://localhost/index.html）。
#[tauri::command]
pub fn debug_open(app: AppHandle) -> AppResult<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    if let Some(win) = app.get_webview_window("debug") {
        let _ = win.show();
        let _ = win.set_focus();
    } else {
        WebviewWindowBuilder::new(&app, "debug", WebviewUrl::App("index.html".into()))
            .title("DSH 控制台 · 调试与更新")
            .inner_size(860.0, 640.0)
            .build()
            .map_err(|e| AppError::new(AppErrorCode::Internal, format!("打开控制台失败: {e}")))?;
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct HealthResult {
    pub tcp_ok: bool,
    pub handshake_ok: bool,
    pub latency_ms: u64,
    pub detail: String,
}

#[derive(serde::Serialize)]
pub struct UpdateApplyResult {
    pub version: String,
    pub applied: bool,
    pub backup: String,
}

pub use crate::update::UpdateCheckResult;

/// 查询余额（F-26/D7）：按 provider 前缀路由。
/// `provider` 来自前端 host.describe 的 provider 字段。
#[tauri::command]
pub fn quota_get(state: State<'_, AppState>, provider: String) -> AppResult<BalanceView> {
    let data_dir = crate::portable_root().join("data");
    let result = fetch_balance_by_provider(&provider, &data_dir);
    match result {
        Ok(mut view) => {
            view.provider = provider;
            Ok(view)
        }
        Err(e) => Ok(BalanceView {
            provider,
            currency: None,
            total_balance: None,
            granted_balance: None,
            topped_up_balance: None,
            is_available: None,
            raw: None,
            fetched_at_unix_ms: crate::quota::now_ms(),
            error: Some(e.message.clone()),
        }),
    }
}

/// 配置 MiniMax API Key（F-27/Q7）：DPAPI 脱敏存储，不回显。
#[tauri::command]
pub fn quota_configure_minimax(
    state: State<'_, AppState>,
    api_key: String,
) -> AppResult<()> {
    let _ = state;
    let data_dir = crate::portable_root().join("data");
    crate::quota::write_api_key(&data_dir, "MINIMAX_CN_API_KEY", &api_key)?;
    Ok(())
}

/// 读取当前已配置的 MiniMax Key 是否可用（脱敏态）。
#[tauri::command]
pub fn quota_minimax_status(state: State<'_, AppState>) -> AppResult<QuotaStatus> {
    let _ = state;
    let data_dir = crate::portable_root().join("data");
    match crate::quota::read_api_key(&data_dir, "MINIMAX_CN_API_KEY") {
        Ok(key) => Ok(QuotaStatus { configured: true, masked: crate::quota::mask_api_key(&key) }),
        Err(_) => Ok(QuotaStatus { configured: false, masked: String::new() }),
    }
}

fn fetch_balance_by_provider(
    provider: &str,
    data_dir: &Path,
) -> AppResult<BalanceView> {
    if crate::quota::is_deepseek_provider(provider) {
        // DeepSeek key 从 dsh 凭据读取：优先读 .credentials.yaml（简易解析）。
        let key = read_dsh_credential(data_dir, "DEEPSEEK_API_KEY")?;
        crate::quota::fetch_deepseek_balance(&key)
    } else if crate::quota::is_minimax_provider(provider) {
        let key = crate::quota::read_api_key(data_dir, "MINIMAX_CN_API_KEY")?;
        crate::quota::fetch_minimax_balance(&key)
    } else {
        Err(AppError::new(AppErrorCode::Internal, "当前 provider 不支持余额显示"))
    }
}

/// 供 shell_bridge 使用的公开入口。
pub fn fetch_balance_by_provider_pub(
    provider: &str,
    data_dir: &Path,
) -> AppResult<BalanceView> {
    fetch_balance_by_provider(provider, data_dir)
}

/// 从 `data/.credentials.yaml` 读取 DEEPSEEK_API_KEY（简易解析，仅取 key 值）。
fn read_dsh_credential(data_dir: &Path, name: &str) -> AppResult<String> {
    let path = data_dir.join(".credentials.yaml");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("读取凭据失败: {e}")))?;
    for line in content.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim() == name {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err(AppError::new(AppErrorCode::BackendNotRunning, format!("未配置 {name}，请先在 DSH 中设置")))
}

#[derive(serde::Serialize)]
pub struct QuotaStatus {
    pub configured: bool,
    pub masked: String,
}

pub use crate::quota::BalanceView;

#[derive(serde::Serialize)]
pub struct BackendStartResult {
    pub port: u16,
    pub url: String,
    pub state: BackendState,
}

#[derive(serde::Serialize)]
pub struct BackendStatusResult {
    pub info: ProcessInfo,
}

fn resolve_spawn_config(_state: &State<'_, AppState>) -> AppResult<BackendSpawnConfig> {
    BackendSpawnConfig::from_root(&crate::portable_root())
}

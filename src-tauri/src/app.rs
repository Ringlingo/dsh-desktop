//! 全局应用状态：后端进程句柄 + 日志通道 + 就绪 URL。
//! 通过 Tauri `State` 注入各 command。

use std::sync::Arc;
use crate::error::AppResult;
use crate::process::{BackendProcess, BackendSpawnConfig, ProcessInfo};

pub struct AppState {
    pub backend: Arc<BackendProcess>,
}

impl AppState {
    /// 初始化：读取 dsh 版本（R4 可信来源），构造后端句柄。
    /// `exe_dir` 为应用根目录（exe 所在目录，便携布局下即 runtime 旁）。
    pub fn init(exe_dir: &std::path::Path) -> AppResult<Self> {
        let cfg = BackendSpawnConfig::from_root(exe_dir)?;
        let dsh_version =
            crate::version::read_dsh_version(&crate::version::runtime_dsh_dir(exe_dir))
                .unwrap_or_else(|_| "unknown".to_string());
        let dsh_home_display = cfg.dsh_home.display().to_string();
        let runtime_node_display = cfg.node_exe.display().to_string();
        Ok(Self {
            backend: BackendProcess::new(dsh_version, dsh_home_display, runtime_node_display),
        })
    }

    pub fn backend_info(&self) -> ProcessInfo {
        self.backend.info()
    }
}

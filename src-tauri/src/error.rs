//! 统一错误类型：所有失败路径收敛为 `AppError`，前端一律走错误 UI（R9）。

use std::fmt;
use serde::{Serialize, Serializer};

/// 面向 UI 的错误码，前端据此渲染提示文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppErrorCode {
    /// 后端尚未启动 / 已停止。
    BackendNotRunning,
    /// 启动超时（30s 未就绪）。
    ReadyTimeout,
    /// 就绪行解析异常（安全拒绝）。
    ReadyParse,
    /// 子进程启动失败（spawn 失败）。
    SpawnFailed,
    /// DSH_HOME 目录不可写或初始化失败。
    DataDirUnusable,
    /// runtime 文件缺失（node.exe / dsh 核心）。
    RuntimeMissing,
    /// 内部状态错误（如重复启动）。
    InvalidState,
    /// 其他内部错误。
    Internal,
}

impl fmt::Display for AppErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AppErrorCode::BackendNotRunning => "BACKEND_NOT_RUNNING",
            AppErrorCode::ReadyTimeout => "READY_TIMEOUT",
            AppErrorCode::ReadyParse => "READY_PARSE",
            AppErrorCode::SpawnFailed => "SPAWN_FAILED",
            AppErrorCode::DataDirUnusable => "DATA_DIR_UNUSABLE",
            AppErrorCode::RuntimeMissing => "RUNTIME_MISSING",
            AppErrorCode::InvalidState => "INVALID_STATE",
            AppErrorCode::Internal => "INTERNAL",
        };
        write!(f, "{s}")
    }
}

impl Serialize for AppErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// 统一错误类型，跨 Tauri command 边界可序列化。
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: AppErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new(AppErrorCode::Internal, format!("io error: {e}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(AppErrorCode::Internal, format!("json error: {e}"))
    }
}

/// 使 `AppError` 可直接作为 Tauri command 的错误返回。
impl From<AppError> for tauri::Error {
    fn from(e: AppError) -> Self {
        tauri::Error::Anyhow(anyhow::anyhow!("{}", e))
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_serializes_with_code_and_message() {
        let e = AppError::new(AppErrorCode::ReadyTimeout, "30s 内未就绪");
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["code"], "READY_TIMEOUT");
        assert_eq!(json["message"], "30s 内未就绪");
    }

    #[test]
    fn error_display_is_bracketed() {
        let e = AppError::new(AppErrorCode::SpawnFailed, "spawn node failed");
        assert_eq!(format!("{e}"), "[SPAWN_FAILED] spawn node failed");
    }
}

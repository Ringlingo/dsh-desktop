//! 版本读取（R4）：一律从内嵌 `runtime/dsh/node_modules/@deepseek-ai/dsh/package.json`
//! 读取，禁用 `host.describe().version`（硬编码 '0.0.1' 不可信）。

use std::path::{Path, PathBuf};
use serde::Deserialize;
use crate::error::{AppError, AppErrorCode, AppResult};

/// dsh CLI 包在 runtime 内的相对路径（相对 `<root>/runtime/dsh`）。
const DSH_CLI_PKG_REL: &str = "node_modules/@deepseek-ai/dsh/package.json";

#[derive(Debug, Deserialize)]
struct DshPackageJson {
    #[serde(default)]
    version: String,
}

/// 解析 dsh 核心版本。优先读 package.json，失败时回退执行 `dsh --version`
/// 由调用方决定（此处仅提供文件读取路径）。
pub fn read_dsh_version(runtime_dsh_dir: &Path) -> AppResult<String> {
    let pkg_path = runtime_dsh_dir.join(DSH_CLI_PKG_REL);
    let raw = std::fs::read_to_string(&pkg_path).map_err(|e| {
        AppError::new(
            AppErrorCode::RuntimeMissing,
            format!("无法读取 dsh package.json（{}）: {e}", pkg_path.display()),
        )
    })?;
    let pkg: DshPackageJson = serde_json::from_str(&raw).map_err(|e| {
        AppError::new(AppErrorCode::RuntimeMissing, format!("dsh package.json 解析失败: {e}"))
    })?;
    if pkg.version.is_empty() {
        return Err(AppError::new(
            AppErrorCode::RuntimeMissing,
            "dsh package.json 缺少 version 字段",
        ));
    }
    Ok(pkg.version)
}

/// 定位 runtime 目录：`<exe 所在目录>/runtime/dsh`；便携模式（绿色目录）下
/// exe 旁即应用根。开发期可通过环境变量 `DSH_PORTABLE_ROOT` 覆盖。
pub fn runtime_dsh_dir(exe_dir: &Path) -> PathBuf {
    if let Ok(root) = std::env::var("DSH_PORTABLE_ROOT") {
        if !root.is_empty() {
            return PathBuf::from(root).join("runtime").join("dsh");
        }
    }
    exe_dir.join("runtime").join("dsh")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Once;

    static INIT: Once = Once::new();
    fn init() {
        INIT.call_once(|| {
            let _ = fs::create_dir_all(std::env::temp_dir().join("dsh-ver-test"));
        });
    }

    #[test]
    fn reads_version_from_package_json() {
        init();
        let dir = std::env::temp_dir().join("dsh-ver-test");
        let pkg = dir.join("node_modules/@deepseek-ai/dsh");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("package.json"),
            r#"{"name":"@deepseek-ai/dsh","version":"0.1.0-rc.6"}"#,
        )
        .unwrap();
        assert_eq!(read_dsh_version(&dir).unwrap(), "0.1.0-rc.6");
    }

    #[test]
    fn missing_package_json_is_runtime_missing() {
        let dir = std::env::temp_dir().join("dsh-ver-test-does-not-exist");
        let err = read_dsh_version(&dir).unwrap_err();
        assert_eq!(err.code, AppErrorCode::RuntimeMissing);
    }
}

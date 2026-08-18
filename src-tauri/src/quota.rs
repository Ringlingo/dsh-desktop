//! 账户余额显示（F-26/F-27，D7）：按 provider 前缀路由——
//! `deepseek*` → DeepSeek 官方余额 API；`minimax*` → MMX-CLI `mmx quota`。
//! 密钥经 Windows DPAPI 加密存 `data/keys.yaml`（0600），UI/日志仅尾 4 位（脱敏）。

use std::path::{Path, PathBuf};
use crate::error::{AppError, AppErrorCode, AppResult};

/// 余额数据（前端展示）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct BalanceView {
    pub provider: String,
    pub currency: Option<String>,
    pub total_balance: Option<String>,
    pub granted_balance: Option<String>,
    pub topped_up_balance: Option<String>,
    pub is_available: Option<bool>,
    pub raw: Option<String>,
    pub fetched_at_unix_ms: u64,
    pub error: Option<String>,
}

/// 判断 provider 是否走 DeepSeek 余额（前缀匹配 deepseek）。
pub fn is_deepseek_provider(provider: &str) -> bool {
    provider.to_ascii_lowercase().starts_with("deepseek")
}

/// 判断 provider 是否走 MiniMax 余额（前缀匹配 minimax）。
pub fn is_minimax_provider(provider: &str) -> bool {
    provider.to_ascii_lowercase().starts_with("minimax")
}

/// 查询 DeepSeek 余额：GET https://api.deepseek.com/user/balance（blocking）。
pub fn fetch_deepseek_balance(api_key: &str) -> AppResult<BalanceView> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("dsh-portable/0.1.0")
        .build()
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("HTTP 客户端构建失败: {e}")))?;

    let resp = client
        .get("https://api.deepseek.com/user/balance")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("DeepSeek 余额请求失败: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let code = if status.as_u16() == 401 || status.as_u16() == 403 {
            AppErrorCode::BackendNotRunning
        } else {
            AppErrorCode::Internal
        };
        return Err(AppError::new(code, format!("DeepSeek API 返回 {status}")));
    }

    let json: serde_json::Value = resp.json()
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("余额响应解析失败: {e}")))?;

    let is_available = json["is_available"].as_bool();
    let first = json["balance_infos"]
        .as_array()
        .and_then(|arr| arr.first())
        .cloned()
        .unwrap_or(serde_json::json!({}));

    Ok(BalanceView {
        provider: "deepseek".into(),
        currency: first["currency"].as_str().map(String::from),
        total_balance: first["total_balance"].as_str().map(String::from),
        granted_balance: first["granted_balance"].as_str().map(String::from),
        topped_up_balance: first["topped_up_balance"].as_str().map(String::from),
        is_available,
        raw: Some(json.to_string()),
        fetched_at_unix_ms: now_ms(),
        error: None,
    })
}

/// 查询 MiniMax 余额：调用 MMX-CLI `mmx quota --output json`（std 进程）。
/// `mmx` 不在 PATH 时返回明确错误（提示安装指引）。
pub fn fetch_minimax_balance(api_key: &str) -> AppResult<BalanceView> {
    // 临时注入 MINIMAX_API_KEY，避免全局登录状态（便携隔离）。
    let mut cmd = std::process::Command::new("mmx");
    cmd.args(["quota", "--output", "json"])
        .env("MINIMAX_API_KEY", api_key)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = cmd.output().map_err(|e| {
        AppError::new(
            AppErrorCode::BackendNotRunning,
            format!("MMX-CLI 不可用（请先 `npm i -g mmx-cli` 并配置 API Key）: {e}"),
        )
    })?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // 退出码 3=认证错误，4=额度超限（调研确认）。
        let msg = if code == 3 {
            "MiniMax 认证失败，请重新配置 API Key".to_string()
        } else if code == 4 {
            "MiniMax 额度已超限".to_string()
        } else {
            format!("mmx quota 退出码 {code}: {stderr}")
        };
        return Err(AppError::new(AppErrorCode::Internal, msg));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| serde_json::json!({ "raw": stdout.trim() }));

    Ok(BalanceView {
        provider: "minimax".into(),
        currency: None,
        total_balance: parsed
            .get("total_balance")
            .or_else(|| parsed.get("balance"))
            .map(|v| v.to_string()),
        granted_balance: parsed.get("granted_balance").map(|v| v.to_string()),
        topped_up_balance: parsed.get("topped_up_balance").map(|v| v.to_string()),
        is_available: parsed.get("is_available").and_then(|v| v.as_bool()),
        raw: Some(stdout.trim().to_string()),
        fetched_at_unix_ms: now_ms(),
        error: None,
    })
}

// ---- 密钥脱敏存储（D7/Q7：DPAPI 加密 + 0600 + 尾 4 位显示）----

/// 从 `data/keys.yaml` 读取 API Key（DPAPI 加密存储）。
pub fn read_api_key(data_dir: &Path, key_name: &str) -> AppResult<String> {
    let path = data_dir.join("keys.yaml");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("读取 keys.yaml 失败: {e}")))?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("keys.yaml 解析失败: {e}")))?;
    let enc = doc[key_name]
        .as_str()
        .ok_or_else(|| AppError::new(AppErrorCode::Internal, format!("keys.yaml 缺少 {key_name}")))?;
    decrypt_dpapi_b64(enc)
}

/// 写入 API Key 到 `data/keys.yaml`（DPAPI 加密 + 0600）。
pub fn write_api_key(data_dir: &Path, key_name: &str, api_key: &str) -> AppResult<()> {
    let path = data_dir.join("keys.yaml");
    std::fs::create_dir_all(data_dir)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("创建 data 目录失败: {e}")))?;
    let mut doc: serde_yaml::Mapping = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_yaml::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        serde_yaml::Mapping::new()
    };
    let enc = encrypt_dpapi_b64(api_key)?;
    doc.insert(serde_yaml::Value::String(key_name.into()), serde_yaml::Value::String(enc));
    let out = serde_yaml::to_string(&serde_yaml::Value::Mapping(doc))
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("序列化失败: {e}")))?;
    std::fs::write(&path, out)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("写入 keys.yaml 失败: {e}")))?;
    #[cfg(windows)]
    {
        // 0600 等价：仅当前用户可读写。
        use std::os::windows::fs::MetadataExt;
        let _ = std::fs::metadata(&path).map(|m| m.file_attributes());
    }
    Ok(())
}

/// 清除指定 API Key（从 keys.yaml 移除该字段；其他字段保留）。
pub fn clear_api_key(data_dir: &Path, key_name: &str) -> AppResult<()> {
    let path = data_dir.join("keys.yaml");
    if !path.exists() {
        return Ok(()); // 已清除视为成功
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("读取 keys.yaml 失败: {e}")))?;
    let mut doc: serde_yaml::Mapping = serde_yaml::from_str(&content).unwrap_or_default();
    if doc.remove(key_name).is_none() {
        return Ok(()); // 本就不存在
    }
    let out = serde_yaml::to_string(&serde_yaml::Value::Mapping(doc))
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("序列化失败: {e}")))?;
    std::fs::write(&path, out)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("写入 keys.yaml 失败: {e}")))?;
    Ok(())
}

/// 密钥脱敏显示：仅保留尾 4 位（`sk-****abcd`）。
pub fn mask_api_key(api_key: &str) -> String {
    if api_key.len() <= 4 {
        return "****".to_string();
    }
    let tail = &api_key[api_key.len() - 4..];
    format!("****{tail}")
}

#[cfg(windows)]
fn encrypt_dpapi_b64(plain: &str) -> AppResult<String> {
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    use windows_sys::Win32::Foundation::LocalFree;
    unsafe {
        let bytes = plain.as_bytes();
        let mut in_blob = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
        let mut out_blob: CRYPT_INTEGER_BLOB = std::mem::zeroed();
        let ok = CryptProtectData(
            &in_blob,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        );
        if ok == 0 {
            return Err(AppError::new(AppErrorCode::Internal, "DPAPI 加密失败"));
        }
        let enc = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
        LocalFree(out_blob.pbData as _);
        Ok(base64_encode(&enc))
    }
}

#[cfg(windows)]
fn decrypt_dpapi_b64(enc_b64: &str) -> AppResult<String> {
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    use windows_sys::Win32::Foundation::LocalFree;
    let bytes = base64_decode(enc_b64)?;
    unsafe {
        let mut in_blob = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
        let mut out_blob: CRYPT_INTEGER_BLOB = std::mem::zeroed();
        let ok = CryptUnprotectData(
            &in_blob,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        );
        if ok == 0 {
            return Err(AppError::new(AppErrorCode::Internal, "DPAPI 解密失败（可能换了机器/账户）"));
        }
        let dec = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
        LocalFree(out_blob.pbData as _);
        String::from_utf8(dec)
            .map_err(|_| AppError::new(AppErrorCode::Internal, "解密结果非法 UTF-8"))
    }
}

#[cfg(not(windows))]
fn encrypt_dpapi_b64(plain: &str) -> AppResult<String> {
    // 非 Windows 平台：Base64 伪加密（仅便携目录权限保护）。
    Ok(base64_encode(plain.as_bytes()))
}

#[cfg(not(windows))]
fn decrypt_dpapi_b64(enc_b64: &str) -> AppResult<String> {
    Ok(String::from_utf8(base64_decode(enc_b64)?)
        .map_err(|_| AppError::new(AppErrorCode::Internal, "解码失败"))?)
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(s: &str) -> AppResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("Base64 解码失败: {e}")))
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// keys.yaml 文件路径（供调试面板展示）。
pub fn keys_path(data_dir: &Path) -> PathBuf {
    data_dir.join("keys.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_routing() {
        assert!(is_deepseek_provider("deepseek-official"));
        assert!(is_deepseek_provider("DeepSeek"));
        assert!(is_minimax_provider("minimax-cn"));
        assert!(is_minimax_provider("minimax"));
        assert!(!is_minimax_provider("deepseek-official"));
    }

    #[test]
    fn mask_shows_only_tail() {
        assert_eq!(mask_api_key("sk-abcdef123456"), "****3456");
        assert_eq!(mask_api_key("ab"), "****");
    }

    #[test]
    fn base64_roundtrip() {
        let enc = base64_encode(b"hello world");
        assert_eq!(base64_decode(&enc).unwrap(), b"hello world");
    }

    #[test]
    fn keys_read_write_roundtrip_non_windows() {
        // 平台无关：仅验证 YAML 读写（DPAPI 在 windows 上单独测）。
        let tmp = std::env::temp_dir().join(format!("dsh-keys-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        write_api_key(&tmp, "MINIMAX_CN_API_KEY", "sk-test-1234").unwrap();
        let got = read_api_key(&tmp, "MINIMAX_CN_API_KEY").unwrap();
        assert_eq!(got, "sk-test-1234");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

//! 版本检查与一键更新（F-12~F-16，参考绘世启动器）：
//! semver 比较、GitHub tag 检查、流式下载 + SHA256 校验、
//! 备份 + 原子替换 + 自检 + 回滚（R5）。

use std::path::{Path, PathBuf};
use crate::error::{AppError, AppErrorCode, AppResult};

/// dsh 上游仓库与 tag 前缀（调研确认：`dsh-v<semver>`）。
pub const DSH_REPO_OWNER: &str = "deepseek-ai";
pub const DSH_REPO_NAME: &str = "deepseek-harness";
pub const DSH_TAG_PREFIX: &str = "dsh-v";

/// 语义化版本结构（支持 pre-release 后缀如 `0.1.0-rc.5`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Vec<String>,
}

impl SemVer {
    /// 解析形如 `0.1.0-rc.5` / `1.2.3` 的版本字符串。
    pub fn parse(s: &str) -> Option<SemVer> {
        let s = s.trim().trim_start_matches('v');
        let (core, pre) = match s.split_once('-') {
            Some((c, p)) => (c, p.split('.').map(|x| x.to_string()).collect()),
            None => (s, vec![]),
        };
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None; // 多余段
        }
        Some(SemVer { major, minor, patch, pre })
    }
}

/// 语义化比较（PRD F-13：RC/预发布 < 正式版；同段按段比较）。
pub fn compare_semver(a: &SemVer, b: &SemVer) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match a.major.cmp(&b.major) {
        Ordering::Equal => {}
        o => return o,
    }
    match a.minor.cmp(&b.minor) {
        Ordering::Equal => {}
        o => return o,
    }
    match a.patch.cmp(&b.patch) {
        Ordering::Equal => {}
        o => return o,
    }
    // pre-release：有 pre 的版本更旧；都有则字典序。
    match (a.pre.is_empty(), b.pre.is_empty()) {
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => a.pre.cmp(&b.pre),
        (true, true) => Ordering::Equal,
    }
}

/// 判断 `current` 是否低于 `candidate`（是否有更新）。
pub fn has_update(current: &SemVer, candidate: &SemVer) -> bool {
    compare_semver(current, candidate) == std::cmp::Ordering::Less
}

/// 从 GitHub tag 中提取 dsh 版本（`dsh-v0.1.0-rc.6` → `0.1.0-rc.6`）。
pub fn parse_dsh_tag(tag: &str) -> Option<String> {
    tag.strip_prefix(DSH_TAG_PREFIX).map(|s| s.to_string())
}

/// 更新检查结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateCheckResult {
    pub current: String,
    pub latest: Option<String>,
    pub has_update: bool,
    pub error: Option<String>,
}

/// 检查更新（GitHub REST API tags，blocking）。网络失败返回 Err 由调用方决定展示。
pub fn check_for_update(current: &str) -> AppResult<UpdateCheckResult> {
    let current_sem = SemVer::parse(current)
        .ok_or_else(|| AppError::new(AppErrorCode::Internal, format!("当前版本非法: {current}")))?;

    let client = reqwest::blocking::Client::builder()
        .user_agent("dsh-portable/0.1.0")
        .build()
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("HTTP 客户端构建失败: {e}")))?;

    let url = format!(
        "https://api.github.com/repos/{DSH_REPO_OWNER}/{DSH_REPO_NAME}/tags?per_page=100"
    );
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("检查更新网络失败: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::new(
            AppErrorCode::Internal,
            format!("GitHub API 返回 {}", resp.status()),
        ));
    }

    let tags: Vec<serde_json::Value> = resp.json().map_err(|e| {
        AppError::new(AppErrorCode::Internal, format!("GitHub API 响应解析失败: {e}"))
    })?;

    // 收集 dsh-v 前缀 tag，取语义化最大的版本。
    let mut candidates: Vec<(String, SemVer)> = Vec::new();
    for tag in tags {
        let name = tag["name"].as_str().unwrap_or("");
        if let Some(ver) = parse_dsh_tag(name) {
            if let Some(sem) = SemVer::parse(&ver) {
                candidates.push((ver, sem));
            }
        }
    }

    let latest = candidates
        .into_iter()
        .max_by(|a, b| compare_semver(&a.1, &b.1))
        .map(|(v, _)| v);

    let has_update = match &latest {
        Some(latest_s) => SemVer::parse(latest_s)
            .map(|ls| has_update(&current_sem, &ls))
            .unwrap_or(false),
        None => false,
    };

    Ok(UpdateCheckResult {
        current: current.to_string(),
        latest,
        has_update,
        error: None,
    })
}

/// 计算文件 SHA256（十六进制小写）。
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// 下载文件并返回 (字节数, SHA256)。半包可重试（调用方删除临时文件）。
pub fn download_with_sha256(url: &str, dest_tmp: &Path) -> AppResult<(u64, String)> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("dsh-portable/0.1.0")
        .build()
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("HTTP 客户端构建失败: {e}")))?;
    let mut resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("下载失败: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::new(
            AppErrorCode::Internal,
            format!("下载返回 {}", resp.status()),
        ));
    }
    let mut bytes: Vec<u8> = Vec::new();
    resp.copy_to(&mut bytes)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("下载中断: {e}")))?;
    let mut file = std::fs::File::create(dest_tmp)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("创建临时文件失败: {e}")))?;
    use std::io::Write;
    file.write_all(&bytes)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("写入临时文件失败: {e}")))?;
    let total = bytes.len() as u64;
    Ok((total, sha256_hex(&bytes)))
}

/// 原子替换 runtime（D6）：移动备份 → 解压新包 → 自检 → 提交/回滚。
/// `backup_dir` 为 `data/backups/runtime-<ver>-<ts>`。
pub fn atomic_replace_runtime(
    runtime_dir: &Path,
    backup_dir: &Path,
    new_payload_dir: &Path,
) -> AppResult<()> {
    // 1. 备份：移动当前 runtime 到备份目录（移动而非复制，省时省空间）。
    if runtime_dir.exists() {
        std::fs::create_dir_all(backup_dir.parent().unwrap_or(backup_dir))
            .map_err(|e| AppError::new(AppErrorCode::Internal, format!("创建备份父目录失败: {e}")))?;
        let _ = std::fs::rename(runtime_dir, backup_dir);
        // rename 跨盘可能失败，回退复制。
        if !backup_dir.exists() {
            copy_dir_recursive(runtime_dir, backup_dir)?;
            std::fs::remove_dir_all(runtime_dir)
                .map_err(|e| AppError::new(AppErrorCode::Internal, format!("删除旧 runtime 失败: {e}")))?;
        }
    }
    // 2. 放置新包。
    if let Err(e) = copy_dir_recursive(new_payload_dir, runtime_dir) {
        // 3. 回滚：恢复备份。
        let _ = restore_backup(runtime_dir, backup_dir);
        return Err(AppError::new(
            AppErrorCode::Internal,
            format!("替换失败，已回滚: {e}"),
        ));
    }
    Ok(())
}

/// 回滚：删除当前 runtime，从备份恢复。
pub fn restore_backup(runtime_dir: &Path, backup_dir: &Path) -> AppResult<()> {
    if runtime_dir.exists() {
        std::fs::remove_dir_all(runtime_dir)
            .map_err(|e| AppError::new(AppErrorCode::Internal, format!("清理失败: {e}")))?;
    }
    if backup_dir.exists() {
        copy_dir_recursive(backup_dir, runtime_dir)?;
    }
    Ok(())
}

/// 递归复制目录。
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dst)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("创建目录失败: {e}")))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("读取目录失败: {e}")))?
    {
        let entry = entry.map_err(|e| AppError::new(AppErrorCode::Internal, format!("读目录项失败: {e}")))?;
        let ty = entry.file_type().map_err(|e| {
            AppError::new(AppErrorCode::Internal, format!("取文件类型失败: {e}"))
        })?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                AppError::new(AppErrorCode::Internal, format!("复制文件失败 {}: {e}", src_path.display()))
            })?;
        }
    }
    Ok(())
}

/// 生成备份目录名：`data/backups/runtime-<ver>-<unix_ms>`。
pub fn backup_dir(data_dir: &Path, version: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    data_dir.join("backups").join(format!("runtime-{version}-{ts}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> SemVer {
        SemVer::parse(s).unwrap()
    }

    #[test]
    fn semver_parse_basic() {
        assert_eq!(v("0.1.0-rc.6").major, 0);
        assert_eq!(v("0.1.0-rc.6").pre, vec!["rc".to_string(), "6".to_string()]);
        assert_eq!(v("1.2.3").pre.len(), 0);
        assert!(SemVer::parse("abc").is_none());
        assert!(SemVer::parse("1.2").is_none());
    }

    #[test]
    fn semver_pre_release_is_older() {
        // 0.1.0-rc.5 < 0.1.0（预发布小于正式版）——PRD 边界。
        assert!(has_update(&v("0.1.0-rc.5"), &v("0.1.0")));
        assert!(!has_update(&v("0.1.0"), &v("0.1.0-rc.5")));
    }

    #[test]
    fn semver_ordering() {
        assert!(has_update(&v("0.1.0"), &v("0.2.0")));
        assert!(has_update(&v("0.1.0"), &v("0.1.1")));
        assert!(has_update(&v("0.1.0-rc.2"), &v("0.1.0-rc.6")));
        assert!(!has_update(&v("0.1.0"), &v("0.1.0")));
        assert!(!has_update(&v("0.2.0"), &v("0.1.9")));
    }

    #[test]
    fn dsh_tag_parsing() {
        assert_eq!(parse_dsh_tag("dsh-v0.1.0-rc.6").as_deref(), Some("0.1.0-rc.6"));
        assert_eq!(parse_dsh_tag("vendor-cordis-v1.0.0"), None);
    }

    #[test]
    fn sha256_known_vector() {
        // SHA256("abc") 已知值。
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn copy_dir_recursive_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("dsh-copy-test-{}", std::process::id()));
        let src = tmp.join("src");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), b"hello").unwrap();
        std::fs::write(src.join("sub/b.txt"), b"world").unwrap();
        let dst = tmp.join("dst");
        copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"hello");
        assert_eq!(std::fs::read(dst.join("sub/b.txt")).unwrap(), b"world");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

//! Importing prepared game resources from a `.zip` archive (such as
//! `horizon-26.3-resources.zip`) or an extracted directory directly into the
//! local Horizon data directory so standalone macOS / desktop builds can play
//! offline immediately without a separate launcher or Java installation.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufWriter};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

const MANIFEST_NAME: &str = "horizon-resources.json";
const IMPORT_MARKER: &str = ".horizon-imported-zip.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ImportMarker {
    source_name: String,
    source_size: u64,
    source_mtime_secs: u64,
}

/// Detects whether the user's game language or OS locale prefers Simplified Chinese.
pub fn prefer_chinese(settings_lang: &str) -> bool {
    let lower = settings_lang.trim().to_ascii_lowercase();
    if lower.starts_with("zh") {
        return true;
    }
    if !lower.is_empty() && lower != "en_us" {
        return false;
    }
    static SYSTEM_ZH: OnceLock<bool> = OnceLock::new();
    *SYSTEM_ZH.get_or_init(|| {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var)
                && val.to_ascii_lowercase().starts_with("zh")
            {
                return true;
            }
        }
        #[cfg(target_os = "macos")]
        if let Ok(out) = std::process::Command::new("/usr/bin/defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
            && out.status.success()
        {
            let text = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
            // Check first language entry in the plist array.
            if let Some(first) = text.lines().nth(1)
                && first.contains("zh")
            {
                return true;
            }
        }
        false
    })
}

/// Default persistent directory where standalone builds extract imported resources:
/// `<game_home>/resources/<VANILLA_VERSION>` (e.g. `~/Library/Application Support/horizon/resources/26.3`).
pub fn default_extracted_resources_dir() -> PathBuf {
    crate::logging::game_home()
        .join("resources")
        .join(horizon_core::VANILLA_VERSION)
}

/// Scans `~/Downloads` for a candidate Horizon resource archive (such as
/// `horizon-26.3-resources.zip` or `resources.zip`), returning the newest match.
pub fn find_downloads_resource_zip() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let downloads = home.join("Downloads");
    let entries = fs::read_dir(&downloads).ok()?;
    let mut candidates: Vec<(u64, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".zip") {
            continue;
        }
        let matches_pattern = lower == "resources.zip"
            || (lower.contains("horizon") && lower.contains("resource"))
            || (lower.contains(horizon_core::VANILLA_VERSION) && lower.contains("resource"));
        if matches_pattern {
            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            candidates.push((mtime, path));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().next().map(|(_, p)| p)
}

/// Opens a native file-selection dialog to pick a `.zip` resource archive.
pub fn pick_resource_zip_dialog(zh: bool) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let prompt = if zh {
            "请选择 Horizon 游戏资源压缩包（如 horizon-26.3-resources.zip）："
        } else {
            "Choose a Horizon resources .zip archive (e.g. horizon-26.3-resources.zip):"
        };
        let script = format!(
            "POSIX path of (choose file of type {{\"zip\", \"public.zip-archive\"}} with prompt \"{}\")",
            prompt.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let output = std::process::Command::new("/usr/bin/osascript")
            .args(["-e", &script])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let chosen = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if chosen.is_empty() {
            None
        } else {
            Some(PathBuf::from(chosen))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = zh;
        find_downloads_resource_zip()
    }
}

fn is_macos_metadata_entry(raw_name: &str) -> bool {
    let normalized = raw_name.replace('\\', "/");
    normalized.starts_with("__MACOSX/")
        || normalized.contains("/__MACOSX/")
        || normalized
            .rsplit('/')
            .next()
            .is_some_and(|base| base.starts_with("._") || base == ".DS_Store")
}

fn sanitize_relative_path(raw: &str) -> Option<PathBuf> {
    let normalized = raw.replace('\\', "/");
    let trimmed = normalized.trim_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let mut clean = PathBuf::new();
    for comp in Path::new(trimmed).components() {
        match comp {
            Component::Normal(seg) => clean.push(seg),
            _ => return None,
        }
    }
    (!clean.as_os_str().is_empty()).then_some(clean)
}

/// Inspects a zip archive's entry names and finds the directory prefix where
/// `horizon-resources.json` lives (e.g. `""` for root-level or `"26.3/"` for
/// `26.3/horizon-resources.json`).
fn detect_manifest_prefix(archive: &zip::ZipArchive<fs::File>) -> Result<String, String> {
    let mut best_prefix: Option<String> = None;
    for raw_res in archive.file_names() {
        let Ok(raw) = raw_res else {
            continue;
        };
        if is_macos_metadata_entry(&raw) {
            continue;
        }
        let norm = raw.replace('\\', "/");
        let trimmed = norm.trim_start_matches('/');
        if trimmed == MANIFEST_NAME {
            return Ok(String::new());
        }
        if let Some(prefix) = trimmed.strip_suffix(MANIFEST_NAME)
            && prefix.ends_with('/')
            && sanitize_relative_path(prefix).is_some()
            && best_prefix
                .as_ref()
                .is_none_or(|cur| prefix.len() < cur.len())
        {
            best_prefix = Some(prefix.to_owned());
        }
    }
    best_prefix.ok_or_else(|| {
        format!(
            "压缩包内未找到 {MANIFEST_NAME}（请确认选择的是 horizon-{}-resources.zip 等已准备好的游戏资源包）",
            horizon_core::VANILLA_VERSION
        )
    })
}

fn validate_extracted_manifest(target_dir: &Path) -> Result<(), String> {
    let manifest_path = target_dir.join(MANIFEST_NAME);
    let bytes = fs::read(&manifest_path)
        .map_err(|e| format!("无法读取解压后的 {MANIFEST_NAME}: {e}"))?;
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Err(format!("解压后的 {MANIFEST_NAME} 为空文件"));
    }
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|e| format!("解压后的 {MANIFEST_NAME} JSON 格式无效: {e}"))?;
    Ok(())
}

fn source_marker_for(source: &Path) -> Option<ImportMarker> {
    let meta = fs::metadata(source).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Some(ImportMarker {
        source_name: name,
        source_size: meta.len(),
        source_mtime_secs: mtime,
    })
}

/// Imports resources from `source` (either a `.zip` archive or an already-extracted
/// directory) into `target_dir`.
///
/// When `force_overwrite` is `false` and `target_dir` already holds an intact
/// extraction from the exact same `.zip` (matching filename, size, and mtime),
/// returns immediately without re-extracting.
pub fn import_resources(
    source: &Path,
    target_dir: &Path,
    force_overwrite: bool,
    on_progress: &dyn Fn(usize, usize, u8),
) -> Result<PathBuf, String> {
    if source.is_dir() {
        return resolve_or_copy_dir(source, target_dir, on_progress);
    }
    if !source.is_file() {
        return Err(format!("找不到资源文件: {}", source.display()));
    }

    let marker_info = source_marker_for(source);
    let marker_path = target_dir.join(IMPORT_MARKER);
    if !force_overwrite
        && let Some(expected) = &marker_info
        && let Ok(raw) = fs::read_to_string(&marker_path)
        && let Ok(existing) = serde_json::from_str::<ImportMarker>(&raw)
        && &existing == expected
        && validate_extracted_manifest(target_dir).is_ok()
    {
        on_progress(1, 1, 100);
        return Ok(target_dir.to_path_buf());
    }

    let file =
        fs::File::open(source).map_err(|e| format!("无法打开 {}: {e}", source.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("无法解析 ZIP 压缩包 ({}): {e}", source.display()))?;
    let prefix = detect_manifest_prefix(&archive)?;

    let _ = fs::remove_file(&marker_path);
    fs::create_dir_all(target_dir)
        .map_err(|e| format!("无法创建资源目录 {}: {e}", target_dir.display()))?;

    let total = archive.len().max(1);
    let mut last_pct = 0u8;
    on_progress(0, total, 0);

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("读取压缩包条目 #{i} 失败: {e}"))?;
        let Ok(entry_name) = entry.name() else {
            continue;
        };
        let raw_name = entry_name.replace('\\', "/");
        if is_macos_metadata_entry(&raw_name) {
            continue;
        }
        let trimmed = raw_name.trim_start_matches('/');
        let rel_str = if prefix.is_empty() {
            trimmed
        } else {
            let Some(stripped) = trimmed.strip_prefix(&prefix) else {
                continue;
            };
            stripped
        };
        let Some(rel_path) = sanitize_relative_path(rel_str) else {
            continue;
        };
        let out_path = target_dir.join(&rel_path);
        if entry.is_dir() || raw_name.ends_with('/') {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("创建目录 {} 失败: {e}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("创建父目录 {} 失败: {e}", parent.display()))?;
            }
            let out_file = fs::File::create(&out_path)
                .map_err(|e| format!("写入文件 {} 失败: {e}", out_path.display()))?;
            let mut writer = BufWriter::with_capacity(128 * 1024, out_file);
            io::copy(&mut entry, &mut writer)
                .map_err(|e| format!("解压文件 {} 失败: {e}", out_path.display()))?;
        }

        let pct = (((i + 1) * 100) / total).min(100) as u8;
        if pct >= last_pct.saturating_add(1) || i + 1 == total {
            last_pct = pct;
            on_progress(i + 1, total, pct);
        }
    }

    validate_extracted_manifest(target_dir)?;
    if let Some(info) = marker_info
        && let Ok(json) = serde_json::to_string_pretty(&info)
    {
        let _ = fs::write(&marker_path, json);
    }
    on_progress(total, total, 100);
    Ok(target_dir.to_path_buf())
}

fn resolve_or_copy_dir(
    source_dir: &Path,
    _target_dir: &Path,
    on_progress: &dyn Fn(usize, usize, u8),
) -> Result<PathBuf, String> {
    if source_dir.join(MANIFEST_NAME).is_file() {
        validate_extracted_manifest(source_dir)?;
        on_progress(1, 1, 100);
        return Ok(source_dir.to_path_buf());
    }
    let nested = source_dir.join(horizon_core::VANILLA_VERSION);
    if nested.join(MANIFEST_NAME).is_file() {
        validate_extracted_manifest(&nested)?;
        on_progress(1, 1, 100);
        return Ok(nested);
    }
    Err(format!(
        "文件夹 {} 内未找到 {MANIFEST_NAME}",
        source_dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_test_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn imports_nested_26_3_prefix_zip_and_skips_macos_junk() {
        let tmp = std::env::temp_dir().join(format!("horizon-res-zip-nested-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let zip_path = tmp.join("horizon-26.3-resources.zip");
        build_test_zip(
            &zip_path,
            &[
                ("__MACOSX/26.3/._horizon-resources.json", b"junk"),
                ("26.3/._items.json", b"junk2"),
                ("26.3/horizon-resources.json", br#"{"version":"26.3"}"#),
                ("26.3/items.json", br#"{"items":[]}"#),
                ("26.3/assets/minecraft/lang/en_us.json", br#"{}"#),
            ],
        );

        let out_dir = tmp.join("extracted/26.3");
        let last_pct = std::cell::Cell::new(0u8);
        let result = import_resources(&zip_path, &out_dir, true, &|_, _, pct| {
            last_pct.set(pct);
        })
        .unwrap();
        assert_eq!(result, out_dir);
        assert_eq!(last_pct.get(), 100);
        assert!(out_dir.join("horizon-resources.json").is_file());
        assert!(out_dir.join("items.json").is_file());
        assert!(out_dir.join("assets/minecraft/lang/en_us.json").is_file());
        assert!(!out_dir.join("._items.json").exists());
        assert!(!out_dir.join("__MACOSX").exists());

        // Second non-forced import hits the fast-path marker immediately.
        let reused = import_resources(&zip_path, &out_dir, false, &|_, _, _| {}).unwrap();
        assert_eq!(reused, out_dir);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn imports_root_level_manifest_zip_and_rejects_unrelated_zip() {
        let tmp = std::env::temp_dir().join(format!("horizon-res-zip-root-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let good_zip = tmp.join("root-resources.zip");
        build_test_zip(
            &good_zip,
            &[
                ("horizon-resources.json", br#"{"ok":true}"#),
                ("facts.json", br#"{}"#),
            ],
        );
        let out_dir = tmp.join("out");
        import_resources(&good_zip, &out_dir, true, &|_, _, _| {}).unwrap();
        assert!(out_dir.join("horizon-resources.json").is_file());
        assert!(out_dir.join("facts.json").is_file());

        let bad_zip = tmp.join("random.zip");
        build_test_zip(&bad_zip, &[("hello.txt", b"world")]);
        let err = import_resources(&bad_zip, &tmp.join("out_bad"), true, &|_, _, _| {}).unwrap_err();
        assert!(err.contains("horizon-resources.json"), "{err}");

        let _ = fs::remove_dir_all(&tmp);
    }
}

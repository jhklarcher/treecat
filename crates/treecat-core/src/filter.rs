use std::collections::HashSet;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::config::Config;
use crate::fswalk::FileInfo;

pub fn filter_files(files: &[FileInfo], cfg: &Config) -> Result<Vec<FileInfo>, String> {
    let include_set = to_lower_set(&cfg.include_exts);
    let exclude_set = to_lower_set(&cfg.exclude_exts);

    let include_glob = build_globset(&cfg.include_globs)?;
    let exclude_glob = build_globset(&cfg.exclude_globs)?;

    let require_include = !include_set.is_empty() || include_glob.is_some();

    let mut out = Vec::new();
    for f in files {
        let base = Path::new(&f.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&f.path);
        let ext = Path::new(base)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if require_include && !matches_include(base, &ext, &include_set, include_glob.as_ref()) {
            continue;
        }
        if matches_exclude(base, &ext, &exclude_set, exclude_glob.as_ref()) {
            continue;
        }
        if let Some(max) = cfg.max_size_bytes {
            if f.size > max {
                continue;
            }
        }
        out.push(f.clone());
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    if let Some(max) = cfg.max_files {
        if out.len() > max {
            out.truncate(max);
        }
    }

    Ok(out)
}

fn matches_include(
    base: &str,
    ext: &str,
    include_set: &HashSet<String>,
    include_glob: Option<&GlobSet>,
) -> bool {
    if include_set.contains(ext) {
        return true;
    }
    if let Some(globs) = include_glob {
        if globs.is_match(base) {
            return true;
        }
    }
    false
}

fn matches_exclude(
    base: &str,
    ext: &str,
    exclude_set: &HashSet<String>,
    exclude_glob: Option<&GlobSet>,
) -> bool {
    if exclude_set.contains(ext) {
        return true;
    }
    if let Some(globs) = exclude_glob {
        if globs.is_match(base) {
            return true;
        }
    }
    false
}

fn build_globset(patterns: &[String]) -> Result<Option<GlobSet>, String> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        let glob = Glob::new(p).map_err(|e| format!("invalid glob {p:?}: {e}"))?;
        builder.add(glob);
    }
    builder
        .build()
        .map(Some)
        .map_err(|e| format!("failed to build glob set: {e}"))
}

fn to_lower_set(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fi(path: &str, size: u64) -> FileInfo {
        FileInfo {
            path: path.into(),
            size,
            is_symlink: false,
        }
    }

    #[test]
    fn include_and_exclude() {
        let files = vec![fi("a.rs", 1), fi("b.go", 1), fi("README.md", 1)];
        let cfg = Config {
            include_exts: vec!["rs".into(), "md".into()],
            exclude_globs: vec!["README.*".into()],
            ..Default::default()
        };
        let out = filter_files(&files, &cfg).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, "a.rs");
    }

    #[test]
    fn max_size_and_max_files() {
        let files = vec![fi("a.rs", 5), fi("b.rs", 50), fi("c.rs", 5)];
        let cfg = Config {
            max_size_bytes: Some(20),
            max_files: Some(1),
            ..Default::default()
        };
        let out = filter_files(&files, &cfg).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, "a.rs");
    }
}

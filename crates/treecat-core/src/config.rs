use std::collections::HashSet;
use std::path::{Component, Path};

/// Color handling for output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

/// Configuration for treecat CLI.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Root path to scan; defaults to current directory when empty.
    pub root_path: String,
    /// Explicit file paths to include in content rendering.
    pub explicit_files: Vec<String>,
    /// If true, only render the tree.
    pub tree_only: bool,
    /// If true, only render file contents.
    pub files_only: bool,
    /// Include globs (basenames).
    pub include_globs: Vec<String>,
    /// Include extensions (without dot).
    pub include_exts: Vec<String>,
    /// Exclude globs (basenames).
    pub exclude_globs: Vec<String>,
    /// Exclude extensions (without dot).
    pub exclude_exts: Vec<String>,
    /// Excluded directory basenames or exact root-relative directory paths.
    pub exclude_dirs: Vec<String>,
    /// Max file size (bytes) for content section.
    pub max_size_bytes: Option<u64>,
    /// Max number of files in content section.
    pub max_files: Option<usize>,
    /// Max traversal depth.
    pub max_depth: Option<usize>,
    /// Follow directory symlinks.
    pub follow_symlinks: bool,
    /// Paths shown relative to root.
    pub relative_paths: bool,
    /// Paths shown absolute.
    pub absolute_paths: bool,
    /// Show version and exit.
    pub show_version: bool,
    /// Color handling.
    pub color_mode: ColorMode,
    /// Copy rendered output to clipboard.
    pub copy_to_clipboard: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExcludeDirRules {
    basename_rules: HashSet<String>,
    exact_path_rules: HashSet<String>,
}

impl ExcludeDirRules {
    pub fn extend_basenames<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.basename_rules
            .extend(values.into_iter().map(Into::into));
    }

    pub fn matches_dir(&self, rel_path: &Path) -> bool {
        if let Some(name) = rel_path.file_name().and_then(|s| s.to_str()) {
            if self.basename_rules.contains(name) {
                return true;
            }
        }

        if let Some(normalized) = normalize_relative_dir_path(rel_path) {
            return self.exact_path_rules.contains(&normalized);
        }

        false
    }
}

pub fn build_exclude_dir_rules(values: &[String]) -> Result<ExcludeDirRules, String> {
    let mut rules = ExcludeDirRules::default();
    for value in values {
        match parse_exclude_dir_rule(value)? {
            ExcludeDirRule::Basename(name) => {
                rules.basename_rules.insert(name);
            }
            ExcludeDirRule::ExactPath(path) => {
                rules.exact_path_rules.insert(path);
            }
        }
    }
    Ok(rules)
}

enum ExcludeDirRule {
    Basename(String),
    ExactPath(String),
}

fn parse_exclude_dir_rule(raw: &str) -> Result<ExcludeDirRule, String> {
    if raw.is_empty() {
        return Err("invalid exclude-dir value \"\": value cannot be empty".into());
    }

    if raw.contains('/') || raw.contains('\\') {
        return normalize_exact_dir_path(raw).map(ExcludeDirRule::ExactPath);
    }

    if raw == "." {
        return Err(format!(
            "invalid exclude-dir value {:?}: '.' is not allowed",
            raw
        ));
    }
    if raw == ".." {
        return Err(format!(
            "invalid exclude-dir value {:?}: '..' is not allowed",
            raw
        ));
    }

    Ok(ExcludeDirRule::Basename(raw.to_string()))
}

fn normalize_exact_dir_path(raw: &str) -> Result<String, String> {
    let raw_path = Path::new(raw);
    if raw_path.is_absolute() || raw.starts_with('/') || raw.starts_with('\\') {
        return Err(format!(
            "invalid exclude-dir value {:?}: absolute paths are not allowed",
            raw
        ));
    }

    let normalized = raw.replace('\\', "/");
    let mut parts = Vec::new();
    for part in normalized.split('/') {
        if part.is_empty() {
            continue;
        }
        match part {
            "." => {
                return Err(format!(
                    "invalid exclude-dir value {:?}: '.' segments are not allowed",
                    raw
                ));
            }
            ".." => {
                return Err(format!(
                    "invalid exclude-dir value {:?}: '..' segments are not allowed",
                    raw
                ));
            }
            _ => parts.push(part),
        }
    }

    if parts.is_empty() {
        return Err(format!(
            "invalid exclude-dir value {:?}: path cannot resolve to the root directory",
            raw
        ));
    }

    Ok(parts.join("/"))
}

fn normalize_relative_dir_path(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            _ => return None,
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn builds_basename_and_exact_path_rules() {
        let rules =
            build_exclude_dir_rules(&["target".into(), "dir/sub".into(), "windows\\path".into()])
                .unwrap();

        assert!(rules.matches_dir(&PathBuf::from("target")));
        assert!(rules.matches_dir(&PathBuf::from("dir/sub")));
        assert!(rules.matches_dir(&PathBuf::from("windows/path")));
        assert!(!rules.matches_dir(&PathBuf::from("other/sub")));
    }

    #[test]
    fn rejects_invalid_exclude_dir_values() {
        let cases = [
            ("", "value cannot be empty"),
            (".", "'.' is not allowed"),
            ("..", "'..' is not allowed"),
            ("/tmp", "absolute paths are not allowed"),
            ("dir/./sub", "'.' segments are not allowed"),
            ("dir/../sub", "'..' segments are not allowed"),
        ];

        for (value, expected) in cases {
            let err = build_exclude_dir_rules(&[value.into()]).unwrap_err();
            assert!(
                err.contains(expected),
                "unexpected error for {value:?}: {err}"
            );
        }
    }
}

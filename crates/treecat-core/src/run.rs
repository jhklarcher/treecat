use std::collections::HashSet;
use std::fs;
use std::io::{ErrorKind, IsTerminal, Write};
use std::path::PathBuf;

use crate::clipboard;
use crate::config::{ColorMode, Config};
use crate::{filter, fswalk, render};
use thiserror::Error;

#[derive(Debug, Clone)]
struct RunData {
    tree: fswalk::TreeNode,
    content_files: Vec<fswalk::FileInfo>,
    tree_allowed_files: Option<HashSet<String>>,
    root_abs: PathBuf,
}

/// Entry point for running treecat.
pub fn run(cfg: &Config) -> Result<(), TreecatError> {
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_with_io_and_clipboard(cfg, &mut stdout, &mut stderr, clipboard::copy_text)
}

fn run_with_io_and_clipboard<W, E, C>(
    cfg: &Config,
    out: &mut W,
    err: &mut E,
    copy_to_clipboard: C,
) -> Result<(), TreecatError>
where
    W: Write,
    E: Write,
    C: Fn(&str) -> Result<(), String>,
{
    let data = collect_run_data(cfg)?;
    let terminal_palette = render::Palette::new(color_enabled(cfg.color_mode));
    let terminal_output = render_output(&data, cfg, &terminal_palette)?;
    out.write_all(terminal_output.as_bytes())
        .map_err(|e| TreecatError::Message(format!("write output: {e}")))?;

    if cfg.copy_to_clipboard {
        match render_output(&data, cfg, &render::Palette::new(false)) {
            Ok(clean_output) => {
                if let Err(copy_err) = copy_to_clipboard(&clean_output) {
                    let _ = writeln!(
                        err,
                        "warning: failed to copy output to clipboard: {copy_err}"
                    );
                }
            }
            Err(render_err) => {
                let _ = writeln!(
                    err,
                    "warning: failed to prepare clipboard output: {render_err}"
                );
            }
        }
    }

    Ok(())
}

fn collect_run_data(cfg: &Config) -> Result<RunData, TreecatError> {
    let (tree, walked_files) = fswalk::walk(cfg).map_err(TreecatError::Message)?;

    let content_candidates = if cfg.explicit_files.is_empty() {
        walked_files.clone()
    } else {
        collect_explicit_files(cfg)?
    };

    let mut content_cfg = cfg.clone();
    if !content_cfg.explicit_files.is_empty() {
        // Explicit files bypass include patterns per spec.
        content_cfg.include_globs.clear();
        content_cfg.include_exts.clear();
    }
    let content_files =
        filter::filter_files(&content_candidates, &content_cfg).map_err(TreecatError::Message)?;

    let tree_allowed_files = if has_tree_file_filters(cfg) {
        let mut tree_cfg = cfg.clone();
        tree_cfg.max_files = None;
        tree_cfg.max_size_bytes = None;
        let tree_files =
            filter::filter_files(&walked_files, &tree_cfg).map_err(TreecatError::Message)?;
        Some(tree_files.into_iter().map(|f| f.path).collect())
    } else {
        None
    };

    let root_abs = PathBuf::from(if cfg.root_path.is_empty() {
        "."
    } else {
        &cfg.root_path
    })
    .canonicalize()
    .map_err(|e| TreecatError::Message(format!("resolve root: {e}")))?;

    let _root_label = root_abs
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(".")
        .to_string();

    Ok(RunData {
        tree,
        content_files,
        tree_allowed_files,
        root_abs,
    })
}

fn render_output(
    data: &RunData,
    cfg: &Config,
    palette: &render::Palette,
) -> Result<String, TreecatError> {
    let mut out = Vec::new();
    if !cfg.files_only {
        render::render_tree(
            &data.tree,
            &data.root_abs,
            cfg,
            palette,
            data.tree_allowed_files.as_ref(),
            &mut out,
        )
        .map_err(|e| TreecatError::Message(e.to_string()))?;
    }
    if !cfg.tree_only {
        if !cfg.files_only && !data.content_files.is_empty() {
            writeln!(&mut out).map_err(|e| TreecatError::Message(e.to_string()))?;
        }
        render::render_contents(&data.content_files, cfg, &data.root_abs, palette, &mut out)
            .map_err(|e| TreecatError::Message(e.to_string()))?;
    }

    Ok(String::from_utf8_lossy(&out).into_owned())
}

#[derive(Debug, Error)]
pub enum TreecatError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

fn collect_explicit_files(cfg: &Config) -> Result<Vec<fswalk::FileInfo>, TreecatError> {
    let root = PathBuf::from(if cfg.root_path.is_empty() {
        "."
    } else {
        &cfg.root_path
    });
    let root_abs = root
        .canonicalize()
        .map_err(|e| TreecatError::Message(format!("resolve root: {e}")))?;
    let mut result = Vec::new();

    for p in &cfg.explicit_files {
        let abs = if PathBuf::from(p).is_absolute() {
            PathBuf::from(p)
        } else {
            root_abs.join(p)
        };
        let meta = fs::metadata(&abs).map_err(|e| {
            let resolved = abs.display().to_string();
            match e.kind() {
                ErrorKind::NotFound => TreecatError::Message(format!(
                    "explicit file not found: {} (looked for {})",
                    p, resolved
                )),
                _ => TreecatError::Message(format!(
                    "failed to read explicit file {} ({}): {}",
                    p, resolved, e
                )),
            }
        })?;
        if meta.is_dir() {
            return Err(TreecatError::Message(format!(
                "explicit path is a directory (expected file): {} ({})",
                p,
                abs.display()
            )));
        }
        let rel = abs
            .strip_prefix(&root_abs)
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|_| abs.to_string_lossy().to_string());

        result.push(fswalk::FileInfo {
            path: rel,
            size: meta.len(),
            is_symlink: false, // metadata resolves symlinks; ok for now
        });
    }

    Ok(result)
}

fn color_enabled(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            if std::env::var_os("NO_COLOR").is_some() {
                return false;
            }
            std::io::stdout().is_terminal()
        }
    }
}

fn has_tree_file_filters(cfg: &Config) -> bool {
    !cfg.include_globs.is_empty()
        || !cfg.include_exts.is_empty()
        || !cfg.exclude_globs.is_empty()
        || !cfg.exclude_exts.is_empty()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use tempfile::tempdir;

    use super::*;

    fn basic_cfg(root: &std::path::Path) -> Config {
        Config {
            root_path: root.to_string_lossy().to_string(),
            files_only: true,
            color_mode: ColorMode::Always,
            copy_to_clipboard: true,
            ..Default::default()
        }
    }

    #[test]
    fn clipboard_failure_warns_but_output_succeeds() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("root.txt"), b"root file\n").unwrap();
        let cfg = basic_cfg(dir.path());

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let result = run_with_io_and_clipboard(&cfg, &mut stdout, &mut stderr, |_text| {
            Err("clipboard unavailable".into())
        });

        assert!(result.is_ok());
        let out = String::from_utf8_lossy(&stdout);
        assert!(out.contains("# "));
        assert!(out.contains("root.txt"));

        let err = String::from_utf8_lossy(&stderr);
        assert!(err.contains("warning: failed to copy output to clipboard"));
    }

    #[test]
    fn clipboard_text_has_no_ansi_sequences() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("root.txt"), b"root file\n").unwrap();
        let cfg = basic_cfg(dir.path());

        let copied = Rc::new(RefCell::new(String::new()));
        let copied_ref = Rc::clone(&copied);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        run_with_io_and_clipboard(&cfg, &mut stdout, &mut stderr, move |text| {
            *copied_ref.borrow_mut() = text.to_string();
            Ok(())
        })
        .unwrap();

        let terminal_output = String::from_utf8_lossy(&stdout);
        assert!(terminal_output.contains("\u{1b}["));

        let copied_output = copied.borrow().clone();
        assert!(copied_output.contains("# root.txt"));
        assert!(copied_output.contains("root file"));
        assert!(!copied_output.contains("\u{1b}["));

        let err = String::from_utf8_lossy(&stderr);
        assert!(err.is_empty());
    }
}

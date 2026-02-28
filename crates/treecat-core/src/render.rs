use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anstyle::{AnsiColor, Style};

use crate::classify::{classify_path, FileKind};
use crate::config::Config;
use crate::fswalk::{FileInfo, TreeNode};
use crate::lang::language_for_path;

/// Simple color palette wrapper.
#[derive(Clone, Copy)]
pub struct Palette {
    enabled: bool,
    dir: Style,
    file: Style,
    header: Style,
    stub: Style,
}

impl Palette {
    pub fn new(enabled: bool) -> Self {
        Palette {
            enabled,
            dir: Style::new().fg_color(Some(AnsiColor::Cyan.into())).bold(),
            file: Style::new(),
            header: Style::new().fg_color(Some(AnsiColor::Cyan.into())).bold(),
            stub: Style::new().fg_color(Some(AnsiColor::Yellow.into())),
        }
    }

    fn apply(&self, style: Style, text: &str) -> String {
        if !self.enabled {
            return text.to_string();
        }
        format!("{style}{text}{reset}", reset = anstyle::Reset)
    }

    pub fn dir<'a>(&self, text: impl Into<std::borrow::Cow<'a, str>>) -> String {
        self.apply(self.dir, text.into().as_ref())
    }

    pub fn file<'a>(&self, text: impl Into<std::borrow::Cow<'a, str>>) -> String {
        self.apply(self.file, text.into().as_ref())
    }

    pub fn header<'a>(&self, text: impl Into<std::borrow::Cow<'a, str>>) -> String {
        self.apply(self.header, text.into().as_ref())
    }

    pub fn stub<'a>(&self, text: impl Into<std::borrow::Cow<'a, str>>) -> String {
        self.apply(self.stub, text.into().as_ref())
    }
}

pub fn render_tree(
    root: &TreeNode,
    root_abs: &Path,
    cfg: &Config,
    palette: &Palette,
    allowed: Option<&std::collections::HashSet<String>>,
    w: &mut dyn Write,
) -> io::Result<()> {
    let root_label = if cfg.absolute_paths {
        palette.dir(root_abs.display().to_string())
    } else {
        palette.dir(&root.name)
    };
    writeln!(w, "{}/", root_label)?;
    let children = if let Some(allowed) = allowed {
        filter_tree_children(&root.children, allowed)
    } else {
        root.children.clone()
    };
    render_children(
        &children,
        allowed,
        w,
        "",
        root_abs,
        cfg.absolute_paths,
        palette,
    )?;
    Ok(())
}

fn render_children(
    children: &[TreeNode],
    allowed: Option<&std::collections::HashSet<String>>,
    w: &mut dyn Write,
    prefix: &str,
    root_abs: &Path,
    absolute: bool,
    palette: &Palette,
) -> io::Result<()> {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let next_prefix = if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };
        let name_styled = if absolute {
            let joined = root_abs.join(&child.path);
            if child.is_dir {
                palette.dir(format!("{}/", joined.display()))
            } else {
                palette.file(joined.display().to_string())
            }
        } else if child.is_dir {
            palette.dir(format!("{}/", child.name))
        } else {
            palette.file(&child.name)
        };
        if child.is_dir {
            writeln!(w, "{prefix}{connector}{name_styled}")?;
            let next_children = if let Some(allowed) = allowed {
                filter_tree_children(&child.children, allowed)
            } else {
                child.children.clone()
            };
            render_children(
                &next_children,
                allowed,
                w,
                &next_prefix,
                root_abs,
                absolute,
                palette,
            )?;
        } else {
            writeln!(w, "{prefix}{connector}{name_styled}")?;
        }
    }
    Ok(())
}

fn filter_tree_children(
    children: &[TreeNode],
    allowed: &std::collections::HashSet<String>,
) -> Vec<TreeNode> {
    children
        .iter()
        .filter_map(|c| {
            if c.is_dir {
                let filtered = filter_tree_children(&c.children, allowed);
                if filtered.is_empty() {
                    None
                } else {
                    Some(TreeNode {
                        name: c.name.clone(),
                        path: c.path.clone(),
                        is_dir: true,
                        children: filtered,
                    })
                }
            } else if allowed.contains(&c.path) {
                Some(c.clone())
            } else {
                None
            }
        })
        .collect()
}

pub fn render_contents(
    files: &[FileInfo],
    cfg: &Config,
    root_abs: &Path,
    palette: &Palette,
    w: &mut dyn Write,
) -> io::Result<()> {
    for (idx, f) in files.iter().enumerate() {
        if idx > 0 {
            writeln!(w)?;
        }
        let abs_path = if PathBuf::from(&f.path).is_absolute() {
            PathBuf::from(&f.path)
        } else {
            root_abs.join(&f.path)
        };
        let display_path = if cfg.absolute_paths {
            abs_path.display().to_string()
        } else {
            f.path.clone()
        };
        writeln!(w, "# {}", palette.header(display_path))?;

        match classify_path(abs_path.to_string_lossy().as_ref()) {
            FileKind::Binary => {
                writeln!(
                    w,
                    "{}",
                    palette.stub(format!(
                        "(skipped: binary/non-text file, size: {})",
                        format_size(f.size)
                    ))
                )?;
            }
            FileKind::Text => {
                let lang = language_for_path(&f.path);
                writeln!(w, "```{}", lang)?;
                print_file_contents(&abs_path, w)?;
                writeln!(w, "```")?;
            }
        }
    }
    Ok(())
}

fn print_file_contents(path: &Path, w: &mut dyn Write) -> io::Result<()> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    w.write_all(&buf)?;
    Ok(())
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit_idx = 0usize;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    format!("{value:.1} {}", UNITS[unit_idx])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn renders_tree_with_pruning() {
        let child = TreeNode {
            name: "file.txt".into(),
            path: "file.txt".into(),
            is_dir: false,
            children: vec![],
        };
        let root = TreeNode {
            name: "root".into(),
            path: ".".into(),
            is_dir: true,
            children: vec![child],
        };
        let mut buf = Vec::new();
        let allowed: std::collections::HashSet<String> = ["file.txt".into()].into_iter().collect();
        render_tree(
            &root,
            &PathBuf::from("."),
            &Config::default(),
            &Palette::new(false),
            Some(&allowed),
            &mut buf,
        )
        .unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("root/"));
        assert!(out.contains("file.txt"));
    }

    #[test]
    fn renders_contents_text_and_binary() {
        let dir = tempdir().unwrap();
        let text_path = dir.path().join("main.rs");
        fs::write(&text_path, b"fn main() {}\n").unwrap();
        let bin_path = dir.path().join("bin.dat");
        fs::write(&bin_path, [0u8, 1, 2]).unwrap();

        let files = vec![
            FileInfo {
                path: "main.rs".into(),
                size: 12,
                is_symlink: false,
            },
            FileInfo {
                path: "bin.dat".into(),
                size: 3,
                is_symlink: false,
            },
        ];
        let mut buf = Vec::new();
        let cfg = Config {
            root_path: dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        render_contents(
            &files,
            &cfg,
            &dir.path().to_path_buf(),
            &Palette::new(false),
            &mut buf,
        )
        .unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("```rust"));
        assert!(out.contains("fn main()"));
        assert!(out.contains("(skipped: binary/non-text file"));
        assert!(out.contains("size: 3 B"));
    }

    #[test]
    fn formats_binary_sizes_human_readable() {
        assert_eq!(format_size(7), "7 B");
        assert_eq!(format_size(24_678), "24.1 KB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
    }
}

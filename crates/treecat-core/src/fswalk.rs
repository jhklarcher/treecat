use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::config::{build_exclude_dir_rules, Config, ExcludeDirRules};

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: String, // relative to root
    pub is_dir: bool,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String, // relative to root
    pub size: u64,
    pub is_symlink: bool,
}

pub fn walk(cfg: &Config) -> Result<(TreeNode, Vec<FileInfo>), String> {
    let root = PathBuf::from(if cfg.root_path.is_empty() {
        "."
    } else {
        &cfg.root_path
    });
    let root_abs = root
        .canonicalize()
        .map_err(|e| format!("failed to resolve root {}: {e}", root.display()))?;
    if !root_abs.is_dir() {
        return Err(format!("root is not a directory: {}", root.display()));
    }

    let mut exclude_dirs =
        build_exclude_dir_rules(&cfg.exclude_dirs).map_err(|e| format!("invalid config: {e}"))?;
    exclude_dirs.extend_basenames(default_excludes());

    let mut files = Vec::new();

    // Build tree nodes keyed by relative path; children are populated after walk.
    let mut nodes = std::collections::BTreeMap::<String, TreeNode>::new();
    nodes.insert(
        ".".into(),
        TreeNode {
            name: root_abs
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| root_abs.display().to_string()),
            path: ".".into(),
            is_dir: true,
            children: Vec::new(),
        },
    );

    let mut visited_symlinks = HashSet::new();
    for entry in WalkDir::new(&root_abs)
        .follow_links(cfg.follow_symlinks)
        .into_iter()
        .filter_entry(|e| {
            should_descend(
                e,
                &root_abs,
                &exclude_dirs,
                cfg.max_depth,
                cfg.follow_symlinks,
                &mut visited_symlinks,
            )
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => return Err(format!("walk error: {err}")),
        };

        let rel_path = match entry.path().strip_prefix(&root_abs) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => continue,
        };
        let is_root = rel_path.is_empty();
        let rel_path = if is_root { ".".into() } else { rel_path };
        if is_root {
            continue;
        }

        let depth = entry.depth();
        if let Some(max) = cfg.max_depth {
            if depth > max {
                continue;
            }
        }

        let is_dir = entry.file_type().is_dir();
        let name = entry
            .file_name()
            .to_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| rel_path.clone());

        let is_symlink = entry.file_type().is_symlink();

        let node = TreeNode {
            name: name.clone(),
            path: rel_path.clone(),
            is_dir,
            children: Vec::new(),
        };
        nodes.insert(rel_path.clone(), node);

        if !is_dir {
            let size = fs::metadata(entry.path()).map(|m| m.len()).unwrap_or(0);
            files.push(FileInfo {
                path: rel_path.clone(),
                size,
                is_symlink,
            });
        }
    }

    // Rebuild hierarchy bottom-up so parents receive populated children.
    let mut paths: Vec<String> = nodes.keys().cloned().collect();
    paths.sort_by_key(|p| {
        if p == "." {
            0
        } else {
            Path::new(p).components().count()
        }
    });
    paths.reverse(); // deepest first

    for path in paths {
        if path == "." {
            continue;
        }
        if let Some(node) = nodes.remove(&path) {
            if let Some(parent_path) = parent_rel_path(&path) {
                if let Some(parent) = nodes.get_mut(&parent_path) {
                    parent.children.push(node.clone());
                }
            }
            // Put node back to preserve for other parents? not needed; single parent.
            nodes.insert(path, node);
        }
    }

    let mut tree_root = nodes.remove(".").unwrap_or(TreeNode {
        name: root_abs
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| root_abs.display().to_string()),
        path: ".".into(),
        is_dir: true,
        children: Vec::new(),
    });
    sort_node(&mut tree_root);

    Ok((tree_root, files))
}

// Sort children lexicographically for determinism
fn sort_node(node: &mut TreeNode) {
    node.children.sort_by(|a, b| a.name.cmp(&b.name));
    for child in node.children.iter_mut() {
        sort_node(child);
    }
}

fn parent_rel_path(rel: &str) -> Option<String> {
    let p = Path::new(rel);
    p.parent().map(|parent| {
        let s = parent.to_string_lossy().to_string();
        if s.is_empty() {
            ".".into()
        } else {
            s
        }
    })
}

fn should_descend(
    entry: &walkdir::DirEntry,
    root_abs: &Path,
    exclude_dirs: &ExcludeDirRules,
    max_depth: Option<usize>,
    follow_symlinks: bool,
    visited_symlinks: &mut HashSet<PathBuf>,
) -> bool {
    let depth = entry.depth();
    if let Some(max) = max_depth {
        if depth > max {
            return false;
        }
    }
    if depth == 0 {
        return true;
    }
    let is_dir = entry.file_type().is_dir();
    let is_symlink_dir = if follow_symlinks && entry.path_is_symlink() {
        entry.metadata().map(|m| m.is_dir()).unwrap_or(false)
    } else {
        false
    };

    if is_dir || is_symlink_dir {
        if let Ok(rel_path) = entry.path().strip_prefix(root_abs) {
            if exclude_dirs.matches_dir(rel_path) {
                return false;
            }
        }
    }
    if is_symlink_dir {
        if let Ok(real) = entry.path().canonicalize() {
            if !visited_symlinks.insert(real) {
                return false;
            }
        }
    }
    true
}

fn default_excludes() -> HashSet<String> {
    [
        ".git",
        ".hg",
        ".svn",
        ".idea",
        ".vscode",
        "node_modules",
        "__pycache__",
        ".venv",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn respects_max_depth_and_exclude_dir() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("root.txt"), b"hi").unwrap();
        fs::create_dir(dir.path().join("keep")).unwrap();
        fs::write(dir.path().join("keep/inner.txt"), b"inner").unwrap();
        fs::create_dir(dir.path().join("skipme")).unwrap();
        fs::write(dir.path().join("skipme/skip.txt"), b"skip").unwrap();

        let max_depth = Some(1);
        let cfg = Config {
            root_path: dir.path().to_string_lossy().to_string(),
            max_depth,
            exclude_dirs: vec!["skipme".into()],
            ..Default::default()
        };

        let (tree, files) = walk(&cfg).unwrap();
        // Files should include only root-level files at depth <= 1
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "root.txt");

        // Tree should include keep dir (without its child), exclude skipme entirely.
        assert_eq!(tree.children.len(), 2); // keep + root.txt
        let names: Vec<_> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"keep"));
        assert!(names.contains(&"root.txt"));
        for child in &tree.children {
            if child.name == "keep" {
                assert!(child.children.is_empty());
            }
            assert_ne!(child.name, "skipme");
        }
    }

    #[test]
    fn basename_exclude_dir_matches_all_directories_with_that_name() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a/skipme")).unwrap();
        fs::create_dir_all(dir.path().join("b/skipme")).unwrap();
        fs::write(dir.path().join("a/skipme/file.txt"), b"a").unwrap();
        fs::write(dir.path().join("b/skipme/file.txt"), b"b").unwrap();
        fs::write(dir.path().join("keep.txt"), b"keep").unwrap();

        let cfg = Config {
            root_path: dir.path().to_string_lossy().to_string(),
            exclude_dirs: vec!["skipme".into()],
            ..Default::default()
        };

        let (tree, files) = walk(&cfg).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "keep.txt");

        let rendered_paths: Vec<_> = tree
            .children
            .iter()
            .map(|child| child.path.as_str())
            .collect();
        assert!(rendered_paths.contains(&"a"));
        assert!(rendered_paths.contains(&"b"));
        assert!(rendered_paths.contains(&"keep.txt"));
        for child in &tree.children {
            if child.path == "a" || child.path == "b" {
                assert!(child.children.is_empty());
            }
        }
    }

    #[test]
    fn exact_path_exclude_dir_only_matches_the_requested_subtree() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a/skipme")).unwrap();
        fs::create_dir_all(dir.path().join("b/skipme")).unwrap();
        fs::write(dir.path().join("a/skipme/file.txt"), b"a").unwrap();
        fs::write(dir.path().join("b/skipme/file.txt"), b"b").unwrap();

        let cfg = Config {
            root_path: dir.path().to_string_lossy().to_string(),
            exclude_dirs: vec!["a/skipme".into()],
            ..Default::default()
        };

        let (tree, files) = walk(&cfg).unwrap();
        let file_paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(file_paths, vec!["b/skipme/file.txt"]);

        let a = tree
            .children
            .iter()
            .find(|child| child.path == "a")
            .unwrap();
        let b = tree
            .children
            .iter()
            .find(|child| child.path == "b")
            .unwrap();
        assert!(a.children.is_empty());
        assert_eq!(b.children.len(), 1);
        assert_eq!(b.children[0].path, "b/skipme");
    }
}

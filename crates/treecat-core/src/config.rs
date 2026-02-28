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
    /// Excluded directory basenames.
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

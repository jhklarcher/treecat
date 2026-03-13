use std::fs;
use std::path::{Path, PathBuf};

use clap::{
    parser::ValueSource, ArgAction, ArgMatches, CommandFactory, FromArgMatches, Parser, ValueEnum,
};
use treecat_core::config::{build_exclude_dir_rules, ColorMode, Config};
use treecat_core::run::run;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const LONG_VERSION: &str = env!("CARGO_PKG_VERSION");
const LONG_ABOUT: &str = "\
Render a directory tree plus selected file contents. Paths are relative to the \
root by default. Filters include/exclude globs or extensions, size limits, depth, \
and optional symlink following. Explicit FILES are added to the content section \
without needing include filters.";
const AFTER_HELP: &str = "\
Examples:
  treecat
  treecat src --max-depth 1
  treecat . README.md src/main.rs --files-only
  treecat . -x rs,md -d target -d src/generated --max-size 200K --max-files 50
  treecat . --color always
  treecat . --copy
  treecat --config ~/.config/treecat/config.toml
  treecat --no-config
";

#[derive(Debug, Parser)]
#[command(
    name = "treecat",
    version = VERSION,
    long_version = LONG_VERSION,
    about = "Directory tree + file contents as clean text",
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP
)]
struct Cli {
    /// Directory root to scan (default: current directory or config value).
    #[arg(value_name = "ROOT", default_value = ".")]
    root: String,

    /// Explicit file paths to include in the content section.
    #[arg(value_name = "FILES")]
    files: Vec<String>,

    /// Load defaults from this config file path.
    #[arg(long = "config", value_name = "PATH")]
    config: Option<String>,

    /// Disable config-file loading.
    #[arg(long = "no-config", action = ArgAction::SetTrue, conflicts_with = "config")]
    no_config: bool,

    /// Only print the directory tree.
    #[arg(short = 't', long = "tree-only")]
    tree_only: bool,

    /// Only print the file content section.
    #[arg(short = 'F', long = "files-only")]
    files_only: bool,

    /// Include files whose basename matches the glob (repeatable).
    #[arg(short = 'i', long = "include-glob", action = ArgAction::Append)]
    include_glob: Vec<String>,

    /// Include files by extension (comma-separated, repeatable, case-insensitive).
    #[arg(short = 'x', long = "include-ext", action = ArgAction::Append)]
    include_ext: Vec<String>,

    /// Exclude files whose basename matches the glob (repeatable).
    #[arg(short = 'e', long = "exclude-glob", action = ArgAction::Append)]
    exclude_glob: Vec<String>,

    /// Exclude files by extension (comma-separated, repeatable, case-insensitive).
    #[arg(short = 'X', long = "exclude-ext", action = ArgAction::Append)]
    exclude_ext: Vec<String>,

    /// Exclude directories by basename or exact root-relative path (repeatable). Defaults include .git, .hg, .svn, .idea, .vscode, node_modules, __pycache__, .venv.
    #[arg(
        short = 'd',
        long = "exclude-dir",
        value_name = "NAME|PATH",
        action = ArgAction::Append
    )]
    exclude_dir: Vec<String>,

    /// Skip files larger than this size (supports B/K/M/G/T suffixes).
    #[arg(long = "max-size", value_parser = parse_size_arg)]
    max_size: Option<u64>,

    /// Limit number of files in content section.
    #[arg(long = "max-files")]
    max_files: Option<usize>,

    /// Limit traversal depth.
    #[arg(long = "max-depth")]
    max_depth: Option<usize>,

    /// Follow directory symlinks (default: no).
    #[arg(long = "follow-symlinks", action = ArgAction::SetTrue)]
    follow_symlinks: bool,

    /// Show paths relative to root (default).
    #[arg(long = "relative", action = ArgAction::SetTrue)]
    relative: bool,

    /// Show absolute paths.
    #[arg(long = "absolute", action = ArgAction::SetTrue)]
    absolute: bool,

    /// Color output: auto (default), always, or never.
    #[arg(long = "color", value_enum, default_value_t = CliColorMode::Auto)]
    color: CliColorMode,

    /// Copy rendered output to the system clipboard.
    #[arg(long = "copy", action = ArgAction::SetTrue)]
    copy: bool,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum CliColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FileConfig {
    root_path: Option<String>,
    explicit_files: Option<Vec<String>>,
    tree_only: Option<bool>,
    files_only: Option<bool>,
    include_globs: Option<Vec<String>>,
    include_exts: Option<Vec<String>>,
    exclude_globs: Option<Vec<String>>,
    exclude_exts: Option<Vec<String>>,
    exclude_dirs: Option<Vec<String>>,
    max_size_bytes: Option<u64>,
    max_files: Option<usize>,
    max_depth: Option<usize>,
    follow_symlinks: Option<bool>,
    relative_paths: Option<bool>,
    absolute_paths: Option<bool>,
    color_mode: Option<FileColorMode>,
    copy_to_clipboard: Option<bool>,
}

#[derive(Debug, Copy, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum FileColorMode {
    Auto,
    Always,
    Never,
}

impl From<FileColorMode> for ColorMode {
    fn from(value: FileColorMode) -> Self {
        match value {
            FileColorMode::Auto => ColorMode::Auto,
            FileColorMode::Always => ColorMode::Always,
            FileColorMode::Never => ColorMode::Never,
        }
    }
}

fn main() {
    let (cli, matches) = parse_cli();
    let cfg = match build_config(&cli, &matches) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };

    if let Err(err) = run(&cfg) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn parse_cli() -> (Cli, ArgMatches) {
    let matches = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    (cli, matches)
}

fn build_config(cli: &Cli, matches: &ArgMatches) -> Result<Config, String> {
    if cli.tree_only && cli.files_only {
        return Err("cannot use --tree-only and --files-only together".into());
    }
    if cli.relative && cli.absolute {
        return Err("cannot use --relative and --absolute together".into());
    }

    let mut cfg = built_in_defaults();

    let (config_path, required) = resolve_config_path(cli)?;
    if let Some(path) = config_path {
        if path.exists() {
            let file_cfg = load_config_file(&path)?;
            apply_file_config(&mut cfg, file_cfg);
        } else if required {
            return Err(format!("config file not found: {}", path.display()));
        }
    }

    apply_cli_overrides(&mut cfg, cli, matches);
    normalize_and_validate_config(&mut cfg)?;

    Ok(cfg)
}

fn built_in_defaults() -> Config {
    Config {
        root_path: ".".to_string(),
        relative_paths: true,
        color_mode: ColorMode::Auto,
        ..Default::default()
    }
}

fn resolve_config_path(cli: &Cli) -> Result<(Option<PathBuf>, bool), String> {
    if cli.no_config {
        return Ok((None, false));
    }

    if let Some(path) = &cli.config {
        return Ok((Some(PathBuf::from(path)), true));
    }

    if let Some(path) = std::env::var_os("TREECAT_CONFIG") {
        let p = PathBuf::from(path);
        if p.as_os_str().is_empty() {
            return Err("TREECAT_CONFIG is set but empty".into());
        }
        return Ok((Some(p), true));
    }

    Ok((default_config_path(), false))
}

fn default_config_path() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("treecat").join("config.toml"));
        }
    }

    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("treecat")
                .join("config.toml")
        })
}

fn load_config_file(path: &Path) -> Result<FileConfig, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read config file {}: {e}", path.display()))?;
    toml::from_str::<FileConfig>(&raw)
        .map_err(|e| format!("failed to parse config file {}: {e}", path.display()))
}

fn apply_file_config(cfg: &mut Config, file_cfg: FileConfig) {
    if let Some(v) = file_cfg.root_path {
        cfg.root_path = v;
    }
    if let Some(v) = file_cfg.explicit_files {
        cfg.explicit_files = v;
    }
    if let Some(v) = file_cfg.tree_only {
        cfg.tree_only = v;
    }
    if let Some(v) = file_cfg.files_only {
        cfg.files_only = v;
    }
    if let Some(v) = file_cfg.include_globs {
        cfg.include_globs = v;
    }
    if let Some(v) = file_cfg.include_exts {
        cfg.include_exts = split_multi(&v);
    }
    if let Some(v) = file_cfg.exclude_globs {
        cfg.exclude_globs = v;
    }
    if let Some(v) = file_cfg.exclude_exts {
        cfg.exclude_exts = split_multi(&v);
    }
    if let Some(v) = file_cfg.exclude_dirs {
        cfg.exclude_dirs = v;
    }
    if let Some(v) = file_cfg.max_size_bytes {
        cfg.max_size_bytes = Some(v);
    }
    if let Some(v) = file_cfg.max_files {
        cfg.max_files = Some(v);
    }
    if let Some(v) = file_cfg.max_depth {
        cfg.max_depth = Some(v);
    }
    if let Some(v) = file_cfg.follow_symlinks {
        cfg.follow_symlinks = v;
    }
    if let Some(v) = file_cfg.relative_paths {
        cfg.relative_paths = v;
    }
    if let Some(v) = file_cfg.absolute_paths {
        cfg.absolute_paths = v;
    }
    if let Some(v) = file_cfg.color_mode {
        cfg.color_mode = v.into();
    }
    if let Some(v) = file_cfg.copy_to_clipboard {
        cfg.copy_to_clipboard = v;
    }
}

fn apply_cli_overrides(cfg: &mut Config, cli: &Cli, matches: &ArgMatches) {
    if is_command_line(matches, "root") {
        cfg.root_path = cli.root.clone();
    }
    if !cli.files.is_empty() {
        cfg.explicit_files = cli.files.clone();
    }
    if cli.tree_only {
        cfg.tree_only = true;
    }
    if cli.files_only {
        cfg.files_only = true;
    }
    if !cli.include_glob.is_empty() {
        cfg.include_globs = cli.include_glob.clone();
    }
    if !cli.include_ext.is_empty() {
        cfg.include_exts = split_multi(&cli.include_ext);
    }
    if !cli.exclude_glob.is_empty() {
        cfg.exclude_globs = cli.exclude_glob.clone();
    }
    if !cli.exclude_ext.is_empty() {
        cfg.exclude_exts = split_multi(&cli.exclude_ext);
    }
    if !cli.exclude_dir.is_empty() {
        cfg.exclude_dirs = cli.exclude_dir.clone();
    }
    if let Some(v) = cli.max_size {
        cfg.max_size_bytes = Some(v);
    }
    if let Some(v) = cli.max_files {
        cfg.max_files = Some(v);
    }
    if let Some(v) = cli.max_depth {
        cfg.max_depth = Some(v);
    }
    if cli.follow_symlinks {
        cfg.follow_symlinks = true;
    }
    if cli.relative {
        cfg.relative_paths = true;
        cfg.absolute_paths = false;
    }
    if cli.absolute {
        cfg.absolute_paths = true;
        cfg.relative_paths = false;
    }
    if is_command_line(matches, "color") {
        cfg.color_mode = match cli.color {
            CliColorMode::Auto => ColorMode::Auto,
            CliColorMode::Always => ColorMode::Always,
            CliColorMode::Never => ColorMode::Never,
        };
    }
    if cli.copy {
        cfg.copy_to_clipboard = true;
    }
}

fn normalize_and_validate_config(cfg: &mut Config) -> Result<(), String> {
    if cfg.tree_only && cfg.files_only {
        return Err("cannot use --tree-only and --files-only together".into());
    }
    if cfg.relative_paths && cfg.absolute_paths {
        return Err("cannot use both relative_paths and absolute_paths in config".into());
    }
    build_exclude_dir_rules(&cfg.exclude_dirs)?;
    if !cfg.relative_paths && !cfg.absolute_paths {
        cfg.relative_paths = true;
    }
    Ok(())
}

fn is_command_line(matches: &ArgMatches, id: &str) -> bool {
    matches.value_source(id) == Some(ValueSource::CommandLine)
}

fn split_multi(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for v in values {
        for part in v.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn parse_size_arg(raw: &str) -> Result<u64, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("size cannot be empty".into());
    }

    let split_idx = s
        .char_indices()
        .find_map(|(idx, ch)| if ch.is_ascii_digit() { None } else { Some(idx) })
        .unwrap_or(s.len());
    let (number_part, suffix_part) = s.split_at(split_idx);

    if number_part.is_empty() {
        return Err("size must start with digits".into());
    }

    let number: u64 = number_part
        .parse()
        .map_err(|e| format!("invalid size number {number_part:?}: {e}"))?;

    let suffix = suffix_part.trim().to_ascii_lowercase();
    let multiplier: u64 = match suffix.as_str() {
        "" | "b" => 1,
        "k" | "kb" | "ki" | "kib" => 1024,
        "m" | "mb" | "mi" | "mib" => 1024_u64.pow(2),
        "g" | "gb" | "gi" | "gib" => 1024_u64.pow(3),
        "t" | "tb" | "ti" | "tib" => 1024_u64.pow(4),
        _ => {
            return Err(format!(
                "unsupported size suffix {suffix:?}; use B, K, M, G, or T"
            ))
        }
    };

    number
        .checked_mul(multiplier)
        .ok_or_else(|| format!("size {raw:?} is too large"))
}

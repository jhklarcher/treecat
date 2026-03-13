# Configuration

treecat supports loading defaults from a TOML config file.

## Discovery order

Unless disabled with `--no-config`, treecat resolves config as:

1. `--config <PATH>` (required if provided)
2. `TREECAT_CONFIG` env var (required if set)
3. `$XDG_CONFIG_HOME/treecat/config.toml` (if `XDG_CONFIG_HOME` is set)
4. `~/.config/treecat/config.toml`

If a discovered default path does not exist, it is ignored.

## Precedence

1. CLI flags
2. Config file values
3. Built-in defaults

## Supported keys

- `root_path` (`string`)
- `explicit_files` (`array<string>`)
- `tree_only` (`bool`)
- `files_only` (`bool`)
- `include_globs` (`array<string>`)
- `include_exts` (`array<string>`)
- `exclude_globs` (`array<string>`)
- `exclude_exts` (`array<string>`)
- `exclude_dirs` (`array<string>`)
- `max_size_bytes` (`integer`)
- `max_files` (`integer`)
- `max_depth` (`integer`)
- `follow_symlinks` (`bool`)
- `relative_paths` (`bool`)
- `absolute_paths` (`bool`)
- `color_mode` (`"auto" | "always" | "never"`)
- `copy_to_clipboard` (`bool`)

Notes:

- Unknown keys are rejected (`deny_unknown_fields`).
- `max_size_bytes` is raw bytes in config files.
- Human suffix parsing (`200K`, `10M`) is CLI-only via `--max-size`.
- `exclude_dirs` entries without path separators match directory basenames anywhere in the tree.
- `exclude_dirs` entries with path separators match exact root-relative subdirectory paths.
- Path-style `exclude_dirs` entries reject absolute paths, `.`, and `..`.

## Example

```toml
root_path = "/path/to/project"
files_only = true
include_exts = ["rs", "md"]
exclude_dirs = ["target", "src/generated"]
max_size_bytes = 204800
max_files = 100
color_mode = "never"
copy_to_clipboard = false
```

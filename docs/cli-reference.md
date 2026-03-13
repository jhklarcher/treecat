# CLI Reference

## Synopsis

```bash
treecat [OPTIONS] [ROOT] [FILES...]
```

- `ROOT`: directory root to scan. Default is `.` (or a configured `root_path`).
- `FILES...`: explicit files for the content section.

## Output section toggles

- `-t`, `--tree-only`: print only the tree section.
- `-F`, `--files-only`: print only the content section.

## Include filters

If any include filter is set, a file must match at least one include rule.

- `-i`, `--include-glob <PATTERN>`: basename glob (repeatable).
- `-x`, `--include-ext <EXT[,EXT...]>`: extension include list (repeatable, case-insensitive).

## Exclude filters

Exclude filters apply after includes.

- `-e`, `--exclude-glob <PATTERN>`: basename glob (repeatable).
- `-X`, `--exclude-ext <EXT[,EXT...]>`: extension exclude list (repeatable, case-insensitive).
- `-d`, `--exclude-dir <NAME|PATH>`: exclude directories by basename or exact root-relative path (repeatable).
  - Values without path separators match any directory with that basename.
  - Values with path separators match one exact root-relative subdirectory.
  - Path values reject absolute paths, `.`, and `..`.

Default excluded directories:

- `.git`, `.hg`, `.svn`, `.idea`, `.vscode`, `node_modules`, `__pycache__`, `.venv`

## Limits and traversal

- `--max-size <SIZE>`: skip files larger than this in the content section.
  - Accepts bytes or suffixes. Examples: `500`, `500B`, `200K`, `10M`, `1G`, `1KiB`, `8MiB`.
- `--max-files <N>`: cap number of files in content section after filtering/sorting.
- `--max-depth <N>`: traversal depth limit.
- `--follow-symlinks`: follow directory symlinks with cycle protection.

## Paths and color

- `--relative`: show relative paths (default).
- `--absolute`: show absolute paths.
- `--color <auto|always|never>`: terminal color mode.
  - `auto` honors `NO_COLOR` and TTY detection.

## Clipboard

- `--copy`: copy rendered output to system clipboard as plain text.
  - Clipboard copy failures are warnings only; normal stdout output still succeeds.

## Config loading

- `--config <PATH>`: load defaults from a specific config file.
- `--no-config`: disable config file loading for this run.
- `TREECAT_CONFIG`: environment override path to config file.

## Misc

- `-h`, `--help`: show help.
- `-V`, `--version`: show version.

## Common examples

```bash
treecat
treecat src --max-depth 1
treecat . README.md src/main.rs --files-only
treecat . -x rs,md -d target -d src/generated --max-size 200K --max-files 50
treecat . --follow-symlinks --absolute
treecat . --copy
treecat --config ~/.config/treecat/config.toml
treecat --no-config
```

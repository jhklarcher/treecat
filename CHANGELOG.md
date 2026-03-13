# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Changed

- `-d, --exclude-dir` now accepts exact root-relative subdirectory paths in addition to basename matches.
- CLI help, README, and docs now describe mixed basename/path exclusion behavior and the `-d` short flag.

## [1.0.0] - 2026-02-28

### Added

- Rust workspace delivered as:
  - `crates/treecat-cli` (CLI entrypoint)
  - `crates/treecat-core` (walk/filter/classify/render pipeline)
- CLI support for:
  - tree/content toggles
  - include/exclude by glob and extension
  - excluded directories, max file size/count, max depth
  - relative/absolute path output
  - optional symlink following with cycle protection
  - color modes (`auto`, `always`, `never`)
- Text/binary classification (extension heuristics + content sniffing).
- Language-tag mapping for fenced content output.
- Deterministic rendering for tree and content sections.
- Optional clipboard copy via `--copy` using a cross-platform backend.
- Config-file defaults with discovery/override support:
  - `--config <PATH>`
  - `--no-config`
  - `TREECAT_CONFIG`
  - default lookup in XDG/Home config paths
- Dedicated documentation set under `docs/`:
  - `docs/cli-reference.md`
  - `docs/output-contract.md`
  - `docs/config.md`
- Unit and integration test coverage for core behavior.
- CI workflow (`fmt`, `clippy -D warnings`, `test`).

### Changed

- Tree rendering applies file include/exclude filters (while still ignoring `--max-files` and `--max-size` for the tree section).
- `--max-size` accepts human-friendly suffixes (`B`, `K`, `M`, `G`, `T`, including `KiB`-style variants).
- Binary/non-text stub sizes render in human-readable units (for example `24.1 KB`).
- README is quickstart-oriented and points to stable docs.

### Fixed

- `--absolute` behavior works cleanly with default relative-path handling.
- Clipboard failures warn without failing normal stdout rendering.

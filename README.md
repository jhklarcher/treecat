# treecat (Rust CLI)

Directory tree + file contents as clean text. Deterministic ordering, safe around binary files, and designed for readable snapshots.

## Install

```bash
# Install from this repo:
cargo install --path crates/treecat-cli
```

## Usage

```bash
treecat [OPTIONS] [ROOT] [FILES...]
```

Examples:

```bash
treecat
treecat src --max-depth 1
treecat . README.md src/main.rs --files-only
treecat . -x rs,md --exclude-dir target --max-size 200K --max-files 50
treecat . --follow-symlinks --absolute
treecat . --copy
treecat --config ~/.config/treecat/config.toml
treecat --no-config
```

## Documentation

- CLI options and examples: `docs/cli-reference.md`
- Output behavior contract: `docs/output-contract.md`
- Config file format and precedence: `docs/config.md`
- Project history and release notes: `CHANGELOG.md`

## Highlights

- Tree and content sections can be toggled independently.
- Include/exclude filters affect tree and content selection.
- Content supports size/count limits; tree remains unaffected by those limits.
- Binary files are skipped safely with human-readable size stubs.
- Clipboard copy is optional and non-fatal on clipboard backend failure.

## Development

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

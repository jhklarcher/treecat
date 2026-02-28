# Output Contract

This document describes the behavior treecat treats as stable.

## Section order

By default, output is:

1. Tree section
2. Blank line (only if both sections are printed and there are content files)
3. Content section

Flags can disable either section:

- `--tree-only`
- `--files-only`

## Tree section

- Root line:
  - relative mode: `<root-basename>/`
  - absolute mode: `<absolute-root-path>/`
- Connectors:
  - `├──`, `└──`
  - continuation prefixes `│   ` and `    `
- Entries are sorted lexicographically within each directory.

Filtering behavior:

- `--exclude-dir` and `--max-depth` always affect traversal/tree.
- File include/exclude filters (`--include-*`, `--exclude-*`) affect tree entries.
- `--max-size` and `--max-files` do not affect the tree.
- When file filters are active, directories without matching descendants are omitted.

## Content section

Each selected file is rendered as:

```text
# <display-path>
```<language>
<file bytes>
```
```

- Blank line between file entries.
- File list is sorted lexicographically by relative path.
- Display path is relative by default, absolute with `--absolute`.

Language tags follow extension mapping in `crates/treecat-core/src/lang.rs`.

## Binary / non-text handling

Binary files are never dumped raw. They are rendered as:

```text
# <display-path>
(skipped: binary/non-text file, size: <human-size>)
```

Size formatting:

- Base-1024 units: `B`, `KB`, `MB`, `GB`, `TB`
- `B` is integer (`3 B`)
- `KB` and above use one decimal (`24.1 KB`)

## Text vs binary classification

Classification uses:

1. Extension-based binary heuristics (common image/archive/media/executable/db extensions).
2. Content sniff fallback (first 4096 bytes):
   - NUL byte => binary
   - low printable-byte ratio => binary

## Explicit file behavior

When `FILES...` are provided:

- Content section is restricted to those files.
- Include filters are bypassed for explicit files.
- Exclude filters and size limits still apply.
- Tree still reflects full filtered traversal, not just explicit files.

## Determinism

- Tree ordering and content file ordering are deterministic.
- Rendering never includes clipboard status text in stdout.
- Clipboard copy (when enabled) is plain text (no ANSI escapes).

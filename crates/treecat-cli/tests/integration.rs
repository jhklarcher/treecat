use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/basic")
        .canonicalize()
        .expect("fixture path")
}

fn run_treecat_output(args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_treecat");
    Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run treecat")
}

fn run_treecat_output_env(args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    run_treecat_output_env_with_removals(args, envs, &[])
}

fn run_treecat_output_env_with_removals(
    args: &[&str],
    envs: &[(&str, &str)],
    removals: &[&str],
) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_treecat");
    let mut cmd = Command::new(bin);
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    for key in removals {
        cmd.env_remove(key);
    }
    cmd.output().expect("failed to run treecat")
}

fn run_treecat(args: &[&str]) -> String {
    let output = run_treecat_output(args);
    assert!(
        output.status.success(),
        "treecat failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn renders_tree_and_contents() {
    let dir = fixture_path();
    let out = run_treecat(&[dir.to_str().unwrap()]);
    assert!(out.contains("root.txt"));
    assert!(out.contains("sub/"));
    assert!(out.contains("sub/nested.txt"));
    assert!(out.contains("# root.txt"));
    assert!(out.contains("# sub/nested.txt"));
    assert!(out.contains("root file"));
    assert!(out.contains("nested"));
}

#[test]
fn explicit_files_bypass_includes() {
    let dir = fixture_path();
    let out = run_treecat(&[
        dir.to_str().unwrap(),
        "root.txt",
        "-x",
        "md",
        "--files-only",
    ]);
    assert!(out.contains("# root.txt"));
    assert!(out.contains("root file"));
}

#[test]
fn missing_explicit_file_has_clear_error() {
    let dir = fixture_path();
    let output = run_treecat_output(&[dir.to_str().unwrap(), "missing.txt"]);
    assert!(
        !output.status.success(),
        "expected failure for missing file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("explicit file not found"));
    assert!(stderr.contains("missing.txt"));
}

fn has_ansi(s: &str) -> bool {
    s.contains("\u{1b}[")
}

#[test]
fn color_always_includes_ansi_even_when_piped() {
    let dir = fixture_path();
    let output = run_treecat_output(&[dir.to_str().unwrap(), "--files-only", "--color=always"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(has_ansi(&stdout));
}

#[test]
fn color_never_suppresses_ansi() {
    let dir = fixture_path();
    let output = run_treecat_output_env(
        &[dir.to_str().unwrap(), "--files-only", "--color=never"],
        &[("FORCE_COLOR", "1")],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!has_ansi(&stdout));
}

#[test]
fn copy_flag_never_fails_stdout_rendering() {
    let dir = fixture_path();
    let output = run_treecat_output(&[dir.to_str().unwrap(), "--files-only", "--copy"]);
    assert!(
        output.status.success(),
        "treecat failed with --copy: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# root.txt"));
}

#[test]
fn loads_default_config_from_home_path() {
    let fixture = fixture_path();
    let home = tempdir().unwrap();
    let cfg_dir = home.path().join(".config/treecat");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        format!(
            "root_path = {:?}\nfiles_only = true\n",
            fixture.to_string_lossy()
        ),
    )
    .unwrap();

    let output = run_treecat_output_env_with_removals(
        &[],
        &[("HOME", home.path().to_str().unwrap())],
        &["XDG_CONFIG_HOME", "TREECAT_CONFIG"],
    );
    assert!(
        output.status.success(),
        "treecat failed with default config: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# root.txt"));
    assert!(stdout.contains("# sub/nested.txt"));
}

#[test]
fn cli_root_overrides_config_root() {
    let fixture = fixture_path();
    let alt = tempdir().unwrap();
    std::fs::write(alt.path().join("alt.txt"), b"alt\n").unwrap();

    let home = tempdir().unwrap();
    let cfg_dir = home.path().join(".config/treecat");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        format!(
            "root_path = {:?}\nfiles_only = true\n",
            fixture.to_string_lossy()
        ),
    )
    .unwrap();

    let output = run_treecat_output_env_with_removals(
        &[alt.path().to_str().unwrap()],
        &[("HOME", home.path().to_str().unwrap())],
        &["XDG_CONFIG_HOME", "TREECAT_CONFIG"],
    );
    assert!(
        output.status.success(),
        "treecat failed with CLI root override: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# alt.txt"));
    assert!(!stdout.contains("# root.txt"));
}

#[test]
fn no_config_ignores_invalid_config_file() {
    let fixture = fixture_path();
    let home = tempdir().unwrap();
    let cfg_dir = home.path().join(".config/treecat");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(cfg_dir.join("config.toml"), "files_only = tru\n").unwrap();

    let output = run_treecat_output_env_with_removals(
        &[fixture.to_str().unwrap(), "--files-only", "--no-config"],
        &[("HOME", home.path().to_str().unwrap())],
        &["XDG_CONFIG_HOME", "TREECAT_CONFIG"],
    );
    assert!(
        output.status.success(),
        "treecat failed with --no-config: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# root.txt"));
}

#[test]
fn tree_respects_include_filters() {
    let dir = fixture_path();
    let output = run_treecat_output(&[
        dir.to_str().unwrap(),
        "--tree-only",
        "--include-glob",
        "root.*",
    ]);
    assert!(
        output.status.success(),
        "treecat failed with include filter: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("root.txt"));
    assert!(!stdout.contains("nested.txt"));
}

#[test]
fn tree_respects_exclude_filters() {
    let dir = fixture_path();
    let output = run_treecat_output(&[
        dir.to_str().unwrap(),
        "--tree-only",
        "--exclude-glob",
        "nested.*",
    ]);
    assert!(
        output.status.success(),
        "treecat failed with exclude filter: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("root.txt"));
    assert!(!stdout.contains("nested.txt"));
}

#[test]
fn max_size_accepts_suffixes() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("small.txt"), b"small\n").unwrap();
    std::fs::write(dir.path().join("large.txt"), vec![b'x'; 2048]).unwrap();

    let output = run_treecat_output(&[
        dir.path().to_str().unwrap(),
        "--files-only",
        "--max-size",
        "1K",
    ]);
    assert!(
        output.status.success(),
        "treecat failed with suffixed size: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# small.txt"));
    assert!(!stdout.contains("# large.txt"));
}

#[test]
fn invalid_max_size_suffix_fails() {
    let dir = fixture_path();
    let output = run_treecat_output(&[dir.to_str().unwrap(), "--files-only", "--max-size", "10Q"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value '10Q'"));
    assert!(stderr.contains("unsupported size suffix"));
}

#[test]
fn tree_without_file_filters_keeps_empty_directories() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("empty")).unwrap();
    std::fs::write(dir.path().join("file.txt"), b"ok\n").unwrap();

    let output = run_treecat_output(&[dir.path().to_str().unwrap(), "--tree-only"]);
    assert!(
        output.status.success(),
        "treecat failed for empty-dir tree case: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("empty/"));
    assert!(stdout.contains("file.txt"));
}

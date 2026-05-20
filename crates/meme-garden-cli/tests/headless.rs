//! End-to-end smoke test for the headless CLI.

use std::io::{BufRead, BufReader};
use std::process::Command;

fn cargo_bin() -> std::path::PathBuf {
    // Locate the just-built binary via cargo's standard env var.
    let exe = env!("CARGO_BIN_EXE_meme-garden");
    std::path::PathBuf::from(exe)
}

fn workspace_root() -> std::path::PathBuf {
    // crates/meme-garden-cli/tests/headless.rs → workspace root is three up.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn headless_writes_required_artifacts() {
    let root = workspace_root();
    let bin = cargo_bin();
    // Use a unique run-id so the test is isolated.
    let run_id = format!("test-headless-{}", std::process::id());

    let status = Command::new(&bin)
        .current_dir(&root)
        .args([
            "headless",
            "--preset",
            "cooperation-vs-selfish-low",
            "--seed",
            "42",
            "--ticks",
            "100",
            "--run-id",
            &run_id,
        ])
        .status()
        .expect("running headless command");
    assert!(status.success(), "headless command failed: {status}");

    let run_dir = root.join("runs").join(&run_id);
    assert!(run_dir.join("config.toml").exists(), "missing config.toml");
    assert!(
        run_dir.join("events.jsonl").exists(),
        "missing events.jsonl"
    );
    assert!(run_dir.join("summary.csv").exists(), "missing summary.csv");

    // First JSONL record is the header.
    let f = std::fs::File::open(run_dir.join("events.jsonl")).unwrap();
    let mut reader = BufReader::new(f);
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    let v: serde_json::Value = serde_json::from_str(first.trim()).unwrap();
    assert_eq!(v.get("kind").and_then(|x| x.as_str()), Some("header"));
    assert_eq!(v.get("schema_version").and_then(|x| x.as_u64()), Some(1));
    assert_eq!(
        v.get("run_id").and_then(|x| x.as_str()),
        Some(run_id.as_str())
    );

    // Summary first column is `tick`; every row except the header should parse.
    let csv = std::fs::read_to_string(run_dir.join("summary.csv")).unwrap();
    let mut lines = csv.lines();
    let header = lines.next().unwrap();
    assert!(header.starts_with("tick,alive,food_count,meme_count"));
    let n_rows = lines.count();
    assert!(n_rows >= 100, "expected ≥100 rows, got {n_rows}");

    // Cleanup so we don't leak run dirs between test runs.
    let _ = std::fs::remove_dir_all(&run_dir);
}

#[test]
fn headless_is_deterministic() {
    let root = workspace_root();
    let bin = cargo_bin();
    let pid = std::process::id();
    let run_a = format!("test-det-a-{pid}");
    let run_b = format!("test-det-b-{pid}");

    for run in &[&run_a, &run_b] {
        let status = Command::new(&bin)
            .current_dir(&root)
            .args([
                "headless",
                "--preset",
                "cooperation-vs-selfish-low",
                "--seed",
                "100",
                "--ticks",
                "80",
                "--run-id",
                run,
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    // Both events.jsonl files must be byte-identical except for the run-id in
    // the header line — so we strip the header line before comparing.
    let strip_header = |path: &std::path::Path| {
        let mut s = std::fs::read_to_string(path).unwrap();
        if let Some(nl) = s.find('\n') {
            s = s[nl + 1..].to_string();
        }
        s
    };
    let a = strip_header(&root.join("runs").join(&run_a).join("events.jsonl"));
    let b = strip_header(&root.join("runs").join(&run_b).join("events.jsonl"));
    assert_eq!(
        a, b,
        "two runs with same (config, seed) produced different events"
    );

    let _ = std::fs::remove_dir_all(root.join("runs").join(&run_a));
    let _ = std::fs::remove_dir_all(root.join("runs").join(&run_b));
}

#[test]
fn list_presets_includes_all_three() {
    let root = workspace_root();
    let bin = cargo_bin();
    let output = Command::new(&bin)
        .current_dir(&root)
        .arg("list-presets")
        .output()
        .unwrap();
    assert!(output.status.success());
    let s = String::from_utf8(output.stdout).unwrap();
    for level in &[
        "cooperation-vs-selfish-low",
        "cooperation-vs-selfish-mid",
        "cooperation-vs-selfish-high",
    ] {
        assert!(
            s.contains(level),
            "list-presets output missing {level}\n{s}"
        );
    }
}

#[test]
fn experiment_design_exits_with_status_2() {
    let root = workspace_root();
    let bin = cargo_bin();
    let status = Command::new(&bin)
        .current_dir(&root)
        .args(["experiment", "design", "study cooperation under famine"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

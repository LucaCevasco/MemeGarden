//! Export round-trip: regenerating summary.csv from events.jsonl matches the
//! original byte-for-byte.

use std::process::Command;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_meme-garden"))
}

fn workspace_root() -> std::path::PathBuf {
    let m = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    m.parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn export_csv_matches_original() {
    let root = workspace_root();
    let bin = cargo_bin();
    let pid = std::process::id();
    let run_id = format!("export-rt-{pid}");

    // Run.
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
        .unwrap();
    assert!(status.success());

    let run_dir = root.join("runs").join(&run_id);
    let original_csv = std::fs::read_to_string(run_dir.join("summary.csv")).unwrap();

    // Re-emit via export.
    let status = Command::new(&bin)
        .current_dir(&root)
        .args(["export", run_dir.to_str().unwrap(), "--to", "csv"])
        .status()
        .unwrap();
    assert!(status.success());

    let regenerated_csv = std::fs::read_to_string(run_dir.join("summary.csv")).unwrap();
    assert_eq!(original_csv, regenerated_csv);

    // Validate JSONL.
    let status = Command::new(&bin)
        .current_dir(&root)
        .args(["export", run_dir.to_str().unwrap(), "--to", "jsonl"])
        .status()
        .unwrap();
    assert!(status.success());

    // Summary md.
    let output = Command::new(&bin)
        .current_dir(&root)
        .args(["export", run_dir.to_str().unwrap(), "--to", "summary-md"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let s = String::from_utf8(output.stdout).unwrap();
    assert!(s.contains("# Run summary"));
    assert!(s.contains("ticks recorded"));

    let _ = std::fs::remove_dir_all(&run_dir);
}

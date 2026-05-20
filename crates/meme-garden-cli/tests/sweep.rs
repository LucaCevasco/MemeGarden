//! Parameter-sweep style integration: three runs differing only in one knob
//! produce three distinct artifact sets, and re-runs are byte-identical.

use std::process::Command;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_meme-garden"))
}

fn workspace_root() -> std::path::PathBuf {
    let m = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    m.parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn sweep_scarcity_produces_distinct_outcomes_and_each_is_deterministic() {
    let root = workspace_root();
    let bin = cargo_bin();
    let pid = std::process::id();

    let mut summary_hashes = Vec::new();
    for level in &["low", "mid", "high"] {
        let run = format!("sweep-{level}-{pid}");
        let status = Command::new(&bin)
            .current_dir(&root)
            .args([
                "headless",
                "--preset",
                &format!("cooperation-vs-selfish-{level}"),
                "--seed",
                "42",
                "--ticks",
                "150",
                "--run-id",
                &run,
            ])
            .status()
            .unwrap();
        assert!(status.success());

        // Determinism: re-run with --run-id <new>; strip-header compare.
        let run2 = format!("sweep-{level}-r2-{pid}");
        let status = Command::new(&bin)
            .current_dir(&root)
            .args([
                "headless",
                "--preset",
                &format!("cooperation-vs-selfish-{level}"),
                "--seed",
                "42",
                "--ticks",
                "150",
                "--run-id",
                &run2,
            ])
            .status()
            .unwrap();
        assert!(status.success());
        let strip_header = |s: String| s.find('\n').map(|i| s[i + 1..].to_string()).unwrap_or(s);
        let a = std::fs::read_to_string(root.join("runs").join(&run).join("events.jsonl")).unwrap();
        let b =
            std::fs::read_to_string(root.join("runs").join(&run2).join("events.jsonl")).unwrap();
        assert_eq!(
            strip_header(a.clone()),
            strip_header(b),
            "{level} not deterministic"
        );

        // Hash the (header-stripped) JSONL for cross-level distinctness check.
        let h = sha_like(strip_header(a).as_bytes());
        summary_hashes.push((level.to_string(), h));

        let _ = std::fs::remove_dir_all(root.join("runs").join(&run));
        let _ = std::fs::remove_dir_all(root.join("runs").join(&run2));
    }

    // All three levels should differ from each other.
    let h_low = &summary_hashes[0].1;
    let h_mid = &summary_hashes[1].1;
    let h_high = &summary_hashes[2].1;
    assert_ne!(h_low, h_mid, "low and mid produced identical streams");
    assert_ne!(h_mid, h_high, "mid and high produced identical streams");
    assert_ne!(h_low, h_high, "low and high produced identical streams");
}

/// Cheap fingerprint — not cryptographic, just enough for distinctness.
fn sha_like(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

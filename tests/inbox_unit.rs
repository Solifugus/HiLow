// Phase 5a: driver for the C-level inbox unit tests (tests/inbox_unit_harness.c).
// Compiles the harness against the runtime with `cc -pthread`, runs it (asserting
// every case passes), then runs it again under valgrind asserting zero definite/
// indirect leaks and zero errors — the same leak criteria as the valgrind gate.
// Through Phase 5a the cross-thread queue path has no HiLow-source surface, so
// this is where enqueue/coalesce/accumulate/retain-drop/drain/teardown are proven.

use std::path::PathBuf;
use std::process::Command;

fn manifest(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p)
}

fn build_harness(tag: &str) -> PathBuf {
    // Unique per test: both tests run in parallel in one process, so a pid-only
    // name would collide (one clobbers/deletes the other's binary mid-run).
    let out = std::env::temp_dir().join(format!("hl_inbox_harness_{}_{}", std::process::id(), tag));
    let status = Command::new("cc")
        .arg("-pthread")
        .arg("-O0")
        .arg("-g")
        .arg("-o")
        .arg(&out)
        .arg(manifest("tests/inbox_unit_harness.c"))
        .arg(manifest("src/runtime/runtime.c"))
        .arg(format!("-I{}", manifest("src/runtime").display()))
        .status()
        .expect("failed to invoke cc for the inbox harness");
    assert!(status.success(), "inbox harness failed to compile");
    out
}

#[test]
fn inbox_unit_cases_pass() {
    let bin = build_harness("cases");
    let output = Command::new(&bin).output().expect("failed to run inbox harness");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "inbox harness exited nonzero:\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("ALL INBOX UNIT TESTS PASSED"),
        "inbox harness did not report success; stdout:\n{}",
        stdout
    );
    let _ = std::fs::remove_file(&bin);
}

#[test]
fn inbox_unit_is_valgrind_clean() {
    // Preflight: valgrind must exist (no silent skip — mirrors valgrind_gate.rs).
    let has_valgrind = Command::new("valgrind")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    assert!(has_valgrind, "valgrind is required for the inbox unit gate but was not found");

    let bin = build_harness("valgrind");
    let output = Command::new("valgrind")
        .arg("--leak-check=full")
        .arg("--errors-for-leak-kinds=definite,indirect")
        .arg("--error-exitcode=99")
        .arg(&bin)
        .output()
        .expect("failed to run valgrind on the inbox harness");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Assert on valgrind's own ERROR SUMMARY (the harness itself exits 0).
    let summary = stderr
        .lines()
        .find(|l| l.contains("ERROR SUMMARY"))
        .unwrap_or_else(|| panic!("no ERROR SUMMARY in valgrind output:\n{}", stderr));
    assert!(
        summary.contains("ERROR SUMMARY: 0 errors"),
        "valgrind reported errors on the inbox harness:\n{}\n---\n{}",
        summary,
        stderr
            .lines()
            .filter(|l| l.contains("lost") || l.contains("ERROR SUMMARY"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let _ = std::fs::remove_file(&bin);
}

// Phase 6a: driver for the C-level shm unit tests (tests/shm_unit_harness.c).
// Compiles the harness against the runtime with -DHL_SHM_TEST_SUPPORT (enables
// the forge hooks) and a short init-wait (so the timeout case is ~ms not ~2s),
// runs it (asserting every case passes), then runs it again under valgrind
// asserting zero definite/indirect leaks and zero errors — the same criteria as
// the valgrind gate. Only i32 is placeable, so the attacher's header-mismatch
// and crashed-mid-init timeout paths have no HiLow-source surface; this is where
// they are proven.

use std::path::PathBuf;
use std::process::Command;

fn manifest(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p)
}

fn build_harness(tag: &str) -> PathBuf {
    let out = std::env::temp_dir().join(format!("hl_shm_harness_{}_{}", std::process::id(), tag));
    let status = Command::new("cc")
        .arg("-pthread")
        .arg("-O0")
        .arg("-g")
        .arg("-DHL_SHM_TEST_SUPPORT")     // enable the forge hooks in runtime.c
        .arg("-DHL_SHM_INIT_WAIT_ITERS=5") // fast timeout for case 4 (~1ms)
        .arg("-o")
        .arg(&out)
        .arg(manifest("tests/shm_unit_harness.c"))
        .arg(manifest("src/runtime/runtime.c"))
        .arg(format!("-I{}", manifest("src/runtime").display()))
        .arg("-lrt") // POSIX shm_open (no-op on modern glibc)
        .status()
        .expect("failed to invoke cc for the shm harness");
    assert!(status.success(), "shm harness failed to compile");
    out
}

#[test]
fn shm_unit_cases_pass() {
    let bin = build_harness("cases");
    let output = Command::new(&bin).output().expect("failed to run shm harness");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "shm harness exited nonzero:\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("ALL SHM UNIT TESTS PASSED"),
        "shm harness did not report success; stdout:\n{}",
        stdout
    );
    let _ = std::fs::remove_file(&bin);
}

#[test]
fn shm_unit_is_valgrind_clean() {
    let has_valgrind = Command::new("valgrind")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    assert!(has_valgrind, "valgrind is required for the shm unit gate but was not found");

    let bin = build_harness("valgrind");
    let output = Command::new("valgrind")
        .arg("--leak-check=full")
        .arg("--errors-for-leak-kinds=definite,indirect")
        .arg("--error-exitcode=99")
        // The harness fork()s children that intentionally exit(1); trace only
        // the parent so a child's expected nonzero exit is not a valgrind error.
        .arg("--trace-children=no")
        .arg(&bin)
        .output()
        .expect("failed to run valgrind on the shm harness");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let summary = stderr
        .lines()
        .find(|l| l.contains("ERROR SUMMARY"))
        .unwrap_or_else(|| panic!("no ERROR SUMMARY in valgrind output:\n{}", stderr));
    assert!(
        summary.contains("ERROR SUMMARY: 0 errors"),
        "valgrind reported errors on the shm harness:\n{}\n---\n{}",
        summary,
        stderr
            .lines()
            .filter(|l| l.contains("lost") || l.contains("ERROR SUMMARY"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let _ = std::fs::remove_file(&bin);
}

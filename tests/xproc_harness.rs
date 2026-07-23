// Phase 6a: two-process harness for cross-process `shared("name")` placement
// (docs/phase6-brief.md §3-6a — "the phase's real infrastructure investment").
//
// Cross-process fixtures need TWO programs sharing one segment; they cannot live
// under tests/programs/ (the valgrind gate auto-discovers those and runs each as
// a standalone single-process program). They live under tests/xproc/ and are
// orchestrated here: the harness rewrites the segment name to a UNIQUE per-run
// value (hermeticity — the spin-rewrite pattern), compiles both halves, runs
// them under valgrind in a controlled order, asserts both outputs + exit codes
// + valgrind cleanliness, and ALWAYS unlinks its segment (a leaked test segment
// is a harness bug). "Runs both under valgrind in the gate lanes" is realized
// here as gate-equivalent valgrind rigor in a dedicated harness.
//
// The fixture pair (tests/xproc/share/{a,b}.hl) shares one i32:
//   A: init 100, writes 105, prints 105   (creator) / ignores init, +5 (attacher)
//   B: init 200, prints 200               (creator) / ignores init, reads (attacher)
// Run A-then-B and B-then-A to cover: creator-initializes, attacher-ignores-init,
// persistence (segment outlives the first process), and same-name sharing.

use std::fs;
use std::path::Path;
use std::process::Command;

const PLACEHOLDER: &str = "xproc_share";

fn require_valgrind() {
    let v = Command::new("valgrind").arg("--version").output();
    assert!(
        v.map(|o| o.status.success()).unwrap_or(false),
        "valgrind is required for the Phase 6a cross-process harness — it does not skip."
    );
}

/// Rewrite the fixture's placeholder segment name to `seg`, write to a temp .hl,
/// and compile it to `out_bin`. Panics on compile failure (fixtures must build).
fn compile_with_seg(src: &str, seg: &str, out_bin: &Path) {
    let source = fs::read_to_string(src).expect("read fixture");
    assert!(
        source.contains(PLACEHOLDER),
        "fixture {} lost its `{}` placeholder — update the harness",
        src, PLACEHOLDER
    );
    let rewritten = source.replace(PLACEHOLDER, seg);
    let tmp_hl = out_bin.with_extension("hl");
    fs::write(&tmp_hl, rewritten).expect("write rewritten fixture");
    let out = Command::new("./target/debug/hilowc")
        .arg(&tmp_hl)
        .arg("-o")
        .arg(out_bin)
        .output()
        .expect("invoke hilowc");
    let _ = fs::remove_file(&tmp_hl);
    assert!(
        out.status.success(),
        "compiling {} (seg {}) failed:\n{}",
        src, seg, String::from_utf8_lossy(&out.stderr)
    );
}

/// Run `bin` under valgrind. Returns (stdout, exit_code, valgrind_error_count).
fn run_under_valgrind(bin: &Path) -> (String, i32, u32) {
    let out = Command::new("valgrind")
        .arg("--leak-check=full")
        .arg("--errors-for-leak-kinds=definite,indirect")
        .arg(bin)
        .output()
        .expect("invoke valgrind");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut errors = u32::MAX;
    for line in stderr.lines() {
        if let Some(idx) = line.find("ERROR SUMMARY: ") {
            let rest = &line[idx + "ERROR SUMMARY: ".len()..];
            errors = rest.split_whitespace().next().unwrap_or("").parse::<u32>().unwrap_or(u32::MAX);
        }
    }
    assert_ne!(errors, u32::MAX, "no valgrind ERROR SUMMARY for {}:\n{}", bin.display(), stderr);
    // The valgrinded child's own exit code (valgrind passes it through).
    let code = out.status.code().unwrap_or(-1);
    (stdout, code, errors)
}

fn unlink_segment(seg: &str) {
    // POSIX shm objects surface at /dev/shm/<name> on Linux; the runtime maps
    // "name" → "/hilow.<name>". Best-effort unlink (may not exist).
    let path = format!("/dev/shm/hilow.{}", seg);
    let _ = fs::remove_file(&path);
}

fn unique_seg(tag: &str) -> String {
    // No collisions across parallel test binaries / leftover runs. std::process
    // id is stable within this test process; the tag disambiguates orderings.
    format!("xproc_{}_{}", std::process::id(), tag)
}

/// One ordering: compile A and B against a fresh unique segment, run `first`
/// then `second` under valgrind, and assert. `first_out`/`second_out` are the
/// expected stdout (trimmed) of whichever program runs first/second.
fn run_ordering(tag: &str, first_src: &str, second_src: &str, first_out: &str, second_out: &str) {
    let seg = unique_seg(tag);
    unlink_segment(&seg); // pre-clean in case a prior crashed run leaked it

    let dir = std::env::temp_dir();
    let bin1 = dir.join(format!("hl_xproc_{}_{}_1", std::process::id(), tag));
    let bin2 = dir.join(format!("hl_xproc_{}_{}_2", std::process::id(), tag));
    compile_with_seg(first_src, &seg, &bin1);
    compile_with_seg(second_src, &seg, &bin2);

    // Sequential launch: `first` creates the segment, writes, and EXITS; the
    // segment persists; `second` then attaches and observes the persisted value.
    let (out1, code1, verr1) = run_under_valgrind(&bin1);
    let (out2, code2, verr2) = run_under_valgrind(&bin2);

    // Always unlink our segment + binaries, even if an assert below fails.
    unlink_segment(&seg);
    let _ = fs::remove_file(&bin1);
    let _ = fs::remove_file(&bin2);

    assert_eq!(out1.trim(), first_out, "[{}] first program stdout", tag);
    assert_eq!(out2.trim(), second_out, "[{}] second program stdout", tag);
    assert_eq!(code1, 0, "[{}] first program exit code", tag);
    assert_eq!(code2, 0, "[{}] second program exit code", tag);
    assert_eq!(verr1, 0, "[{}] first program valgrind errors", tag);
    assert_eq!(verr2, 0, "[{}] second program valgrind errors", tag);
}

// A-then-B: A creates (100 → writes 105, prints 105); segment persists at 105;
// B attaches (ignores its init 200), reads the persisted 105, prints 105.
// Proves: creator initializes + writes, persistence across process exit,
// attacher ignores its own initializer and observes the shared value.
#[test]
fn xproc_share_a_then_b() {
    require_valgrind();
    run_ordering(
        "a_then_b",
        "tests/xproc/share/a.hl",
        "tests/xproc/share/b.hl",
        "105",
        "105",
    );
}

// B-then-A: B creates (200, prints 200); segment persists at 200; A attaches
// (ignores its init 100), reads 200, adds 5, prints 205. Proves the creator/
// attacher role is decided by launch order (whoever wins O_CREAT|O_EXCL), and
// the attacher reads AND writes the same shared slot.
#[test]
fn xproc_share_b_then_a() {
    require_valgrind();
    run_ordering(
        "b_then_a",
        "tests/xproc/share/b.hl",
        "tests/xproc/share/a.hl",
        "200",
        "205",
    );
}

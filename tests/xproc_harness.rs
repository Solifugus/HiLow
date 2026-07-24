// Phase 6a/6b: two-process harness for cross-process `shared("name")` placement
// (docs/phase6-brief.md §3-6a/§3-6b — "the phase's real infrastructure
// investment"). Cross-process fixtures need programs sharing a segment; they
// cannot live under tests/programs/ (the valgrind gate runs those standalone).
// They live under tests/xproc/ and are orchestrated here: each fixture's
// segment token `XSEG` is rewritten to a UNIQUE per-run prefix (hermeticity),
// both/all halves compile, run under valgrind (gate-equivalent rigor), and the
// harness ALWAYS unlinks every segment with that prefix (a leaked test segment
// is a harness bug). Assertions are order-insensitive (5b discipline).
//
// 6a fixtures (share/): sequential — creator initializes, attacher observes.
// 6b fixtures: single-process dedup (dedup_same, dedup_cross) and concurrent
// cross-process delivery (prodcons, watcher_exits) with a `ready` handshake so
// the producer only writes once the consumer is watching (no startup race).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const TOKEN: &str = "XSEG";

fn require_valgrind() {
    let v = Command::new("valgrind").arg("--version").output();
    assert!(
        v.map(|o| o.status.success()).unwrap_or(false),
        "valgrind is required for the Phase 6a/6b cross-process harness — it does not skip."
    );
}

fn unique_prefix(tag: &str) -> String {
    format!("xp{}_{}", std::process::id(), tag)
}

/// Rewrite the fixture's `XSEG` token to `prefix` and compile to `out_bin`.
fn compile_fixture(src: &str, prefix: &str, out_bin: &Path) {
    let source = fs::read_to_string(src).expect("read fixture");
    assert!(source.contains(TOKEN), "fixture {} lost its `{}` token", src, TOKEN);
    let tmp_hl = out_bin.with_extension("hl");
    fs::write(&tmp_hl, source.replace(TOKEN, prefix)).expect("write rewritten fixture");
    let out = Command::new("./target/debug/hilowc")
        .arg(&tmp_hl).arg("-o").arg(out_bin)
        .output().expect("invoke hilowc");
    let _ = fs::remove_file(&tmp_hl);
    assert!(out.status.success(), "compiling {} failed:\n{}", src, String::from_utf8_lossy(&out.stderr));
}

/// Remove every POSIX shm object /dev/shm/hilow.<prefix>* (Linux). Best-effort.
fn unlink_prefix(prefix: &str) {
    let needle = format!("hilow.{}", prefix);
    if let Ok(rd) = fs::read_dir("/dev/shm") {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with(&needle) {
                let _ = fs::remove_file(e.path());
            }
        }
    }
}

fn valgrind_child(bin: &Path) -> Child {
    Command::new("valgrind")
        .arg("--leak-check=full")
        .arg("--errors-for-leak-kinds=definite,indirect")
        .arg(bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn valgrind")
}

/// Wait for a spawned valgrind child; return (stdout, exit_code, valgrind_errors).
fn finish(child: Child, label: &str) -> (String, i32, u32) {
    let out = child.wait_with_output().expect("wait valgrind child");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut errors = u32::MAX;
    for line in stderr.lines() {
        if let Some(idx) = line.find("ERROR SUMMARY: ") {
            errors = line[idx + 15..].split_whitespace().next().unwrap_or("").parse().unwrap_or(u32::MAX);
        }
    }
    assert_ne!(errors, u32::MAX, "[{}] no valgrind ERROR SUMMARY:\n{}", label, stderr);
    (stdout, out.status.code().unwrap_or(-1), errors)
}

fn assert_clean(label: &str, got: &(String, i32, u32), expected_stdout: &str) {
    assert_eq!(got.0.trim(), expected_stdout, "[{}] stdout", label);
    assert_eq!(got.1, 0, "[{}] exit code", label);
    assert_eq!(got.2, 0, "[{}] valgrind errors", label);
}

fn bin_path(tag: &str, which: &str) -> PathBuf {
    std::env::temp_dir().join(format!("hl_xproc_{}_{}_{}", std::process::id(), tag, which))
}

// ---- 6c: crash-robustness support -------------------------------------------
//
// A process we intend to SIGKILL is run WITHOUT valgrind: a killed process never
// reaches valgrind's ERROR SUMMARY line, so `finish` could not evaluate it. The
// mini-brief scopes the leak check to the SURVIVING processes, which are all
// still valgrinded below. Running the victim natively also keeps it fast, so it
// is reliably deep inside its work when the kill lands.
fn native_child(bin: &Path) -> Child {
    Command::new(bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn native child")
}

/// SIGKILL `child` (Rust's `Child::kill` is SIGKILL on Unix — no unwinding, no
/// atexit, no destructors: the segment is abandoned exactly as in a real crash)
/// and reap it. Asserts the victim did NOT exit on its own beforehand, which
/// would silently void the crash scenario.
fn kill_and_reap(mut child: Child, label: &str) {
    match child.try_wait().expect("try_wait victim") {
        Some(status) => panic!(
            "[{}] victim exited on its own ({:?}) before the SIGKILL — the crash \
             scenario did not happen; the fixture must spin until killed.",
            label, status
        ),
        None => {}
    }
    child.kill().expect("SIGKILL victim");
    let _ = child.wait().expect("reap victim");
}

fn settle(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

// ---- 6a: sequential creator/attacher (share) --------------------------------

fn run_share_ordering(tag: &str, first_src: &str, second_src: &str, first_out: &str, second_out: &str) {
    let prefix = unique_prefix(tag);
    unlink_prefix(&prefix);
    let b1 = bin_path(tag, "1");
    let b2 = bin_path(tag, "2");
    compile_fixture(first_src, &prefix, &b1);
    compile_fixture(second_src, &prefix, &b2);
    // Sequential: `first` creates + writes + EXITS (segment persists); `second`
    // then attaches and observes the persisted value.
    let r1 = finish(valgrind_child(&b1), "first");
    let r2 = finish(valgrind_child(&b2), "second");
    unlink_prefix(&prefix);
    let _ = fs::remove_file(&b1);
    let _ = fs::remove_file(&b2);
    assert_clean("first", &r1, first_out);
    assert_clean("second", &r2, second_out);
}

#[test]
fn xproc_share_a_then_b() {
    require_valgrind();
    run_share_ordering("a_then_b", "tests/xproc/share/a.hl", "tests/xproc/share/b.hl", "105", "105");
}

#[test]
fn xproc_share_b_then_a() {
    require_valgrind();
    run_share_ordering("b_then_a", "tests/xproc/share/b.hl", "tests/xproc/share/a.hl", "200", "205");
}

// ---- 6b: single-process dedup -----------------------------------------------

fn run_single(tag: &str, src: &str, expected: &str) {
    let prefix = unique_prefix(tag);
    unlink_prefix(&prefix);
    let b = bin_path(tag, "0");
    compile_fixture(src, &prefix, &b);
    let r = finish(valgrind_child(&b), tag);
    unlink_prefix(&prefix);
    let _ = fs::remove_file(&b);
    assert_clean(tag, &r, expected);
}

// Three same-thread writes fire (changed) exactly once each; the pull adds
// nothing (R1 exactness on a placed cell). Output "3".
#[test]
fn xproc_dedup_same_thread() {
    require_valgrind();
    run_single("dedup_same", "tests/xproc/dedup_same/prog.hl", "3");
}

// An async producer's writes are delivered to main by the epoch pull (not the
// inbox — placed cross-thread skips it), no double. Output "done".
#[test]
fn xproc_dedup_cross_thread() {
    require_valgrind();
    run_single("dedup_cross", "tests/xproc/dedup_cross/prog.hl", "done");
}

// ---- 6b: concurrent cross-process delivery ----------------------------------

/// Compile consumer+producer against a fresh unique prefix, run BOTH under
/// valgrind concurrently, assert both, unlink. `ready`-handshake in the fixtures
/// removes the startup race.
fn run_concurrent(tag: &str, consumer_src: &str, producer_src: &str, cons_out: &str, prod_out: &str) {
    let prefix = unique_prefix(tag);
    unlink_prefix(&prefix);
    let bc = bin_path(tag, "cons");
    let bp = bin_path(tag, "prod");
    compile_fixture(consumer_src, &prefix, &bc);
    compile_fixture(producer_src, &prefix, &bp);
    // Spawn both, THEN wait — they run in parallel (separate OS processes).
    let cc = valgrind_child(&bc);
    let pc = valgrind_child(&bp);
    let rc = finish(cc, "consumer");
    let rp = finish(pc, "producer");
    unlink_prefix(&prefix);
    let _ = fs::remove_file(&bc);
    let _ = fs::remove_file(&bp);
    assert_clean("consumer", &rc, cons_out);
    assert_clean("producer", &rp, prod_out);
}

// Producer's remote write to the placed counter is delivered to the consumer's
// watcher by the consumer's epoch pull; consumer observes the threshold.
#[test]
fn xproc_prodcons_delivery() {
    require_valgrind();
    run_concurrent(
        "prodcons",
        "tests/xproc/prodcons/consumer.hl",
        "tests/xproc/prodcons/producer.hl",
        "consumer observed threshold",
        "producer done",
    );
}

// The watcher process exits after one change; the producer completes its full
// burst regardless (no cross-process coupling — R-B, no queues).
#[test]
fn xproc_watcher_exits_no_coupling() {
    require_valgrind();
    run_concurrent(
        "watcher_exits",
        "tests/xproc/watcher_exits/consumer.hl",
        "tests/xproc/watcher_exits/producer.hl",
        "consumer exits early",
        "producer done",
    );
}

// ---- 6c: crash robustness ---------------------------------------------------
//
// These three are the content of Phase 6c (docs/phase6-brief.md §3-6c): a
// process tier is only credible if a participant can die VIOLENTLY — no
// unwinding, no cleanup hook, no chance to detach — without corrupting the
// segment, wedging a survivor, or destroying the shared state.

// Case 1: the PRODUCER is SIGKILLed mid-write-loop. Two phases, both with
// deterministic ordering — no sleeps, because each fixture's own exit condition
// sequences the test:
//   A. the consumer runs against the LIVE producer and verifies that no
//      delivered value ever goes backwards, then exits on a fire-count floor.
//   B. the producer is killed (kill_and_reap asserts it was still running, so
//      the kill provably landed inside the unbounded write loop), and only then
//      does a fresh survivor attach — proving the abandoned segment is still
//      well-formed, still valued, and does not wedge the pull.
#[test]
fn xproc_crash_producer_mid_write() {
    require_valgrind();
    let tag = "crash_producer";
    let prefix = unique_prefix(tag);
    unlink_prefix(&prefix);
    let bc = bin_path(tag, "cons");
    let bp = bin_path(tag, "prod");
    let bs = bin_path(tag, "surv");
    compile_fixture("tests/xproc/crash_producer/consumer.hl", &prefix, &bc);
    compile_fixture("tests/xproc/crash_producer/producer.hl", &prefix, &bp);
    compile_fixture("tests/xproc/crash_producer/survivor.hl", &prefix, &bs);

    // Phase A — live observation. The producer never exits on its own, so the
    // consumer's fire floor is always reached.
    let cc = valgrind_child(&bc); // survivor — valgrinded
    let pc = native_child(&bp); // victim — native (a SIGKILLed process never
                                // reaches valgrind's ERROR SUMMARY)
    let rc = finish(cc, "consumer");

    // Phase B — violent death, then re-attach.
    kill_and_reap(pc, "producer"); // asserts: still mid-write-loop when killed
    let rs = finish(valgrind_child(&bs), "survivor");

    unlink_prefix(&prefix);
    let _ = fs::remove_file(&bc);
    let _ = fs::remove_file(&bp);
    let _ = fs::remove_file(&bs);
    // "ok" ⟺ no delivered value ever went backwards ("CORRUPT" otherwise).
    assert_clean("consumer", &rc, "ok");
    // "intact" ⟺ attach verified, pull kept progressing, value survived.
    assert_clean("survivor", &rs, "intact");
}

// Case 2: the CONSUMER is SIGKILLed while attached. The producer must complete
// its full write burst unaffected (no queues, no back-pressure — R-B), and a
// fresh process must then attach to an intact segment holding every write.
#[test]
fn xproc_crash_consumer_producer_unaffected() {
    require_valgrind();
    let tag = "crash_consumer";
    let prefix = unique_prefix(tag);
    unlink_prefix(&prefix);
    let bv = bin_path(tag, "victim");
    let bp = bin_path(tag, "prod");
    let bf = bin_path(tag, "fresh");
    compile_fixture("tests/xproc/crash_consumer/victim.hl", &prefix, &bv);
    compile_fixture("tests/xproc/crash_consumer/producer.hl", &prefix, &bp);
    compile_fixture("tests/xproc/crash_consumer/fresh.hl", &prefix, &bf);

    let vc = native_child(&bv); // victim attaches + signals ready, then spins
    let pc = valgrind_child(&bp); // survivor — writes 500000 increments (~1.5s)
    settle(400); // kill lands well inside the producer's burst (measured)
    kill_and_reap(vc, "consumer");
    let rp = finish(pc, "producer");
    // Segment now has zero live attachers; a fresh process must still attach.
    let rf = finish(valgrind_child(&bf), "fresh");

    unlink_prefix(&prefix);
    let _ = fs::remove_file(&bv);
    let _ = fs::remove_file(&bp);
    let _ = fs::remove_file(&bf);
    assert_clean("producer", &rp, "producer done");
    // Every one of the producer's writes landed, despite the peer dying mid-burst.
    assert_clean("fresh", &rf, "500000");
}

// Case 3: FULL population turnover — one participant dies violently, the other
// exits normally, and for an interval nobody holds the segment. It persists with
// its value intact (ruling c1: persistence is semantic, not a side effect of an
// open handle).
#[test]
fn xproc_crash_persist_across_turnover() {
    require_valgrind();
    let tag = "crash_persist";
    let prefix = unique_prefix(tag);
    unlink_prefix(&prefix);
    let bv = bin_path(tag, "victim");
    let bw = bin_path(tag, "writer");
    let br = bin_path(tag, "reader");
    compile_fixture("tests/xproc/crash_persist/victim.hl", &prefix, &bv);
    compile_fixture("tests/xproc/crash_persist/writer.hl", &prefix, &bw);
    compile_fixture("tests/xproc/crash_persist/reader.hl", &prefix, &br);

    let vc = native_child(&bv);
    let rw = finish(valgrind_child(&bw), "writer"); // writes 42, exits normally
    kill_and_reap(vc, "victim"); // now: zero attachers
    let rr = finish(valgrind_child(&br), "reader"); // attaches to a segment nobody held

    unlink_prefix(&prefix);
    let _ = fs::remove_file(&bv);
    let _ = fs::remove_file(&bw);
    let _ = fs::remove_file(&br);
    assert_clean("writer", &rw, "wrote");
    assert_clean("reader", &rr, "42");
}

// The spec's Cross-Process Watchers example (docs/hilow-design.md), realized as
// two programs: the producer increments a named shared counter to 100; the
// watcher program observes the crossing and prints "Done!". Enforces that the
// spec example compiles and runs from this commit onward.
#[test]
fn xproc_spec_example_done() {
    require_valgrind();
    run_concurrent(
        "spec_example",
        "tests/xproc/spec_example/watcher.hl",
        "tests/xproc/spec_example/producer.hl",
        "Done!",
        "",
    );
}

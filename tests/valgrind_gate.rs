// Phase 1.5b: valgrind gate (docs/cell-migration-audit.md §4.4 item 10).
//
// Compiles every entry program under tests/programs/ and runs it under
// valgrind, failing on ANY valgrind error — including definite/indirect
// leaks. This catches what the binaries' internal alloc/free counter cannot:
// use-after-free, invalid reads/writes, double-free, and leaks of
// allocations the counter never saw.
//
// Requires valgrind to be installed; the gate fails loudly if it is not.
// There is deliberately no silent skip.
//
// Phase 6 maintenance (2026-07-23): the gate is split into two lanes so the
// common single-threaded corpus is not held hostage to a handful of slow
// threaded fixtures:
//   * valgrind_gate_single_threaded — every program that does NOT engage
//     threaded_mode (no `async`, no `shared`). The bulk of the corpus; runs
//     under plain valgrind, back near the pre-Phase-5 runtime.
//   * valgrind_gate_threaded — the programs that DO engage threaded_mode.
//     Run with --fair-sched=yes (without it, valgrind's serialized scheduler
//     starves a producer thread behind a busy-wait for tens of seconds; with
//     it, even the spin fixtures are sub-second). The spin-wait fixtures are
//     additionally compiled at a REDUCED iteration/threshold N (env
//     HILOW_GATE_SPIN_N, default GATE_SPIN_N_DEFAULT), injected by source
//     rewrite before compilation — see SPIN_FIXTURES.
// Routing is by source-token detection (is_threaded_source), so a newly added
// threaded fixture auto-routes to the threaded lane rather than silently
// reintroducing the slowdown in the fast lane.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Programs that are compile-rejection fixtures: the gate expects hilowc to
/// FAIL on them and reports a gate failure if one unexpectedly compiles
/// (keeps this list honest). Paths relative to tests/programs/.
const REJECTION_FIXTURES: &[&str] = &[
    "type_mismatch.hl",                              // test at integration_tests.rs (is_err)
    "bad_equals.hl",                                 // is_err test
    "bad_qualifier.hl",                              // is_err test
    "method_this_outside_method_error.hl",           // is_err test
    "match_non_exhaustive_expression_error.hl",      // is_err test
    "reject_money_mismatch.hl",                      // is_err test
    "reject_tuple_arity_mismatch.hl",                // is_err test
    "array/type_mismatch.hl",                        // is_err test
    "array_moved_alias_rejected_on_changed.hl",      // is_err test
    "wrong_context.hl",                              // rejection fixture; test commented out
    "stealth_return_rejected.hl",                    // is_err test test_stealth_return_rejected (wired in 1.5d, audit §4.4 item 9)
    "phase3/types2.hl",                              // verify_phase3.rs test_types2_fails_type_check
    "phase3/types3.hl",                              // verify_phase3.rs test_types3_fails_type_check
    "unknown_with_options.hl",                       // Phase 9b deferral: unknown constructor with options is UnsupportedFeature in codegen (generate_unknown_constructor)
    "optional_i64_rejected.hl",                      // is_err test test_optional_i64_rejected — Phase 2b step zero (audit §5 item 7): optional payload matrix lands in Phase 3
    "optional_bool_let_rejected.hl",                 // is_err test test_optional_bool_let_rejected — same adjudication
    "optional_return_mismatch_rejected.hl",          // is_err test test_optional_return_mismatch_rejected — narrow optional-return type check (Phase 2b step zero)
    "watcher_deep_scalar_rejected.hl",               // is_err test test_watcher_deep_scalar_rejected — (deep) is containers-only until scalars box (Phase 2d; objects joined in 2e)
    "object_watch_added_rejected.hl",                // is_err test test_object_watch_added_rejected — Phase 2e adjudication: ADDED has no reachable trigger until dynamic property addition lands (STATUS.md open question)
    "object_watch_removed_rejected.hl",              // is_err test test_object_watch_removed_rejected — no REMOVED event: property removal unimplemented (tombstone ruling)
    "watcher_destructured_binding_rejected.hl",       // is_err test test_watcher_destructured_binding_rejected — Phase 3b adjudication E: destructured bindings do not box yet
    "watcher_mixed_array_object_rejected.hl",        // is_err test test_watcher_mixed_array_object_rejected — body prologue casts to one container type (Phase 2e single-container-kind watchers)
    "watcher_mixed_assigned_content_rejected.hl",    // is_err test test_watcher_mixed_assigned_content_rejected — Phase 3e-α: slot-kind + value-kind subscriptions in one watcher hit the mixed-scalar-container gate
    "watcher_decl_name_return_rejected.hl",          // is_err test test_watcher_decl_name_return_rejected — Phase 3c adjudication A: decl-form watcher names are not first-class values
    "watcher_decl_name_alias_rejected.hl",           // is_err test test_watcher_decl_name_alias_rejected — same adjudication
    "modules/watcher_in_module/app.hl",              // is_err test test_module_level_watcher_rejected — Phase 3c: module initialization semantics unspecified (STATUS.md open question)
    "shared_container_rejected.hl",                  // is_err test test_shared_container_rejected — Phase 5c scope fence: `shared` is scalar-only (i32); shared containers rejected
    "shared_deep_rejected.hl",                       // is_err test test_shared_deep_rejected — Phase 5c scope fence: `(deep)` across `shared` (shared is scalar-only → deep-on-scalar, rejected by typecheck)
    "placed_container_rejected.hl",                  // is_err test test_placed_container_rejected — Phase 6a placeability fence: containers are pointer-bearing, not placeable
    "placed_watcher_rejected.hl",                    // is_err test test_placed_watcher_rejected — Phase 6a placeability fence: watchers are address-identity, not placeable
    "placed_bad_name_rejected.hl",                   // is_err test test_placed_bad_name_rejected — Phase 6a: invalid segment-name character
    "placed_long_name_rejected.hl",                  // is_err test test_placed_long_name_rejected — Phase 6a: segment name over the 64-char limit
];

/// Programs with a KNOWN, adjudicated memory bug: they compile and run, but
/// valgrind reports errors. The gate expects errors from them and FAILS if
/// one comes back clean — the fix must remove the entry (keeps this list
/// honest, mirroring REJECTION_FIXTURES).
///
/// Adding an entry requires a citation in its comment and a matching
/// STATUS.md record. (The object double-release class that originally
/// populated this list — 17 programs — was fixed in Phase 1.5c "object
/// ownership discipline". The §3.4(b)/(c)/(d) env-keying bugs were fixed in
/// Phase 2a: watcher values subscribe at construction and unsubscribe at
/// release. The last entry — §3.4(a), the .move 2-arg env-dropping casts —
/// was fixed in Phase 2c: all mutators fire through hl_cell_notify with the
/// one (env, cell, delta) body ABI. EMPTY as of Phase 2c.)
const KNOWN_MEMORY_BUGS: &[&str] = &[];

/// Spin-wait threaded fixtures whose busy-wait loop is pathologically slow
/// under valgrind's serialized scheduler. `--fair-sched=yes` (applied to the
/// whole threaded lane) is the dominant fix; on top of it, these fixtures are
/// compiled at a reduced iteration/threshold N to bound the instrumented work
/// further. The gate's pass/fail invariant is memory-cleanliness, which is
/// N-independent and order-insensitive by design, so reducing N does not
/// weaken the check. (The separate integration tests compile the PRISTINE .hl
/// at full N and verify actual output — that is where N matters.)
///
/// Each entry lists the exact literal substrings that carry the fixture's
/// default N and their `{N}` templates. `str::replace` rewrites ALL
/// occurrences (e.g. the three worker loops in shared_reactive_counter). If a
/// listed substring is NOT present in the source, the gate FAILS — this keeps
/// the table honest against fixture drift (mirrors REJECTION_FIXTURES).
/// Paths relative to tests/programs/.
const SPIN_FIXTURES: &[(&str, &[(&str, &str)])] = &[
    (
        "shared_reactive_counter.hl",
        &[("counter >= 20", "counter >= {N}"), ("counter < 20", "counter < {N}")],
    ),
    (
        "async_watch_threshold.hl",
        &[("counter >= 5", "counter >= {N}"), ("i < 5", "i < {N}")],
    ),
];

/// Reduced iteration/threshold applied to SPIN_FIXTURES under the gate,
/// overridable via env HILOW_GATE_SPIN_N. Any N >= 1 preserves the invariant.
const GATE_SPIN_N_DEFAULT: u32 = 4;

fn gate_spin_n() -> u32 {
    std::env::var("HILOW_GATE_SPIN_N")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(GATE_SPIN_N_DEFAULT)
}

/// A program engages threaded_mode (codegen `uses_async || uses_shared`) iff
/// its source uses `async` or `shared`. Detected from source so a new threaded
/// fixture routes to the threaded lane automatically. Line comments are
/// stripped first so a fixture that merely mentions the keyword in prose is
/// not misrouted (harmless if it were — it would just run in the slower lane).
fn is_threaded_source(path: &Path) -> bool {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    src.lines().any(|line| {
        let code = match line.split_once("//") {
            Some((before, _)) => before,
            None => line,
        };
        has_word(code, "async") || has_word(code, "shared")
    })
}

/// Whole-word (identifier-boundary) match, so `shared` matches but a
/// hypothetical `shared_thing` identifier does not.
fn has_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(word) {
        let i = start + pos;
        let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
        let after = i + word.len();
        let after_ok = after >= bytes.len() || !is_ident_byte(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        start = i + 1;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// If `program` is a spin fixture, materialize a reduced-N rewrite next to the
/// binary and return its path (the caller compiles that instead). Returns the
/// original path unchanged for non-spin programs. Errors if a listed
/// substitution substring is missing (fixture drift) or the temp write fails.
fn spin_rewrite_source(program: &Path, binary: &Path, n: u32) -> Result<PathBuf, String> {
    let rel = program
        .strip_prefix("tests/programs")
        .unwrap_or(program)
        .to_string_lossy()
        .to_string();
    let entry = SPIN_FIXTURES.iter().find(|(name, _)| rel == *name);
    let (_, subs) = match entry {
        Some(e) => e,
        None => return Ok(program.to_path_buf()),
    };
    let mut src = fs::read_to_string(program)
        .map_err(|e| format!("{}: could not read for spin rewrite: {}", program.display(), e))?;
    for (needle, template) in *subs {
        if !src.contains(needle) {
            return Err(format!(
                "{}: SPIN_FIXTURES substring {:?} not found — fixture drifted, update SPIN_FIXTURES",
                program.display(),
                needle
            ));
        }
        let replacement = template.replace("{N}", &n.to_string());
        src = src.replace(needle, &replacement);
    }
    let rewritten = binary.with_extension("spin.hl");
    fs::write(&rewritten, src)
        .map_err(|e| format!("{}: could not write spin rewrite: {}", program.display(), e))?;
    Ok(rewritten)
}

fn collect_entries(dir: &Path, entries: &mut Vec<PathBuf>) {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for item in fs::read_dir(dir).expect("read_dir failed") {
        let path = item.expect("dir entry failed").path();
        if path.is_dir() {
            dirs.push(path);
        } else if path.extension().map(|e| e == "hl").unwrap_or(false) {
            files.push(path);
        }
    }
    // Entry rule: a directory containing app.hl is a multi-file module
    // project — app.hl is the entry, sibling .hl files are import libraries.
    let has_app = files.iter().any(|f| f.file_name().map(|n| n == "app.hl").unwrap_or(false));
    for f in files {
        if !has_app || f.file_name().map(|n| n == "app.hl").unwrap_or(false) {
            entries.push(f);
        }
    }
    for d in dirs {
        collect_entries(&d, entries);
    }
}

fn valgrind_error_count(binary: &Path, extra_args: &[&str]) -> Result<u32, String> {
    let output = Command::new("valgrind")
        .arg("--leak-check=full")
        .arg("--errors-for-leak-kinds=definite,indirect")
        .args(extra_args)
        .arg(binary)
        .output()
        .map_err(|e| format!("failed to run valgrind: {}", e))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The program's own exit code is irrelevant here (two fixtures exit
    // nonzero by design); only valgrind's ERROR SUMMARY matters.
    for line in stderr.lines() {
        if let Some(idx) = line.find("ERROR SUMMARY: ") {
            let rest = &line[idx + "ERROR SUMMARY: ".len()..];
            let count_str = rest.split_whitespace().next().unwrap_or("");
            return count_str
                .parse::<u32>()
                .map_err(|_| format!("unparsable ERROR SUMMARY line: {}", line));
        }
    }
    Err(format!("no ERROR SUMMARY in valgrind output:\n{}", stderr))
}

/// Last ~15 relevant valgrind lines for a failure report.
fn valgrind_excerpt(binary: &Path, extra_args: &[&str]) -> String {
    let output = Command::new("valgrind")
        .arg("--leak-check=full")
        .arg("--errors-for-leak-kinds=definite,indirect")
        .args(extra_args)
        .arg(binary)
        .output();
    match output {
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            stderr
                .lines()
                .filter(|l| {
                    l.contains("definitely lost")
                        || l.contains("indirectly lost")
                        || l.contains("Invalid ")
                        || l.contains("ERROR SUMMARY")
                        || l.contains("at 0x")
                })
                .take(15)
                .collect::<Vec<_>>()
                .join("\n")
        }
        Err(e) => format!("(excerpt unavailable: {})", e),
    }
}

fn require_valgrind() {
    let version = Command::new("valgrind").arg("--version").output();
    assert!(
        version.map(|o| o.status.success()).unwrap_or(false),
        "valgrind is required for the memory-safety gate (Phase 1.5b). \
         Install it (e.g. apt install valgrind) — the gate does not skip."
    );
}

/// Run one lane of the gate over `entries`. `threaded` selects the threaded
/// behavior: --fair-sched=yes on every valgrind run (so a busy-wait producer
/// is not starved by valgrind's serialized scheduler) and reduced-N source
/// rewrites for SPIN_FIXTURES. Returns (clean-checked count, failures).
fn run_lane(entries: Vec<PathBuf>, threaded: bool) -> (usize, Vec<String>) {
    let rejections: Vec<PathBuf> = REJECTION_FIXTURES
        .iter()
        .map(|r| Path::new("tests/programs").join(r))
        .collect();
    let known_bugs: Vec<PathBuf> = KNOWN_MEMORY_BUGS
        .iter()
        .map(|r| Path::new("tests/programs").join(r))
        .collect();

    let extra_args: &[&str] = if threaded { &["--fair-sched=yes"] } else { &[] };
    let spin_n = gate_spin_n();

    let work: Mutex<Vec<PathBuf>> = Mutex::new(entries);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let checked_count: Mutex<usize> = Mutex::new(0);

    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8);

    std::thread::scope(|scope| {
        for _ in 0..n_threads {
            scope.spawn(|| loop {
                let program = match work.lock().unwrap().pop() {
                    Some(p) => p,
                    None => break,
                };
                let is_rejection = rejections.contains(&program);

                let binary = std::env::temp_dir().join(format!(
                    "hl_vg_{}_{}",
                    std::process::id(),
                    program
                        .strip_prefix("tests/programs")
                        .unwrap()
                        .display()
                        .to_string()
                        .replace('/', "_")
                        .replace(".hl", "")
                ));

                // Threaded lane: spin fixtures compile at reduced N via a
                // rewritten source next to the binary (non-spin programs and
                // the whole single-threaded lane compile the pristine source).
                let source = if threaded {
                    match spin_rewrite_source(&program, &binary, spin_n) {
                        Ok(p) => p,
                        Err(e) => {
                            failures.lock().unwrap().push(e);
                            continue;
                        }
                    }
                } else {
                    program.clone()
                };

                let compile = Command::new("./target/debug/hilowc")
                    .arg(&source)
                    .arg("-o")
                    .arg(&binary)
                    .output()
                    .expect("failed to invoke hilowc");
                if source != program {
                    let _ = fs::remove_file(&source);
                }

                if is_rejection {
                    if compile.status.success() {
                        failures.lock().unwrap().push(format!(
                            "{}: rejection fixture UNEXPECTEDLY COMPILES — update REJECTION_FIXTURES",
                            program.display()
                        ));
                        let _ = fs::remove_file(&binary);
                    }
                    continue;
                }

                if !compile.status.success() {
                    failures.lock().unwrap().push(format!(
                        "{}: failed to compile (not on the rejection skiplist):\n{}",
                        program.display(),
                        String::from_utf8_lossy(&compile.stderr)
                            .lines()
                            .take(5)
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                    continue;
                }

                let is_known_bug = known_bugs.contains(&program);
                match valgrind_error_count(&binary, extra_args) {
                    Ok(0) if is_known_bug => {
                        failures.lock().unwrap().push(format!(
                            "{}: UNEXPECTEDLY CLEAN — bug fixed? Remove it from KNOWN_MEMORY_BUGS",
                            program.display()
                        ));
                    }
                    Ok(0) => {
                        *checked_count.lock().unwrap() += 1;
                    }
                    Ok(_) if is_known_bug => {
                        // Documented Known Bug (see KNOWN_MEMORY_BUGS entry
                        // comments) — counted, not failed
                        *checked_count.lock().unwrap() += 1;
                    }
                    Ok(n) => {
                        let excerpt = valgrind_excerpt(&binary, extra_args);
                        failures.lock().unwrap().push(format!(
                            "{}: {} valgrind error(s)\n{}",
                            program.display(),
                            n,
                            excerpt
                        ));
                    }
                    Err(e) => {
                        failures.lock().unwrap().push(format!("{}: {}", program.display(), e));
                    }
                }
                let _ = fs::remove_file(&binary);
            });
        }
    });

    (
        checked_count.into_inner().unwrap(),
        failures.into_inner().unwrap(),
    )
}

/// Partition all discovered entry programs into (single-threaded, threaded)
/// by source-token detection.
fn partition_entries() -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = Vec::new();
    collect_entries(Path::new("tests/programs"), &mut entries);
    entries.sort();
    assert!(
        entries.len() > 200,
        "suspiciously few programs discovered ({}) — did tests/programs/ move?",
        entries.len()
    );
    entries.into_iter().partition(|p| !is_threaded_source(p))
}

#[test]
fn valgrind_gate_single_threaded() {
    require_valgrind();
    let (single, _threaded) = partition_entries();
    assert!(
        single.len() > 200,
        "suspiciously few single-threaded programs ({})",
        single.len()
    );
    let (checked, failures) = run_lane(single, false);
    assert!(
        failures.is_empty(),
        "valgrind gate (single-threaded lane): {} clean, {} FAILING:\n\n{}",
        checked,
        failures.len(),
        failures.join("\n\n")
    );
    // Belt-and-suspenders: the lane must have actually checked programs.
    assert!(checked > 200, "single-threaded lane checked only {} programs", checked);
}

#[test]
fn valgrind_gate_threaded() {
    require_valgrind();
    let (_single, threaded) = partition_entries();
    // The threaded set is small but must not silently vanish (that would hide
    // the async/shared fixtures from the memory gate entirely).
    assert!(
        threaded.len() >= 10,
        "threaded lane found only {} programs — expected the async/shared fixtures",
        threaded.len()
    );
    let run_count = threaded
        .iter()
        .filter(|p| {
            let rel = p.strip_prefix("tests/programs").unwrap().to_string_lossy().to_string();
            !REJECTION_FIXTURES.contains(&rel.as_str())
        })
        .count();
    let (checked, failures) = run_lane(threaded, true);
    assert!(
        failures.is_empty(),
        "valgrind gate (threaded lane): {} clean, {} FAILING:\n\n{}",
        checked,
        failures.len(),
        failures.join("\n\n")
    );
    // Every non-rejection threaded fixture must have run clean under valgrind.
    assert_eq!(
        checked, run_count,
        "threaded lane ran {} clean but expected {} runnable fixtures",
        checked, run_count
    );
}

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
    "watcher/expression_return_rejected/main.hl",    // is_err test
    "watcher/escape_function_local_rejected/main.hl", // is_err test
    "array/type_mismatch.hl",                        // is_err test
    "array_moved_alias_rejected_on_changed.hl",      // is_err test
    "wrong_context.hl",                              // rejection fixture; test commented out
    "stealth_return_rejected.hl",                    // is_err test test_stealth_return_rejected (wired in 1.5d, audit §4.4 item 9)
    "string_watcher_rejected.hl",                    // is_err test test_string_watcher_rejected — adjudicated audit §5 item 2: compile-time diagnostic until strings inherit cell semantics (Phase 2)
    "weak_after_death_unknown.hl",                   // EXPECTED-BEHAVIOR pin for Phase 1.5e (dead-weak read → unknown "weak referent released"); does not compile today (.reason on object-typed value). When 1.5e lands, this compiles — remove the entry and un-ignore its test
    "phase3/types2.hl",                              // verify_phase3.rs test_types2_fails_type_check
    "phase3/types3.hl",                              // verify_phase3.rs test_types3_fails_type_check
    "unknown_with_options.hl",                       // Phase 9b deferral: unknown constructor with options is UnsupportedFeature in codegen (generate_unknown_constructor)
];

/// Programs with a KNOWN, adjudicated memory bug: they compile and run, but
/// valgrind reports errors. The gate expects errors from them and FAILS if
/// one comes back clean — the fix must remove the entry (keeps this list
/// honest, mirroring REJECTION_FIXTURES).
///
/// Adding an entry requires a citation in its comment and a matching
/// STATUS.md record. (The object double-release class that originally
/// populated this list — 17 programs — was fixed in Phase 1.5c "object
/// ownership discipline".)
///
/// Current entries are the four §3.4 env-keying latent bugs, adjudicated
/// (audit §5 item 4, 2026-07-14) to stay broken until Phase 2 replaces the
/// env machinery — each has a matching #[ignore]d expected-behavior test in
/// integration_tests.rs (Phase 1.5d) and a STATUS.md Known issues record.
const KNOWN_MEMORY_BUGS: &[&str] = &[
    "watcher_move_capture_env.hl",               // audit §3.4(a): .move fires bodies with a 2-arg cast dropping the env → segfault with captures; fixed in Phase 2c (one firing ABI)
    "watcher_multi_array_capture_unregister.hl", // audit §3.4(b): one env on two arrays unregisters from only the last → UAF when the other fires after scope death; fixed in Phase 2b (watcher-owned envs)
    "watcher_null_key_scope_death.hl",           // audit §3.4(c): no-capture registrations keyed "NULL", never unregistered → invalid reads + firing after scope death; fixed in Phase 2b
    "watcher_temp_env_statement.hl",             // audit §3.4(d) territory: statement-temporary watcher expr (call argument) never registers its subscriptions and leaks its allocation; fixed in Phase 2b
];

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

fn valgrind_error_count(binary: &Path) -> Result<u32, String> {
    let output = Command::new("valgrind")
        .arg("--leak-check=full")
        .arg("--errors-for-leak-kinds=definite,indirect")
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
fn valgrind_excerpt(binary: &Path) -> String {
    let output = Command::new("valgrind")
        .arg("--leak-check=full")
        .arg("--errors-for-leak-kinds=definite,indirect")
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

#[test]
fn valgrind_gate() {
    // Preflight: valgrind must exist. No silent skip.
    let version = Command::new("valgrind").arg("--version").output();
    assert!(
        version.map(|o| o.status.success()).unwrap_or(false),
        "valgrind is required for the memory-safety gate (Phase 1.5b). \
         Install it (e.g. apt install valgrind) — the gate does not skip."
    );

    let mut entries: Vec<PathBuf> = Vec::new();
    collect_entries(Path::new("tests/programs"), &mut entries);
    entries.sort();
    assert!(
        entries.len() > 200,
        "suspiciously few programs discovered ({}) — did tests/programs/ move?",
        entries.len()
    );

    let rejections: Vec<PathBuf> = REJECTION_FIXTURES
        .iter()
        .map(|r| Path::new("tests/programs").join(r))
        .collect();
    let known_bugs: Vec<PathBuf> = KNOWN_MEMORY_BUGS
        .iter()
        .map(|r| Path::new("tests/programs").join(r))
        .collect();

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

                let compile = Command::new("./target/debug/hilowc")
                    .arg(&program)
                    .arg("-o")
                    .arg(&binary)
                    .output()
                    .expect("failed to invoke hilowc");

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
                match valgrind_error_count(&binary) {
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
                        let excerpt = valgrind_excerpt(&binary);
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

    let failures = failures.into_inner().unwrap();
    let checked = checked_count.into_inner().unwrap();
    assert!(
        failures.is_empty(),
        "valgrind gate: {} clean, {} FAILING:\n\n{}",
        checked,
        failures.len(),
        failures.join("\n\n")
    );
    // Belt-and-suspenders: the gate must have actually checked programs.
    assert!(checked > 200, "gate ran but checked only {} programs", checked);
}

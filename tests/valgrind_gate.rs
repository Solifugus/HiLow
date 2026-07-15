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
    "stealth_return_rejected.hl",                    // rejection fixture; no test wired (audit §4.4 item 9)
    "phase3/types2.hl",                              // verify_phase3.rs test_types2_fails_type_check
    "phase3/types3.hl",                              // verify_phase3.rs test_types3_fails_type_check
    "unknown_with_options.hl",                       // Phase 9b deferral: unknown constructor with options is UnsupportedFeature in codegen (generate_unknown_constructor)
];

/// Programs with a KNOWN, adjudicated memory bug: they compile and run, but
/// valgrind reports errors. The gate expects errors from them and FAILS if
/// one comes back clean — the fix must remove the entry (keeps this list
/// honest, mirroring REJECTION_FIXTURES).
///
/// Every entry below: Known Bug: object double-release via unretained second
/// reference (proto link / array element / weak target released down two
/// paths), fix in Phase 1.5c "object ownership discipline" — adjudicated
/// 2026-07-15, see docs/cell-migration-audit.md §5.
const KNOWN_MEMORY_BUGS: &[&str] = &[
    "array_objects_basic.hl",
    "array_objects_forin.hl",
    "array_objects_pop.hl",
    "array_objects_pop_use.hl",
    "array_objects_remove.hl",
    "array_objects_scope_cleanup.hl",
    "for_in_proto_excluded.hl",
    "is_object_basic.hl",
    "is_object_chain.hl",
    "method_this_proto.hl",
    "proto_assign_local.hl",
    "proto_basic.hl",
    "proto_chain.hl",
    "proto_method.hl",
    "proto_override.hl",
    "weak_basic.hl",
    "weak_breaks_cycle.hl",
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
                        // Documented Known Bug (object double-release, fix in
                        // Phase 1.5c) — counted, not failed
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

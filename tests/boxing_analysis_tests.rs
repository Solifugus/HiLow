// Phase 3a: boxing analysis tests.
//
// Every test asserts concrete boxing decisions on a concrete program,
// derived from the invariant (see src/typecheck/boxing.rs): a declaration
// boxes iff a subscription resolves to it, or a watcher body references it
// across the watcher boundary. Programs must typecheck — the analysis is
// only defined for valid programs.

use hilowc::parser::Parser;
use hilowc::typecheck::boxing::{self, BoxReason, BoxingAnalysis};
use hilowc::typecheck::TypeChecker;

fn analyze_program(input: &str) -> BoxingAnalysis {
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    let top_level = result.unwrap();

    let mut type_checker = TypeChecker::new();
    let checked = type_checker.check(&top_level);
    assert!(checked.is_ok(), "Type check failed: {:?}", checked);

    boxing::analyze(&top_level)
}

// --- correctly unboxed ---------------------------------------------------

#[test]
fn test_plain_scalars_unboxed() {
    let a = analyze_program(
        "high program(): i32 {
            let x = 1
            let y = x + 2
            print(y)
            return 0
        }",
    );
    assert_eq!(a.decisions_for("x").len(), 1);
    assert!(!a.decisions_for("x")[0].boxed, "unwatched x must stay a raw local");
    assert!(!a.decisions_for("y")[0].boxed, "unwatched y must stay a raw local");
    assert_eq!(a.boxed_count(), 0, "nothing in this program subscribes or captures");
}

#[test]
fn test_closure_capture_does_not_box() {
    // Function-expression closures are NOT watchers; their captures stay raw.
    let a = analyze_program(
        "high program(): i32 {
            function makeCounter(): function(): i32 {
                let count = 0
                return function(): i32 {
                    count += 1
                    return count
                }
            }
            let c = makeCounter()
            print(c())
            return 0
        }",
    );
    assert_eq!(a.decisions_for("count").len(), 1);
    assert!(
        !a.decisions_for("count")[0].boxed,
        "closure-captured scalar is not watcher-captured — must not box"
    );
    assert_eq!(a.boxed_count(), 0);
}

// --- correctly boxed: subscriptions --------------------------------------

#[test]
fn test_decl_form_subscription_boxes() {
    let a = analyze_program(
        "high program(): i32 {
            let counter: i32 = 0
            watcher onCounter((changed)counter) {
                print(counter)
            }
            counter = 5
            return 0
        }",
    );
    let counters = a.decisions_for("counter");
    // [0] the outer declaration, [1] the watcher-body subscription binding.
    assert_eq!(counters.len(), 2);
    assert!(counters[0].boxed, "subscribed declaration must box");
    assert_eq!(counters[0].reason, Some(BoxReason::Subscribed));
    assert!(!counters[1].boxed, "the body's snapshot binding is not the boxed cell");
    assert_eq!(a.boxed_count(), 1);
}

#[test]
fn test_expression_form_subscription_boxes() {
    let a = analyze_program(
        "high program(): i32 {
            let x = 1
            let w = watcher((changed)x) { print(\"changed\") }
            x = 2
            return 0
        }",
    );
    let xs = a.decisions_for("x");
    assert!(xs[0].boxed, "subscribed declaration must box");
    assert_eq!(xs[0].reason, Some(BoxReason::Subscribed));
    assert!(!a.decisions_for("w")[0].boxed, "the watcher binding itself is not boxed");
    assert_eq!(a.boxed_count(), 1);
}

// --- correctly boxed: watcher captures -----------------------------------

#[test]
fn test_watcher_capture_read_boxes() {
    let a = analyze_program(
        "high program(): i32 {
            let x = 1
            let z = 5
            let w = watcher((changed)x) { print(z) }
            x = 2
            return 0
        }",
    );
    assert!(a.decisions_for("x")[0].boxed);
    assert_eq!(a.decisions_for("x")[0].reason, Some(BoxReason::Subscribed));
    let zs = a.decisions_for("z");
    assert!(zs[0].boxed, "scalar read from a watcher body must box");
    assert_eq!(zs[0].reason, Some(BoxReason::WatcherCaptured));
    assert_eq!(a.boxed_count(), 2);
}

#[test]
fn test_watcher_capture_write_boxes() {
    // An assignment TARGET inside a watcher body is a reference too.
    let a = analyze_program(
        "high program(): i32 {
            let x = 1
            let hits = 0
            let w = watcher((changed)x) { hits = hits + 1 }
            x = 2
            print(hits)
            return 0
        }",
    );
    let hits = a.decisions_for("hits");
    assert!(hits[0].boxed, "scalar written from a watcher body must box");
    assert_eq!(hits[0].reason, Some(BoxReason::WatcherCaptured));
}

#[test]
fn test_decl_form_capture_boxes() {
    // Decl-form watcher names are first-class Watcher-typed variables today,
    // so scope-boundness is not provable — captures box (uncertain => box).
    let a = analyze_program(
        "high program(): i32 {
            let counter = 0
            let extra = 7
            watcher onCounter((changed)counter) {
                print(extra)
            }
            counter = 1
            return 0
        }",
    );
    assert!(a.decisions_for("extra")[0].boxed);
    assert_eq!(a.decisions_for("extra")[0].reason, Some(BoxReason::WatcherCaptured));
}

// --- the escape case -----------------------------------------------------

#[test]
fn test_escape_via_factory_boxes_capture() {
    // The 2b laundering shape: a watcher capturing a function local escapes
    // through a helper's return value. This compiles today (direct returns
    // of capture-unsafe bindings are rejected; laundering is the documented
    // residual hole). The conservative criterion boxes the local regardless
    // of whether escape is provable — this is exactly the soundness case.
    let a = analyze_program(
        "high program(): i32 {
            function launder(w: watcher): watcher {
                return w
            }
            function attach(arr: [i32]): watcher {
                let local = 3
                return launder(watcher((added)arr) { print(local) })
            }
            let xs = []: [i32]
            let w = attach(xs)
            xs.push(1)
            return 0
        }",
    );
    let locals = a.decisions_for("local");
    assert_eq!(locals.len(), 1);
    assert!(locals[0].boxed, "escaping watcher's capture must box");
    assert_eq!(locals[0].reason, Some(BoxReason::WatcherCaptured));
}

// --- the shadowing cases -------------------------------------------------

#[test]
fn test_shadowing_inner_subscribed_outer_unboxed() {
    let a = analyze_program(
        "high program(): i32 {
            let x = 1
            if (x ?= 1) {
                let x = 2
                let w = watcher((changed)x) { print(\"inner\") }
                x = 3
            }
            print(x)
            return 0
        }",
    );
    let xs = a.decisions_for("x");
    // [0] outer let, [1] inner let, [2] the watcher-body subscription binding.
    assert_eq!(xs.len(), 3);
    assert!(!xs[0].boxed, "outer x is never subscribed — must stay raw");
    assert!(xs[1].boxed, "inner (shadowing) x is the subscribed declaration");
    assert_eq!(xs[1].reason, Some(BoxReason::Subscribed));
    assert!(!xs[2].boxed);
    assert_eq!(a.boxed_count(), 1);
}

#[test]
fn test_body_local_shadow_does_not_capture_outer() {
    // The non-inheritance pin for the 2e finding: the legacy codegen scan
    // (find_variable_in_outer_scope) would record outer `y` as a capture
    // here because it ignores the body-local shadowing declaration. This
    // analysis must not.
    let a = analyze_program(
        "high program(): i32 {
            let y = 7
            let x = 1
            let w = watcher((changed)x) {
                let y = 1
                print(y)
            }
            x = 2
            print(y)
            return 0
        }",
    );
    let ys = a.decisions_for("y");
    // [0] outer y, [1] the body-local y.
    assert_eq!(ys.len(), 2);
    assert!(!ys[0].boxed, "body-local shadow means the outer y is never referenced from the body");
    assert!(!ys[1].boxed, "the body-local y is neither subscribed nor captured");
    assert_eq!(a.boxed_count(), 1, "only the subscribed x boxes");
}

#[test]
fn test_subscribed_name_use_in_body_is_not_a_phantom_capture() {
    // Body uses of the subscribed name resolve to the subscription binding
    // (the snapshot parameter), not to the outer declaration — the outer
    // declaration boxes via the subscription, with Subscribed as its reason,
    // and no WatcherCaptured marking occurs. (The legacy scan records this
    // very name as a capture — the 2e finding.)
    let a = analyze_program(
        "high program(): i32 {
            let x = 1
            let w = watcher((changed)x) { print(x) }
            x = 2
            return 0
        }",
    );
    let xs = a.decisions_for("x");
    assert_eq!(xs.len(), 2);
    assert!(xs[0].boxed);
    assert_eq!(
        xs[0].reason,
        Some(BoxReason::Subscribed),
        "reason must be the subscription, not a phantom capture of the body use"
    );
    assert!(!xs[1].boxed, "the subscription binding itself is body-local");
    assert_eq!(a.boxed_count(), 1);
}

// --- container subsumption -----------------------------------------------

#[test]
fn test_subscribed_container_is_marked_but_subsumed() {
    // The analysis is type-agnostic: a subscribed array is marked like any
    // declaration. For containers the mark is a documented no-op — arrays
    // and objects already ARE cells (Phases 2a/2e); 3b applies the
    // attribute to scalars only.
    let a = analyze_program(
        "high program(): i32 {
            let xs = [1, 2]
            let w = watcher((added)xs) { print(\"added\") }
            xs.push(3)
            return 0
        }",
    );
    let arrays = a.decisions_for("xs");
    assert!(arrays[0].boxed, "the fact 'xs is subscribed' is recorded uniformly");
    assert_eq!(arrays[0].reason, Some(BoxReason::Subscribed));
}

#[test]
fn test_alias_binding_not_boxed() {
    let a = analyze_program(
        "high program(): i32 {
            let xs = [1, 2]
            let w = watcher((n=added)xs) { print(n) }
            xs.push(3)
            return 0
        }",
    );
    let aliases = a.decisions_for("n");
    assert_eq!(aliases.len(), 1);
    assert!(!aliases[0].boxed, "the alias is a body-local delta binding, never a cell");
}

// --- nested boundary crossing --------------------------------------------

#[test]
fn test_reference_from_nested_watcher_boxes_outer_local() {
    // A reference inside an inner watcher that resolves outside BOTH
    // watchers still crosses the (innermost) boundary and boxes its target.
    let a = analyze_program(
        "high program(): i32 {
            let flag = 0
            let x = 1
            let y = 2
            let w = watcher((changed)x) {
                let inner = watcher((changed)y) { print(flag) }
            }
            x = 5
            return 0
        }",
    );
    let flags = a.decisions_for("flag");
    assert!(flags[0].boxed, "reference from the nested watcher crosses out — must box");
    assert_eq!(flags[0].reason, Some(BoxReason::WatcherCaptured));
    assert!(a.decisions_for("x")[0].boxed);
    assert!(a.decisions_for("y")[0].boxed);
}

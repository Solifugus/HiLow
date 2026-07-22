//! Phase 3a: boxing analysis.
//!
//! Compile-time pass computing which variable declarations must box into
//! cells when scalar lowering lands (Phase 3b). The criterion, stated as an
//! invariant:
//!
//! > A variable declaration D is marked boxed iff (a) some subscription —
//! > decl-form or expression-form — resolves to D, or (b) some watcher body
//! > contains a reference (read or write) that resolves to D from outside
//! > that watcher's own scope.
//!
//! Conservative-correct direction: failing to box is a soundness bug; boxing
//! unnecessarily is only a performance bug — when uncertain, box. (b) covers
//! the escape-soundness requirement (audit §5 item 1) with no dataflow
//! analysis: expression-form watcher values are first-class and can escape
//! (including through the known 2b laundering hole), and decl-form watcher
//! names are also declared as first-class Watcher-typed variables today, so
//! scope-boundness is not provable for either form — captures from both
//! forms box. Function-expression closures are NOT watchers and do not box
//! their captures — unless the closure itself sits inside a watcher body, in
//! which case its outer references still cross the watcher boundary and box.
//!
//! The analysis is type-agnostic: it marks DECLARATIONS, not types. For
//! containers (arrays, objects, strings) the mark is subsumed — they already
//! ARE cells — and Phase 3b applies the attribute to scalars only.
//!
//! This walker deliberately does NOT read `WatcherExpr.captures`: that list
//! is produced by the legacy scan (`find_variable_in_outer_scope`), which
//! ignores shadowing and therefore records subscribed names as captures (the
//! Phase 2e finding). The resolver here maintains its own scope stack in
//! which subscription bindings, aliases, params, and body-local lets shadow
//! outer names correctly — pinned by the shadowing tests in
//! tests/boxing_analysis_tests.rs.

use crate::ast::*;
use crate::lexer::Position;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxReason {
    /// A subscription (decl-form or expression-form) targets this declaration.
    Subscribed,
    /// A watcher body references this declaration across the watcher boundary.
    WatcherCaptured,
    /// Phase 5c: a `shared let` — always a cell (atomic + cross-context watchable).
    Shared,
}

#[derive(Debug, Clone)]
pub struct BoxDecision {
    pub name: String,
    pub position: Position,
    pub boxed: bool,
    pub reason: Option<BoxReason>,
    /// Phase 3e-α: a subscription REQUIRES the variable be lowered to a
    /// slot cell (HiLowScalar): any `(assigned)` subscription (either form —
    /// (assigned) is inherently about the variable), or any subscription on
    /// a string-typed variable (strings have no value cell to subscribe).
    /// Expression-form content subscriptions on containers deliberately do
    /// NOT set this (value-identity subscription, audit §5 item 10b).
    pub slot_required: bool,
}

/// Result of the analysis: one decision per variable declaration, in
/// encounter (declaration) order. Phase 3b addition: per-watcher capture
/// lists, keyed by the watcher's source position — shadow-correct (unlike the
/// legacy `WatcherExpr.captures` scan), in first-reference order, covering
/// EVERY enclosing watcher a reference crosses (a nested watcher's outer
/// reference appears in its own list and in each enclosing watcher's list,
/// since each env must be able to pack the inner one).
#[derive(Debug, Default)]
pub struct BoxingAnalysis {
    decisions: Vec<BoxDecision>,
    watcher_captures: HashMap<(usize, usize), Vec<String>>,
}

impl BoxingAnalysis {
    /// Query by declaration identity (name + declaration position).
    pub fn is_boxed(&self, name: &str, decl_pos: &Position) -> bool {
        self.decisions
            .iter()
            .any(|d| d.boxed && d.name == name && d.position == *decl_pos)
    }

    /// All decisions for declarations of `name`, in declaration order —
    /// shadowing tests assert by index instead of hardcoding positions.
    pub fn decisions_for(&self, name: &str) -> Vec<&BoxDecision> {
        self.decisions.iter().filter(|d| d.name == name).collect()
    }

    pub fn boxed_count(&self) -> usize {
        self.decisions.iter().filter(|d| d.boxed).count()
    }

    pub fn decisions(&self) -> &[BoxDecision] {
        &self.decisions
    }

    /// Phase 3e-α: does this declaration need a SLOT cell (see
    /// BoxDecision::slot_required)? Query by declaration identity.
    pub fn needs_slot(&self, name: &str, decl_pos: &Position) -> bool {
        self.decisions
            .iter()
            .any(|d| d.slot_required && d.name == name && d.position == *decl_pos)
    }

    /// Names a watcher's body references from outside the watcher's own
    /// scope (its env captures), in first-reference order. Keyed by the
    /// watcher's position (decl-form `Watcher.position` or
    /// `WatcherExpr.position`). Subscribed names are NOT captures — body uses
    /// of them resolve to the subscription binding inside the boundary.
    pub fn captures_for(&self, pos: &Position) -> &[String] {
        self.watcher_captures
            .get(&(pos.line, pos.column))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// Run the analysis over a type-checked program. The walker resolves names
/// itself, so it has no dependency on the checker's transient scope state;
/// callers run it after `TypeChecker::check` succeeds (the analysis is
/// meaningless for programs that don't typecheck).
pub fn analyze(top_level: &TopLevel) -> BoxingAnalysis {
    let mut a = Analyzer {
        result: BoxingAnalysis::default(),
        scopes: Vec::new(),
        watcher_boundaries: Vec::new(),
        watcher_keys: Vec::new(),
    };
    a.push_scope();
    match top_level {
        TopLevel::Program(p) => a.walk_program(p),
        TopLevel::Module(m) => a.walk_module(m),
    }
    a.pop_scope();
    a.result
}

struct Analyzer {
    result: BoxingAnalysis,
    /// name -> decision index, innermost scope last. Keyed lookups only —
    /// no iteration, so map order can't leak into results.
    scopes: Vec<HashMap<String, usize>>,
    /// Scope-stack index of each enclosing watcher's own scope. A reference
    /// resolving to a scope BELOW the innermost boundary crosses out of that
    /// watcher and boxes its target. (Resolutions inside the innermost
    /// watcher are also inside every outer one, so one check suffices for
    /// marking; the capture lists check every boundary.)
    watcher_boundaries: Vec<usize>,
    /// Position key of each enclosing watcher, parallel to
    /// `watcher_boundaries` — capture-list recording (Phase 3b).
    watcher_keys: Vec<(usize, usize)>,
}

impl Analyzer {
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, position: &Position) {
        let idx = self.result.decisions.len();
        self.result.decisions.push(BoxDecision {
            name: name.to_string(),
            position: position.clone(),
            boxed: false,
            reason: None,
            slot_required: false,
        });
        self.scopes
            .last_mut()
            .expect("analyzer scope stack is never empty")
            .insert(name.to_string(), idx);
    }

    /// Innermost declaration of `name`, with the scope index it lives in.
    fn resolve(&self, name: &str) -> Option<(usize, usize)> {
        for (scope_idx, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(&decl_idx) = scope.get(name) {
                return Some((decl_idx, scope_idx));
            }
        }
        None
    }

    fn mark(&mut self, decl_idx: usize, reason: BoxReason) {
        let d = &mut self.result.decisions[decl_idx];
        d.boxed = true;
        if d.reason.is_none() {
            d.reason = Some(reason);
        }
    }

    /// A subscription targets the declaration visible in the scope OUTSIDE
    /// the watcher (call before pushing the watcher scope).
    fn mark_subscription(&mut self, sub: &Subscription, decl_form: bool) {
        if let Some((decl_idx, _)) = self.resolve(&sub.variable_name) {
            self.mark(decl_idx, BoxReason::Subscribed);
            // Phase 3e-α: does this subscription require a SLOT cell?
            // (assigned) always does — it watches the variable, not the
            // value. Any subscription on a string does — strings have no
            // subscribable value cell. Phase 3e-β: any DECL-FORM subscription
            // on a container does — the watcher follows rebinding through the
            // slot (content modifiers need the FOLLOW node; (changed) is a
            // slot subscription — audit §5 item 10b). Expression-form
            // container content subscriptions stay excluded (value
            // subscriptions by identity). The analysis runs post-typecheck,
            // so the resolved type refcell is populated.
            let is_string = matches!(
                *sub.resolved_var_type.borrow(),
                Some(Type::Primitive(PrimitiveType::String))
            );
            let is_container = matches!(
                *sub.resolved_var_type.borrow(),
                Some(Type::DynamicArray(_)) | Some(Type::Object(_))
            );
            if matches!(sub.modifier, SubscriptionModifier::Assigned)
                || is_string
                || (decl_form && is_container)
            {
                self.result.decisions[decl_idx].slot_required = true;
            }
        }
        // Unresolved: typecheck already errored; nothing to mark.
    }

    /// An identifier use. Boxes its declaration iff the reference crosses
    /// the innermost enclosing watcher boundary, and records the name in the
    /// capture list of EVERY enclosing watcher it crosses (each env packs
    /// the next one in — Phase 3b).
    fn reference(&mut self, name: &str) {
        if let Some((decl_idx, scope_idx)) = self.resolve(name) {
            if let Some(&boundary) = self.watcher_boundaries.last() {
                if scope_idx < boundary {
                    self.mark(decl_idx, BoxReason::WatcherCaptured);
                }
            }
            for (i, &boundary) in self.watcher_boundaries.iter().enumerate() {
                if scope_idx < boundary {
                    let key = self.watcher_keys[i];
                    let list = self.result.watcher_captures.entry(key).or_default();
                    if !list.iter().any(|n| n == name) {
                        list.push(name.to_string());
                    }
                }
            }
        }
    }

    // --- top-level -------------------------------------------------------

    fn walk_program(&mut self, p: &Program) {
        for param in &p.params {
            self.declare(&param.name, &param.position);
        }
        if let Some(body) = &p.body {
            self.walk_items(&body.items);
        }
    }

    fn walk_module(&mut self, m: &Module) {
        for l in &m.lets {
            self.walk_let(l);
        }
        for f in &m.items {
            self.walk_function(f);
        }
        for w in &m.watchers {
            self.walk_decl_watcher(w);
        }
    }

    // --- declarations and blocks ----------------------------------------

    fn walk_items(&mut self, items: &[BlockItem]) {
        for item in items {
            match item {
                BlockItem::Statement(s) => self.walk_statement(s),
                BlockItem::Function(f) => self.walk_function(f),
                BlockItem::Watcher(w) => self.walk_decl_watcher(w),
            }
        }
    }

    fn walk_block(&mut self, block: &Block) {
        self.push_scope();
        self.walk_items(&block.items);
        self.pop_scope();
    }

    fn walk_let(&mut self, l: &LetDecl) {
        // Initializer first: it sees the OUTER binding of a shadowed name.
        if let Some(init) = &l.initializer {
            self.walk_expression(init);
        }
        match &l.pattern {
            LetPattern::Identifier(name, _) => {
                self.declare(name, &l.position);
                // Phase 5c: a shared scalar is a cell (atomic + cross-context
                // watchable), so it boxes whether or not it is watched.
                if l.is_shared {
                    let idx = self.result.decisions.len() - 1;
                    self.mark(idx, BoxReason::Shared);
                }
            }
            LetPattern::Tuple(names) => {
                for name in names {
                    self.declare(name, &l.position);
                }
            }
        }
    }

    fn walk_function(&mut self, f: &Function) {
        self.push_scope();
        for param in &f.params {
            self.declare(&param.name, &param.position);
        }
        if let Some(body) = &f.body {
            self.walk_items(&body.items);
        }
        self.pop_scope();
    }

    /// Decl-form watcher: subscriptions resolve outside; the name itself is
    /// a Watcher-typed variable in the enclosing scope; body references
    /// crossing the boundary box (scope-boundness is not provable — the
    /// name is first-class today; narrow in 3c if its lifecycle pins it).
    fn walk_decl_watcher(&mut self, w: &Watcher) {
        self.declare(&w.name, &w.position);
        for sub in &w.subscriptions {
            self.mark_subscription(sub, true);
        }
        self.push_scope();
        self.watcher_boundaries.push(self.scopes.len() - 1);
        self.watcher_keys.push((w.position.line, w.position.column));
        for sub in &w.subscriptions {
            self.declare(&sub.variable_name, &sub.position);
            if let Some(alias) = &sub.alias {
                self.declare(alias, &sub.position);
            }
        }
        self.walk_items(&w.body.items);
        self.watcher_keys.pop();
        self.watcher_boundaries.pop();
        self.pop_scope();
    }

    /// Phase 5b: an `async` block is a capture boundary like a watcher — any
    /// outer variable it references (read OR write) must box so the spawned
    /// thread reaches it as a heap cell (a program-scope static), atomically
    /// refcounted. Reuses the watcher-boundary machinery; the recorded reason
    /// is `WatcherCaptured` (async has no separate env struct — the body
    /// references program-scope statics directly, so the capture list under
    /// this key is unused). This boundary only exists in `async`-containing
    /// (threaded) programs, so the single-threaded corpus is untouched.
    fn walk_async(&mut self, body: &Block, pos: &Position) {
        self.push_scope();
        self.watcher_boundaries.push(self.scopes.len() - 1);
        self.watcher_keys.push((pos.line, pos.column));
        self.walk_items(&body.items);
        self.watcher_keys.pop();
        self.watcher_boundaries.pop();
        self.pop_scope();
    }

    fn walk_watcher_expr(&mut self, w: &WatcherExpr) {
        for sub in &w.subscriptions {
            self.mark_subscription(sub, false);
        }
        self.push_scope();
        self.watcher_boundaries.push(self.scopes.len() - 1);
        self.watcher_keys.push((w.position.line, w.position.column));
        for sub in &w.subscriptions {
            self.declare(&sub.variable_name, &sub.position);
            if let Some(alias) = &sub.alias {
                self.declare(alias, &sub.position);
            }
        }
        self.walk_items(&w.body.items);
        self.watcher_keys.pop();
        self.watcher_boundaries.pop();
        self.pop_scope();
    }

    // --- statements ------------------------------------------------------

    fn walk_statement(&mut self, s: &Statement) {
        match s {
            Statement::Let(l) => self.walk_let(l),
            Statement::Return(r) => {
                if let Some(v) = &r.value {
                    self.walk_expression(v);
                }
            }
            Statement::If(i) => {
                self.walk_expression(&i.condition);
                self.walk_block(&i.then_block);
                if let Some(e) = &i.else_block {
                    self.walk_block(e);
                }
            }
            Statement::While(w) => {
                self.walk_expression(&w.condition);
                self.walk_block(&w.body);
            }
            Statement::Loop(l) => self.walk_block(&l.body),
            Statement::ForIn(f) => {
                self.walk_expression(&f.iterable);
                self.push_scope();
                self.declare(&f.key_name, &f.position);
                self.declare(&f.value_name, &f.position);
                self.walk_items(&f.body.items);
                self.pop_scope();
            }
            Statement::Switch(sw) => {
                self.walk_expression(&sw.value);
                for case in &sw.cases {
                    self.push_scope();
                    for st in &case.body {
                        self.walk_statement(st);
                    }
                    self.pop_scope();
                }
                if let Some(default) = &sw.default {
                    self.push_scope();
                    for st in default {
                        self.walk_statement(st);
                    }
                    self.pop_scope();
                }
            }
            Statement::Break(_) | Statement::Continue(_) => {}
            Statement::Assign(a) => {
                // Assignment targets count as references: a watcher body
                // writing an outer variable needs it boxed just as a read does.
                self.walk_expression(&a.target);
                self.walk_expression(&a.value);
            }
            Statement::QualifiedOp(q) => {
                self.walk_expression(&q.lhs);
                self.walk_expression(&q.rhs);
                for spec in &q.qualifiers {
                    if let Some(arg) = &spec.arg {
                        self.walk_expression(arg);
                    }
                }
            }
            Statement::StealthBlock(b, _) => self.walk_block(b),
            Statement::Async(b, pos) => self.walk_async(b, pos),
            Statement::ExprStatement(e) => self.walk_expression(e),
        }
    }

    // --- expressions -----------------------------------------------------

    fn walk_expression(&mut self, e: &Expression) {
        match e {
            Expression::Ident { name, .. } => self.reference(name),
            Expression::This(_) => {}
            Expression::IntLit(..)
            | Expression::FloatLit(..)
            | Expression::StringLit(..)
            | Expression::DurationLit(..)
            | Expression::MoneyLit(..)
            | Expression::BoolLit(..)
            | Expression::Nothing(_) => {}
            Expression::FString(f) => {
                for part in &f.parts {
                    if let FStringPart::Expression(expr, _) = part {
                        self.walk_expression(expr);
                    }
                }
            }
            Expression::BinaryOp(b) => {
                self.walk_expression(&b.lhs);
                self.walk_expression(&b.rhs);
            }
            Expression::UnaryOp(u) => self.walk_expression(&u.operand),
            Expression::Call(c) => {
                self.walk_expression(&c.callee);
                for arg in &c.args {
                    self.walk_expression(arg);
                }
            }
            Expression::MemberAccess(m) => self.walk_expression(&m.object),
            Expression::IndexAccess(i) => {
                self.walk_expression(&i.object);
                self.walk_expression(&i.index);
            }
            Expression::IsCheck(i) => self.walk_expression(&i.expression),
            Expression::ObjectIsCheck(o) => {
                self.walk_expression(&o.lhs);
                self.walk_expression(&o.rhs);
            }
            Expression::QualifiedOp(q) => {
                self.walk_expression(&q.lhs);
                self.walk_expression(&q.rhs);
                for spec in &q.qualifiers {
                    if let Some(arg) = &spec.arg {
                        self.walk_expression(arg);
                    }
                }
            }
            Expression::ObjectLiteral(o) => {
                for (_, prop) in &o.properties {
                    self.walk_expression(prop);
                }
            }
            Expression::FunctionExpr(f) => {
                // Closures are not watchers: no boundary is pushed. But a
                // closure INSIDE a watcher body leaves the boundary in
                // place, so its outer references still box — conservative.
                self.push_scope();
                for param in &f.params {
                    self.declare(&param.name, &param.position);
                }
                self.walk_items(&f.body.items);
                self.pop_scope();
            }
            Expression::Match(m) => {
                self.walk_expression(&m.value);
                for arm in &m.arms {
                    match &arm.body {
                        MatchBody::Expression(expr) => self.walk_expression(expr),
                        MatchBody::Block(b) => self.walk_block(b),
                    }
                }
            }
            Expression::WeakRef(inner, _) => self.walk_expression(inner),
            Expression::Unknown(u) => {
                self.walk_expression(&u.reason);
                if let Some(options) = &u.options {
                    self.walk_expression(options);
                }
            }
            Expression::TupleLit(elems, _) => {
                for elem in elems {
                    self.walk_expression(elem);
                }
            }
            Expression::TupleAccess(inner, _, _) => self.walk_expression(inner),
            Expression::ArrayLit(elems, _) => {
                for elem in elems {
                    self.walk_expression(elem);
                }
            }
            Expression::TypeAscription(inner, _, _) => self.walk_expression(inner),
            Expression::WatcherExpr(w) => self.walk_watcher_expr(w),
        }
    }
}

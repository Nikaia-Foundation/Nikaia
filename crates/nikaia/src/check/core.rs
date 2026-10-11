//! **The Rust side of the checker's core** (ADR-294, #125, #558, C-6, U-c0): the
//! baton the state travels in, and the host the moved code asks the rest of the
//! checker through.
//!
//! **The state is a baton.** `Checker::s` holds it as `Baton`, a
//! `Option<Box<CheckState>>` that dereferences to the state and panics while the
//! state is out on a crossing: a forgotten swap is a panic, never a silent empty
//! state. A stub (`Checker::nikaia`) takes the box out, runs a Nikaia function
//! on `&mut *state`, and puts it back. A host method (`Core::hosted`) swaps the
//! state it is handed with the heap spare, lends the box to the checker, runs
//! the Rust, and swaps it back.
//!
//! **Never build a `CheckState` in a stub's or a host method's frame**: it is
//! some kilobytes, and a frame of that size on each crossing is what the stack
//! of a deeply nested program does not have (measured on 8 MB: a state by value
//! costs 8.6 KB a crossing, a box with a cold spare 192 bytes). The spares come
//! from a pool, `Checker::spares`, and a `#[cold]` constructor makes one when it
//! is empty.

use super::*;
use nikaia_std::tools::build_eval::BtRead;
use nikaia_std::tools::check_host::{CheckBuilt, CheckComputed, CheckHost, CheckPlace};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

/// The checker's state, which is out of the checker while Nikaia code runs.
pub(super) struct Baton(Option<Box<CheckState>>);

impl Baton {
    pub(super) fn new(state: Box<CheckState>) -> Baton {
        Baton(Some(state))
    }

    /// The state, for good: what the walk found, once it is over.
    pub(super) fn into_state(self) -> CheckState {
        *self.0.expect("the state is out on a crossing")
    }
}

impl Deref for Baton {
    type Target = CheckState;

    fn deref(&self) -> &CheckState {
        self.0
            .as_deref()
            .expect("the state is out on a crossing (core.rs)")
    }
}

impl DerefMut for Baton {
    fn deref_mut(&mut self) -> &mut CheckState {
        self.0
            .as_deref_mut()
            .expect("the state is out on a crossing (core.rs)")
    }
}

/// **A state with nothing in it**, on the heap: what a spare is made of, and
/// what `walked` fills in.
#[cold]
#[inline(never)]
pub(super) fn hollow() -> Box<CheckState> {
    Box::new(CheckState {
        flow: CheckFlow::start(false, false),
        findings: Vec::new(),
        structs: BTreeMap::new(),
        copies_in_the_package: BTreeSet::new(),
        enums: BTreeMap::new(),
        enum_payloads: BTreeMap::new(),
        variant_parts: BTreeMap::new(),
        variant_owner: BTreeMap::new(),
        walks_fields: BTreeMap::new(),
        grammars: Default::default(),
        scope: Vec::new(),
        std_in_scope: Default::default(),
        std_modules: Default::default(),
        expected: None,
        type_parameters: BTreeMap::new(),
        struct_parameters: BTreeMap::new(),
        borrowing_self: false,
        enclosing: BTreeMap::new(),
        subject_arguments: Vec::new(),
        declared_bounds: BTreeMap::new(),
        modules: Default::default(),
        opaque_handles: Default::default(),
        foreign_names: Default::default(),
        run_code: BTreeSet::new(),
        dsl_drivers: Default::default(),
        said_rings: BTreeSet::new(),
        computing_default: false,
        set_receiver: None,
        stamped_condition: None,
        inside_a_door: false,
        inside_an_action: None,
        inside_a_sync_function: None,
        moved_into_a_task: Vec::new(),
        walked: Vec::new(),
        branch: Vec::new(),
        reads_on_paths: Vec::new(),
        lent_lets: BTreeSet::new(),
        handed: Vec::new(),
        read_at: Vec::new(),
        written_at: Vec::new(),
        parts_out: Vec::new(),
        gives: Vec::new(),
        ways_out: Vec::new(),
        reading_only: false,
        read_seq: 0,
        task_bindings: Vec::new(),
        changed: Vec::new(),
        said_mut: BTreeSet::new(),
        facts: CheckFacts::empty(),
        methods: BTreeMap::new(),
        stored_code: BTreeMap::new(),
        stored_frames: Vec::new(),
        rule_results: BTreeMap::new(),
        current: None,
        open_comptimes: BTreeMap::new(),
        next_binding: 1 << 40,
        handed_over: None,
        in_parallel: None,
        scoped_task: false,
        inside_unsafe: false,
        repeats: Vec::new(),
    })
}

/// What the moved code sees of the checker: the host for the services that
/// stay Rust, and the checker itself for the hubs, `expr` and `block`.
pub(super) struct Core<'c, 'a> {
    c: RefCell<&'c mut Checker<'a>>,
    h: host::Host<'a>,
    spare: RefCell<Option<Box<CheckState>>>,
}

impl<'a> Checker<'a> {
    /// Run `f` with the state handed to Nikaia: the box leaves the checker for
    /// the time, with the world and a host that can bring it back.
    pub(super) fn nikaia<R>(
        &mut self,
        f: impl FnOnce(&mut CheckState, &CheckWorld<'a>, &Core<'_, 'a>) -> R,
    ) -> R {
        let mut state = self
            .s
            .0
            .take()
            .expect("the state is already out on a crossing");
        let spare = self.spares.pop().unwrap_or_else(hollow);
        let world = self.world;
        let h = self.host();
        let core = Core {
            c: RefCell::new(&mut *self),
            h,
            spare: RefCell::new(Some(spare)),
        };
        let result = f(&mut state, world, &core);
        let spare = core
            .spare
            .into_inner()
            .expect("a spare state is still lent out");
        self.spares.push(spare);
        self.s.0 = Some(state);
        result
    }
}

impl<'a> Core<'_, 'a> {
    /// Run Rust code on behalf of Nikaia: the state it holds goes back into the
    /// checker (by swapping the box contents with the spare, so no state is
    /// built or moved by value), and comes out again at the end.
    fn hosted<R>(&self, state: &mut CheckState, run: impl FnOnce(&mut Checker<'a>) -> R) -> R {
        let mut checker = self.c.borrow_mut();
        let mut lent = self
            .spare
            .borrow_mut()
            .take()
            .expect("the spare state is out: the host was entered twice");
        std::mem::swap(state, &mut *lent);
        assert!(checker.s.0.is_none(), "the state was in two places at once");
        checker.s.0 = Some(lent);
        let result = run(&mut checker);
        let mut back = checker
            .s
            .0
            .take()
            .expect("the state was taken during a hosted call");
        std::mem::swap(state, &mut *back);
        *self.spare.borrow_mut() = Some(back);
        result
    }
}

impl CheckHost for Core<'_, '_> {
    fn hole(&self, site: u32, text: &str) -> Option<Expr> {
        self.h.hole(site, text)
    }

    fn kept_call(
        &self,
        name: winnow_grammar::Symbol,
        arity: i64,
    ) -> (Vec<winnow_grammar::Symbol>, Expr) {
        self.h.kept_call(name, arity)
    }

    fn intern(&self, text: &str) -> winnow_grammar::Symbol {
        self.h.intern(text)
    }

    fn wrap_hole(&self, hole: &Expr, wraps: &Vec<(i64, winnow_grammar::Symbol)>) -> Expr {
        self.h.wrap_hole(hole, wraps)
    }

    fn literal_holes(&self, literal: &Expr) -> Vec<Expr> {
        self.h.literal_holes(literal)
    }

    fn evaluate(&self, value: &Expr, held: &BTreeMap<String, build_time::Value>) -> CheckBuilt {
        self.h.evaluate(value, held)
    }

    fn compute_comptime(
        &self,
        value: &Expr,
        found: &Ty,
        bound: &str,
        known: &BTreeMap<String, build_time::Value>,
        steps: Option<u64>,
        bytes: Option<u64>,
    ) -> CheckComputed {
        self.h
            .compute_comptime(value, found, bound, known, steps, bytes)
    }

    fn somewhere(&self) -> bool {
        self.h.somewhere()
    }

    fn decoded(&self, literal: &str) -> Option<String> {
        self.h.decoded(literal)
    }

    fn read(&self, path: &str) -> BtRead {
        self.h.read(path)
    }

    fn assets_read_in(&self, value: &Expr, read: &BTreeMap<String, String>) -> Expr {
        self.h.assets_read_in(value, read)
    }

    fn value_fn(&self) -> String {
        self.h.value_fn()
    }

    fn place_of(&self, place: &str, here: i64) -> CheckPlace {
        self.h.place_of(place, here)
    }

    fn written(&self, value: &Expr) -> String {
        self.h.written(value)
    }

    fn expr(&self, state: &mut CheckState, node: &Expr, span: &Span) -> Ty {
        self.hosted(state, |checker| checker.expr(node, span))
    }

    fn block(&self, state: &mut CheckState, block: &Block) -> Ty {
        self.hosted(state, |checker| checker.block(block))
    }
}

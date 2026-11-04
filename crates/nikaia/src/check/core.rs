//! **The Rust side of the checker's core** (ADR-294, #125, #558, C-6, U-c0): the
//! baton the state travels in, and the crossing into the moved code, which asks
//! the rest of the compiler through `host::Host`.
//!
//! **The state is a baton.** `Checker::s` holds it as `Baton`, a
//! `Option<Box<CheckState>>` that dereferences to the state and panics while the
//! state is out on a crossing: a forgotten swap is a panic, never a silent empty
//! state. A stub (`Checker::nikaia`) takes the box out, runs a Nikaia function
//! on `&mut *state`, and puts it back.
//!
//! **Never build a `CheckState` in a stub's frame**: it is some kilobytes, and
//! a frame of that size on each crossing is what the stack of a deeply nested
//! program does not have (measured on 8 MB: a state by value costs 8.6 KB a
//! crossing). The one state of a walk is made on the heap (`hollow`).

use super::*;
use nikaia_std::tools::check_narrow::CheckNarrowing;
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

/// **A state with nothing in it**, on the heap: what `walked` fills in.
#[cold]
#[inline(never)]
pub(super) fn hollow() -> Box<CheckState> {
    Box::new(CheckState {
        flow: CheckFlow::start(false, false),
        narrowing: CheckNarrowing::nothing(),
        arm_types: BTreeMap::new(),
        got_past_a_jump: BTreeSet::new(),
        arm_bound: BTreeMap::new(),
        arm_compared: Vec::new(),
        discarded: BTreeSet::new(),
        arm_disagreements: BTreeMap::new(),
        findings: Vec::new(),
        structs: BTreeMap::new(),
        copies_in_the_package: BTreeSet::new(),
        parts_beside: BTreeMap::new(),
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
        call_keepers: BTreeMap::new(),
        walked: Vec::new(),
        branch: Vec::new(),
        reads_on_paths: Vec::new(),
        lent_lets: BTreeSet::new(),
        text_bytes_lets: BTreeSet::new(),
        text_positions: BTreeMap::new(),
        handed: Vec::new(),
        read_at: Vec::new(),
        written_at: Vec::new(),
        parts_out: Vec::new(),
        view_places: BTreeMap::new(),
        gives: Vec::new(),
        ways_out: Vec::new(),
        cleanups_pending: Vec::new(),
        reading_only: false,
        read_seq: 0,
        task_bindings: Vec::new(),
        changed: Vec::new(),
        drained: BTreeSet::new(),
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

impl<'a> Checker<'a> {
    /// Run `f` with the state handed to Nikaia: the box leaves the checker for
    /// the time, with the world and the host the moved code asks the compiler
    /// through.
    #[track_caller]
    pub(super) fn nikaia<R>(
        &mut self,
        f: impl FnOnce(&mut CheckState, &CheckWorld<'a, 'a, 'a, 'a, 'a, 'a, 'a>, &host::Host<'a>) -> R,
    ) -> R {
        let mut state = crate::baton::out_of(self.s.0.take(), "the state");
        let world = self.world;
        let h = self.host();
        let result = f(&mut state, world, &h);
        self.s.0 = Some(state);
        result
    }
}

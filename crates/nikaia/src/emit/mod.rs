// crates/nikaia/src/emit/mod.rs
//
// Stage 0 of the bootstrap compiler: Nikaia in, Rust out.
//
// The three things this file exists for are the three halves of the grammar
// protocol that the parser backend already implements and Nikaia could not yet
// reach (README, "Bootstrap Compiler"):
//
//   * `grammar Name { rule ... }`  ->  `grammar! { grammar Name { ... } }`
//   * `@frame(boundary: "\n")`     ->  `#[frame(boundary = "\n")]`
//   * `Name.rule(input)`            ->  the generated `par_fold` driver, with
//                                      the `Parallelism` the build asks for
//
// What it is *not* is a type checker. The lowering is syntactic: every action
// block, every `init`/`step`/`merge` and every function body is emitted as
// written. Where Nikaia and Rust disagree about what a name means - a `&mut
// self` method used as a monoid's merge, `std::fs` - the emitted program says
// so by not compiling, rather than this file guessing. That is deliberate:
// guessing a receiver would be a semantic decision made in a printer.
//
// It emits a [`SourceMap`] alongside the code, because a transpiler that only
// emits code can only be told about errors in a file nobody wrote (ADR-300).

use crate::contracts::LedgerOps;
use std::collections::{HashMap, HashSet};

use anyhow::Result;

mod host;
pub(crate) mod seeds;
pub mod template;
use winnow_grammar::Symbol;

use crate::assets::Reads;
use crate::ast::{Block, Expr, Span, Spanned, Stmt, Type};
use crate::parser::Parsed;
use crate::refused;

/// The machine a program is built for (ADR-037 D1), and the build switches:
/// `tools/emit_build.nika`. The default target is the machine the compiler
/// runs on, [`host_target`], a value of the compiler's own build, so it stays
/// here ([`default_target`]).
pub use nikaia_std::tools::emit_build::Target;

/// **The machine this compiler is running on.** The compiler is a native
/// program, so the triple it was built for (`build.rs`) is the machine it
/// runs on: `x86_64-linux` on a Linux PC, `aarch64-linux-android` on a
/// phone, `aarch64-apple-darwin` on a Mac.
///
/// `None` only for a triple Rust's target list does not know: a build
/// there has to name its target.
pub fn host_target() -> Option<Target> {
    Target::parse(env!("NIKAIA_HOST_TRIPLE")).ok()
}

/// The target a build without `--target` is for: the host, or `x86_64-linux`
/// on a machine Rust's target list does not know.
pub fn default_target() -> Target {
    host_target().unwrap_or(Target::X86_64Linux)
}

/// Whether the user's code may run concurrently, and whether the program
/// carries the re-entrancy check: `tools/emit_build.nika`.
pub use nikaia_std::tools::emit_build::{ReentrancyCheck, UserParallelism};

/// The build switches together (ADR-037, [ADR-281](../../docs/specification/adr/adr-281.md) D36).
///
/// One value rather than three parameters: a fourth switch is then a field, not
/// a change at every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Build {
    pub target: Target,
    pub user_parallelism: UserParallelism,
    pub reentrancy_check: ReentrancyCheck,
    /// `--optimization=remove-bounds-checks:on`
    /// ([ADR-306](../../docs/specification/adr/adr-306.md) D1): how hard the
    /// compiler works to prove an index inside. Nothing a program means
    /// depends on it.
    pub bounds: crate::bounds::BoundsChecks,
    /// `--optimization=remove-overflow-checks:on`
    /// ([ADR-306](../../docs/specification/adr/adr-306.md) D6).
    pub overflow: crate::bounds::OverflowChecks,
    /// `--optimization=skip-free-at-exit:on`
    /// ([ADR-348](../../docs/specification/adr/adr-348.md) D1): what `main`
    /// still holds at its end is not freed.
    pub skip_free_at_exit: bool,
    /// `--optimization=prove-through-loops:on`
    /// ([ADR-361](../../docs/specification/adr/adr-361.md) D10): a check a
    /// loop leaves open is asked once more through the loop's invariant. It
    /// changes what is proved, never what a program means.
    pub prove_through_loops: bool,
    /// **Code run while the program is built counts its steps**
    /// ([ADR-321](../../docs/specification/adr/adr-321.md) D7): every function
    /// it enters and every turn of a loop, against the budget
    /// `nikaia_std::build_time` holds. Set for a `comptime`'s program and
    /// nothing else.
    pub counts_steps: bool,
    /// **The lowering is asked what the program uses, not what it computes**:
    /// its check leaves every `comptime` and default alone. Set by
    /// [`Needs::of`], whose answer no build-time value changes, and which would
    /// otherwise compute each one a second time.
    pub leaves_values: bool,
    /// **A library C calls** (`artifact = "c-library"`,
    /// [ADR-284](../../docs/specification/adr/adr-284.md) D1, D2): its entry
    /// points may be called from two of the caller's threads at once, whatever
    /// `user_parallelism` says, so a `Shared` keeps the count the analysis
    /// gives it - the atomic floor at a public signature or field, the plain
    /// one only where nothing crosses (#113).
    pub called_from_c: bool,
}

impl Default for Build {
    fn default() -> Build {
        Build {
            target: default_target(),
            user_parallelism: UserParallelism::No,
            reentrancy_check: ReentrancyCheck::Yes,
            bounds: crate::bounds::BoundsChecks::Kept,
            overflow: crate::bounds::OverflowChecks::Kept,
            skip_free_at_exit: false,
            prove_through_loops: false,
            counts_steps: false,
            leaves_values: false,
            called_from_c: false,
        }
    }
}

/// A build switch's refusal from `tools/emit_build.nika`, in the words it gave.
fn switch(thrown: nikaia_std::error::Thrown<nikaia_std::tools::Refused>) -> anyhow::Error {
    refused!("{thrown}")
}

impl Build {
    pub fn parse(target: &str, user_parallelism: &str, reentrancy_check: &str) -> Result<Build> {
        Ok(Build {
            target: Target::parse(target).map_err(switch)?,
            user_parallelism: UserParallelism::parse(user_parallelism).map_err(switch)?,
            reentrancy_check: ReentrancyCheck::parse(reentrancy_check).map_err(switch)?,
            bounds: crate::bounds::BoundsChecks::Kept,
            overflow: crate::bounds::OverflowChecks::Kept,
            skip_free_at_exit: false,
            prove_through_loops: false,
            counts_steps: false,
            leaves_values: false,
            called_from_c: false,
        })
    }

    /// Whether a value may cross a thread the program's own code did not
    /// start: one of its tasks at `user_parallelism = yes`, or a caller's
    /// thread for a library C calls.
    pub fn crossings_are_possible(self) -> bool {
        self.user_parallelism == UserParallelism::Yes || self.called_from_c
    }

    /// The default machine, with parallelism asked for.
    pub fn parallel() -> Build {
        Build {
            user_parallelism: UserParallelism::Yes,
            ..Build::default()
        }
    }

    /// How the generated driver is asked to cut the input.
    fn parallelism(self) -> String {
        self.user_parallelism.parallelism(self.target.has_threads())
    }

    /// **May two pieces of the program's own code run at the same time?**
    /// (ADR-037 D2.)
    ///
    /// This gates `task::both` and nothing else. `task::both(|| …, || …)` puts
    /// two user closures on two threads, so it is out at
    /// `user_parallelism = no` whatever the analysis says - ADR-292 decides
    /// whether two operations *may* overlap, and this decides whether there is
    /// anything to overlap them with. A target without threads answers no for
    /// the second reason: `rayon::join` does not link on `wasm32-unknown`
    /// (Part III 15.3).
    ///
    /// **Not the same question as [`Build::overlaps_operations`]**, and the
    /// whole of ADR-292 D6 is that difference. "Two pieces of your code at
    /// once" is forbidden at `no`; "two operations in flight at once" is not,
    /// and never was - Part I 8.4's runtime has had a second thread at `no`
    /// since ADR-303 D4, and it is safe precisely because *user code never
    /// runs there*. What `no` forbids is that anything **you** wrote is in
    /// flight twice at once (Part I 1.2), which is why the word in
    /// `user_parallelism` is `user`.
    pub fn overlaps_user_code(self) -> bool {
        self.user_parallelism.is_concurrent() && self.target.has_threads()
    }

    /// **May two operations be in flight at the same time?** (ADR-292 D6.)
    ///
    /// True at **both** settings of `user_parallelism`, because the vehicle it
    /// gates carries no code the program wrote: two reads handed to the
    /// runtime are two operations the kernel performs while the program is
    /// suspended at both of them, and `std` is what waits. Nothing of the
    /// user's is in flight twice, so ADR-037 D2's promise is untouched - and
    /// `nikaia_std::rt` keeps it by construction rather than by convention,
    /// since an I/O worker's inbox takes operations and no variant of one can
    /// carry a closure (ADR-303).
    ///
    /// What it asks of the build is therefore only whether `std`'s runtime is
    /// there at all. `user_parallelism` has no say in it; the *target* does.
    pub fn overlaps_operations(self) -> bool {
        self.target.has_runtime()
    }
}

/// How a view's lifetime is spelled here. The source never says either way
/// (ADR-283); the position does.
use nikaia_std::tools::emit_kinds::EmitLifetimes as Lifetimes;

// --- The output, and the way back ---

/// What the lowering produces: the Rust, and where each part of it came from.
#[derive(Debug, Clone)]
pub struct Lowered {
    pub rust: String,
    pub map: SourceMap,
    /// What the prover publishes of each function's contract, for the ledger
    /// ([ADR-269](../../../docs/specification/adr/adr-269.md) D18).
    pub published: std::collections::BTreeMap<String, crate::prove::Published>,
}

/// Emitted byte range -> the `.nika` span that produced it.
///
/// This is the whole answer to "the error points at a line nobody wrote": the
/// compiler that consumes the emitted Rust reports offsets into it, and every
/// one of them can be traded back for a place in the source the user has open.
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    /// Innermost first (`tools/emit_out.nika`): a lookup returns the first
    /// range that contains the offset. A single-file program has one unit,
    /// and it is 0.
    entries: Vec<nikaia_std::tools::emit_out::EmitMapEntry>,
}

impl SourceMap {
    /// The narrowest source span whose emitted text covers `offset`, and the
    /// file it is in.
    pub fn locate(&self, offset: usize) -> Option<(usize, Span)> {
        nikaia_std::tools::emit_out::emit_out_locate(&self.entries, offset as i64)
            .map(|e| (e.unit as usize, e.source))
    }

    /// The narrowest source span whose emitted text covers `offset`.
    pub fn source_span(&self, offset: usize) -> Option<Span> {
        self.locate(offset).map(|(_, span)| span)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// This map, as it reads once its module's text has been placed `by` bytes
    /// into a larger file - and tagged with which file it came from
    /// (`emit_out_placed`).
    pub fn placed(mut self, by: usize, unit: usize) -> SourceMap {
        nikaia_std::tools::emit_out::emit_out_placed(&mut self.entries, by as i64, unit as i64);
        self
    }

    /// Take everything `other` holds, which is already placed.
    pub fn extend(&mut self, other: SourceMap) {
        self.entries.extend(other.entries);
    }

    /// Every entry as `(generated start, source start, unit)`, in the order the
    /// emitter recorded them - which is outermost first for any one place.
    ///
    /// For the abort table (`emit_needs_abort_table`), which needs all of
    /// them rather than a lookup.
    pub fn rows(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        self.entries
            .iter()
            .map(|e| (e.start as usize, e.source.at(), e.unit as usize))
    }
}

/// The emitted text under construction, and the map being built with it
/// (`tools/emit_out.nika`).
type Out = nikaia_std::tools::emit_out::EmitOut;

pub fn emit_program(parsed: &Parsed, build: Build) -> Result<Lowered> {
    let trust = crate::contracts::trust::analyse(parsed, std_ledger());
    Emitter::new(parsed, build, trust.provenance).program()
}

/// The same, told what this build may read while it builds
/// ([ADR-310](../../docs/specification/adr/adr-310.md)).
///
/// **A second entry point rather than a field on [`Build`]**, for the reason
/// `beside` is a parameter: `Build` is the machine and the switches, copied
/// freely, and what a build may read is a fact about the *invocation* with a
/// lifetime on it. Every caller that has nothing to say passes
/// [`assets::Reads::none`], which is D1.
pub fn emit_program_reading(parsed: &Parsed, build: Build, reads: &Reads) -> Result<Lowered> {
    // **A default is compiled where a `comptime` is** (ADR-321 D1): the ledger
    // this lowering infers records it from the same workshop.
    crate::comptime_run::defaults_compiled_in(reads.workshop().place(), || {
        let trust = crate::contracts::trust::analyse(parsed, std_ledger());
        Emitter::new_reading(parsed, build, trust.provenance, reads).program()
    })
}

/// The same, with the provenance already decided.
///
/// The CLI analyses once and prints the answer under `--trust`; this is what it
/// hands back in so the emitted code and the explanation cannot disagree.
pub fn emit_program_with_trust(
    parsed: &Parsed,
    build: Build,
    provenance: crate::contracts::Provenance,
) -> Result<Lowered> {
    Emitter::new(parsed, build, provenance).program()
}

/// The same, for a module that reads Rust it was handed described: a tool of
/// `nikaia-std`'s, whose tree names are `winnow_grammar::Symbol`s
/// ([ADR-294](../../docs/specification/adr/adr-294.md) D9.3).
pub fn emit_std_against(
    parsed: &Parsed,
    beside: &[&Parsed],
    own: &crate::contracts::Ledger,
    described: &crate::contracts::Ledger,
) -> Result<Lowered> {
    // Named rather than defaulted: the default is the machine this runs on,
    // and the committed text must not depend on where it was lowered.
    let build = Build {
        target: Target::X86_64Linux,
        ..Build::default()
    };
    let mut library = std_ledger().clone();
    library.types.extend(described.types.clone());
    library.functions.extend(described.functions.clone());
    let package: Vec<&Parsed> = std::iter::once(parsed)
        .chain(beside.iter().copied())
        .collect();
    let trust = crate::contracts::trust::analyse_package(&package, &library);
    Emitter::with_contracts(
        parsed,
        beside,
        build,
        trust.provenance,
        own.clone(),
        described,
        &Reads::none(),
        None,
    )
    .for_std()
    .program()
}

/// The same at a given build, for the test that holds the package to lowering
/// the same at both settings of `user_parallelism` (ADR-002 D4).
pub fn emit_std_items_against_at(
    parsed: &Parsed,
    beside: &[&Parsed],
    own: &crate::contracts::Ledger,
    described: &crate::contracts::Ledger,
    build: Build,
    provenance: crate::contracts::Provenance,
    // **The facts of the check that already ran on this file**, when it ran
    // exactly as the emitter's own would (`check::propagation_against`: the
    // same `beside`, `own`, library, no modules, nothing newly, no reads), so
    // the checker walks the file once and not twice. `None` runs it here.
    checked: Option<crate::check::CheckFacts>,
) -> Result<Lowered> {
    Emitter::with_contracts(
        parsed,
        beside,
        build,
        provenance,
        own.clone(),
        described,
        &Reads::none(),
        checked,
    )
    .for_std()
    .items_only()
}

/// The same, saying whether these items are the crate root's.
///
/// Only the crate root may carry the `fn main` Rust runs, and ADR-303 D4 makes
/// that one generated function rather than the program's own - so a module is
/// emitted with `false` and a `main` in it stays as written.
#[allow(clippy::too_many_arguments)]
pub fn emit_module_body_at(
    parsed: &Parsed,
    // **The program's other files**, for the one thing the emitter asks the
    // checker that needs a body: what a `comptime` came to. See
    // `check::propagation_against`.
    beside: &[&Parsed],
    build: Build,
    provenance: crate::contracts::Provenance,
    contracts: &crate::contracts::Ledger,
    // **The described crates' boundary** (ADR-290 D20), for whether a call into
    // one can fail. Empty where the program declares none.
    described: &crate::contracts::Ledger,
    entry: bool,
    // **And what it may read**, for the same reason one line up: the check the
    // emitter runs has to be the check that refused, or the two halves
    // disagree about one program ([ADR-310](../../docs/specification/adr/adr-310.md)).
    reads: &Reads,
    // The facts of a check already run as `check::propagation_against` would
    // run it, or `None` for the emitter to run it.
    checked: Option<crate::check::CheckFacts>,
) -> Result<Lowered> {
    Emitter::with_contracts(
        parsed,
        beside,
        build,
        provenance,
        contracts.clone(),
        described,
        reads,
        checked,
    )
    .for_entry(entry)
    .items_only()
}

/// **The names a call must not hand a fast map to** (ADR-010 D9, barriers):
/// the methods of a trait that hold a map in their signature, the `pub`
/// functions that hand a map on to one, and the functions of described Rust
/// crates that take one. A program that calls one is no more trusted than the
/// keyed hash, because the map arrives there with std's hasher.
pub fn barrier_names(
    units: &[&Parsed],
    described: &crate::contracts::Ledger,
) -> std::collections::BTreeSet<String> {
    seeds::Seeds::read_with(
        units,
        &nikaia_std::tools::emit_tables::emit_tables_described_maps(described),
    )
    .barrier_names()
}

/// What a program's preamble has to say, over all of its files
/// (`tools/emit_needs.nika`).
pub use nikaia_std::tools::emit_needs::EmitNeeds as Needs;

/// What one file needs, read off its items with no emitter built (#727).
pub fn needs_of(parsed: &Parsed) -> Needs {
    Needs::of(&EmitWorld {
        words: &parsed.interner,
        aliases: &parsed.aliases,
        items: &parsed.program.items,
        library: std_ledger(),
    })
}

/// `std`'s shipped contracts, parsed once.
///
/// A malformed ledger is a bug in this repository rather than in a user's
/// program, and the tests read the same file - so failing to parse it here
/// means treating the input as untrusted, which is the safe direction
/// (ADR-010 D1) and never a silent upgrade.
fn std_ledger() -> &'static crate::contracts::Ledger {
    crate::contracts::std_ledger()
}

/// The words the **language below** reserves.
///
/// Most of them this language does not reserve. Four it does - `ref`, `trait`,
/// `extern` and `unsafe` - and they stay because a reserved word is still a name
/// after `.` and `::` ([ADR-298](../../../docs/specification/adr/adr-298.md)
/// D3): a member called `ref` that a library declares is written `x.r#ref`
/// below. `tests/reserved_below.rs` holds all four to the escape.
///
/// Measured rather than copied out of a reference, and measured **twice**: the
/// first sweep read `rustc`'s *"found keyword"* at a `let` and came back with
/// twenty-four, missing `crate`, `super` and `box` because those three fail
/// there with a different message each (`E0532`, `E0433`, *"expected pattern"*).
/// The sweep in `tests/reserved_below.rs` compiles every word in every position
/// instead, which is what found them.
///
/// `gen` is in since the emitted code is Edition 2024, which reserves it
/// (ADR-305). `union` is deliberately **absent**: Rust takes it as an
/// identifier, so escaping it would be a change nothing asked for.
///
/// `crate`, `super`, `self` and `Self` are absent for the opposite reason —
/// **Rust forbids a raw identifier for exactly those four**, so there is no
/// escape to write. They are refused instead (`NK1128`, and `NK1119` for
/// `self`), which is [ADR-298](../../../docs/specification/adr/adr-298.md) D11.
/// A source name, written so the language below can read it.
///
/// **This is [ADR-296](../../../docs/specification/adr/adr-296.md) D17 paying one
/// of its bills.** The emitter writes a name for a name and resolves nothing, so
/// a Nikaia name that Rust happens to spell as a keyword went out verbatim and
/// `rustc` answered *"expected identifier, found keyword `type`"* about a file
/// nobody wrote, with *"escape `type` to use it as an identifier"* as the help —
/// advice that means nothing in this language
/// ([Part III C.1](../../../docs/specification/30-nikaia-tooling.md)).
///
/// **Escaped rather than reserved**, which is
/// [ADR-298](../../../docs/specification/adr/adr-298.md) D9: reserving these
/// words in Nikaia would let the backend decide what this language's vocabulary
/// is, and `type` is the field name of every tagged record anybody has ever
/// written.
///
/// **`const` and `loop` joined the list when they left Nikaia's own**
/// ([ADR-298](../../../docs/specification/adr/adr-298.md) D6), which is that
/// record's *answered as it is for `type`*: they were not here before because a
/// Nikaia name could not be one of them, and now it can. `macro` was already
/// here, and `quote` is a keyword in neither language. The three the backend
/// cannot escape at all — `crate`, `super` and `Self` — are `NK1128`'s and not
/// this list's.
///
/// Borrowed where nothing changes, which is every name in every program written
/// today.
pub fn escaped(name: &str) -> std::borrow::Cow<'_, str> {
    match nikaia_std::tools::emit_misc::is_reserved(name) {
        true => std::borrow::Cow::Owned(format!("r#{name}")),
        false => std::borrow::Cow::Borrowed(name),
    }
}

use nikaia_std::tools::emit_state::{EmitState, EmitWalk, EmitWorld};

struct Emitter<'p> {
    parsed: &'p Parsed,
    build: Build,
    /// ADR-292: whether two statements that meet on nothing may overlap.
    /// The same, by method name alone, for the places where the receiver's type
    /// is not written down. `None` marks a name two impls disagree about - and
    /// an ambiguous name is left alone rather than resolved by preference.
    /// What is worked out before the first line is written
    /// (`tools/emit_state.nika`).
    s: EmitState,
    /// The places the walk writes to as it goes, borrowed briefly and never
    /// across a call back into the emitter.
    /// The walk is a baton (`host.rs`): `None` while it is out on a crossing.
    w: std::cell::RefCell<Option<EmitWalk>>,
    /// An error raised by Rust code under a crossing (`host.rs`).
    failed: std::cell::RefCell<Option<anyhow::Error>>,
    /// What the emitter looks at and does not own.
    world: EmitWorld<'p, 'p, 'p, 'p>,
    /// **Which `pub` items hold map positions** (ADR-010 D9): read from every
    /// file of the build, so a program knows the hidden parameters of the
    /// library types it names.
    seeds: seeds::Seeds,
    /// **What a give-up lowers to** (ADR-views-3 D9, `view_changes`): an
    /// assignment that becomes a binding of its own, and the `let`s that need
    /// no `mut` because of it.
    give_ups: crate::view_changes::GiveUps,
    /// **The counters `bounds::proven` already found never negative**, by the
    /// address of the body in `parsed` (`Proven::nonnegative_locals`).
    nonnegative_bodies: HashMap<usize, HashSet<Symbol>>,
    /// **Which checks there are and which are proved**, for the order a
    /// block's statements are written in (ADR-379).
    order_proofs: nikaia_std::tools::emit_order::EmitOrderProofs,
    /// **The program's `main`'s parameters as its definition wrote them**,
    /// `name: type`, for the entry that reads the words into them
    /// ([ADR-360](../../docs/specification/adr/adr-360.md) D1).
    main_params: std::cell::RefCell<Vec<String>>,
}

/// **The oldest Rust a Nikaia installation works with**
/// ([ADR-305](../../docs/specification/adr/adr-305.md)).
///
/// **Measured, not derived**: the whole test suite - the compiler, `std`, and
/// every program the tests emit and compile - passes on this version, and CI
/// runs it there. It is the highest of what each part needs: the emitted code
/// writes `impl AsyncFn` and `async` closures (1.85), `std` uses
/// `NonNull::map_addr` (1.84) and its tests `io::pipe` (1.87), and the
/// compiler's dependencies ask for 1.88.
///
/// Every generated `Cargo.toml` carries it, and the build compares
/// `rustc --version` against it before handing anything to `cargo` - because
/// Cargo's own *package requires rustc 1.88 or newer* is a message about a
/// generated file, which [Part III C.1](../../docs/specification/30-nikaia-tooling.md)
/// forbids.
///
/// **It is a different fact from the channel.** `rust-toolchain.toml` names
/// that and stays the only place that does
/// ([ADR-001](../../docs/specification/adr/adr-001.md) D1). It rises by a
/// record and a measurement, never silently: `scripts/check-floor.sh` reads it
/// from here and runs the suite on it.
pub const RUST_FLOOR: &str = "1.88";

impl<'p> Emitter<'p> {
    fn new(parsed: &'p Parsed, build: Build, provenance: crate::contracts::Provenance) -> Self {
        Self::new_reading(parsed, build, provenance, &Reads::none())
    }

    /// The same, told what this build may read
    /// ([ADR-310](../../docs/specification/adr/adr-310.md)).
    fn new_reading(
        parsed: &'p Parsed,
        build: Build,
        provenance: crate::contracts::Provenance,
        reads: &Reads,
    ) -> Self {
        let own = crate::contracts::Ledger::infer(parsed);
        Self::with_contracts(
            parsed,
            &[],
            build,
            provenance,
            own,
            &crate::contracts::Ledger::blank(),
            reads,
            None,
        )
    }

    /// The same, against contracts that already exist - a program's rather than
    /// a file's. The state is put together in `tools/emit_contracts.nika`;
    /// what stays here is what only Rust reaches (the seeds, the check and the
    /// prover over the parsed file, the view changes).
    #[allow(clippy::too_many_arguments)]
    fn with_contracts(
        parsed: &'p Parsed,
        beside: &[&Parsed],
        build: Build,
        provenance: crate::contracts::Provenance,
        own_contracts: crate::contracts::Ledger,
        // **The described crates' boundary** (ADR-290 D20): whether a call into
        // one can fail, and what it keeps, which the propagation below asks.
        described: &crate::contracts::Ledger,
        reads: &Reads,
        // The facts of a check already run as `propagation_against` would run
        // it (`emit_std_items_against_at`).
        checked: Option<crate::check::CheckFacts>,
    ) -> Self {
        let seeds_of = {
            let mut all: Vec<&Parsed> = beside.to_vec();
            if !all.iter().any(|p| std::ptr::eq(*p, parsed)) {
                all.push(parsed);
            }
            seeds::Seeds::read_with(
                &all,
                &nikaia_std::tools::emit_tables::emit_tables_described_maps(described),
            )
        };
        // The check's keep plans, where it made them as this does (#727).
        let mut checked = checked;
        let checked_plans = checked.as_mut().and_then(|facts| facts.keep_plans.take());
        let library = std_ledger();
        let units: Vec<&Parsed> = std::iter::once(parsed)
            .chain(beside.iter().copied())
            .collect();
        let unit_views: Vec<_> = units
            .iter()
            .map(|p| crate::check::parsed_view_of(p))
            .collect();
        let beside_views: Vec<_> = beside
            .iter()
            .map(|p| crate::check::parsed_view_of(p))
            .collect();
        let host = ContractsHost {
            parsed,
            beside,
            units: &units,
            described,
            reads,
            library,
            build: &build,
            seeds: &seeds_of,
            nonnegative_locals: std::cell::RefCell::new(HashMap::new()),
        };
        let made = nikaia_std::tools::emit_contracts::emit_contracts_state(
            &host,
            parsed,
            &unit_views,
            &beside_views,
            &parsed.program,
            own_contracts,
            described,
            library,
            checked,
            checked_plans,
            &parsed.refused_moves,
            &nikaia_std::tools::emit_contracts::EmitContractsBuild {
                leaves_values: build.leaves_values,
                bounds_removed: build.bounds == crate::bounds::BoundsChecks::Removed,
                overflow_removed: build.overflow == crate::bounds::OverflowChecks::Removed,
                trusted: provenance == crate::contracts::Provenance::Trusted,
            },
        );
        let nonnegative_bodies = host.nonnegative_locals.take();
        Self {
            parsed,
            build,
            s: made.state,
            give_ups: made.give_ups,
            w: std::cell::RefCell::new(Some(EmitWalk::fresh())),
            failed: std::cell::RefCell::new(None),
            world: EmitWorld {
                words: &parsed.interner,
                aliases: &parsed.aliases,
                items: &parsed.program.items,
                library,
            },
            seeds: seeds_of,
            nonnegative_bodies,
            main_params: std::cell::RefCell::new(Vec::new()),
            order_proofs: made.order_proofs,
        }
    }

    /// The same, said of a module rather than of the crate root.
    fn for_entry(mut self, entry: bool) -> Self {
        self.s.entry = entry;
        self
    }

    /// This is `std` itself - see [`emit_std`].
    fn for_std(mut self) -> Self {
        self.s.is_std = true;
        self
    }

    fn text(&self, sym: Symbol) -> &str {
        self.parsed.text(sym)
    }

    /// A module's items and nothing else - no preamble, no `mod` header.
    fn items_only(&self) -> Result<Lowered> {
        self.unit(false)
    }

    fn program(&self) -> Result<Lowered> {
        self.unit(true)
    }

    /// The unit's text (`tools/emit_program.nika`), with the source map; a
    /// whole program that carries `main` ends with an empty abort table.
    fn unit(&self, whole: bool) -> Result<Lowered> {
        let mut out = Out::empty();
        let crossed = self.nikaia(|walk| {
            self.s
                .emit_program(&self.world, walk, &mut out, self, whole)
        });
        self.crossed(crossed)?;
        Ok(Lowered {
            rust: out.buf,
            map: SourceMap {
                entries: out.entries,
            },
            published: self.s.facts.published.clone(),
        })
    }

    // --- Types ---

    /// A type, with the atomic floor for any `Shared` inside it.
    ///
    /// Every position whose count is actually known names it with
    /// [`Emitter::ty_counted`] instead. The floor is what a position nothing
    /// decided gets, which is ADR-037 D9's answer and never a wrong one - only a
    /// slower one.
    fn ty(&self, ty: &Type, lifetimes: Lifetimes) -> String {
        self.nikaia(|walk| {
            self.s.emit_core_ty(
                &self.world,
                walk,
                self.build.crossings_are_possible(),
                self.build.overlaps_user_code(),
                ty,
                &lifetimes,
            )
        })
    }

    /// **A type the package writes outside a signature** (a grammar rule's
    /// result, the accumulator of a fold): a `pub` type with map parameters
    /// takes what the package fills an ordinary mention with, `Fast` where it
    /// is trusted and `Keyed` where it is not, not the type's default.
    fn ty_filled(&self, ty: &Type, lifetimes: Lifetimes) -> String {
        self.seeded(self.ty(ty, lifetimes))
    }

    /// **A written type, with the hidden map parameters in** (ADR-010 D9),
    /// on the walk this hosted call holds (`tools/emit_seeds.nika`).
    fn seeded(&self, written: String) -> String {
        let mut held = self.w.borrow_mut();
        let walk = held
            .as_mut()
            .expect("a type is filled only inside a hosted call");
        nikaia_std::tools::emit_seeds::emit_seeds_seeded(
            &self.s.seeds,
            walk,
            self.s.trusted_input,
            written,
        )
    }

    // --- Statements and expressions ---

    /// Whether a **branch of an `overlap`** can pause
    /// ([ADR-292](../../../docs/specification/adr/adr-292.md) D6).
    ///
    /// A walk and not a lookup, because the question is about *this statement*
    /// and the answers the ledger and the checker give are keyed one by callee
    /// and one by statement. Both are consulted here: a free call by the key it
    /// resolves to, a method call by what the checker said about the statement
    /// it stands in - which is the same pair of sources `call` and the method
    /// arm use, asked ahead of time.
    ///
    /// It was ADR-292's, for the pair the compiler chose to overlap; D1
    /// withdrew that and D6 asks the same question about a branch the
    /// programmer chose.
    fn branch_pauses(&self, stmt: &Spanned<Stmt>) -> bool {
        self.s
            .branch_pauses(&self.world, &stmt.node, stmt.span.at() as i64)
    }
}

/// What `tools/emit_contracts.nika` asks of the parsed file and the analyses
/// only Rust holds; it keeps what the prover proved of the bodies' locals for
/// the emitter that follows.
struct ContractsHost<'a> {
    parsed: &'a Parsed,
    beside: &'a [&'a Parsed],
    units: &'a [&'a Parsed],
    described: &'a crate::contracts::Ledger,
    reads: &'a Reads,
    library: &'a crate::contracts::Ledger,
    build: &'a Build,
    seeds: &'a seeds::Seeds,
    nonnegative_locals: std::cell::RefCell<HashMap<usize, HashSet<Symbol>>>,
}

impl nikaia_std::tools::emit_contracts::EmitContractsHost for ContractsHost<'_> {
    fn propagation_leaving_values(
        &self,
        own: &crate::contracts::Ledger,
    ) -> crate::check::CheckFacts {
        crate::check::propagation_leaving_values(
            self.parsed,
            self.beside,
            own,
            self.described,
            self.reads,
        )
    }

    fn propagation_against(&self, own: &crate::contracts::Ledger) -> crate::check::CheckFacts {
        crate::check::propagation_against(self.parsed, self.beside, own, self.described, self.reads)
    }

    fn proven(
        &self,
        facts: &crate::check::CheckFacts,
        published: &crate::contracts::Ledger,
    ) -> nikaia_std::tools::emit_contracts::EmitContractsProven {
        let proven = crate::bounds::proven(
            self.parsed,
            &crate::bounds::Sites {
                lengths: &facts.std_lengths,
                sized: &facts.sized_lengths,
                char_codes: &facts.char_codes,
                arithmetic: &facts.arithmetic,
                negations: &facts.negations,
                whole_fields: &facts.whole_fields,
                array_lengths: &facts.array_lengths,
                shifts: &facts.shifts,
                std_receivers: &facts.std_receivers,
                simd_loads: &facts.simd_loads,
                comptime_values: &facts.comptime_values,
            },
            &[published, std_ledger()],
            &std::collections::BTreeSet::new(),
            self.build.prove_through_loops,
        );
        *self.nonnegative_locals.borrow_mut() = proven.nonnegative_locals;
        nikaia_std::tools::emit_contracts::EmitContractsProven {
            indices: proven.indices.into_iter().collect(),
            arithmetic: proven.arithmetic.into_iter().collect(),
            negations: proven.negations.into_iter().collect(),
            calls: proven.calls,
        }
    }

    fn shared(
        &self,
        own: &crate::contracts::Ledger,
    ) -> std::collections::BTreeMap<String, nikaia_std::tools::ty::Count> {
        crate::contracts::sharing::analyse_program(
            self.parsed,
            own,
            self.library,
            self.build.crossings_are_possible(),
        )
        .counts
    }

    fn has_seeds(&self) -> bool {
        !self.seeds.is_empty()
    }

    fn seeded_calls(&self) -> std::collections::BTreeMap<String, (i64, i64)> {
        self.seeds.calls().clone()
    }

    fn seeds(&self) -> nikaia_std::tools::emit_seeds::EmitSeedsTables {
        self.seeds.tables().clone()
    }

    fn carries_input(
        &self,
        own: &crate::contracts::Ledger,
    ) -> std::collections::BTreeMap<i64, std::collections::BTreeSet<String>> {
        crate::views::carried(self.parsed, own, self.library)
            .into_iter()
            .map(|(at, symbols)| {
                (
                    at as i64,
                    symbols
                        .into_iter()
                        .map(|s| self.parsed.text(s).to_string())
                        .collect(),
                )
            })
            .collect()
    }

    fn buffer_plans(
        &self,
        own: &crate::contracts::Ledger,
    ) -> Vec<nikaia_std::tools::buffers::BufferPlan> {
        crate::contracts::keep::buffer_plans(
            self.parsed,
            Some(&crate::contracts::keep::beside(self.units)),
            own,
            self.library,
        )
    }

    fn hole_wraps(&self) -> std::collections::BTreeMap<i64, Vec<(i64, Symbol)>> {
        self.parsed
            .hole_wraps
            .iter()
            .map(|(at, wraps)| {
                (
                    *at as i64,
                    wraps.iter().map(|(n, s)| (*n as i64, *s)).collect(),
                )
            })
            .collect()
    }
}

// --- Free helpers ---

/// **The same, with the types another file of the package declares**
/// (Part I 9.1, #544): `outside` names the ones known to hold a view, so a
/// struct here that holds an `enum` declared beside this file holds a view
/// too. By name, because a `Symbol` belongs to the parse that interned it.
pub(crate) fn borrowing_structs_beside(
    parsed: &Parsed,
    outside: &std::collections::BTreeSet<String>,
) -> HashSet<Symbol> {
    nikaia_std::tools::emit_tables::emit_tables_borrowing_structs_beside(
        &parsed.interner,
        &parsed.program.items,
        outside,
    )
    .into_iter()
    .collect()
}

/// Every view-holding type the files of a package declare, by name: each
/// file's [`borrowing_structs_beside`] read against what the others found,
/// until nothing more is found - a struct in one file can hold an `enum` of a
/// second that holds a struct of a third.
pub(crate) fn package_borrowing(units: &[&Parsed]) -> std::collections::BTreeSet<String> {
    let views: Vec<_> = units
        .iter()
        .map(|p| crate::check::parsed_view_of(p))
        .collect();
    nikaia_std::tools::emit_tables::emit_tables_package_borrowing(&views)
}

/// **Which branches an `overlap` starts first**, for `--overlaps`
/// ([ADR-292](../../../docs/specification/adr/adr-292.md) D6).
///
/// The report is `contracts::order`'s and the answer is the emitter's: whether a
/// branch can pause is the ledger's `sync` column and the checker's answers
/// about method calls, and no analysis in that file may reach for either
/// (ADR-292b, which is why nothing in that file takes a build setting). So
/// it is handed over as a function, closed over an emitter built exactly as the
/// one that will lower the program.
pub fn branch_starts_first<'p>(
    parsed: &'p Parsed,
    build: Build,
    contracts: &crate::contracts::Ledger,
) -> impl Fn(&Spanned<Stmt>) -> bool + 'p + use<'p> {
    let emitter = Emitter::with_contracts(
        parsed,
        &[],
        build,
        crate::contracts::Provenance::Trusted,
        contracts.clone(),
        &crate::contracts::Ledger::blank(),
        // This asks whether a branch pauses, which no file a build read can
        // change — so D1's default is the honest answer here.
        &Reads::none(),
        None,
    );
    move |stmt| emitter.branch_pauses(stmt)
}

pub(crate) fn visit_block<'a>(block: &'a Block, f: &mut impl FnMut(&'a Expr)) {
    for expr in nikaia_std::tools::emit_walk::visited_in_block(block) {
        f(expr);
    }
}

/// **An `f"…"` as the format string the language below reads**
/// ([ADR-309](../../docs/specification/adr/adr-309.md) D9): each run of text
/// as written - an escape and a doubled brace mean there what they mean here
/// - and each hole as `{}`, or `{:spec}` where a format follows its `:`.
pub(crate) fn format_of(parts: &[crate::ast::FPart]) -> String {
    nikaia_std::tools::emit_text::format_of(parts)
}

/// Every expression a literal holds - the holes of an interpolated string, and
/// the holes of a template (`tools/emit_holes.nika`). A hole that does not
/// parse yields nothing here. The Rust analyses that still take it as a closure
/// call this until they move.
pub(crate) fn literal_expressions(parsed: &Parsed, expr: &Expr) -> Vec<Expr> {
    nikaia_std::tools::emit_holes::emit_holes_literal(parsed, &parsed.interner, expr)
}

/// **The parsed file is what the walks over holes ask**
/// (`tools/emit_holes.nika`): a template's hole parsed once per text at one
/// site, and an `f"…"`'s hole with what the tier pass hands over inside it.
impl nikaia_std::tools::emit_holes::EmitHolesHost for Parsed {
    fn hole(&self, site: u32, text: String) -> nikaia_std::tools::emit_holes::EmitHole {
        match Parsed::hole(self, site, &text) {
            Ok(expr) => nikaia_std::tools::emit_holes::EmitHole::Parsed(expr),
            Err(error) => nikaia_std::tools::emit_holes::EmitHole::Refused(error.to_string()),
        }
    }

    fn wrapped_hole(&self, at: u32, hole: &Expr) -> Option<Expr> {
        let wraps = self.hole_wraps.get(&at)?;
        let mut wrapped = hole.clone();
        crate::text_tiers::wrap_hole(&mut wrapped, wraps);
        Some(wrapped)
    }

    fn cached_holes(&self) -> std::collections::BTreeMap<(i64, String), Expr> {
        self.cached_holes()
            .into_iter()
            .map(|((site, text), expr)| ((site as i64, text), expr))
            .collect()
    }

    fn cached_kept_calls(
        &self,
    ) -> std::collections::BTreeMap<(String, i64), nikaia_std::tools::emit_state::EmitKeptCall>
    {
        self.cached_kept_calls()
            .into_iter()
            .map(|((name, arity), (names, call))| {
                (
                    (name, arity as i64),
                    nikaia_std::tools::emit_state::EmitKeptCall { names, call },
                )
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// The tether: where a buffer lives, written out
// ([ADR-283](../../docs/specification/adr/adr-283.md))
// ---------------------------------------------------------------------------

impl Emitter<'_> {}

/// **The counters of a function body that are never negative**
/// ([`Emitter::nonnegative`]): a `let` of a non-negative literal whose name is
/// only ever given or added a non-negative literal, and a range's binding where
/// the range starts at one. A name declared twice, or handed to a call - which
/// might change it - is left out.
/// [`nonnegative_locals`], for [`crate::bounds`].
pub(crate) fn nonnegative_names(body: &Block) -> HashSet<Symbol> {
    nonnegative_locals(body)
}

fn nonnegative_locals(body: &Block) -> HashSet<Symbol> {
    nikaia_std::tools::emit_locals::nonnegative_locals(body)
        .into_iter()
        .collect()
}

#[cfg(test)]
mod target_tests {
    use super::Target;

    /// Every triple Rust knows is a target (ADR-037 D9); a short name and its
    /// triple are one target; a bare-metal one is refused with what is missing.
    #[test]
    fn every_rust_triple_is_a_target() {
        let android = Target::parse("aarch64-android").expect("android");
        assert_eq!(
            android,
            Target::parse("aarch64-linux-android").expect("triple")
        );
        assert_eq!(android.triple(), "aarch64-linux-android");
        assert!(android.has_threads() && android.unbuildable().is_none());
        assert_eq!(
            Target::parse("x86_64-unknown-linux-gnu").expect("host triple"),
            Target::X86_64Linux
        );
        let board = Target::parse("thumbv7em-none-eabihf").expect("bare metal");
        assert!(board.is_bare_metal() && !board.has_threads());
        assert!(board.unbuildable().is_some());
        assert!(Target::parse("z80-spectrum").is_err());
    }
}

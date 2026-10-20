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

use anyhow::{Result, anyhow};

mod host;
mod rust_targets;
pub(crate) mod seeds;
pub mod template;
use winnow_grammar::Symbol;

use crate::assets::Reads;
use crate::ast::{
    BinaryOp, Block, Expr, FoldSpec, FrameAttr, GrammarDef, GrammarRule, Item, Pattern, Repeat,
    SelectArm, Span, Spanned, Stmt, Type, UnaryOp, VariantFields,
};
use crate::parser::Parsed;
use crate::{refused, refused_at};

/// The machine a program is built for (ADR-037 D1).
///
/// It decides what `std` can offer and what a panic does. It decides nothing
/// about what a program means: the same source compiles for every target and
/// prints the same bytes.
///
/// **The default is the machine the compiler runs on** ([`Target::host`]), and
/// the triple is handed to Cargo as `--target` whichever one is named. Until
/// 0.0.218 it was only ever printed: every build was for the host, so on an ARM
/// machine `x86_64-linux` - the default - produced ARM code under an x86 name,
/// which is the one thing D1 says a target never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    X86_64Linux,
    Aarch64Linux,
    Wasm32Unknown,
    /// Any other triple Rust knows (ADR-037 D9), named as Rust names it:
    /// `aarch64-linux-android`, `thumbv7em-none-eabihf`. It is let through to
    /// the backend; what it offers is read from the triple.
    Rust(&'static str),
}

impl Default for Target {
    fn default() -> Target {
        Target::host().unwrap_or(Target::X86_64Linux)
    }
}

/// Whether the **user's** code may run concurrently at all (ADR-037 D2).
///
/// A yes-or-no question, deliberately: *how many* threads or cores serve that
/// answer is the runtime's business, and a number in the language would be a
/// promise the language cannot keep on a machine it has not seen.
///
/// It does not bind the compiler - `fs::map` may still validate its text on
/// four cores at `No`, because that is not code the user wrote and it changes
/// nothing the program prints (ADR-016 D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UserParallelism {
    /// `no` - the default. Nothing the user wrote ever runs concurrently.
    #[default]
    No,
    /// `yes` - it may, and the runtime decides how widely.
    Yes,
}

/// **Whether the program carries the runtime re-entrancy check**
/// ([ADR-281](../../docs/specification/adr/adr-281.md) D36).
///
/// Taking a lock while a lock is held is refused when the program is compiled
/// (`NK2203`, Part II 12.3), so under D2 this check cannot fire in a correct
/// compiler — and that is its role: **self-control of D2's rule, not error
/// handling.** No input can trigger it; if it fires, the compiler has a hole,
/// and without it such a hole is a silent hang instead.
///
/// **For every program that obeys the nesting rule, both builds behave
/// identically**, which is what keeps Part I 1.2's *how, never what* intact:
/// the switch decides only whether a violation is **noticed**.
///
/// It is not named *debug* and is not a development aid to be removed later. It
/// is a guarantee that can be declined, which is
/// [ADR-292](../../docs/specification/adr/adr-292.md) D7's precedent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReentrancyCheck {
    /// `yes` — the default. The program carries the check.
    #[default]
    Yes,
    /// `no` — it does not, and a hole in this compiler is a hang.
    No,
}

impl ReentrancyCheck {
    pub fn parse(value: &str) -> Result<ReentrancyCheck> {
        match value {
            "yes" => Ok(ReentrancyCheck::Yes),
            "no" => Ok(ReentrancyCheck::No),
            // **A third spelling is refused rather than guessed at**, which is
            // what `user-parallelism` does one switch over: `on` and `off` read
            // like this option and are not it, and a build that silently took
            // the default for a word it did not know would ship the guarantee
            // the manifest declined.
            other => Err(refused!(
                "`reentrancy-check` is `yes` or `no`, not `{other}`."
            )),
        }
    }

    /// Whether the emitted program carries it.
    pub fn is_on(self) -> bool {
        matches!(self, ReentrancyCheck::Yes)
    }
}

/// The build switches together (ADR-037, [ADR-281](../../docs/specification/adr/adr-281.md) D36).
///
/// One value rather than three parameters: a fourth switch is then a field, not
/// a change at every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

impl Build {
    pub fn parse(target: &str, user_parallelism: &str, reentrancy_check: &str) -> Result<Build> {
        Ok(Build {
            target: Target::parse(target)?,
            user_parallelism: UserParallelism::parse(user_parallelism)?,
            reentrancy_check: ReentrancyCheck::parse(reentrancy_check)?,
            bounds: crate::bounds::BoundsChecks::default(),
            overflow: crate::bounds::OverflowChecks::default(),
            skip_free_at_exit: false,
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
    fn parallelism(self) -> &'static str {
        self.user_parallelism.parallelism(self.target)
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

    /// The build's settings as the core asks about them, all plain values
    /// (`tools/emit_kinds.nika`).
    // The first caller is the type unit (U1a), which reads these in Nikaia.
    #[allow(dead_code)]
    pub(crate) fn for_emit(self) -> nikaia_std::tools::emit_kinds::EmitBuild {
        nikaia_std::tools::emit_kinds::EmitBuild {
            overlaps_user_code: self.overlaps_user_code(),
            user_parallelism: self.user_parallelism == UserParallelism::Yes,
            crossings_are_possible: self.crossings_are_possible(),
            counts_steps: self.counts_steps,
            parallelism: self.parallelism().to_string(),
            reentrancy: self.reentrancy_check == ReentrancyCheck::Yes,
            leaves_values: self.leaves_values,
            called_from_c: self.called_from_c,
            has_runtime: self.overlaps_operations(),
        }
    }
}

impl Target {
    /// Every target, in the order a refusal lists them.
    pub const ALL: [Target; 3] = [
        Target::X86_64Linux,
        Target::Aarch64Linux,
        Target::Wasm32Unknown,
    ];

    /// The short names first, then the short names of Android (ADR-037 D9),
    /// then any triple Rust knows, written as Rust writes it. A triple that
    /// has a short name is that target, not a second one.
    pub fn parse(name: &str) -> Result<Target> {
        if let Some(target) = Target::ALL.into_iter().find(|target| target.name() == name) {
            return Ok(target);
        }
        let triple = match name {
            "aarch64-android" => "aarch64-linux-android",
            "x86_64-android" => "x86_64-linux-android",
            "armv7-android" => "armv7-linux-androideabi",
            other => other,
        };
        if let Some(target) = Target::ALL
            .into_iter()
            .find(|target| target.triple() == triple)
        {
            return Ok(target);
        }
        match rust_targets::RUST_TARGETS
            .iter()
            .find(|known| **known == triple)
        {
            Some(known) => Ok(Target::Rust(known)),
            None => Err(refused!(
                "There's no target called `{name}`. Use x86_64-linux, aarch64-linux, \
                 wasm32-unknown, aarch64-android, x86_64-android, armv7-android, or a \
                 triple from `rustc --print target-list`."
            )),
        }
    }

    /// No operating system under the program (ADR-037 D9): Rust's `-none`
    /// triples, and UEFI. There is no `std` there to build on.
    pub fn is_bare_metal(self) -> bool {
        match self {
            Target::Rust(triple) => {
                triple
                    .split('-')
                    .any(|part| part == "none" || part.starts_with("none"))
                    || triple.ends_with("-uefi")
            }
            _ => false,
        }
    }

    /// The word a manifest and `--target` use for it.
    pub fn name(self) -> &'static str {
        match self {
            Target::X86_64Linux => "x86_64-linux",
            Target::Aarch64Linux => "aarch64-linux",
            Target::Wasm32Unknown => "wasm32-unknown",
            Target::Rust(triple) => triple,
        }
    }

    /// The triple handed to the backend: Cargo's `--target`.
    pub fn triple(self) -> &'static str {
        match self {
            Target::X86_64Linux => "x86_64-unknown-linux-gnu",
            Target::Aarch64Linux => "aarch64-unknown-linux-gnu",
            Target::Wasm32Unknown => "wasm32-unknown-unknown",
            Target::Rust(triple) => triple,
        }
    }

    /// The machine this compiler is running on. The compiler is a native
    /// program, so the triple it was built for (`build.rs`) is the machine it
    /// runs on: `x86_64-linux` on a Linux PC, `aarch64-linux-android` on a
    /// phone, `aarch64-apple-darwin` on a Mac.
    ///
    /// `None` only for a triple Rust's target list does not know: a build
    /// there has to name its target.
    pub fn host() -> Option<Target> {
        Target::parse(env!("NIKAIA_HOST_TRIPLE")).ok()
    }

    /// Whether this machine has threads at all. A target without them bounds
    /// `user_parallelism` to `0` however it is set.
    pub fn has_threads(self) -> bool {
        match self {
            Target::X86_64Linux | Target::Aarch64Linux => true,
            Target::Wasm32Unknown => false,
            Target::Rust(triple) => !self.is_bare_metal() && !triple.starts_with("wasm"),
        }
    }

    /// Whether `std`'s own runtime exists for this machine (ADR-303 D4): the
    /// I/O worker that is running before `main`, and the completion queue where
    /// the kernel has one.
    ///
    /// The same answer as [`Target::has_threads`] today and a different
    /// question, which is why it is a second method rather than a second
    /// caller. `wasm32-unknown` has neither `io_uring` nor a thread to fall
    /// back to; a machine with threads but no completion queue has the runtime,
    /// and `std` is what decides there what a pair costs.
    pub fn has_runtime(self) -> bool {
        self.has_threads()
    }

    /// What a target still needs before a program can be built for it.
    ///
    /// `Some(reason)` is a refusal that names the gap, never code emitted for
    /// a different machine (ADR-037 D1).
    pub fn unbuildable(self) -> Option<&'static str> {
        match self {
            Target::Wasm32Unknown => Some(
                "`std` reaches for a memory mapping and a thread pool, and neither \
                 exists on wasm32-unknown-unknown; what `std::fs` offers there is \
                 undecided",
            ),
            Target::Rust(_) if self.is_bare_metal() => Some(
                "there is no operating system under a program here, so no `std` to \
                 build on; how such a program starts and waits (a loop, interrupts) is \
                 a build profile's, which does not exist yet (ADR-037 D9)",
            ),
            Target::X86_64Linux | Target::Aarch64Linux | Target::Rust(_) => None,
        }
    }
}

impl UserParallelism {
    pub fn parse(value: &str) -> Result<UserParallelism> {
        match value {
            "no" => Ok(UserParallelism::No),
            "yes" => Ok(UserParallelism::Yes),
            // A number is the plausible mistake, and it has a reason rather
            // than a typo behind it. Say which.
            other if other.parse::<u32>().is_ok() => Err(refused!(
                "`user-parallelism` is `yes` or `no`, not a number: the runtime decides \
                 how many threads to use."
            )),
            other => Err(refused!(
                "`user-parallelism` is `yes` or `no`, not `{other}`."
            )),
        }
    }

    /// Whether any code the user wrote may run concurrently.
    pub fn is_concurrent(self) -> bool {
        matches!(self, UserParallelism::Yes)
    }

    /// How the generated driver is asked to cut the input.
    ///
    /// A target without threads pins this to `Off` whatever was asked for: the
    /// switch bounds what may run at once, it cannot conjure a thread the
    /// machine does not have.
    fn parallelism(self, target: Target) -> &'static str {
        if !target.has_threads() {
            return "Parallelism::Off";
        }
        match self {
            UserParallelism::No => "Parallelism::Off",
            UserParallelism::Yes => "Parallelism::Auto",
        }
    }
}

/// The lifetime the parser backend gives its input. A Nikaia view (`&str`) is
/// tied to it, which is the whole of what `&` means here: ADR-283 - a view
/// marker, never an annotation the user writes.
const INPUT_LIFETIME: &str = "'a";

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

/// **[ADR-300](../../../docs/specification/adr/adr-300.md) D9's table, as a Rust
/// item to append to a program.**
///
/// One row per line of the generated file that came from a `.nika` line: the
/// generated line, the file, and the line there. Sorted by the generated line,
/// because the lookup at run time is a binary search and sorting once here costs
/// the program nothing.
///
/// **Every mapped line, and not the ones that "can abort".** Which constructs
/// abort is a list that would have to be kept correct as the emitter grows, and
/// getting it wrong means an abort with no Nikaia line - the defect this closes.
/// The source map already knows which lines came from somewhere, so the table is
/// that knowledge kept rather than a second judgement about it. What it costs is
/// a row per line in the binary, which the record accepts: *"paid by every
/// program and needed by the ones that fail, which is the same trade a panic
/// message itself already makes."*
///
/// **Appended, never prepended.** A table written above the program would move
/// every line it names, and correcting for its own height is a circle. Rust does
/// not care where an item sits.
///
/// `paths` are the program's files, indexed the way the map indexes them.
pub fn abort_table(rust: &str, map: &SourceMap, paths: &[String], sources: &[&str]) -> String {
    // Where each line of the generated file starts, so an offset becomes a line
    // in one binary search rather than by counting newlines per entry.
    let mut starts = vec![0usize];
    starts.extend(
        rust.char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(i, _)| i + 1),
    );
    let line_of = |offset: usize| match starts.binary_search(&offset) {
        Ok(i) => i + 1,
        Err(i) => i,
    };

    // The `.nika` line of a span, per file.
    let nika: Vec<Vec<usize>> = sources
        .iter()
        .map(|source| {
            let mut starts = vec![0usize];
            starts.extend(
                source
                    .char_indices()
                    .filter(|(_, c)| *c == '\n')
                    .map(|(i, _)| i + 1),
            );
            starts
        })
        .collect();

    // **The outermost entry wins**, which is the opposite of what a lookup
    // wants. `SourceMap::locate` answers with the innermost span covering an
    // offset, and that is right for a diagnostic about a byte; a whole generated
    // *line* is better named by the statement it came from than by the
    // sub-expression that happens to start it. A `BTreeMap` keyed by the
    // generated line and filled in map order gives that: the emitter records a
    // node before anything it encloses, so the first entry for a line is the
    // outermost one.
    let mut rows: std::collections::BTreeMap<usize, (usize, usize)> =
        std::collections::BTreeMap::new();
    for entry in map.rows() {
        let generated = line_of(entry.0);
        let Some(starts) = nika.get(entry.2) else {
            continue;
        };
        let line = match starts.binary_search(&entry.1) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        rows.entry(generated).or_insert((entry.2, line));
    }

    let mut out = String::new();
    out.push_str("\n// ADR-300 D9: the generated line, and the `.nika` line it came from.\n");
    out.push_str(&format!(
        "const {ABORT_TABLE}: &[nikaia_std::abort::Site] = &[\n"
    ));
    for (generated, (unit, line)) in rows {
        let path = paths.get(unit).map(String::as_str).unwrap_or("<unknown>");
        out.push_str(&format!("    ({generated}, {path:?}, {line}),\n"));
    }
    out.push_str("];\n");
    out
}

/// Emitted byte range -> the `.nika` span that produced it.
///
/// This is the whole answer to "the error points at a line nobody wrote": the
/// compiler that consumes the emitted Rust reports offsets into it, and every
/// one of them can be traded back for a place in the source the user has open.
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    /// Innermost first: a lookup returns the first range that contains the
    /// offset, and the emitter records a node before recording anything that
    /// encloses it.
    entries: Vec<MapEntry>,
}

#[derive(Debug, Clone)]
struct MapEntry {
    generated: std::ops::Range<usize>,
    source: Span,
    /// Which file it came from, as an index into the program's units. A
    /// single-file program has one, and it is 0.
    unit: usize,
}

impl SourceMap {
    /// The narrowest source span whose emitted text covers `offset`, and the
    /// file it is in.
    pub fn locate(&self, offset: usize) -> Option<(usize, Span)> {
        self.entries
            .iter()
            .find(|e| e.generated.contains(&offset))
            .map(|e| (e.unit, e.source))
    }

    /// The narrowest source span whose emitted text covers `offset`.
    pub fn source_span(&self, offset: usize) -> Option<Span> {
        self.locate(offset).map(|(_, span)| span)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The map of an `Out`, whose entries are `usize` ranges here.
    fn from_entries(entries: Vec<nikaia_std::tools::emit_out::EmitMapEntry>) -> SourceMap {
        SourceMap {
            entries: entries
                .into_iter()
                .map(|e| MapEntry {
                    generated: e.start as usize..e.end as usize,
                    source: e.source,
                    unit: e.unit as usize,
                })
                .collect(),
        }
    }

    /// This map, as it reads once its module's text has been placed `by` bytes
    /// into a larger file - and tagged with which file it came from.
    ///
    /// A program of several modules is emitted one module at a time and joined;
    /// this is what keeps ADR-300's promise across the join. Without it a
    /// `rustc` message about the third module would be traded back for a place
    /// in the first.
    pub fn placed(mut self, by: usize, unit: usize) -> SourceMap {
        for entry in &mut self.entries {
            entry.generated = entry.generated.start + by..entry.generated.end + by;
            entry.unit = unit;
        }
        self
    }

    /// Take everything `other` holds, which is already placed.
    pub fn extend(&mut self, other: SourceMap) {
        self.entries.extend(other.entries);
    }

    /// Every entry as `(generated start, source start, unit)`, in the order the
    /// emitter recorded them - which is outermost first for any one place.
    ///
    /// For [`abort_table`], which needs all of them rather than a lookup.
    pub fn rows(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        self.entries
            .iter()
            .map(|e| (e.generated.start, e.source.at(), e.unit))
    }
}

/// The emitted text under construction, and the map being built with it
/// (`tools/emit_out.nika`).
type Out = nikaia_std::tools::emit_out::EmitOut;

/// What `std` has no entry for yet: `String::pop` and `String::insert_str`.
trait OutText {
    fn take_lend(&mut self) -> bool;
    fn insert_after_first_brace(&mut self, text: &str);
    fn insert_before_last_brace(&mut self, text: &str);
}

impl OutText for Out {
    /// **Takes back a `&` just written**, where what follows is already the
    /// reference it asked for (0.0.250): `&*get(&v, i)` is `get(&v, i)`, and
    /// the long form is what `clippy` refuses in `std`. Not `&&`, which is the
    /// operator, and not `&mut `, which ends in a space. A `&` at the end of the
    /// buffer is inside no recorded span, so the map is untouched.
    fn take_lend(&mut self) -> bool {
        if self.buf.ends_with('&') && !self.buf.ends_with("&&") {
            self.buf.pop();
            return true;
        }
        false
    }

    /// Insert text just inside the first `{`, moving the map entries after it.
    fn insert_after_first_brace(&mut self, text: &str) {
        let Some(at) = self.buf.find('{').map(|at| at + 1) else {
            return;
        };
        self.buf.insert_str(at, text);
        self.shift_entries(at as i64, text.len() as i64);
    }
    fn insert_before_last_brace(&mut self, text: &str) {
        let Some(at) = self.buf.rfind('}') else {
            return;
        };
        self.buf.insert_str(at, text);
        self.shift_entries(at as i64, text.len() as i64);
    }
}

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

/// **`std`'s own Nikaia half**, which is the one program that must not import
/// the prelude: it *is* the prelude ([ADR-014](../../../docs/specification/adr/adr-014.md)
/// D1, [ADR-002](../../../docs/specification/adr/adr-002.md) D4).
///
/// Every other program gets `pub use nikaia_std::prelude::*;` whether or not it
/// asked, because what a program uses is not a list this emitter keeps and a name
/// the prelude provides has to resolve. `std` is where that stops being true, and
/// it is one program rather than a class - so it says so here rather than being
/// detected.
pub fn emit_std(parsed: &Parsed) -> Result<Lowered> {
    let own = crate::contracts::Ledger::infer_package(&[parsed], std_ledger());
    emit_std_against(parsed, &[], &own, &crate::contracts::Ledger::blank())
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

/// [`emit_std_against`]'s items and nothing else, for one file of the
/// toolchain's package (ADR-294): the package writes one preamble for all of
/// them.
pub fn emit_std_items_against(
    parsed: &Parsed,
    beside: &[&Parsed],
    own: &crate::contracts::Ledger,
    described: &crate::contracts::Ledger,
) -> Result<Lowered> {
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
    let provenance = crate::contracts::trust::analyse_package(&package, &library).provenance;
    emit_std_items_against_at(parsed, beside, own, described, build, provenance, None)
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

/// A module's items, with no preamble and no `mod` around them.
///
/// The driver writes the preamble once for the whole program and the `mod`
/// header itself, because only it knows what the program is made of
/// (`modules::collect`).
///
/// `contracts` are the **program's**, not this file's: a call to
/// `page::render(entries, total)` fills in Kap 5.1's defaults from the
/// declaration, and the declaration is in another file.
pub fn emit_module_body(
    parsed: &Parsed,
    build: Build,
    provenance: crate::contracts::Provenance,
    contracts: &crate::contracts::Ledger,
) -> Result<Lowered> {
    emit_module_body_at(
        parsed,
        &[],
        build,
        provenance,
        contracts,
        &crate::contracts::Ledger::blank(),
        false,
        &Reads::none(),
        None,
    )
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

/// **Whether a package has `pub` items with map positions** (ADR-010 D9): the
/// ones a program that uses it fills with its own hasher, and so the ones whose
/// body must be as trusted as the program that fills them.
pub fn holds_map_positions(package: &[&Parsed]) -> bool {
    let seeds = seeds::Seeds::read(package);
    !seeds.is_empty() || seeds.trait_maps()
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
    seeds::Seeds::read_with(units, &described_maps(described)).barrier_names()
}

/// The functions of a described Rust crate with a map in their signature.
fn described_maps(described: &crate::contracts::Ledger) -> Vec<String> {
    described
        .functions
        .iter()
        .filter(|(_, c)| {
            c.signature.as_ref().is_some_and(|s| {
                let text = s.text();
                text.contains("HashMap") || text.contains("HashSet")
            })
        })
        .map(|(key, _)| key.rsplit("::").next().unwrap_or(key).to_string())
        .collect()
}

/// What a program's preamble has to say, over all of its files.
///
/// `uses_std` and the rest are per-file facts, and the preamble is written
/// once - so they are joined here rather than guessed at from the entry.
#[derive(Debug, Clone, Default)]
pub struct Needs {
    pub grammar: bool,
    pub driver: bool,
    pub std: bool,
    /// Kap 7.1: a function here declares `throws`, so the error surface is
    /// reachable and `std`'s is what carries it.
    pub fails: bool,
    /// Kap 4.7 across a package: every trait an `impl` in this unit names with
    /// a package in front of it
    /// ([ADR-286](../../docs/specification/adr/adr-286.md)).
    ///
    /// Rust needs a trait **in scope** before its methods can be called, and
    /// `impl http::Handler for Fixed` does not put it there. So a trait a
    /// package publishes could be implemented and then not called, in the
    /// backend's words about the generated file: *"trait `Handler` … is
    /// implemented but not in scope; perhaps you want to import it"* — a rule
    /// the program has no way to satisfy, because Nikaia has no import to
    /// write ([ADR-286](../../docs/specification/adr/adr-286.md) D11 brings no
    /// names in).
    ///
    /// The **same shape in one file compiles**, which is what says this is one
    /// missing emitted line rather than a question: there the trait is in the
    /// same module and needs no import.
    pub foreign_traits: std::collections::BTreeSet<String>,
}

impl Needs {
    pub fn of(parsed: &Parsed) -> Needs {
        // **Read off the items, with no emitter built** (#727): what a
        // program needs is which items it declares and which grammar entries
        // its functions enter, the same reading `Emitter::with_contracts`
        // makes of them, and building an emitter for it inferred the ledger
        // and ran the checker and the prover once more per file.
        let mut grammars: std::collections::BTreeMap<String, GrammarDef> =
            std::collections::BTreeMap::new();
        let (mut std, mut fails) = (false, false);
        for item in &parsed.program.items {
            match &item.node {
                Item::Grammar(def) => {
                    grammars.insert(parsed.text(def.name).to_string(), def.clone());
                }
                Item::Import { .. } => std = true,
                Item::Fn {
                    can_throw: true, ..
                } => fails = true,
                _ => {}
            }
        }
        let world = EmitWorld {
            words: &parsed.interner,
            aliases: &parsed.aliases,
            items: &parsed.program.items,
            library: std_ledger(),
        };
        Needs {
            grammar: !grammars.is_empty(),
            driver: nikaia_std::tools::emit_below::emit_uses_driver(&world, &grammars),
            std,
            fails,
            foreign_traits: foreign_traits(parsed),
        }
    }

    pub fn join(mut self, other: Needs) -> Needs {
        self.foreign_traits.extend(other.foreign_traits);
        Needs {
            grammar: self.grammar || other.grammar,
            driver: self.driver || other.driver,
            std: self.std || other.std,
            fails: self.fails || other.fails,
            foreign_traits: self.foreign_traits,
        }
    }

    /// The lines at the top of the emitted file.
    pub fn preamble(self) -> String {
        let mut out = String::new();
        if self.grammar {
            out.push_str("use winnow_grammar::grammar;\n");
        }
        if self.driver {
            out.push_str("use winnow_grammar::rt::Parallelism;\n");
            out.push_str("use winnow_grammar::ParseContext;\n");
        }
        // **Always, and allowed to be unused.** It used to be written only where
        // the program had a `use std::…` line of its own, which is not the same
        // question: `HashMap` is a name the prelude provides and a program may
        // write it without importing anything, and such a program lowered to a
        // file where `TrustedMap` was undeclared - `rustc` about a file nobody
        // wrote (Part III, C.1). What a program uses is not a list this emitter
        // keeps, and it does not need one: the import is one line, a locally
        // declared name shadows a glob, and `#[allow]` is what keeps an unused
        // one from reaching a reader (the same move the emitted `use super::*`
        // made).
        // Nikaia's `std` is a crate rather than a table in this file, so
        // `fs::map`, `cli::args` and `HashMap` resolve as written and what
        // they mean is code someone can read.
        // `pub use`, because the grammar module the backend generates
        // reaches these names through a glob of its own, and a private
        // import is not re-exported into one.
        out.push_str("#[allow(unused_imports)]\npub use nikaia_std::prelude::*;\n");
        if self.fails {
            // Kap 7.1: `error.full()` is a method on whatever a `catch` bound, so
            // the trait has to be in scope even in a program that imports no `std`
            // module of its own - `throw` needs `std` whether or not the source
            // mentions it.
            out.push_str("#[allow(unused_imports)]\npub use nikaia_std::error::Full;\n");
        }
        // **A trait reached across a package is imported**
        // ([ADR-286](../../docs/specification/adr/adr-286.md)). Written as the
        // path the program already wrote, because that is the path the
        // generated crate resolves: `impl http::Handler for Fixed` names the
        // crate `http`, and so does this.
        //
        // `allow(unused_imports)` for the same reason the two lines above carry
        // it - a program may implement a trait and never call the method, and a
        // warning about a line nobody wrote is
        // [Part III C.1](../../docs/specification/30-nikaia-tooling.md) one
        // severity down.
        for path in &self.foreign_traits {
            out.push_str(&format!("#[allow(unused_imports)]\npub use {path};\n"));
        }
        out
    }
}

/// Every trait an `impl` in this unit names with a package in front of it.
///
/// **Qualified only**, and that is the whole of the test: a trait of this
/// program's own is in the same module below and needs no import, while a
/// `::` in the name is exactly what says the declaration is in another crate.
/// A trait the compiler reads rather than a `.nika` file wrote - `impl Error
/// for ConfigError` ([ADR-023](../../docs/specification/adr/adr-023.md) D3) -
/// carries no `::` either, so it is left alone by the same test.
/// What each `extern "C"` declaration in this file takes
/// ([ADR-302](../../docs/specification/adr/adr-302.md) D5), by the name a call
/// writes.
fn foreign_params(parsed: &Parsed) -> std::collections::BTreeMap<String, Vec<Type>> {
    nikaia_std::tools::emit_items::foreign_params(&parsed.interner, &parsed.program.items)
}

/// The opaque handles this file declares, by name, with the function that ends
/// each one's life ([ADR-302](../../docs/specification/adr/adr-302.md) D7).
fn opaque_handles(parsed: &Parsed) -> std::collections::BTreeMap<String, String> {
    nikaia_std::tools::emit_items::opaque_handles(&parsed.interner, &parsed.program.items)
}

/// What each `extern "C"` declaration hands back, by the name a call writes
/// ([ADR-302](../../docs/specification/adr/adr-302.md) D12).
fn foreign_results(parsed: &Parsed) -> std::collections::BTreeMap<String, Type> {
    nikaia_std::tools::emit_items::foreign_results(&parsed.interner, &parsed.program.items)
}

fn foreign_traits(parsed: &Parsed) -> std::collections::BTreeSet<String> {
    nikaia_std::tools::emit_items::foreign_traits(&parsed.interner, &parsed.program.items)
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

/// `<'a, T>`, or nothing at all where there is nothing to declare.
///
/// One place, because three positions write one - a `fn`, a `struct` and the
/// `impl` over it - and a list that is empty has to be written as the empty
/// string rather than as `<>`, which is not Rust.
fn angled(parts: &[String]) -> String {
    match parts.is_empty() {
        true => String::new(),
        false => format!("<{}>", parts.join(", ")),
    }
}

use nikaia_std::tools::emit_state::{EmitPair, EmitState, EmitWalk, EmitWorld};

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
    /// **What a give-up lowers to** (ADR-353 D3, `view_changes`): an
    /// assignment that becomes a binding of its own, and the `let`s that need
    /// no `mut` because of it.
    give_ups: crate::view_changes::GiveUps,
    /// Where the parameters of the type being written come from.
    place: std::cell::RefCell<seeds::Place>,
    /// A signature of a barrier is being written: its maps are the keyed ones.
    floor: std::cell::Cell<bool>,
    /// A grammar's copy over bytes is being written (ADR-366 D2): what Nikaia
    /// types as text is taken through the backend's checking `text(...)`.
    grammar_bytes: std::cell::Cell<bool>,
    /// Inside `bytes(...)` of a grammar's copy over bytes: `any` there is the
    /// backend's raw `byte`, since what is captured is not checked.
    grammar_raw: std::cell::Cell<bool>,
    /// **The counters `bounds::proven` already found never negative**, by the
    /// address of the body in `parsed` (`Proven::nonnegative_locals`).
    nonnegative_bodies: HashMap<usize, HashSet<Symbol>>,
    /// **The program's `main`'s parameters as its definition wrote them**,
    /// `name: type`, for the entry that reads the words into them
    /// ([ADR-360](../../docs/specification/adr/adr-360.md) D1).
    main_params: std::cell::RefCell<Vec<String>>,
}

/// What an `impl` says about a method. Enough to adapt a `&mut self` method to
/// the monoid a `par_fold` needs, and never more than the declaration states.
use nikaia_std::tools::emit_kinds::EmitMethod as Method;

/// How `Self::dsl` is spelled in the source, and what it becomes below.
///
/// ADR-296 D5 gives a driver one way to name the type a DSL string generates,
/// and the emitted Rust needs a name for it that no program can collide with.
const SELF_DSL: &str = "Self::dsl";
const DSL_PARAMETER: &str = "NikaiaDsl";

/// **The name a length a parameter left open is declared under**
/// ([ADR-184](../../docs/specification/adr/adr-184.md) D3).
///
/// `xs: Array[T]` says *an array of any length*, and the call is what says
/// which — so the language below gets a `const` parameter and the position it
/// belongs to, which is what makes two arrays in one signature two lengths.
/// The word is this compiler's and no program can collide with it: a Nikaia
/// name cannot begin with `__`.
const LENGTH_PARAMETER: &str = "__NIKAIA_N";

/// The name a Nikaia program gives its entry point.
const MAIN: &str = "main";

/// **The allocator of a program built with `skip-free-at-exit:on`**: the
/// system's, with a free that it skips once `main` has ended
/// ([ADR-348](../../docs/specification/adr/adr-348.md) D1). Written here and
/// not in `std`, which has no `unsafe` (ADR-218).
const ALLOCATOR: &str = "
struct NikaiaHeap;
// SAFETY: every call is the system allocator's; a free that is skipped leaves
// the block allocated until the process ends, which is always sound.
unsafe impl std::alloc::GlobalAlloc for NikaiaHeap {
    #[inline]
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, layout) }
    }
    #[inline]
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        unsafe { std::alloc::GlobalAlloc::alloc_zeroed(&std::alloc::System, layout) }
    }
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        if nikaia_std::rt::exit::frees() {
            unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, ptr, layout) }
        }
    }
    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        unsafe { std::alloc::GlobalAlloc::realloc(&std::alloc::System, ptr, layout, size) }
    }
}
#[global_allocator]
static NIKAIA_HEAP: NikaiaHeap = NikaiaHeap;
";
/// **A precondition checked where it stands** (ADR-269 D20), and what its
/// failure says (ADR-269 D2): the condition, the `assert`'s message, and the
/// value of each parameter it reads.
fn failed_check(check: &crate::prove::Check) -> String {
    let message = match &check.message {
        Some(text) => format!("Some(&{text:?})"),
        None => "None".to_string(),
    };
    let operands: String = check
        .operands
        .iter()
        .map(|(name, value)| {
            format!("({name:?}, (&&nikaia_std::abort::Operand(&{value})).shown()), ")
        })
        .collect();
    format!(
        "if !{} {{ nikaia_std::abort::assertion_failed({:?}, {message}, &[{operands}]) }} ",
        check.rust, check.written
    )
}

/// The suffix of a function's unchecked entry
/// ([ADR-269](../../docs/specification/adr/adr-269.md) D20): `f` checks its
/// precondition and calls `f__unchecked`, which is the body.
const UNCHECKED: &str = "__unchecked";

/// The type several parts of a program own at once (Part I 6.2).
///
/// The one name whose lowering is decided per **value**: what it expands to is a
/// reference count, and which of the two a particular value gets is
/// `contracts::sharing`'s answer ([ADR-037](../../../docs/specification/adr/adr-037.md)
/// D7).
use crate::contracts::ty::ARRAY;

/// **The functions of this file the build calls** (#446): what a `comptime`
/// and an option's default call, and what those call in turn.
///
/// Their values reach the Rust as literals, so the Rust may never call them,
/// and `rustc` would say *never used* about a function the program used. Each
/// gets `#[allow(dead_code)]`; a function nothing calls is still warned.
fn built_at_build_time(parsed: &Parsed) -> HashSet<String> {
    use crate::contracts::sync::{visit_expr, visit_expr_blocks, visit_stmt, visit_stmt_blocks};

    fn calls(parsed: &Parsed, expr: &Expr, found: &mut Vec<String>) {
        visit_expr(parsed, expr, &mut |inner| {
            if let Expr::Call { func, .. } = inner
                && let Expr::Variable(name, _) = func.as_ref()
            {
                found.push(parsed.text(*name).to_string());
            }
        });
        visit_expr_blocks(expr, &mut |block| block_calls(parsed, block, found));
    }
    fn block_calls(parsed: &Parsed, block: &Block, found: &mut Vec<String>) {
        for stmt in &block.stmts {
            visit_stmt(parsed, &stmt.node, &mut |expr| calls(parsed, expr, found));
            visit_stmt_blocks(&stmt.node, &mut |inner| block_calls(parsed, inner, found));
        }
    }
    fn comptimes(parsed: &Parsed, block: &Block, found: &mut Vec<String>) {
        for stmt in &block.stmts {
            if let Stmt::Comptime { value, .. } = &stmt.node {
                calls(parsed, value, found);
            }
            visit_stmt_blocks(&stmt.node, &mut |inner| comptimes(parsed, inner, found));
        }
    }

    let mut bodies: HashMap<String, &Block> = HashMap::new();
    let mut waiting: Vec<String> = Vec::new();
    let mut functions: Vec<(&Item, bool)> = Vec::new();
    for item in &parsed.program.items {
        match &item.node {
            Item::Comptime { value, .. } => calls(parsed, value, &mut waiting),
            Item::Impl { methods, .. } => {
                functions.extend(methods.iter().map(|method| (&method.node, false)));
            }
            other => functions.push((other, true)),
        }
    }
    for (item, free) in functions {
        let Item::Fn {
            name, config, body, ..
        } = item
        else {
            continue;
        };
        for option in config {
            calls(parsed, &option.default, &mut waiting);
        }
        comptimes(parsed, body, &mut waiting);
        if let Some(name) = name
            && free
        {
            bodies.insert(parsed.text(*name).to_string(), body);
        }
    }
    let mut built = HashSet::new();
    while let Some(name) = waiting.pop() {
        if let Some(body) = bodies.get(&name)
            && built.insert(name)
        {
            block_calls(parsed, body, &mut waiting);
        }
    }
    built
}

/// **`std`'s own handle** ([ADR-302](../../docs/specification/adr/adr-302.md)
/// D4): text a C library owns, which no `extern "C"` block declares because
/// `std` does.
const C_STRING: &str = "CStr";

/// A written type's own name, with the module it is reached through taken off
/// ([ADR-313](../../docs/specification/adr/adr-313.md) D3).
///
/// `foreign::CStr` is what a declaration writes now; what this file asks about
/// is the **type**, which is the last segment. The full name is what goes into
/// the emitted Rust, because the module is in `std`'s own prelude there.
fn base(name: &str) -> &str {
    crate::contracts::ty::base(name)
}

/// The call that never comes back
/// ([Part III A.2](../../docs/specification/30-nikaia-tooling.md), Part I 1.3).
const PANIC: &str = "panic";

/// Where a write through the brackets holds its value while the read in it
/// finishes ([ADR-293](../../docs/specification/adr/adr-293.md) D2).
const STORED: &str = "__nikaia_stored";

/// The name an arm's value is bound to before it is put back in an `Ok`
/// ([ADR-292](../../docs/specification/adr/adr-292.md) D18).
///
/// **The binding is what keeps `rustc` quiet.** `Ok({ … })` around a block
/// holding one expression is `unused_braces` — a warning about a file nobody
/// wrote, which is [Part III C.1](../../docs/specification/30-nikaia-tooling.md)
/// one severity down. A block with a `let` in it is a block the lint has
/// nothing to say about.
const ARM_VALUE: &str = "__nikaia_value";

/// The name a **pausing** sequence is bound to for the length of its loop
/// ([ADR-172](../../docs/specification/adr/adr-172.md) D1).
///
/// A `while let` steps a value rather than consuming an expression, so the
/// sequence needs a name — and a generated one, for the reason every other
/// generated name here has: a program may already have bound `lines`.
const SEQUENCE: &str = "__nikaia_sequence";

/// The slot a function's result is filed under, as `contracts::sharing` keys it.
/// Whether a result whose source set is empty is written `'static`.
const EMPTY_SET_IS_STATIC: bool = true;

const SHARED_RESULT: &str = "<result>";

/// The pseudo-function a `<struct>.<field>` slot is filed under, the same.
const SHARED_FIELDS: &str = "<field>";

/// The slot a part of a variant is filed under, beside a struct's
/// `<struct>.<field>`: `E.Add#0` (0.0.244).
fn variant_slot(owner: &str, variant: &str, at: usize) -> String {
    nikaia_std::tools::emit_items::variant_slot(owner, variant, at as i64)
}

/// What the program's own `main` is called in the emitted Rust.
///
/// `fn main` belongs to the runtime now
/// ([ADR-303](../../../docs/specification/adr/adr-303.md) D4): it starts the
/// I/O worker, calls this, and drains. The name is spelled so that no Nikaia
/// program plausibly collides with it, and a `rustc` diagnostic about the
/// program's body still lands on the `.nika` source because the body's spans
/// are unchanged (ADR-300).
const PROGRAM_MAIN: &str = "__nikaia_main";

/// The generated name of [ADR-300](../../../docs/specification/adr/adr-300.md)
/// D1's table. `__nikaia_` for the same reason `__nikaia_main` is: a name a
/// program could also have chosen would be a name this compiler took from it.
const ABORT_TABLE: &str = "__NIKAIA_SITES";

/// What the **last** statement of a block is, which is two questions and not
/// one.
///
/// Nikaia's blocks are expressions (Part I, 3.1), so a block's last statement
/// is usually its value. That much was a `bool`. What a `bool` could not say is
/// *whose* value it is, and exactly one rewrite in this file needs to know:
/// `return x` at the end of a function body is written as `x`, because the
/// value a function body ends in **is** what the function hands back.
///
/// That is true of a function body. It is not true of a `match` arm, a `catch`
/// handler, or a block used as a value - their last statement is the value of
/// the *expression around them*, and a `return` inside one leaves the function
/// past it. Writing such a `return` as a bare value is a different program:
/// where the function returns something it is rustc's `E0308` about a file
/// nobody wrote, and where it returns nothing **nothing complains at all** and
/// the function runs on past the point the source said to leave
/// ([`docs/history/nightly-cost.md`](../../../docs/history/nightly-cost.md) §3.3).
///
/// So the rewrite is allowed where this says [`Tail::Return`] and nowhere else.
/// Everywhere else a `return` stays a `return`, which is always a legal Rust
/// statement and always means what the source meant - ADR-296 D17's "the
/// lowering is syntactic and never guesses a meaning", applied to the one place
/// it had been guessing.
use nikaia_std::tools::emit_kinds::EmitTail as Tail;

/// Which type a method belongs to, and which trait declared it.
///
/// `target` is what makes the ledger key — `Counter::record` rather than
/// `record` — and the key is what a `Shared` position looks its count up by
/// ([`crate::contracts::sharing`]).
///
/// `declared_by` is the trait an `impl` names, where it names one, and it
/// exists because [ADR-288](../../docs/specification/adr/adr-288.md) D25 makes
/// the **declaration** the wider claim: a trait method without `sync` may
/// pause, so its implementations are `async fn` whether or not their own
/// bodies do. A body that never pauses under such a declaration is correct and
/// still has to hand back a future.
#[derive(Clone, Copy)]
struct MethodOf<'a> {
    target: &'a str,
    declared_by: Option<&'a str>,
    /// **The own half of a bridged method** (ADR-288 D2, D25): `Some` where
    /// the method is narrower than the trait's declaration (`sync`, or no
    /// `throws`) and is written into an inherent `impl`, which a call on the
    /// concrete type reaches; the trait's `impl` is then a bridge to it. It
    /// keeps the trait's parameter shapes and channel, and is `async` only if
    /// its own body pauses. The flag is whether the type is `pub`, which makes
    /// the method `pub`.
    bridged: Option<bool>,
}

/// What the declaration a body belongs to says about that body.
///
/// These four travel together because the statements inside a body cannot see
/// any of them: the `Ok` a `throws` function's value is wrapped in, whether
/// there is a value at all, which of its parameters a call has to await, and
/// the ledger key the body is filed under. They are one argument rather than
/// four for the same reason `Argument` is one - a function that takes eight
/// loose values is a function whose callers are easy to get wrong.
#[derive(Clone, Copy)]
struct Declared<'a> {
    /// The function may fail, so its value is handed back as an `Ok`.
    throws: bool,
    /// The function hands back a value at all, so its last statement is that
    /// value rather than a statement like any other.
    returns_value: bool,
    /// The code parameters of this declaration whose type may pause
    /// ([ADR-277](../../docs/specification/adr/adr-277.md) D11): a call to one
    /// of these carries an `.await`.
    awaited: &'a [Symbol],
    /// The ledger key of the function these statements are in: `main`, or
    /// `Counter::record` for a method. Two things read it - a `Shared` value's
    /// count is filed under it (`contracts::sharing`), and a `throw` raised
    /// here reports the function's **own** name, which is the key's last
    /// segment (ADR-023 D6: a `throw` in `main` says `main`).
    key: &'a str,
    /// The type this function's failures travel in, exactly as the signature
    /// above wrote it ([ADR-292](../../docs/specification/adr/adr-292.md) D17).
    ///
    /// A vehicle that takes fallible branches - `overlap`, `select` - has to
    /// name an error type for the `async` blocks it writes, because one with a
    /// `?` in it and nothing to infer from is *type annotations needed* about a
    /// file nobody wrote. Naming the box was right while every channel was one;
    /// since [ADR-280](../../docs/specification/adr/adr-280.md) a channel can
    /// be a `Thrown<E>`, a library's type bare or a generated sum, and the `?`
    /// on the branch has to convert into whichever it is.
    channel: &'a str,
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

/// What surrounds the statements being emitted.
#[derive(Debug, Clone, Copy)]
struct Flow<'a> {
    /// Kap 7.1: the enclosing function is `throws`, so a `return` carries `Ok`.
    throws: bool,
    /// The function a `throw` inside this flow is raised from.
    ///
    /// ADR-023 D6 wants a raise site keyed by what the source says rather than
    /// by where it sits, so that an edit above it does not move it. A name is
    /// the part of that key this compiler has; the ordinal within the function
    /// waits on the mark table.
    origin: &'a str,
    /// Kap 7.1: what is being emitted is the guarded half of a `catch`, so a
    /// failure in it is handled here rather than propagated.
    ///
    /// It reaches inward for the same reason `sequential` does: in
    /// `outer(inner()) catch { … }` the handler runs for either call, so
    /// neither propagates. The handler's own body is emitted with the flow the
    /// `catch` was written in, because a failure raised *there* leaves the
    /// function like any other.
    caught: bool,
    /// What is being emitted is a `catch`'s **handler**, and the error it was
    /// handed came through a **named** channel
    /// ([ADR-280](../../../docs/specification/adr/adr-280.md) D10).
    ///
    /// One statement in a handler reads it: `throw error`, which passes the
    /// error on with the site it was raised at rather than wrapping it again.
    /// It does **not** reach inward past a function boundary, for the reason
    /// the binding does not either: a lambda inside a handler has its own
    /// `error` or none.
    caught_named: bool,
    /// The **sum** the error a handler was handed travels in, where it is one
    /// ([ADR-280](../../../docs/specification/adr/adr-280.md) D17).
    ///
    /// `match error { … }` reads it: the patterns the source writes name
    /// variants of the sum's **members**, so the match has to be taken apart by
    /// member, and the sum's own name is what the outer arms are written with.
    caught_sum: Option<&'a str>,
    /// The **member** of that sum whose arm is being written, where one is
    /// ([ADR-280](../../../docs/specification/adr/adr-280.md) D17).
    ///
    /// `throw error` reads it: inside a member's arm the handler holds the
    /// member, so passing the error on is putting it back in the variant it
    /// came out of.
    caught_member: Option<&'a str>,
    /// What is being emitted is a **place** rather than a value
    /// ([ADR-293](../../../docs/specification/adr/adr-293.md) D4).
    ///
    /// The left of an assignment is the only one: `self.bodies[0].vx = …`
    /// writes **through** the index, so the brackets there are the language
    /// below's own and not a read. A read is what answers a `T?` on a map; a
    /// place has to stay a place or there is nothing to assign to.
    in_a_place: bool,
    /// The `match` being emitted is a **statement**: its arms hand nothing
    /// back, so a block arm's last statement keeps its semicolon, as an `if`
    /// in statement position does. Without it `ledger.sources.insert(…)` at
    /// the end of an arm was the arm's value (found moving the ledger's reader
    /// into Nikaia, ADR-294).
    arms_are_statements: bool,
    /// The byte the statement being emitted starts at.
    ///
    /// It is here because the checker's answer about method calls is keyed by
    /// it (`check::Checked::fallible_methods`), and an expression has no span
    /// of its own to look itself up by. `stmt` sets it for every statement it
    /// emits, so a statement nested in a block carries its own and not the
    /// outer one's - which is what the checker records, because it walks
    /// statements the same way.
    statement: usize,
    /// The key the ledger records the function being emitted under - `main`, or
    /// `Counter::record` for a method.
    ///
    /// It is here for the same reason `statement` is: the answer about a
    /// `Shared` value is keyed by the function it is written in and the name the
    /// source gives it (`contracts::sharing`), and a `let` has no way to look
    /// itself up without knowing which function it stands in. `origin` is the
    /// name a `throw` reports and is deliberately not this: an error raised in a
    /// method says the method's own name (ADR-023 D6), and a ledger key says
    /// `Type::method`.
    function: &'a str,
    /// This expression **is** the guarded half of a `catch`, rather than
    /// sitting somewhere inside it
    /// ([ADR-292](../../../docs/specification/adr/adr-292.md) D18).
    ///
    /// `caught` reaches inward, because in `outer(inner()) catch { … }` the
    /// handler runs for either call. This does not: it is set on the one
    /// expression the `match` is written around, and only where that
    /// expression is a block that joins. A joining block's outcome is a
    /// `Result` the handler consumes, and one nested in an argument is a tuple
    /// like any other value.
    handled_here: bool,
    /// The type the enclosing function's failures travel in
    /// ([ADR-292](../../../docs/specification/adr/adr-292.md) D17).
    ///
    /// Read by the two vehicles that wrap a fallible branch in an `Ok`:
    /// `overlap` and `select` write the error type out, and it has to be the
    /// one the `?` below them converts into. It reaches inward like `function`
    /// does and for the same reason - a nested block is still that function -
    /// and `Flow::PLAIN`'s box is what a context with no function around it
    /// gets.
    channel: &'a str,
    /// What is being emitted sits inside a **lambda's body**
    /// ([ADR-055](../../../docs/specification/adr/adr-055.md) §6).
    ///
    /// A lambda is a closure in the language below, and Rust has no stable
    /// `async` closure - so a call that pauses cannot be written inside one, and
    /// the lowering says so in Nikaia's words rather than handing `rustc` a file
    /// nobody wrote (Part III, C.1).
    ///
    /// **It is here and not in the checker on purpose.** A lambda that reads a
    /// file is a correct Nikaia program - `examples/fortunes.nika`'s route
    /// handler is one - and refusing a correct program is the one thing the
    /// compiler may never do (Part III, C.4). So the type check accepts it and
    /// the *lowering* is what cannot write it, which is a limit of this compiler
    /// and is where a program that is only checked never meets it.
    ///
    /// It reaches inward the way `sequential` and `caught` do: a call written in
    /// a block inside the lambda is still written inside the lambda. A function
    /// body starts a `Flow` of its own, which is the boundary it does not cross.
    in_lambda: bool,
    /// The lambda parameters written **`mut`** that are in scope here
    /// ([ADR-281](../../docs/specification/adr/adr-281.md) D12).
    ///
    /// `kasse.update fn(mut v) { v += 100 }` lowers to a closure whose
    /// parameter is `&mut T`, because D1 hands the block the **address** in the
    /// lock. So every mention of `v` in the body is written `(*v)`: `+=` and an
    /// assignment need the dereference outright, and a method call and a field
    /// read would get it from Rust anyway — writing it everywhere is one rule
    /// rather than a list of positions, and a parenthesis nobody needs is a
    /// warning about the generated file (Part III C.1), which is why the one
    /// here is always needed.
    ///
    /// **It accumulates rather than replacing**, so a lambda written inside an
    /// `update` block does not lose the dereference for a name it captures.
    changed: &'a [Symbol],
    /// The **code parameters whose type may pause**
    /// ([ADR-277](../../../docs/specification/adr/adr-277.md) D11) that are in
    /// scope here.
    ///
    /// Such a parameter lowers to a closure returning a boxed future, so a call
    /// to it is a call that hands back a future and carries an `.await`. The
    /// emitter can answer this one itself — the type is written in the
    /// declaration it is standing in, which is not the case for anything
    /// `pausing_key` looks up ([ADR-288](../../../docs/specification/adr/adr-288.md)).
    ///
    /// **It does not accumulate the way `changed` does.** A lambda written
    /// inside the body has parameters of its own, and a name it binds is not
    /// this function's parameter any more; the list is the declaration's and
    /// stops at every boundary that starts a `Flow::PLAIN`.
    awaited: &'a [Symbol],
    /// The name the statement being emitted **binds**, where it binds one.
    ///
    /// It is here for the reason `function` is: the count a `Shared` was given is
    /// keyed by the function and the name (`contracts::sharing`), and a hull
    /// written by a **call** ([ADR-281](../../../docs/specification/adr/adr-281.md)
    /// D2) has to look that up from inside an expression. An expression carries no
    /// span and no name of its own.
    ///
    /// Empty where the statement binds nothing, which is the floor's case and
    /// never a wrong answer - only a slower one.
    bound: &'a str,
    /// What is being emitted is **inside a constant this pass decided to write
    /// in the wider type** ([ADR-285](../../../docs/specification/adr/adr-285.md)
    /// D1), so every integer literal in it carries the `i64` suffix.
    ///
    /// It reaches inward, and it can only reach literals: an expression folds
    /// only if it is built from literals and arithmetic, so nothing else is
    /// down there to be reached. It is set on the **outermost** expression that
    /// folds, which is why the arms below ask for it before folding again - one
    /// decision per expression, not one per operator.
    widen: bool,
    /// What is being emitted stands where **Rust's own inference gives the
    /// number its type** - a sequence index and a repeat count, both `usize`.
    ///
    /// Those are the two positions a literal is deliberately left bare in
    /// (`index::at` and `count::of` are skipped there, because `at(0)` has
    /// nothing to infer from), and a suffix written into one would pin the type
    /// the position is supposed to decide. So `widen` does not start here, and
    /// what an index that overflows an `i32` gets is the message it gets today
    /// rather than a worse one.
    inferred: bool,
    /// What is being emitted sits inside the **body of a loop** - so a `break`
    /// or a `continue` here has somewhere to go (Part I, 3.3).
    ///
    /// **The backstop, and the checker is the diagnostic.** `NK1132` is what a
    /// program actually meets, and it is the one worth writing because it names
    /// which of a lambda, a task, an `overlap` branch or a fold's step stands in
    /// the way. This is the guarantee underneath it: a walk that missed a
    /// corner would otherwise put `break;` into a closure and let `rustc` answer
    /// about a file nobody wrote (Part III, C.1), and *"the checker's walk is
    /// complete"* is a thing to hope for rather than a thing that holds. Every
    /// statement is emitted through one place, so asking here cannot be evaded.
    ///
    /// It reaches inward the way `caught` and `in_lambda` do, and it stops at
    /// exactly the constructs that are a function below: each of them starts
    /// from [`Flow::PLAIN`], where this is false.
    in_loop: bool,
    /// **The `catch` a failing call breaks out to** (#721,
    /// `EmitFlow::breaks_to`): the node of the expression it guards, `-1`
    /// where none is around.
    breaks_to: i64,
    /// The channel a `task::scope`'s tasks fail in, and whose it is
    /// (`EmitScope::tasks_channel`, ADR-346 D7).
    tasks_channel: &'a str,
    tasks_function: &'a str,
}

impl<'a> Flow<'a> {
    const PLAIN: Flow<'static> = Flow {
        tasks_channel: "",
        tasks_function: "",
        changed: &[],
        awaited: &[],
        throws: false,
        origin: "",
        caught: false,
        caught_named: false,
        caught_sum: None,
        caught_member: None,
        in_a_place: false,
        arms_are_statements: false,
        statement: usize::MAX,
        function: "",
        channel: "Box<dyn std::error::Error>",
        handled_here: false,
        in_lambda: false,
        bound: "",
        widen: false,
        inferred: false,
        in_loop: false,
        breaks_to: -1,
    };

    /// The same surroundings, for the body of a loop.
    fn inside_a_loop(self) -> Self {
        Flow {
            in_loop: true,
            ..self
        }
    }

    /// The left of an assignment
    /// ([ADR-293](../../../docs/specification/adr/adr-293.md) D4).
    fn place(self) -> Self {
        Flow {
            in_a_place: true,
            ..self
        }
    }

    /// The same surroundings, for the statement that starts at this byte.
    fn at(self, statement: usize) -> Self {
        Flow { statement, ..self }
    }

    /// The same surroundings, for a statement that binds this name.
    fn binding<'b>(self, bound: &'b str) -> Flow<'b>
    where
        Self: 'b,
    {
        Flow { bound, ..self }
    }

    /// The same surroundings, where the position decides the number's type.
    fn inferred(self) -> Self {
        Flow {
            inferred: true,
            ..self
        }
    }
}

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
    /// a file's.
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
            seeds::Seeds::read_with(&all, &described_maps(described))
        };
        let mut grammars: std::collections::BTreeMap<String, GrammarDef> =
            std::collections::BTreeMap::new();
        let mut structs = HashSet::new();
        let mut methods: std::collections::BTreeMap<String, Method> =
            std::collections::BTreeMap::new();
        let mut by_name: std::collections::BTreeMap<String, Option<Method>> =
            std::collections::BTreeMap::new();
        let mut uses_std = false;
        let mut fails = false;

        for item in &parsed.program.items {
            match &item.node {
                Item::Grammar(def) => {
                    grammars.insert(parsed.text(def.name).to_string(), def.clone());
                }
                Item::Struct { name, .. } => {
                    structs.insert(*name);
                }
                Item::Import { .. } => uses_std = true,
                Item::Fn {
                    can_throw: true, ..
                } => fails = true,
                Item::Impl {
                    trait_name: _,
                    target,
                    methods: body,
                } => {
                    for method in body {
                        let Item::Fn {
                            name: Some(name),
                            receiver,
                            args,
                            ret_type,
                            ..
                        } = &method.node
                        else {
                            continue;
                        };

                        let info = Method {
                            receiver: *receiver,
                            returns_value: ret_type.is_some(),
                            args: args.clone(),
                        };
                        methods.insert(
                            format!("{}::{}", parsed.text(target.name), parsed.text(*name)),
                            info.clone(),
                        );
                        by_name
                            .entry(parsed.text(*name).to_string())
                            .and_modify(|known| {
                                // Two impls, one name: nothing here can tell
                                // them apart, so neither is used.
                                if known.is_some() {
                                    *known = None;
                                }
                            })
                            .or_insert(Some(info));
                    }
                }
                _ => {}
            }
        }

        // One type check, both of its answers about where a failure leaves.
        //
        // ADR-025 D7 for the loops: the type checker knows which `for` iterates
        // something whose step can fail, because it infers the iterator's type
        // and reads the ledger. Matching on the name `io::lines` here would
        // have caught the one-line form and quietly missed
        // `let s = io::lines()` followed by `for line in s`. ADR-288 for the
        // method calls: a receiver's type is the type checker's to know, and
        // there is one type checker (ADR-288).
        // The check's keep plans, where it made them as this does (#727).
        let mut checked = checked;
        let checked_plans = checked.as_mut().and_then(|facts| facts.keep_plans.take());
        let propagation = match build.leaves_values {
            false if let Some(facts) = checked => facts,
            true => crate::check::propagation_leaving_values(
                parsed,
                beside,
                &own_contracts,
                described,
                reads,
            ),
            false => {
                crate::check::propagation_against(parsed, beside, &own_contracts, described, reads)
            }
        };
        // **Proved at every build, written without the check only where the
        // build says so** (ADR-306 D14): what is proved does not depend on
        // `remove-bounds-checks` or `remove-overflow-checks`.
        let mut proven = crate::bounds::proven(
            parsed,
            &crate::bounds::Sites {
                lengths: &propagation.std_lengths,
                sized: &propagation.sized_lengths,
                char_codes: &propagation.char_codes,
                arithmetic: &propagation.arithmetic,
                negations: &propagation.negations,
                whole_fields: &propagation.whole_fields,
                array_lengths: &propagation.array_lengths,
                shifts: &propagation.shifts,
                std_receivers: &propagation.std_receivers,
            },
            &[&own_contracts, std_ledger()],
            &std::collections::BTreeSet::new(),
        );

        // ADR-037 D7: which count each `Shared` value gets. Computed over the
        // whole unit, because a count belongs to an allocation and a handle's
        // class may reach into another function.
        let library = std_ledger();
        let shared = crate::contracts::sharing::analyse_program(
            parsed,
            &own_contracts,
            library,
            build.crossings_are_possible(),
        )
        .counts;

        // ADR-055 D6, before `own_contracts` is moved into place.
        let reach = pausing_reach(parsed, &own_contracts);

        // **The classes of the file's maps** (ADR-010 D9, #581): where the
        // package is untrusted, the `let`s, parameters and results whose class
        // no untrusted key reaches keep the fast hash.
        let plan = match provenance {
            crate::contracts::Provenance::Trusted => crate::contracts::trust::MapPlan {
                lets: Default::default(),
                slots: Default::default(),
                // Only where a body may hold type parameters is it asked.
                escaped: match seeds_of.is_empty() {
                    true => Default::default(),
                    false => crate::contracts::trust::trusted_maps(parsed, library).escaped,
                },
            },
            crate::contracts::Provenance::Untrusted => {
                crate::contracts::trust::trusted_maps(parsed, library)
            }
        };
        let units: Vec<&Parsed> = std::iter::once(parsed)
            .chain(beside.iter().copied())
            .collect();
        let package_wide = package_borrowing(&units);
        let state = EmitState {
            described: described.clone(),
            view_slots: view_slots(&units, &package_wide)
                .into_iter()
                .chain(read_view_slots(described))
                .collect(),
            borrowing: borrowing_structs_beside(parsed, &package_wide)
                .into_iter()
                .map(|s| parsed.text(s).to_string())
                .collect(),
            tethered: tethered_types(&own_contracts)
                .into_iter()
                .chain(beside.iter().flat_map(|other| {
                    borrowing_structs(other)
                        .into_iter()
                        .map(|s| other.text(s).to_string())
                }))
                .collect(),
            declared_errors: std::iter::once(parsed)
                .chain(beside.iter().copied())
                .flat_map(declared_errors)
                .collect(),
            carries_input: crate::views::carried(parsed, &own_contracts, library)
                .into_iter()
                .map(|(at, symbols)| {
                    (
                        at as i64,
                        symbols
                            .into_iter()
                            .map(|s| parsed.text(s).to_string())
                            .collect(),
                    )
                })
                .collect(),
            fold_callees: fold_callees(parsed, &grammars).into_iter().collect(),
            by_name: by_name
                .into_iter()
                .filter_map(|(name, method)| Some((name, method?)))
                .collect(),
            grammars,
            methods,
            keep_plans: checked_plans
                .unwrap_or_else(|| {
                    crate::contracts::keep::buffer_plans(parsed, &own_contracts, library)
                })
                .into_iter()
                .map(|plan| (plan.key.clone(), plan))
                .collect(),
            structs: structs
                .into_iter()
                .map(|s| parsed.text(s).to_string())
                .collect(),
            structs_beside: beside
                .iter()
                .flat_map(|other| {
                    other
                        .program
                        .items
                        .iter()
                        .filter_map(|item| match &item.node {
                            Item::Struct { name, .. } => Some(other.text(*name).to_string()),
                            _ => None,
                        })
                })
                .collect(),
            template_holes: Default::default(),
            template_errors: Default::default(),
            hole_wraps: parsed
                .hole_wraps
                .iter()
                .map(|(at, wraps)| {
                    (
                        *at as i64,
                        wraps.iter().map(|(n, s)| (*n as i64, *s)).collect(),
                    )
                })
                .collect(),
            kept_calls: Default::default(),
            uses_std,
            is_std: false,
            fails,
            trusted_input: provenance == crate::contracts::Provenance::Trusted,
            trusted_lets: plan.lets.clone(),
            escaped_lets: plan.escaped.clone(),
            fast_slots: plan.slots.clone(),
            seeded_calls: seeds_of
                .calls()
                .iter()
                .map(|(name, (n, fixed))| (name.clone(), (*n as i64, *fixed as i64)))
                .collect(),
            proven_indices: propagation
                .list_indices
                .iter()
                .copied()
                .filter(|node| {
                    build.bounds == crate::bounds::BoundsChecks::Removed
                        && proven.indices.contains(node)
                })
                .collect(),
            // **A shift proved to count inside its width** (ADR-306 D18) is
            // written as an operation proved inside its type is.
            proven_arithmetic: propagation
                .arithmetic
                .iter()
                .chain(propagation.shifts.iter())
                .filter(|(at, _)| {
                    build.overflow == crate::bounds::OverflowChecks::Removed
                        && proven.arithmetic.contains(at)
                })
                .map(|(at, ty)| (*at, ty.clone()))
                .collect(),
            proven_negations: propagation
                .negations
                .iter()
                .filter(|(at, _)| {
                    build.overflow == crate::bounds::OverflowChecks::Removed
                        && proven.negations.contains(at)
                })
                .map(|(at, ty)| (*at, ty.clone()))
                .collect(),
            facts: propagation,
            cleaned_types: own_contracts
                .implementations
                .get("Cleanup")
                .cloned()
                .unwrap_or_default(),
            shared,
            built_at_build_time: built_at_build_time(parsed).into_iter().collect(),
            pausing_reach: reach,
            own_methods: nikaia_std::tools::ty::method_index(&own_contracts),
            library_methods: nikaia_std::tools::ty::method_index(library),
            own_contracts,
            dsl_drivers: crate::dsl::drivers(parsed).into_iter().collect(),
            foreign_params: foreign_params(parsed),
            opaque_handles: opaque_handles(parsed),
            foreign_results: foreign_results(parsed),
            entry: true,
            sums: std::collections::BTreeMap::new(),
        };
        let mut state = state;
        let (holes, errors) = template_hole_tables(parsed);
        state.template_holes = holes;
        state.template_errors = errors;
        state.kept_calls = parsed
            .cached_kept_calls()
            .into_iter()
            .map(|((name, arity), (names, call))| {
                (
                    (name, arity as i64),
                    nikaia_std::tools::emit_state::EmitKeptCall { names, call },
                )
            })
            .collect();
        let give_ups = crate::view_changes::analyse(
            parsed,
            &state.own_contracts,
            library,
            &state.facts.lent_lets,
            &crate::view_changes::coalesced(&state.facts),
        )
        .1;
        let mut made = Self {
            parsed,
            build,
            s: state,
            give_ups,
            w: std::cell::RefCell::new(Some(EmitWalk::fresh())),
            failed: std::cell::RefCell::new(None),
            world: EmitWorld {
                words: &parsed.interner,
                aliases: &parsed.aliases,
                items: &parsed.program.items,
                library,
            },
            seeds: seeds_of,
            place: std::cell::RefCell::new(seeds::Place::default()),
            floor: std::cell::Cell::new(false),
            grammar_bytes: std::cell::Cell::new(false),
            grammar_raw: std::cell::Cell::new(false),
            nonnegative_bodies: std::mem::take(&mut proven.nonnegative_locals),
            main_params: std::cell::RefCell::new(Vec::new()),
        };
        made.s.sums = made.s.error_sums(&made.world);
        made
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

    /// Reads the walk's places. The borrow ends when `f` returns, so nothing
    /// held here can meet a write made by a call back into the emitter.
    fn walk<R>(&self, f: impl FnOnce(&EmitWalk) -> R) -> R {
        f(self
            .w
            .borrow()
            .as_ref()
            .expect("the walk is out on a crossing"))
    }

    /// Writes the walk's places, briefly: `f` calls nothing of the emitter's.
    fn walking<R>(&self, f: impl FnOnce(&mut EmitWalk) -> R) -> R {
        f(self
            .w
            .borrow_mut()
            .as_mut()
            .expect("the walk is out on a crossing"))
    }

    fn text(&self, sym: Symbol) -> &str {
        self.parsed.text(sym)
    }

    /// The same text, written so the language below can read it (`escaped`).
    ///
    /// Deliberately **not** folded into `text` above, although that would be one
    /// line instead of every position below. `text` is also what the recorded
    /// answers from the checker are keyed by — `pausing_methods`,
    /// `fallible_methods`, `nullable_fields` are all keyed by the name the
    /// *source* wrote — so escaping there would make every one of those lookups
    /// miss, silently, on exactly the programs this is for.
    /// **A `comptime` whose uses were written out as literals**
    /// ([ADR-287](../../docs/specification/adr/adr-287.md) D21, #700): an open
    /// number takes the type each use asks for, so the constant may be read
    /// nowhere below, and the language below is told so. One no use reads
    /// keeps its warning.
    fn folded(&self, sym: Symbol) -> &'static str {
        match self.s.facts.folded_comptimes.contains(self.text(sym)) {
            true => "#[allow(dead_code)]\n",
            false => "",
        }
    }

    fn name(&self, sym: Symbol) -> std::borrow::Cow<'_, str> {
        escaped(self.text(sym))
    }

    /// **A binding's name** (#703): one named like a `comptime` of the
    /// package is written under a name of its own (`emit_local_name`).
    fn local(&self, name: &str) -> String {
        nikaia_std::tools::emit_misc::emit_local_name(&self.s.facts.comptime_names, name)
    }

    /// **A read of a name**: under the binding's own name where it reads a
    /// binding that shadows a `comptime` (#703), the source's otherwise.
    fn read_name(&self, sym: Symbol, expr: &Expr) -> String {
        match self
            .s
            .facts
            .shadowing_reads
            .contains(&crate::check::value_node(expr))
        {
            true => self.local(self.text(sym)),
            false => self.name(sym).into_owned(),
        }
    }

    /// The name of a function whose body walks a type's fields, where this item
    /// is one ([ADR-304](../../docs/specification/adr/adr-304.md) D6).
    ///
    /// Read off the **checker's** answer rather than off the body, which is
    /// this emitter's rule everywhere: what a type is, and therefore which
    /// functions were instantiated with what, is the checker's
    /// ([ADR-288](../../docs/specification/adr/adr-288.md)).
    fn walks_a_shape(&self, item: &Item) -> Option<String> {
        self.s.walks_a_shape(&self.world, item)
    }

    /// The concrete type the parameter stands for, while a copy is being
    /// written.
    fn standing_for(&self) -> Option<String> {
        self.walk(|w| w.specialising.as_ref().map(|pair| pair.second.clone()))
    }

    /// The fields a `for` over `T::fields` walks, where this is one
    /// ([ADR-304](../../docs/specification/adr/adr-304.md) D6).
    ///
    /// Asked of the **iterated expression** rather than of the statement,
    /// because that is where the shape is named — and answered from the
    /// checker's table, which is the only thing that knows which copy this is.
    fn unrolls_here(&self, iter: &Expr) -> Option<Vec<crate::contracts::FieldContract>> {
        let pair = self.walk(|w| w.specialising.clone())?;
        self.s
            .unrolls_here(&self.world, &pair.first, &pair.second, iter)
    }

    /// The type parameter a shape walk stands on — the `T` of
    /// `fn describe[T: Struct](value: T)`.
    ///
    /// The **first** one, which is what the checker bound: a second parameter
    /// beside it is an ordinary generic and keeps its place in the signature.
    fn walked_parameter(&self, item: &Item) -> String {
        self.s.walked_parameter(&self.world, item).to_string()
    }

    /// A module's items and nothing else - no preamble, no `mod` header.
    fn items_only(&self) -> Result<Lowered> {
        let mut out = Out::empty();
        self.shadow_types(&mut out);
        self.write_error_sums(&mut out);
        for item in &self.parsed.program.items {
            // **One copy per type it was used with, and no generic original**
            // ([ADR-304](../../docs/specification/adr/adr-304.md) D6): the
            // generic body holds a loop over a shape, which has no form in the
            // language below. It is **unrolled**, which is
            // [ADR-304](../../docs/specification/adr/adr-304.md) D5's *one loop*
            // arrived at rather than added - `T::fields` is known while the
            // program is built, so there is no run-time reading to rule out.
            if let Some(name) = self.walks_a_shape(&item.node) {
                let copies: Vec<String> = self
                    .s
                    .facts
                    .unrolled
                    .keys()
                    .filter(|(written, _)| *written == name)
                    .map(|(_, on)| on.clone())
                    .collect();
                for on in copies {
                    let walked = self.walked_parameter(&item.node);
                    self.walking(|w| {
                        w.specialising = Some(EmitPair {
                            first: walked,
                            second: on,
                        })
                    });
                    let st = out.mark();
                    let written = self.item(&mut out, item);
                    self.walking(|w| w.specialising = None);
                    written?;
                    out.record(&item.span, st);
                    out.push("\n");
                }
                continue;
            }
            // **One copy per type it was used with, and no generic original**
            // ([ADR-304](../../docs/specification/adr/adr-304.md) D6): the
            // generic body holds a loop over a shape, which has no form in the
            // language below. It is **unrolled**, which is
            // [ADR-304](../../docs/specification/adr/adr-304.md) D5's *one
            // loop* arrived at rather than added — `T::fields` is known while
            // the program is built, so there is no run-time reading to rule
            // out.
            if let Some(name) = self.walks_a_shape(&item.node) {
                let copies: Vec<String> = self
                    .s
                    .facts
                    .unrolled
                    .keys()
                    .filter(|(written, _)| *written == name)
                    .map(|(_, on)| on.clone())
                    .collect();
                for on in copies {
                    let walked = self.walked_parameter(&item.node);
                    self.walking(|w| {
                        w.specialising = Some(EmitPair {
                            first: walked,
                            second: on,
                        })
                    });
                    let st = out.mark();
                    let written = self.item(&mut out, item);
                    self.walking(|w| w.specialising = None);
                    written?;
                    out.record(&item.span, st);
                    out.push("\n");
                }
                continue;
            }
            {
                let st = out.mark();
                self.item(&mut out, item)?;
                out.record(&item.span, st);
            }
            out.push("\n");
        }
        self.tether_wrappers(&mut out);
        self.entry_point(&mut out);
        Ok(Lowered {
            rust: out.buf,
            map: SourceMap::from_entries(out.entries),
            published: self.s.facts.published.clone(),
        })
    }

    fn program(&self) -> Result<Lowered> {
        let mut out = Out::empty();
        out.push("// Generated by the Nikaia bootstrap compiler (Stage 0).\n");
        out.push("// Edit the .nika source, not this file.\n\n");

        if self
            .parsed
            .program
            .items
            .iter()
            .any(|i| matches!(i.node, Item::Grammar(_)))
        {
            out.push("use winnow_grammar::grammar;\n");
        }
        if self.uses_driver() {
            out.push("use winnow_grammar::rt::Parallelism;\n");
            out.push("use winnow_grammar::ParseContext;\n");
        }
        // Always, except in `std` itself, which is the prelude - see
        // [`emit_std`] and [`Needs::preamble`], the copy of this that a program
        // of several files uses.
        if !self.s.is_std {
            out.push("#[allow(unused_imports)]\npub use nikaia_std::prelude::*;\n");
        }
        if self.s.fails {
            // Kap 7.1: `error.full()` is a method on whatever a `catch` bound, so
            // the trait has to be in scope even in a program that imports no `std`
            // module of its own - `throw` needs `std` whether or not the source
            // mentions it.
            out.push("#[allow(unused_imports)]\npub use nikaia_std::error::Full;\n");
        }
        out.push("\n");

        self.shadow_types(&mut out);
        self.write_error_sums(&mut out);

        for item in &self.parsed.program.items {
            // **One copy per type it was used with, and no generic original**
            // ([ADR-304](../../docs/specification/adr/adr-304.md) D6): the
            // generic body holds a loop over a shape, which has no form in the
            // language below. It is **unrolled**, which is
            // [ADR-304](../../docs/specification/adr/adr-304.md) D5's *one loop*
            // arrived at rather than added - `T::fields` is known while the
            // program is built, so there is no run-time reading to rule out.
            if let Some(name) = self.walks_a_shape(&item.node) {
                let copies: Vec<String> = self
                    .s
                    .facts
                    .unrolled
                    .keys()
                    .filter(|(written, _)| *written == name)
                    .map(|(_, on)| on.clone())
                    .collect();
                for on in copies {
                    let walked = self.walked_parameter(&item.node);
                    self.walking(|w| {
                        w.specialising = Some(EmitPair {
                            first: walked,
                            second: on,
                        })
                    });
                    let st = out.mark();
                    let written = self.item(&mut out, item);
                    self.walking(|w| w.specialising = None);
                    written?;
                    out.record(&item.span, st);
                    out.push("\n");
                }
                continue;
            }
            // **One copy per type it was used with, and no generic original**
            // ([ADR-304](../../docs/specification/adr/adr-304.md) D6): the
            // generic body holds a loop over a shape, which has no form in the
            // language below. It is **unrolled**, which is
            // [ADR-304](../../docs/specification/adr/adr-304.md) D5's *one
            // loop* arrived at rather than added — `T::fields` is known while
            // the program is built, so there is no run-time reading to rule
            // out.
            if let Some(name) = self.walks_a_shape(&item.node) {
                let copies: Vec<String> = self
                    .s
                    .facts
                    .unrolled
                    .keys()
                    .filter(|(written, _)| *written == name)
                    .map(|(_, on)| on.clone())
                    .collect();
                for on in copies {
                    let walked = self.walked_parameter(&item.node);
                    self.walking(|w| {
                        w.specialising = Some(EmitPair {
                            first: walked,
                            second: on,
                        })
                    });
                    let st = out.mark();
                    let written = self.item(&mut out, item);
                    self.walking(|w| w.specialising = None);
                    written?;
                    out.record(&item.span, st);
                    out.push("\n");
                }
                continue;
            }
            {
                let st = out.mark();
                self.item(&mut out, item)?;
                out.record(&item.span, st);
            }
            out.push("\n");
        }
        self.tether_wrappers(&mut out);
        self.entry_point(&mut out);
        if self.user_main().is_some() {
            // **An empty table, and the reason it is empty.** `fn main` installs
            // the hook that reads it (ADR-300 D10), so the name has to be defined
            // or the program does not compile - and a table needs the *file* each
            // line came from, which this path does not know: it is handed one
            // `Parsed` and no path. The path a user takes goes through
            // `modules::Program`, which knows every file and appends the real
            // table (`abort_table`); this one is `emit_program`, which the tests
            // and `lower-std` use.
            //
            // Empty is the fallback rather than a lie: a location the table does
            // not know is handed to the hook installed before ours, which is
            // Rust's own, so such a program is exactly as well off as it was.
            out.push(&format!(
                "\n// ADR-300 D9: no table - this program was emitted without its file name.\nconst {ABORT_TABLE}: &[nikaia_std::abort::Site] = &[];\n"
            ));
        }

        Ok(Lowered {
            rust: out.buf,
            map: SourceMap::from_entries(out.entries),
            published: self.s.facts.published.clone(),
        })
    }

    /// The program's own `main`, if this file carries one the runtime can wrap.
    ///
    /// `Some(throws)` for the plain shape - `fn main()` or `fn main() throws`,
    /// no receiver, no parameters, no returned value - and `None` for anything
    /// else, including a `main` in a module rather than at the crate root. A
    /// `main` this does not recognise is emitted exactly as written and keeps
    /// its name, because a wrapper that guessed at a signature would be worse
    /// than no wrapper at all.
    fn user_main(&self) -> Option<bool> {
        self.s.user_main(&self.world)
    }

    /// The `fn main` Rust runs: ADR-303 D4, as five lines of generated code.
    ///
    /// The runtime is started *before* the program's first statement and
    /// drained after its last, so an operation inside the program costs no
    /// thread start and no thread wake-up - which is
    /// [ADR-292](../../../docs/specification/adr/adr-292.md)'s finding
    /// read the other way round.
    ///
    /// What starts is what
    /// [ADR-037](../../../docs/specification/adr/adr-037.md) D2 allows, and
    /// the *compiler* is what knows which: `user_parallelism` is a build
    /// switch, so it is written into this call rather than read from the
    /// operator's runtime configuration file (ADR-303 D5 has four settings and
    /// this is not one of them).
    fn entry_point(&self, out: &mut Out) {
        let concurrent = matches!(self.build.user_parallelism, UserParallelism::Yes);
        let channel = self.error_channel(MAIN, Lifetimes::Elided, false);
        // **`main`'s parameters as its definition spells them**
        // ([ADR-360](../../docs/specification/adr/adr-360.md) D1): the words
        // are read into locals that own them, and a parameter the body only
        // reads (`&[T]`, `&str`) is lent from its local: `passing` writes the
        // argument, `{}` standing for the local.
        let mut types: Vec<String> = Vec::new();
        let mut passing: Vec<String> = Vec::new();
        for param in self.main_params.borrow().iter() {
            let ty = param.split_once(": ").map_or("_", |(_, ty)| ty);
            let (owned, pass) = match ty {
                "&str" => ("String".to_string(), "&{}"),
                "Option<&str>" => ("Option<String>".to_string(), "{}.as_deref()"),
                _ => match ty.strip_prefix("&[").and_then(|t| t.strip_suffix(']')) {
                    Some(element) => (format!("Vec<{element}>"), "&{}"),
                    // Any other view the body only reads, `&Option<String>`
                    // for an unused `String?` among them (#703), is lent
                    // from the local that owns the word.
                    None => match ty.strip_prefix('&').filter(|t| !t.starts_with("mut ")) {
                        Some(held) => (held.to_string(), "&{}"),
                        None => (ty.trim_start_matches("&mut ").to_string(), "{}"),
                    },
                },
            };
            types.push(owned);
            passing.push(pass.to_string());
        }
        let entry = self
            .s
            .entry_point(&self.world, concurrent, &channel, &types, &passing);
        if entry.is_empty() {
            return;
        }
        out.push(&entry);
        // **The program's allocator is the system's**, with a free that
        // `skip-free-at-exit:on` skips once `main` has ended
        // ([ADR-348](../../docs/specification/adr/adr-348.md) D1).
        if self.build.skip_free_at_exit {
            out.push(ALLOCATOR);
        }
    }

    /// Whether any grammar entry in the program reaches a **parallel** rule,
    /// which is what decides whether the piece driver's names are in the
    /// preamble.
    ///
    /// The entry is a call now ([ADR-296](../../docs/specification/adr/adr-296.md)
    /// D1), so the rule is the one the program **named** rather than the one
    /// this file used to pick. Found by being wrong: the lowering was right and
    /// the preamble was not, and `rustc` answered *"use of undeclared type
    /// `ParseContext`"* about the generated file.
    fn uses_driver(&self) -> bool {
        self.s.uses_driver(&self.world)
    }

    /// **What a declared type derives** ([ADR-204](../../../docs/specification/adr/adr-204.md)
    /// D1).
    ///
    /// `Debug` and `Clone` always: every emitted type is printable and copyable,
    /// which is what the rest of this file already assumes of one.
    ///
    /// `PartialEq` where the checker says every part of it compares, and `Eq`
    /// beside it where no part is a float. Without them `==` had no lowering at
    /// all: `rustc` answered *binary operation `==` cannot be applied to type
    /// `P`* about a file nobody wrote, with *consider annotating `P` with
    /// `#[derive(PartialEq)]`* as the help — [Part III C.1 and
    /// C.2](../../../docs/specification/30-nikaia-tooling.md) at once.
    ///
    /// **Never both without the first**, which is Rust's own rule: `Eq` is an
    /// `impl` over `PartialEq`, and `compares_totally` is a subset by
    /// construction.
    fn derives(&self, name: winnow_grammar::Symbol) -> String {
        self.s.derives(&self.world, name)
    }

    fn item(&self, out: &mut Out, written: &Spanned<Item>) -> Result<()> {
        let (item, span) = (&written.node, &written.span);
        match item {
            Item::Grammar(def) => self.grammar(out, def),
            Item::Enum {
                name,
                variants,
                is_public,
                is_extern,
            } => {
                out.push(&self.derives(*name));
                // **A tagged union in C's layout** (ADR-284 D32).
                if *is_extern {
                    out.push("#[repr(C)]\n");
                }
                let vis = if *is_public { "pub " } else { "" };
                let params = if self.s.borrowing.contains(self.text(*name)) {
                    format!("<{INPUT_LIFETIME}>")
                } else {
                    String::new()
                };
                out.push(&format!("{vis}enum {}{params} {{\n", self.name(*name)));
                let enum_name = self.text(*name);
                for variant in variants {
                    let name = self.name(variant.name);
                    match &variant.fields {
                        VariantFields::Unit => out.push(&format!("    {name},\n")),
                        VariantFields::Tuple(types) => {
                            let owner = self.text(variant.name);
                            let parts: Vec<String> = types
                                .iter()
                                .enumerate()
                                .map(|(at, t)| {
                                    let member = crate::check::boxed_member(owner, Some(at), None);
                                    let count = self.count_at(
                                        SHARED_FIELDS,
                                        &variant_slot(enum_name, owner, at),
                                    );
                                    self.boxed_ty(
                                        enum_name,
                                        &member,
                                        self.ty_counted(t, Lifetimes::Named, count),
                                    )
                                })
                                .collect();
                            out.push(&format!("    {name}({}),\n", parts.join(", ")));
                        }
                        VariantFields::Named(fields) => {
                            let owner = self.text(variant.name);
                            let parts: Vec<String> = fields
                                .iter()
                                .map(|f| {
                                    let member = crate::check::boxed_member(
                                        owner,
                                        None,
                                        Some(self.text(f.name)),
                                    );
                                    format!(
                                        "{}: {}",
                                        self.name(f.name),
                                        self.boxed_ty(
                                            enum_name,
                                            &member,
                                            self.ty(&f.ty, Lifetimes::Named)
                                        )
                                    )
                                })
                                .collect();
                            out.push(&format!("    {name} {{ {} }},\n", parts.join(", ")));
                        }
                    }
                }
                out.push("}\n");
                Ok(())
            }
            Item::Struct {
                name,
                generics,
                fields,
                is_public,
                is_extern,
                ..
            } => {
                // **A `pub` struct's map positions are its hidden type
                // parameters** (ADR-010 D9), numbered by the fields in order.
                let seeded = *is_public && !*is_extern && self.seeds.arity(self.text(*name)) > 0;
                if seeded {
                    self.place.borrow_mut().declare_type(0);
                }
                let mut derives = self.derives(*name);
                // `Keyed` is `std`'s own hasher, which does not compare: the
                // comparison is written by hand for the fields, over any seed.
                let by_hand = seeded
                    && generics.is_empty()
                    && !self.s.borrowing.contains(self.text(*name))
                    && derives.contains("PartialEq");
                if by_hand {
                    derives = derives.replace(", PartialEq", "").replace(", Eq", "");
                }
                out.push(&derives);
                // **C's layout** (ADR-284 D14), and every field `pub` (D15).
                if *is_extern {
                    out.push("#[repr(C)]\n");
                }
                let vis = if *is_public { "pub " } else { "" };
                let mut body = Out::empty();
                // **One lifetime per view field** (ADR-353 D1): `'a`, `'a1`, ...
                let slots = self.s.view_slots.get(self.text(*name));
                for field in fields {
                    // Public, because the actions that build this struct are
                    // generated into the grammar's own module.
                    let slot = format!("{}.{}", self.text(*name), self.text(field.name));
                    let lowered = self.ty_counted(
                        &field.ty,
                        Lifetimes::Named,
                        self.count_at(SHARED_FIELDS, &slot),
                    );
                    let lowered = match slots
                        .and_then(|v| v.iter().position(|f| f == self.text(field.name)))
                    {
                        Some(at) if at > 0 => renamed_lifetime(
                            &lowered,
                            INPUT_LIFETIME,
                            &format!("{INPUT_LIFETIME}{at}"),
                        ),
                        _ => lowered,
                    };
                    body.push(&format!(
                        "    {}{}: {},\n",
                        if field.is_public || *is_extern {
                            "pub "
                        } else {
                            ""
                        },
                        self.name(field.name),
                        self.boxed_ty(self.text(*name), self.text(field.name), lowered)
                    ));
                }
                let hidden = match seeded {
                    true => self.place.borrow_mut().finish(0),
                    false => 0,
                };
                // The input lifetime first and the type parameters after it,
                // which is the order Rust wants them in (ADR-295 D3); the
                // hidden ones last, where their defaults may stand.
                let mut parts: Vec<String> = Vec::new();
                if self.s.borrowing.contains(self.text(*name)) {
                    parts.push(INPUT_LIFETIME.to_string());
                    let more = slots.map_or(0, Vec::len).saturating_sub(1);
                    parts.extend((1..=more).map(|at| format!("{INPUT_LIFETIME}{at}")));
                }
                parts.extend(generics.iter().map(|g| self.bounded(g)));
                parts.extend(seeds::Seeds::declared(0, hidden, true));
                let params = angled(&parts);
                out.push(&format!("{vis}struct {}{params} {{\n", self.name(*name)));
                out.append(body);
                out.push("}\n");
                if by_hand {
                    let declares = angled(&seeds::Seeds::declared(0, hidden, false));
                    let applied = angled(&seeds::Seeds::applied(0, hidden));
                    let me = self.name(*name);
                    let same: Vec<String> = fields
                        .iter()
                        .map(|f| format!("self.{0} == other.{0}", self.name(f.name)))
                        .collect();
                    out.push(&format!(
                        "impl{declares} PartialEq for {me}{applied} {{\n    fn eq(&self, other: &Self) -> bool {{ {} }}\n}}\n",
                        if same.is_empty() { "true".to_string() } else { same.join(" && ") }
                    ));
                    if self.s.facts.compares_totally.contains(self.text(*name)) {
                        out.push(&format!("impl{declares} Eq for {me}{applied} {{}}\n"));
                    }
                }
                Ok(())
            }
            Item::Fn { name, .. } => {
                if name.is_some_and(|name| self.s.built_at_build_time.contains(self.text(name))) {
                    out.push("#[allow(dead_code)]\n");
                }
                self.function(
                    out,
                    written,
                    0,
                    Lifetimes::Elided,
                    self.s.carries_input.get(&(span.at() as i64)),
                    None,
                )
            }
            Item::Impl {
                trait_name,
                target,
                methods,
            } => {
                // A type that holds a view carries the input lifetime, and the
                // impl has to declare the lifetime its methods are written with.
                let borrows = self.borrows(target.name);
                let params = if borrows {
                    format!("<{INPUT_LIFETIME}>")
                } else {
                    String::new()
                };
                let lifetimes = if borrows {
                    Lifetimes::Inner
                } else {
                    Lifetimes::Elided
                };
                let target_name = self.text(target.name).to_string();

                // Kap 7.1: `Error` is the one trait the compiler reads rather
                // than relays. An error travels in the failure channel, which
                // the language below spells `Box<dyn std::error::Error>`, and a
                // type gets in there by being `Display` plus `Error` - neither
                // of which the `.nika` source mentions, because neither is a
                // decision the author makes. `message` is what they wrote.
                let is_error_impl = trait_name.is_some_and(|t| self.text(t) == "Error");
                if is_error_impl {
                    return self.error_impl(out, &target_name, &params, methods, lifetimes);
                }

                // `impl Stack[T]` declares `T` and writes `Stack<T>`; the
                // lifetime, where the type carries one, stands in front of it
                // (ADR-295 D4). The target's arguments are written as the
                // source wrote them, so `impl Stack[i64]` stays `Stack<i64>`
                // and declares nothing.
                let declared = crate::contracts::declared_types(self.parsed);
                let generics = crate::contracts::impl_parameters(self.parsed, target, &declared);
                let mut head_parts: Vec<String> = Vec::new();
                if borrows {
                    head_parts.push(INPUT_LIFETIME.to_string());
                }
                head_parts.extend(generics.iter().cloned());
                let declares = angled(&head_parts);
                let mut target_parts: Vec<String> = Vec::new();
                if borrows {
                    target_parts.push(self.s.lifetime_args(&target_name, INPUT_LIFETIME));
                }
                target_parts.extend(
                    target
                        .generics
                        .iter()
                        .map(|g| self.ty(g, Lifetimes::Elided)),
                );
                let applied = angled(&target_parts);
                // **`impl Cleanup for T` is the type's own `cleanup` and a
                // one-line bridge to std's trait** (ADR-297 D1): the method
                // is written as the author wrote it, paused and failing as the
                // ledger says, and `Cleaned<T>` reaches it through the bridge
                // when the value dies.
                // **`impl Fatal for T` is a marker** (ADR-345 D1): the
                // compiler reads it, and nothing below it needs a trait.
                let fatal = trait_name.is_some_and(|t| self.text(t) == "Fatal")
                    && !self.parsed.program.items.iter().any(|item| {
                        matches!(&item.node, Item::Trait { name, .. } if self.text(*name) == "Fatal")
                    });
                if fatal {
                    // It answers `Ending` for the status it sets (ADR-345 D4,
                    // ADR-346 D6) and so that a supervisor does not restart it.
                    out.push(&format!(
                        "impl{declares} nikaia_std::process::Ending for {target_name}{applied} {{\n    fn exit_code(&self) -> Option<i32> {{\n        None\n    }}\n    fn fatal_code(&self) -> Option<i32> {{\n        Some(1)\n    }}\n}}\n\n"
                    ));
                    return Ok(());
                }
                let is_cleanup = trait_name.is_some_and(|t| self.text(t) == "Cleanup");
                if is_cleanup {
                    out.push(&format!("impl{declares} {target_name}{applied} {{\n"));
                    for method in methods {
                        let carries = self.s.carries_input.get(&(method.span.at() as i64));
                        {
                            let st = out.mark();
                            out.push("    ");
                            self.function(
                                out,
                                method,
                                1,
                                lifetimes,
                                carries,
                                Some(MethodOf {
                                    target: &target_name,
                                    declared_by: None,
                                    bridged: None,
                                }),
                            )?;
                            out.record(&method.span, st);
                        }
                    }
                    out.push("}\n");
                    let key = format!("{target_name}::cleanup");
                    let awaited = if self.pauses(&key) { ".await" } else { "" };
                    let (bridge, send) = match self.build.overlaps_user_code() {
                        true => ("CleanupSend", " + Send"),
                        false => ("Cleanup", ""),
                    };
                    out.push(&format!(
                        "impl{declares} nikaia_std::cleanup::{bridge} for {target_name}{applied} {{\n\
                         \x20   fn cleanup(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>>{send} + '_>> {{\n\
                         \x20       Box::pin(async move {{ nikaia_std::cleanup::Outcome::reported({target_name}::cleanup(self){awaited}) }})\n\
                         \x20   }}\n\
                         }}\n"
                    ));
                    return Ok(());
                }
                // **`impl Eq` and `impl Ord` are the language's** (ADR-341 D1):
                // the method is the author's, as `Cleanup`'s is, and a bridge
                // gives the standard library's trait the answer. A program that
                // declares a trait of the name has its own and is relayed.
                let standard = trait_name
                    .map(|t| self.text(t).to_string())
                    .filter(|t| matches!(t.as_str(), "Eq" | "Ord"))
                    .filter(|t| {
                        !self.parsed.program.items.iter().any(|item| {
                            matches!(&item.node, Item::Trait { name, .. } if self.text(*name) == t)
                        })
                    });
                if let Some(standard) = standard {
                    out.push(&format!("impl{declares} {target_name}{applied} {{\n"));
                    for method in methods {
                        let carries = self.s.carries_input.get(&(method.span.at() as i64));
                        let st = out.mark();
                        out.push("    ");
                        self.function(
                            out,
                            method,
                            1,
                            lifetimes,
                            carries,
                            Some(MethodOf {
                                target: &target_name,
                                declared_by: None,
                                bridged: None,
                            }),
                        )?;
                        out.record(&method.span, st);
                    }
                    out.push("}\n");
                    let me = format!("{target_name}{applied}");
                    match standard.as_str() {
                        "Eq" => out.push(&format!(
                            "impl{declares} PartialEq for {me} {{\n    fn eq(&self, other: &Self) -> bool {{ {target_name}::eq(self, other) }}\n}}\nimpl{declares} Eq for {me} {{}}\n"
                        )),
                        _ => out.push(&format!(
                            "impl{declares} PartialOrd for {me} {{\n    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {{ Some(std::cmp::Ord::cmp(self, other)) }}\n}}\nimpl{declares} Ord for {me} {{\n    fn cmp(&self, other: &Self) -> std::cmp::Ordering {{ {target_name}::compare(self, other) }}\n}}\n"
                        )),
                    }
                    return Ok(());
                }
                // **The methods of a type that declares map parameters take
                // them** (ADR-010 D9): the `impl` is generic over them, and a
                // mention of the type inside it is `Self`'s.
                let hidden = self.seeds.arity(&target_name);
                let (declares, applied) = match hidden > 0 {
                    true => (
                        angled(
                            &head_parts
                                .iter()
                                .cloned()
                                .chain(seeds::Seeds::declared(0, hidden, false))
                                .collect::<Vec<_>>(),
                        ),
                        angled(
                            &target_parts
                                .iter()
                                .cloned()
                                .chain(seeds::Seeds::applied(0, hidden))
                                .collect::<Vec<_>>(),
                        ),
                    ),
                    false => (declares, applied),
                };
                if hidden > 0 {
                    self.place
                        .borrow_mut()
                        .enter(&target_name, seeds::Seeds::applied(0, hidden));
                }
                // **A method narrower than the trait's declaration is the
                // type's own, and the trait reaches it through a bridge**
                // (ADR-288 D2, D25): `sync` or without `throws` where the trait
                // may pause or fail. A call on the concrete type follows the
                // method's own contract - no `.await`, no `?` - so the method is
                // written into an inherent `impl`, which Rust prefers on such a
                // call, and the trait's `impl` answers with the declaration's
                // signature and calls it, as `Eq`'s bridge above does.
                let declared_by = trait_name.map(|t| self.text(t));
                let wider: Vec<Option<&crate::ast::TraitMethod>> = methods
                    .iter()
                    .map(|method| self.declared_wider(declared_by, &target_name, &method.node))
                    .collect();
                if wider.iter().any(Option::is_some) {
                    let public = self.parsed.program.items.iter().any(|item| {
                        matches!(&item.node,
                            Item::Struct { name, is_public: true, .. }
                            | Item::Enum { name, is_public: true, .. }
                                if self.text(*name) == target_name)
                    });
                    out.push(&format!("impl{declares} {target_name}{applied} {{\n"));
                    for (method, _) in methods.iter().zip(&wider).filter(|(_, w)| w.is_some()) {
                        let carries = self.s.carries_input.get(&(method.span.at() as i64));
                        let st = out.mark();
                        out.push("    ");
                        self.function(
                            out,
                            method,
                            1,
                            lifetimes,
                            carries,
                            Some(MethodOf {
                                target: &target_name,
                                declared_by,
                                bridged: Some(public),
                            }),
                        )?;
                        out.record(&method.span, st);
                    }
                    out.push("}\n");
                }
                let head = match trait_name {
                    Some(t) => format!(
                        "impl{declares} {} for {target_name}{applied}",
                        self.text(*t)
                    ),
                    None => format!("impl{declares} {target_name}{applied}"),
                };
                out.push(&format!("{head} {{\n"));
                for (method, wider) in methods.iter().zip(&wider) {
                    if let Some(declared) = wider {
                        out.push("    ");
                        self.bridge(out, declared, &target_name, &method.node)?;
                        continue;
                    }
                    let carries = self.s.carries_input.get(&(method.span.at() as i64));
                    {
                        let st = out.mark();
                        out.push("    ");
                        self.function(
                            out,
                            method,
                            1,
                            lifetimes,
                            carries,
                            Some(MethodOf {
                                target: &target_name,
                                declared_by,
                                bridged: None,
                            }),
                        )?;
                        out.record(&method.span, st);
                    }
                }
                self.place.borrow_mut().leave();
                out.push("}\n");
                Ok(())
            }
            // Kap 4.7: `trait Summarize { … }` becomes the same trait below, and
            // the method is a signature with a `;` where an `impl`'s has a body
            // ([ADR-295](../../docs/specification/adr/adr-295.md) D6).
            //
            // **No method here is `async`.** The ledger's `sync` column decides
            // that for a function (ADR-055 D1), and a declaration has no body
            // for `sync::infer` to read - so ADR-295 D9 asserts `sync` for a
            // trait's methods, which is the only answer this position can
            // write: `async fn` in a trait is something the emitter has no way
            // to ask for. What that costs is a trait whose method genuinely
            // pauses, which issue #253 carries with its reproduction.
            Item::Trait {
                name,
                methods,
                is_public,
            } => {
                let vis = if *is_public { "pub " } else { "" };
                out.push(&format!("{vis}trait {} {{\n", self.name(*name)));
                for method in methods {
                    {
                        let st = out.mark();
                        out.push("    ");
                        self.trait_method(out, &method.node, method.node.is_sync, false)?;
                        out.record(&method.span, st);
                    }
                }
                out.push("}\n");
                Ok(())
            }
            Item::Import { path, alias } => {
                // The names come in through `nikaia_std::prelude`, emitted once
                // in the preamble; the import itself is kept as a comment so the
                // generated file still says where they were asked for.
                //
                // An alias is **resolved away** rather than emitted as a Rust
                // `use … as …` (ADR-286 D12): it is one file's name for a package
                // and every file's items are in one crate, so a Rust alias at the
                // root would be the whole program's. `Parsed::unaliased` is where
                // the resolving happens, on the way to every name.
                let path = path
                    .iter()
                    .map(|s| self.text(*s))
                    .collect::<Vec<_>>()
                    .join("::");
                // **Inside `std` there is no prelude to bring it in**
                // (0.0.248): a module of `std`'s Nikaia half is lowered without
                // one (`emit_std`), so `use std::text` has to be the import
                // itself, or `text::parse_i64` below it names nothing and
                // `rustc` says so about the generated file.
                if self.s.is_std
                    && alias.is_none()
                    && let [first, module] = path.split("::").collect::<Vec<_>>()[..]
                    && first == "std"
                {
                    out.push(&format!(
                        "#[allow(unused_imports)]\nuse nikaia_std::{module};\n"
                    ));
                    return Ok(());
                }
                match alias {
                    Some(alias) => out.push(&format!("// use {path} as {}\n", self.text(*alias))),
                    None => out.push(&format!("// use {path}\n")),
                }
                Ok(())
            }
            // **The same `const` the body form writes, one level out**
            // ([ADR-287](../../docs/specification/adr/adr-287.md)). The
            // spelling is the checker's, handed over keyed by the item's own
            // byte offset the way a statement's is - this emitter knows no
            // types ([ADR-296](../../docs/specification/adr/adr-296.md) D17), so
            // what `1000` is called below is not a question it can answer.
            //
            // `pub` is Part I 9.2's rule for Constants and reaches Rust as
            // Rust's own ([ADR-286](../../docs/specification/adr/adr-286.md)
            // D2): a package's crate is what another package reads.
            Item::Comptime { name, public, .. } => {
                let bound = self.name(*name);
                let Some((below, written)) = self
                    .s
                    .facts
                    .comptime_values
                    .get(&(span.at() as i64))
                    .cloned()
                else {
                    return Err(refused_at!(
                        span.at(),
                        "`{bound}` has no value computed at build time. This is a compiler \
                         bug: the check should have reported it first."
                    ));
                };
                let vis = match public {
                    true => "pub ",
                    false => "",
                };
                out.push(self.folded(*name));
                out.push(&format!("{vis}const {bound}: {below} = {written};\n"));
                Ok(())
            }
            // **Rust's own `extern` block**
            // ([ADR-302](../../docs/specification/adr/adr-302.md) D1), which is
            // the whole of the lowering: the form means the same thing on both
            // sides, and a declaration is `trait_method`'s shape with `sync`
            // said by the caller (D2).
            //
            // **C's convention, and not written** (ADR-324 D6): `extern "C"`
            // below, whatever the source said, and a source that named one is
            // the checker's `NK1227`.
            Item::Extern {
                abi,
                declarations,
                opaque,
                ..
            } => {
                // **The convention is not written** (ADR-324 D6): the checker
                // says `NK1227` about one that is; this is the backstop.
                if abi.is_some() {
                    return Err(refused_at!(
                        span.at(),
                        "[NK1227] `extern` takes no convention: write `extern {{ … }}`."
                    ));
                }
                // **A handle is a type of its own, outside the block**
                // ([ADR-302](../../docs/specification/adr/adr-302.md) D7). It
                // is written before the declarations, because they name it.
                for handle in opaque {
                    {
                        let st = out.mark();
                        self.opaque_type(out, &handle.node);
                        out.record(&handle.span, st);
                    }
                }
                out.push("unsafe extern \"C\" {\n");
                for declaration in declarations {
                    {
                        let st = out.mark();
                        out.push("    ");
                        self.trait_method(out, &declaration.node, true, true)?;
                        out.record(&declaration.span, st);
                    }
                }
                out.push("}\n");
                Ok(())
            }
            other => Err(refused_at!(
                span.at(),
                "The compiler can't translate this item yet: {other:?}"
            )),
        }
    }

    /// **An opaque handle, and the `cleanup` that ends its life**
    /// ([ADR-302](../../docs/specification/adr/adr-302.md) D7).
    ///
    /// ```ignore
    /// #[repr(transparent)]
    /// struct sqlite3(*mut core::ffi::c_void);
    /// impl Drop for sqlite3 { fn drop(&mut self) { unsafe { sqlite3_close(…) } } }
    /// ```
    ///
    /// **`repr(transparent)` and not a plain newtype**, because the whole point
    /// of the type is its layout: a handle *is* the address, so `&mut T` at the
    /// boundary is `T**` the way C writes it, and a handle passed by value is
    /// the pointer. A newtype Rust may lay out as it likes would be a different
    /// program at the boundary.
    ///
    /// **`Drop` and not a call the author writes**, which is Part I 6.4's
    /// `cleanup` read at the C boundary: the release runs at the end of the
    /// handle's scope, so a handle cannot be forgotten. What it cannot answer
    /// for is a C function that keeps the address past its own call, which is
    /// the one thing this language cannot check and D3 says so.
    ///
    /// **The release takes a handle and is handed one**, which is what keeps
    /// this from recursing: a value moved into an `extern "C"` function is the
    /// callee's, and C runs no `Drop`. Measured rather than reasoned — the
    /// release fires once per scope and not twice.
    ///
    /// **The address inside is `NonNull`**, which is
    /// [ADR-302](../../docs/specification/adr/adr-302.md) D11: a handle holds an
    /// address and `T?` is the absence of one, and those are the two states C
    /// spells with a pointer and `NULL`. Rust lays `Option<T>` over the same
    /// word for a type shaped like this, so a nullable handle costs nothing and
    /// `&mut T?` is `T **` exactly as C writes it — the right words around the
    /// right memory, which at this boundary is the only kind that counts.
    fn opaque_type(&self, out: &mut Out, handle: &crate::ast::OpaqueType) {
        out.push(&nikaia_std::tools::emit_misc::opaque_type_text(
            &self.name(handle.name),
            &self.name(handle.released_by),
        ));
    }

    /// **A type in an `extern "C"` declaration, as the pointer C wants**
    /// ([ADR-302](../../docs/specification/adr/adr-302.md) D5).
    ///
    /// The four forms the record writes, and each lives **for the call**, which
    /// is what a view is everywhere else in this language
    /// ([ADR-094](../../docs/specification/adr/adr-094.md)):
    ///
    /// | written | below |
    /// | :--- | :--- |
    /// | `&T` | `*const T` |
    /// | `&mut T` | `*mut T` |
    /// | `&[T]` | `*const T` |
    /// | `&mut [T]` | `*mut T` |
    ///
    /// **A slice loses its length here, and that is the point.** C takes a
    /// pointer and a count as two parameters, and D2 is the rule that keeps
    /// them one fact: the declaration names both and the call is checked.
    /// Rust's own `&[T]` is a *fat* pointer, so writing it in a declaration
    /// would be a signature the two languages disagree about — and what a
    /// reader would get for it is `rustc`'s `improper_ctypes` about a file
    /// nobody wrote (Part III, C.1).
    ///
    /// Anything that is not a view is the ordinary lowering: an `i32` is an
    /// `i32` at both ends.
    fn foreign_ty(&self, ty: &Type) -> String {
        // **`ref Array[T]` is what a run is written as**
        // ([ADR-184](../../docs/specification/adr/adr-184.md) D3, D4), here as
        // everywhere else. At this boundary it is still D1's address beside
        // D2's count and not Rust's fat pointer — two writers for one spelling,
        // which is the arrangement [ADR-302](../../docs/specification/adr/adr-302.md)
        // chose and this only renames.
        if self.writes_a_run(ty) {
            let element = match ty.generics.first() {
                Some(element) => self.foreign_ty(element),
                None => "u8".to_string(),
            };
            return format!("*{} {element}", pointing(ty.is_mut));
        }
        if ty.is_view {
            let pointed = Type {
                is_view: false,
                is_mut: false,
                ..ty.clone()
            };
            return format!("*{} {}", pointing(ty.is_mut), self.foreign_ty(&pointed));
        }
        self.ty(ty, Lifetimes::Elided)
    }

    /// The handle a type names, where it names one
    /// ([ADR-302](../../docs/specification/adr/adr-302.md) D7,
    /// [ADR-302](../../docs/specification/adr/adr-302.md) D10) — through a `?`,
    /// because `CStr?` is a handle that may be absent and is still a handle.
    ///
    /// `None` for a view or a slice of one: what a `&T` at the boundary is is
    /// D1's pointer, and the handle is what it points at.
    fn handle_named(&self, ty: &Type) -> Option<String> {
        self.s.handle_named(&self.world, ty)
    }

    /// What the address inside a handle points at, which is the one thing this
    /// language never looks through.
    fn pointed_at(&self, ty: &Type) -> String {
        self.s.pointed_at(&self.world, ty)
    }

    /// **Whether this written type is a run of elements somebody else keeps**
    /// ([ADR-184](../../docs/specification/adr/adr-184.md) D3).
    ///
    /// `ref Array[T]` is the spelling, and `Array[T, N]` is **not** one: the
    /// count is what makes an array a type laid out inline
    /// ([ADR-152](../../docs/specification/adr/adr-152.md) D4), so the number
    /// of arguments is what tells the two apart and nothing else has to.
    ///
    /// `is_slice` is the bracket form the grammar no longer reads (D4); it
    /// stays here because a ledger written before 0.0.134 still carries it and
    /// a description is read back through the same tree.
    fn writes_a_run(&self, ty: &Type) -> bool {
        self.s.writes_a_run(&self.world, ty)
    }

    /// One method of a `trait`: a signature and a `;`.
    ///
    /// Deliberately **not** `function` with the body switched off. That one
    /// reads the ledger for `sync`, for the `Shared` counts of each position and
    /// for whether the call can fail, and every one of those answers is about a
    /// *body* - which a declaration does not have. A second, smaller writer says
    /// what a declaration is instead of what a definition happens to omit.
    ///
    /// `sync` is the caller's rather than the declaration's, because the same
    /// shape reads two ways: a **trait** method without the word may pause
    /// ([ADR-288](../../docs/specification/adr/adr-288.md) D24), and an
    /// **`extern "C"`** declaration never can
    /// ([ADR-302](../../docs/specification/adr/adr-302.md) D2) — C has no
    /// suspension point, and a C function that sleeps blocks a thread, which is
    /// `println`'s question and not this one.
    fn trait_method(
        &self,
        out: &mut Out,
        method: &crate::ast::TraitMethod,
        is_sync: bool,
        foreign: bool,
    ) -> Result<()> {
        // A trait's methods are barriers (ADR-010 D9): the keyed map.
        self.floor.set(true);
        let done = self.trait_method_written(out, method, is_sync, foreign);
        self.floor.set(false);
        done
    }

    fn trait_method_written(
        &self,
        out: &mut Out,
        method: &crate::ast::TraitMethod,
        is_sync: bool,
        // **Whether this is a C declaration**
        // ([ADR-302](../../docs/specification/adr/adr-302.md) D5), which is
        // what decides how a view is written: at the C boundary it is the
        // pointer C wants, and everywhere else it is the borrow Rust wants.
        // The same `&str` is a thin pointer to C and a fat one to Rust, so a
        // declaration that wrote the second would be a signature the two
        // languages disagree about - and `rustc` says so, about a file nobody
        // wrote (Part III, C.1).
        foreign: bool,
    ) -> Result<()> {
        let mut params: Vec<String> = Vec::new();
        if let Some(receiver) = &method.receiver {
            params.push(
                match (receiver.is_ref, receiver.is_mut) {
                    (true, true) => "&mut self",
                    (true, false) => "&self",
                    _ => "self",
                }
                .to_string(),
            );
        }
        for arg in &method.args {
            // **`mut` lowers to `&mut T` here as it does in the impl**
            // (ADR-094 D3): a declaration that wrote the value would be a
            // signature the impl disagrees with.
            let reference = if arg.mutable && !foreign { "&mut " } else { "" };
            let written = match foreign {
                true => self.foreign_ty(&arg.ty),
                false => self.ty(&arg.ty, Lifetimes::Elided),
            };
            // **A list the method only reads is a slice**, as it is in the
            // impl and in every function that hands it on: a declaration that
            // kept `&Vec<T>` disagreed with both, and `rustc` refused the call
            // from a function whose `ref Vec[T]` is already a `&[T]`.
            let plain_list = !foreign
                && self.text(arg.ty.name) == "Vec"
                && arg.ty.generics.len() == 1
                && !arg.ty.is_nullable
                && !arg.ty.is_slice;
            let written = match written
                .strip_prefix("&Vec<")
                .and_then(|w| w.strip_suffix('>'))
            {
                Some(element) if plain_list => format!("&[{element}]"),
                _ => written,
            };
            params.push(format!("{}: {reference}{written}", self.name(arg.name)));
        }
        // Kap 5.1: the language below has neither named arguments nor defaults,
        // so an option is an ordinary parameter here too - the same rule a
        // definition follows, because a caller fills in one list either way.
        for option in &method.config {
            params.push(format!(
                "{}: {}",
                self.name(option.name),
                self.ty(&option.ty, Lifetimes::Elided)
            ));
        }
        let returned = match &method.ret_type {
            // **A handle comes back as the address it is**
            // ([ADR-302](../../docs/specification/adr/adr-302.md) D12), and the
            // call is where it becomes a handle. Declaring the *type* here
            // would be a lie the moment C hands back nothing: the hull under a
            // handle is non-null, so a null arriving in one is undefined
            // before any check could run.
            // **Except text a C library owns**, which comes back as the
            // `Option<CText>` it is: one word, `None` for `NULL`, and the
            // declaration is where *this is a C string* is promised (ADR-218).
            Some(ty) if foreign && base(self.text(ty.name)) == C_STRING => {
                "Option<nikaia_std::foreign::CText>".to_string()
            }
            Some(ty) if foreign && self.handle_named(ty).is_some() => {
                format!("*mut {}", self.pointed_at(ty))
            }
            Some(ty) => match foreign {
                true => self.foreign_ty(ty),
                false => self.ty(ty, Lifetimes::Elided),
            },
            None => "()".to_string(),
        };
        let outcome = if method.can_throw {
            format!("Result<{returned}, Box<dyn std::error::Error>>")
        } else {
            returned
        };
        // **A method that may pause is declared in the return-position form**
        // ([ADR-288](../../docs/specification/adr/adr-288.md) D26): `-> impl
        // Future<Output = …>`, which an `async fn` in the `impl` satisfies.
        //
        // **The sugar `async fn` is not written here**, and that is the reason
        // the long form is: `async_fn_in_trait` warns on a public trait whose
        // future carries no `Send` bound, and a warning about the generated file
        // is a defect in this project (Part III C.1).
        //
        // **`+ Send` follows the executor, not the type.** At
        // `user_parallelism = yes` a task crosses threads and a spawn needs a
        // `Send` future; at `no` nothing crosses, the counts are plain
        // ([ADR-037](../../docs/specification/adr/adr-037.md) D7) and a `Send`
        // demand would refuse them. It is a requirement of the executor this
        // program is built for, written here — not a claim about a type, which
        // is `contracts::send`'s and is the same at both settings.
        let ret = match (is_sync, method.ret_type.is_some() || method.can_throw) {
            (false, _) => {
                let send = match self.build.user_parallelism {
                    UserParallelism::Yes => " + Send",
                    UserParallelism::No => "",
                };
                format!(" -> impl std::future::Future<Output = {outcome}>{send}")
            }
            (true, true) => format!(" -> {outcome}"),
            (true, false) => String::new(),
        };
        let generics: Vec<String> = method.generics.iter().map(|g| self.bounded(g)).collect();
        out.push(&format!(
            "fn {}{}({}){ret};\n",
            self.name(method.name),
            angled(&generics),
            params.join(", ")
        ));
        Ok(())
    }

    /// The trait's declaration of `method`, where it is wider than the
    /// method itself (ADR-288 D2, D25): it may pause and the method's own
    /// body does not, or it may fail and the method does not. `None` for a
    /// method of a trait this program does not declare.
    fn declared_wider(
        &self,
        trait_name: Option<&str>,
        target: &str,
        method: &Item,
    ) -> Option<&crate::ast::TraitMethod> {
        let trait_name = trait_name?;
        let Item::Fn {
            name: Some(name),
            can_throw,
            ..
        } = method
        else {
            return None;
        };
        let name = self.text(*name);
        let declared = self
            .parsed
            .program
            .items
            .iter()
            .find_map(|item| match &item.node {
                Item::Trait { name, methods, .. } if self.text(*name) == trait_name => {
                    Some(methods)
                }
                _ => None,
            })?;
        let declared = declared
            .iter()
            .map(|m| &m.node)
            .find(|m| self.text(m.name) == name)?;
        let pauses_less = !declared.is_sync && !self.pauses(&format!("{target}::{name}"));
        let fails_less = declared.can_throw && !*can_throw;
        (pauses_less || fails_less).then_some(declared)
    }

    /// The trait's half of a bridged method: the declaration's own signature,
    /// written as the trait writes it, and a body that calls the type's own
    /// method - awaited where that one pauses, and its value wrapped in `Ok`
    /// where only the declaration fails. An inherent method is found before a
    /// trait's, so `Self::name` is the type's own.
    fn bridge(
        &self,
        out: &mut Out,
        declared: &crate::ast::TraitMethod,
        target: &str,
        method: &Item,
    ) -> Result<()> {
        let own_throws = matches!(
            method,
            Item::Fn {
                can_throw: true,
                ..
            }
        );
        let name = self.text(declared.name);
        let mut written = Out::empty();
        self.trait_method(&mut written, declared, declared.is_sync, false)?;
        let signature = written.buf.trim_end().trim_end_matches(';').to_string();
        let passed: Vec<String> = declared
            .receiver
            .iter()
            .map(|_| "self".to_string())
            .chain(declared.args.iter().map(|a| self.name(a.name).to_string()))
            .chain(
                declared
                    .config
                    .iter()
                    .map(|o| self.name(o.name).to_string()),
            )
            .collect();
        let awaited = match self.pauses(&format!("{target}::{name}")) {
            true => ".await",
            false => "",
        };
        let called = format!(
            "Self::{}({}){awaited}",
            self.name(declared.name),
            passed.join(", ")
        );
        let body = match declared.can_throw && !own_throws {
            true => format!("Ok({called})"),
            false => called,
        };
        let body = match declared.is_sync {
            true => body,
            false => format!("async move {{ {body} }}"),
        };
        out.push(&format!("{signature} {{ {body} }}\n"));
        Ok(())
    }

    fn bounded(&self, param: &crate::ast::GenericParam) -> String {
        nikaia_std::tools::emit_below::emit_bounded(&self.world, param)
    }

    /// A function or a method. Kap 4.2: a `pub fn` with no name is the
    /// anonymous constructor, called as `Type(…)`; Rust has no such thing, so
    /// it is emitted as `new` and the call sites follow.
    /// Kap 7.1: `impl Error for T` becomes the three impls the failure channel
    /// needs, from the one method the source wrote.
    ///
    /// The author writes `message`. Rust wants `Display` for the text, and
    /// `std::error::Error` for the value to be accepted as an error at all;
    /// `Box<dyn Error>` then takes it. None of that is a decision - it is the
    /// same transcription ADR-296 D17 asks of every other lowering - so the
    /// `.nika` file says the part that is one and the emitter supplies the rest.
    ///
    /// `Debug` comes along because `std::error::Error` requires it, and a
    /// derived one is what a user would have written.
    fn error_impl(
        &self,
        out: &mut Out,
        target: &str,
        params: &str,
        methods: &[Spanned<Item>],
        lifetimes: Lifetimes,
    ) -> Result<()> {
        out.push(&format!("impl{params} {target}{params} {{\n"));
        for method in methods {
            let carries = self.s.carries_input.get(&(method.span.at() as i64));
            {
                let st = out.mark();
                out.push("    ");
                self.function(
                    out,
                    method,
                    1,
                    lifetimes,
                    carries,
                    Some(MethodOf {
                        target,
                        declared_by: None,
                        bridged: None,
                    }),
                )?;
                out.record(&method.span, st);
            }
        }
        out.push("}\n");

        out.push(&format!(
            "impl{params} std::fmt::Display for {target}{params} {{\n\
             \x20   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n\
             \x20       f.write_str(&self.message())\n\
             \x20   }}\n\
             }}\n"
        ));
        out.push(&format!(
            "impl{params} std::error::Error for {target}{params} {{}}\n"
        ));
        Ok(())
    }

    fn function(
        &self,
        out: &mut Out,
        // The function with what stands before it: `@inline` (ADR-370) is
        // read off its attributes.
        written: &Spanned<Item>,
        depth: usize,
        lifetimes: Lifetimes,
        // The parameters written as views of the input buffer rather than of
        // the call (`Emitter::carries_input`). `None` where there is no such
        // parameter, which is almost every function.
        carries_input: Option<&std::collections::BTreeSet<String>>,
        // The type this is a method of, where it is one. It is what makes the
        // ledger key - `Counter::record` rather than `record` - and the key is
        // what a `Shared` position looks its count up by (`contracts::sharing`).
        owner: Option<MethodOf<'_>>,
    ) -> Result<()> {
        let Item::Fn {
            name,
            generics,
            receiver,
            args,
            config,
            spread,
            ret_type,
            body,
            is_sync,
            is_public,
            can_throw: throws,
            ..
        } = &written.node
        else {
            return Err(anyhow!("not a function"));
        };

        // **A trait's method answers the trait's declaration** (#732): its
        // result is not widened to `'static`, which would be narrower than the
        // trait's and refused by `rustc`. A bridged method's own half may be.
        let answers_a_trait = owner
            .as_ref()
            .is_some_and(|o| o.declared_by.is_some() && o.bridged.is_none());
        let pad = "    ".repeat(depth);
        if *is_sync {
            out.push(&format!(
                "// sync (Part II, 12.1): pure CPU, cannot pause. Checked before\n{pad}// this was written - see `contracts::sync`.\n{pad}"
            ));
        }

        // The key the ledger - and therefore `contracts::sharing` - records this
        // function under, arrived at the same way both of them arrive at it: the
        // anonymous constructor of Part I 4.2 is `new`, and a method carries its
        // type in front.
        let own_name = match name {
            Some(name) => self.text(*name).to_string(),
            None => "new".to_string(),
        };
        let key = match owner {
            Some(owner) => format!("{}::{own_name}", owner.target),
            None => own_name.clone(),
        };

        // **A function that stands alone** (not a method, not `pub`, no type
        // parameters): every call is seen, so its map parameters and result
        // may keep the fast hash of their class (ADR-010 D9).
        let standalone = owner.is_none() && !*is_public && generics.is_empty() && name.is_some();
        // **A `pub` function's map positions are its hidden type parameters**
        // (ADR-010 D9), numbered by the parameters and the result in order.
        // Inside the `impl` of a type that declares some, a method's own are
        // not built yet and stay as written.
        let in_seeded_impl = self.place.borrow().within();
        let base = match (&owner, in_seeded_impl) {
            (Some(o), true) => self.seeds.arity(o.target),
            _ => 0,
        };
        // **A signature of a barrier** (D9): a method of a trait and its
        // implementations, and a function that hands its map to one.
        let barrier = owner.is_some_and(|o| o.declared_by.is_some())
            || (owner.is_none_or(|o| o.declared_by.is_none())
                && name.is_some_and(|n| self.seeds.is_fixed(self.text(n))));
        self.floor.set(barrier);
        let seeded_fn = !self.seeds.is_empty()
            && !barrier
            && name.is_some()
            && owner.is_none_or(|o| o.declared_by.is_none())
            && args
                .iter()
                .map(|a| &a.ty)
                .chain(ret_type.iter())
                .any(|t| match *is_public {
                    true => self.seeds.positions(self.parsed, t) > 0,
                    false => self.seeds.mentions_a_struct(self.parsed, t),
                });
        if seeded_fn {
            let shape = match (*is_public, name) {
                (true, Some(n)) => {
                    let key = match &owner {
                        Some(o) => format!("{}::{}", o.target, self.text(*n)),
                        None => self.text(*n).to_string(),
                    };
                    self.seeds.shape(&key).cloned()
                }
                _ => None,
            };
            match shape {
                Some(shape) => self.place.borrow_mut().declare_fn(base, shape),
                None => self.place.borrow_mut().declare(base),
            }
        }
        let mut params = Vec::new();
        if let Some(receiver) = receiver {
            params.push(
                match (receiver.is_ref, receiver.is_mut) {
                    (true, true) => "&mut self",
                    (true, false) => "&self",
                    _ => "self",
                }
                .to_string(),
            );
        }
        // A parameter the subject's buffer covers is written as a view of that
        // buffer, so the signature says what the body does with it; everything
        // else keeps the position's own spelling.
        // **A function that takes a keep** (ADR-283 D10) writes the positions
        // its views leave through with the keep's lifetime, so `rustc` holds
        // the body to exactly what the plan says.
        let kept = self.kept_lifetimes(&key, lifetimes);
        // **A view handed back out of a parameter that holds views**
        // ([ADR-282](../../docs/specification/adr/adr-282.md) D20):
        // `fn first(xs: Vec[ref String]) -> ref String` lends `xs`, and the
        // result is one of the views inside it, not a borrow of the list. Two
        // lifetimes are in that parameter - the loan and the buffer - and the
        // result's is the buffer's, so it is named: `fn first<'a>(xs:
        // &Vec<&'a str>) -> &'a str`. Only where that one parameter is all
        // there is to borrow from; with two, which buffer is ADR-283's
        // question and already its refusal.
        let inner_views = match (lifetimes, receiver, kept, ret_type) {
            (Lifetimes::Elided, None, None, Some(ty))
                if self.carries_a_view(ty) && carries_input.is_none_or(|set| set.is_empty()) =>
            {
                let carrying: Vec<_> = args
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| self.carries_a_view(&a.ty))
                    .collect();
                // **Lent**, by the declaration's own test below: a parameter
                // handed over is one lifetime, which the elision names.
                let lent = |at: usize| {
                    self.s.own_contracts.functions.get(&key).is_some_and(|c| {
                        crate::contracts::keeps::lends_in_own(
                            c,
                            at,
                            &[self.world.library, &self.s.described],
                        ) && !c
                            .signature
                            .as_ref()
                            .and_then(|s| s.params.get(at))
                            .is_some_and(|(_, ty)| ty.is_a_view())
                    })
                };
                match carrying.as_slice() {
                    [(at, one)] if !one.ty.is_view && !one.ty.either && lent(*at) => Some(one.name),
                    _ => None,
                }
            }
            _ => None,
        };
        // **A parameter the body does not keep is a view**
        // ([ADR-094](../../docs/specification/adr/adr-094.md) D2). The `&` is
        // written here, in the declaration, and at the call — one answer read
        // twice (`contracts::keeps::lends`), because the two disagreeing is a
        // `&&T` or a moved value in the language below.
        let lent = self.s.own_contracts.functions.get(&key);
        // **A view of a parameter kept in another parameter's place**
        // ([ADR-283](../../docs/specification/adr/adr-283.md) D27): the ledger's
        // `stores` says `v:s`, and the two share one lifetime,
        // `s: &'a str, v: &mut Vec<&'a str>`. Not where the subject already
        // names the buffer (D1, `carries_input`).
        let stores: Vec<(String, String)> = match lifetimes {
            Lifetimes::Elided => lent
                .map(|c| {
                    c.stores
                        .iter()
                        .filter_map(|e| e.split_once(':'))
                        .map(|(p, s)| (p.to_string(), s.to_string()))
                        .filter(|(_, s)| !carries_input.is_some_and(|set| set.contains(s)))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        // **Only a place whose type is written with the lifetime**: a list the
        // text tiers made text of its own takes a copy at the push, and ties
        // nothing.
        let stores: Vec<(String, String)> = stores
            .into_iter()
            .filter(|(place, _)| {
                args.iter()
                    .find(|a| self.text(a.name) == place.as_str())
                    .is_some_and(|a| {
                        self.ty_counted(&a.ty, Lifetimes::Named, self.count_at(&key, place))
                            .contains("'a")
                    })
            })
            .collect();
        let store_source = |name: &str| stores.iter().any(|(_, s)| s == name);
        let store_place = |name: &str| stores.iter().any(|(p, _)| p == name);
        // **A parameter lowered as a view** ([ADR-283](../../docs/specification/adr/adr-283.md)
        // D24): one the body changes (`&mut`), or one it only reads, which is
        // a `&` below whatever its written type says (Part I 6.5).
        let lowered_as_a_view = |at: usize, a: &crate::ast::FnArg| {
            let at = at + usize::from(receiver.is_some());
            self.carries_a_view(&a.ty)
                || lent.is_some_and(|c| {
                    c.signature
                        .as_ref()
                        .is_some_and(|s| s.mutable.iter().any(|m| m == self.text(a.name)))
                        || crate::contracts::keeps::lends_in_own(
                            c,
                            at,
                            &[self.world.library, &self.s.described],
                        )
                })
        };
        // **A result points into one parameter, and the signature says which**
        // ([ADR-283](../../docs/specification/adr/adr-283.md) D25): the ledger's
        // `ref(..)` is what the body was read to, and the one parameter in it
        // and the result share one named lifetime, `'r`; no other parameter is
        // tied. With none, the result borrows from nothing a caller passed:
        // `'static`, or the keep's. The receiver alone is the elision's own
        // answer and needs nothing written.
        let result_is_a_view = ret_type
            .as_ref()
            .is_some_and(|ty| self.text(ty.name) != SELF_DSL && self.carries_a_view(ty));
        let plain_signature = matches!(lifetimes, Lifetimes::Elided | Lifetimes::Inner)
            && inner_views.is_none()
            && carries_input.is_none_or(|set| set.is_empty())
            && result_is_a_view;
        let result_borrows: Option<&Vec<String>> = lent.map(|c| &c.borrows);
        let lowered_views = args
            .iter()
            .enumerate()
            .filter(|(at, a)| lowered_as_a_view(*at, a))
            .count();
        let source_at: Option<usize> = match result_borrows {
            Some(named) if plain_signature && named.len() == 1 && named[0] != "self" => args
                .iter()
                .position(|a| self.text(a.name) == named[0])
                .filter(|at| lowered_as_a_view(*at, &args[*at]))
                .filter(|at| {
                    // A parameter that holds views of its own has two
                    // lifetimes, which elision cannot choose between.
                    let a = &args[*at];
                    let loan = a.ty.is_view
                        || lent.is_some_and(|c| {
                            crate::contracts::keeps::lends_in_own(
                                c,
                                *at + usize::from(receiver.is_some()),
                                &[self.world.library, &self.s.described],
                            )
                        });
                    receiver.is_some()
                        || lowered_views > 1
                        || kept.is_some()
                        || (loan
                            && self.carries_a_view(&a.ty)
                            && (self.borrows(a.ty.name)
                                || a.ty.generics.iter().any(|g| self.carries_a_view(g))))
                }),
            _ => None,
        };
        // **A value holding views of several parameters at once**
        // ([ADR-352](../../docs/specification/adr/adr-352.md)): every one of
        // them and the result share the one lifetime `'r`. Only where each is
        // a plain view, so that `'r` is the whole of what it lends.
        let several_at: Vec<usize> = match result_borrows {
            Some(named)
                if plain_signature
                    && named.len() >= 2
                    && receiver.is_none_or(|r| r.is_ref && !r.is_mut)
                    && kept.is_none()
                    && source_at.is_none() =>
            {
                // **The receiver among them** is `&'r self`.
                let named: Vec<&String> = named
                    .iter()
                    .filter(|n| receiver.is_none() || n.as_str() != "self")
                    .collect();
                let found: Vec<usize> = named
                    .iter()
                    .filter_map(|n| args.iter().position(|a| self.text(a.name) == n.as_str()))
                    .filter(|at| lowered_as_a_view(*at, &args[*at]))
                    .filter(|at| {
                        !self.borrows(args[*at].ty.name) && args[*at].ty.generics.is_empty()
                    })
                    .collect();
                match found.len() == named.len() {
                    true => found,
                    false => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        if receiver.is_some()
            && !several_at.is_empty()
            && result_borrows.is_some_and(|named| named.iter().any(|n| n == "self"))
            && params.first().is_some_and(|p| p == "&self")
        {
            params[0] = "&'r self".to_string();
        }
        // **An option is a parameter too**: a view among them is the source
        // where the body hands it back.
        // **A `String` option the body does not keep is lent**, as a
        // `String` parameter is (`lends::lends_option`): below it is the
        // view one written `ref String` is.
        let lent_option = |at: usize| {
            lent.is_some_and(|c| {
                c.signature
                    .as_ref()
                    .and_then(|s| s.config.get(at))
                    .is_some_and(|o| nikaia_std::tools::lends::lends_option(c, o))
            })
        };
        let source_option: Option<usize> = match result_borrows {
            Some(named) if plain_signature && named.len() == 1 && source_at.is_none() => {
                config.iter().enumerate().position(|(at, c)| {
                    self.text(c.name) == named[0] && (self.carries_a_view(&c.ty) || lent_option(at))
                })
            }
            _ => None,
        };
        // **What the parameter's views point into, not the parameter**: a
        // `ref` to a struct (or a list) of views has the loan and the buffer,
        // and the result is the buffer's - `&Txt<'r>` and `&'r str`.
        let source_nested = source_at.is_some_and(|at| {
            let a = &args[at];
            self.carries_a_view(&a.ty)
                && self.borrows(a.ty.name)
                && a.ty.generics.is_empty()
                && ret_type.as_ref().is_some_and(|r| r.name != a.ty.name)
        });
        let source_name: Option<Symbol> = source_at
            .or(several_at.first().copied())
            .map(|at| args[at].name)
            .or(source_option.map(|at| config[at].name));
        // **Tied to the keep** where a way out hands back a view of a buffer
        // the body made (D23) as well: one lifetime over both.
        let source_kept = source_name.is_some()
            && kept.is_some()
            && self.tethered_position(&key, crate::contracts::tether::RESULT);
        let shared = source_name.is_some() && !source_kept;
        // **No parameter at all**: the literal outlives everything. Not with a
        // keep, whose buffers the result may point into.
        let nothing_borrowed = EMPTY_SET_IS_STATIC
            && lifetimes == Lifetimes::Elided
            && plain_signature
            && kept.is_none()
            && result_borrows.is_some_and(|named| named.is_empty())
            && ret_type
                .as_ref()
                .is_some_and(nikaia_std::tools::views::holds_view)
            && (receiver.is_some()
                || lowered_views > 0
                || args.iter().any(|a| self.carries_a_view(&a.ty)));
        let how = |name: Symbol| {
            if store_place(self.text(name))
                || (store_source(self.text(name))
                    && args.iter().any(|a| a.name == name && a.ty.is_view))
            {
                return Lifetimes::Named;
            }
            if let Some(kept) = kept.filter(|_| self.tethered_position(&key, self.text(name))) {
                return kept;
            }
            if let (Some(kept), true) = (kept, source_kept && source_name == Some(name)) {
                return kept;
            }
            if inner_views == Some(name) {
                return Lifetimes::Named;
            }
            let viewed = args.iter().any(|a| a.name == name && a.ty.is_view);
            match (
                carries_input.is_some_and(|set| set.contains(self.text(name))),
                viewed,
            ) {
                (true, true) => lifetimes.of_the_input(),
                // **A struct parameter that carries a view parameter's buffer**
                // is written over it: `s: &mut Summary<'a>, name: &'a str`.
                (true, false) => lifetimes.over_the_input(),
                (false, _) => lifetimes,
            }
        };
        // **`xs: Array[T]` is an array of any length, and the call says which**
        // ([ADR-184](../../docs/specification/adr/adr-184.md) D3). The language
        // below has the same shape and the same name for it — a `const`
        // parameter — so the function is written once and monomorphised per
        // length, which is what *beliebig, aber fest* means when it reaches a
        // machine.
        //
        // **One per parameter**, because two arrays in one signature are two
        // lengths: `fn zip(a: Array[i64], b: Array[i64])` takes any two and not
        // two of the same.
        //
        // The name carries the position rather than the parameter's own name,
        // for the reason the escape exists (ADR-298 D10): a parameter may be
        // called anything, including something the language below reserves.
        let lengths: Vec<Option<String>> = args
            .iter()
            .enumerate()
            .map(|(at, a)| {
                let any = !a.ty.is_view
                    && !a.ty.is_slice
                    && a.ty.count.is_none()
                    && a.ty.generics.len() == 1
                    && self.text(a.ty.name) == ARRAY;
                any.then(|| format!("{LENGTH_PARAMETER}{at}"))
            })
            .collect();
        let mut either_first: Vec<String> = Vec::new();
        params.extend(args.iter().enumerate().map(|(at, a)| {
            let is_source = source_at == Some(at) || several_at.contains(&at);
            // The **source** name is what a `Shared` position is counted by
            // (`count_at`), and the **escaped** one is what is written: two uses
            // of one name that must not be collapsed, or the lookup misses on
            // exactly the programs the escape is for (ADR-298 D10).
            let name = self.text(a.name);
            // The receiver is a parameter of the contract and not of `args`, so
            // the position in the signature is one further along where there is
            // one.
            let at = at + usize::from(receiver.is_some());
            // **And only where the written type does not already carry one.**
            // `&Vec[Entry]` in a declaration is the assertion D2 keeps: the
            // parameter is a view either way, and what D1 changes is the
            // *call*, where the reference is written for both kinds alike.
            let written_as_a_view = lent
                .and_then(|c| c.signature.as_ref())
                .and_then(|s| s.params.get(at))
                .is_some_and(|(_, ty)| ty.is_a_view());
            // **`mut` is a third state and comes first**
            // ([ADR-094](../../docs/specification/adr/adr-094.md) D3): a
            // parameter the callee changes in place lowers to `&mut T`, and the
            // caller's value is what changes. `lends` withholds its own claim
            // on such a position, so the two never both answer.
            let changes = lent
                .and_then(|c| c.signature.as_ref())
                .is_some_and(|s| s.mutable.iter().any(|m| m == name));
            let reference = if changes {
                match (
                    is_source && shared && !source_nested,
                    is_source && source_kept,
                ) {
                    (true, _) => "&'r mut ",
                    (_, true) => "&'k mut ",
                    _ => "&mut ",
                }
            } else if lent.is_some_and(|c| {
                crate::contracts::keeps::lends_in_own(
                    c,
                    at,
                    &[self.world.library, &self.s.described],
                )
            }) && !written_as_a_view
            {
                if !is_source && store_source(name) {
                    "&'a "
                } else {
                    match (
                        is_source && shared && !source_nested,
                        is_source && source_kept,
                    ) {
                        (true, _) => "&'r ",
                        (_, true) => "&'k ",
                        _ => "&",
                    }
                }
            } else {
                ""
            };
            // **An array of any length is written with the length this
            // signature declares for it** (ADR-184 D3), rather than through
            // `ty_counted`, which would write the name `Array` — and there is
            // no type by that name in the language below.
            let written = match lengths.get(at.wrapping_sub(usize::from(receiver.is_some()))) {
                Some(Some(length)) => {
                    let element =
                        a.ty.generics
                            .first()
                            .map(|e| self.ty_counted(e, how(a.name), self.count_at(&key, name)))
                            .unwrap_or_else(|| "u8".to_string());
                    format!("[{element}; {length}]")
                }
                _ => {
                    // **Run is the absence of `keeps`**, off the very contract
                    // `lends` above was read from
                    // ([ADR-277](../../docs/specification/adr/adr-277.md) D11,
                    // [ADR-277](../../docs/specification/adr/adr-277.md) D8).
                    // A contract this build does not have reads as run, which
                    // is the same answer the *call* writer gives an unresolved
                    // callee - the two have to agree or one parameter gets two
                    // shapes.
                    let runs = lent.is_none_or(|c| !c.keeps.iter().any(|k| k == name));
                    let held =
                        self.walking(|w| std::mem::replace(&mut w.code_parameter_runs, runs));
                    let inside = self.walking(|w| std::mem::replace(&mut w.in_parameter, true));
                    let position = match (is_source && shared, source_nested) {
                        (true, true) => Lifetimes::Within,
                        (true, false) => Lifetimes::Shared,
                        _ => how(a.name),
                    };
                    let fast = standalone
                        && self.s.fast_slots.contains(&format!(
                            "p:{own_name}:{}",
                            at - usize::from(receiver.is_some())
                        ));
                    let outer = self.walking(|w| std::mem::replace(&mut w.trusting, fast));
                    let written = self.ty_counted(&a.ty, position, self.count_at(&key, name));
                    self.walking(|w| w.trusting = outer);
                    self.walking(|w| w.in_parameter = inside);
                    self.walking(|w| w.code_parameter_runs = held);
                    written
                }
            };
            // **A `String` the body only reads is a `&str`**
            // ([ADR-282](../../docs/specification/adr/adr-282.md) D5), not a
            // `&String`. Every caller's `String` reaches it through the `&` the
            // call already writes, and a literal reaches it as it is - which is
            // what makes `greet("Ada")` cost nothing, where a `&String` would
            // have needed a `String` built to be pointed at.
            //
            // A parameter both kinds of text flow into is one of these too
            // ([ADR-282](../../docs/specification/adr/adr-282.md) D14): a body
            // that only reads needs neither kind, only the text, so nothing is
            // wrapped for it and each caller lends what it has.
            let plain_text = self.text(a.ty.name) == "String"
                && a.ty.generics.is_empty()
                && (!a.ty.is_view || a.ty.either)
                && !a.ty.is_nullable
                && !a.ty.is_slice;
            // **A list the body only reads is a slice**, for the reason a
            // `String` is a `&str`: a `&Vec<T>` is a pointer to a pointer, and
            // every read through it loads the buffer and the length again
            // where a `&[T]` holds them in two registers. Every caller's `Vec`
            // reaches it through the `&` the call already writes.
            let plain_list = self.text(a.ty.name) == "Vec"
                && a.ty.generics.len() == 1
                && !a.ty.is_nullable
                && !a.ty.is_slice
                && written.ends_with('>');
            // **A `Bytes` the body only reads is a `&[u8]`**, as a `String`
            // is a `&str` (ADR-320 D11): a buffer read whole and a text's own
            // bytes (`s.bytes`) are both handed over without a copy.
            let plain_bytes = self.text(a.ty.name) == "Bytes"
                && a.ty.generics.is_empty()
                && !a.ty.is_nullable
                && !a.ty.is_slice;
            let written = match (reference, plain_text, plain_list) {
                ("&" | "&'r " | "&'k " | "&'a ", _, _) if plain_bytes => "[u8]".to_string(),
                ("&" | "&'r " | "&'k " | "&'a ", true, _) => "str".to_string(),
                ("&" | "&'r " | "&'k ", _, true) if written.starts_with("Vec<") => {
                    format!("[{}]", &written[4..written.len() - 1])
                }
                ("", _, true) if written.starts_with("&Vec<") => {
                    format!("&[{}]", &written[5..written.len() - 1])
                }
                _ => written,
            };
            // **A `ref Bytes` the body keeps takes the `Bytes` with its owner**
            // (ADR-373 D1, D4): a clone of it is a count, where a run of bytes
            // would have to be copied. One the result points into stays a run,
            // so a caller lending a mapping's `Bytes` borrows no temporary.
            let written = match lent.is_some_and(|c| {
                plain_bytes
                    && receiver.is_none()
                    && nikaia_std::tools::keeps::keeps_its_bytes(c, at as i64)
            }) {
                true => match written.strip_suffix("[u8]") {
                    Some(reference) if reference.starts_with('&') => format!("{reference}Bytes"),
                    _ => written,
                },
                false => written,
            };
            // **A published parameter both kinds of text flow into takes
            // whatever hands over as either** (ADR-282 D23): a caller elsewhere
            // hands over text of its own, as the ledger says, and this
            // package's own calls hand over what they have. It is taken as it
            // goes in on the body's first line.
            if *is_public && a.ty.either && a.ty.generics.is_empty() && reference.is_empty() {
                let (by, into) = match a.ty.is_nullable {
                    true => ("IntoEitherMaybe", "into_either_maybe"),
                    false => ("IntoEither", "into_either"),
                };
                let name = self.local(name);
                either_first.push(format!("let {name} = {name}.{into}();"));
                return format!(
                    "{name}: impl nikaia_std::either_text::{by}<Either = \
                     nikaia_std::either_text::EitherText<'e>>"
                );
            }
            format!("{}: {reference}{written}", self.local(name))
        }));
        // Kap 5.1: the language below has neither named arguments nor defaults,
        // so an option becomes an ordinary parameter here - in declaration
        // order, which is the order every call site fills in. The names stay
        // the source's, so a `rustc` diagnostic about one still lands on the
        // parameter the programmer wrote (ADR-300).
        params.extend(config.iter().enumerate().map(|(at, c)| {
            let name = self.text(c.name);
            let position = match source_option == Some(at) && shared {
                true => Lifetimes::Shared,
                false => how(c.name),
            };
            let mut ty = c.ty.clone();
            ty.is_view |= lent_option(at);
            format!(
                "{}: {}",
                self.local(name),
                self.ty_counted(&ty, position, self.count_at(&key, name))
            )
        }));

        // ADR-296 D5: `...args: Self::dsl` is the one parameter whose type the
        // *call site* decides, because the DSL string it comes from decides it.
        // A generic parameter is what that is in the language below, and Rust
        // monomorphises it per DSL string exactly as D5 asks - so the driver is
        // written once and pays no heap traffic per statement.
        let dsl = spread.as_ref().map(|name| {
            params.push(format!("{}: {DSL_PARAMETER}", self.text(*name)));
            DSL_PARAMETER.to_string()
        });
        // **The keep, last**, because every call writes it last - after the
        // options and the spread, which is the only order there is.
        if let Some(kept) = kept {
            params.push(format!(
                "{KEEP_PARAM}: {}nikaia_std::tether::Keep",
                kept.reference()
            ));
        }

        // **`fn hand[T](x: T)` is `fn hand<T>(x: T)`**
        // ([ADR-295](../../docs/specification/adr/adr-295.md) D3). The `[T]` was
        // read by the parser and then fell out here, because this position had
        // no slot for one - so a program that passed every stage of this
        // compiler asked `rustc` about a type nobody had declared, which is
        // Part III C.1's class exactly.
        //
        // **A copy has no type parameter left**
        // ([ADR-304](../../docs/specification/adr/adr-304.md) D6): the whole of
        // what the parameter was for is the shape, and the shape is written out
        // here. What stands in the signature is the type itself.
        // **And a length a parameter left open is a `const` parameter**
        // (ADR-184 D3), declared beside the type parameters because that is
        // what it is: a name the call fills in.
        let open: Vec<String> = lengths
            .iter()
            .flatten()
            .map(|length| format!("const {length}: usize"))
            .collect();
        let declared: Vec<String> = match self.walk(|w| w.specialising.is_some()) {
            true => dsl.clone().into_iter().chain(open).collect(),
            false => generics
                .iter()
                .map(|g| self.bounded(g))
                .chain(dsl.clone())
                .chain(open)
                .collect(),
        };
        let declared: Vec<String> = match kept {
            Some(Lifetimes::Kept) => std::iter::once("'k".to_string()).chain(declared).collect(),
            _ => declared,
        };
        // **The buffer a struct parameter carries is named on the function**
        // where no `impl` names it already.
        let declared: Vec<String> = match lifetimes == Lifetimes::Elided
            && (carries_input.is_some_and(|set| !set.is_empty())
                || inner_views.is_some()
                || !stores.is_empty())
        {
            true => std::iter::once("'a".to_string()).chain(declared).collect(),
            false => declared,
        };
        let declared: Vec<String> = match shared {
            true => std::iter::once("'r".to_string()).chain(declared).collect(),
            false => declared,
        };
        // The lifetime a published parameter's `EitherText` is of (ADR-282 D23).
        let declared: Vec<String> = match either_first.is_empty() {
            true => declared,
            false => std::iter::once("'e".to_string()).chain(declared).collect(),
        };
        self.walking(|w| w.either_prelude = either_first);

        // Kap 7.1: `throws` becomes a `Result` in the emitted Rust, over
        // `Box<dyn Error>` because Nikaia's own error types are not lowered
        // yet - the `?` the DSL driver needs works against it, and every error
        // keeps its own type behind it.
        let returned = match ret_type {
            // `Self::dsl` in the result position is the same type the spread
            // parameter has: a driver that hands the bound parameters back
            // names them the only way D5 gives it to name them.
            Some(ty) if self.text(ty.name) == SELF_DSL => {
                if dsl.is_none() {
                    // **The body's first statement**, which is the nearest
                    // place this walk has: a declaration has no span of its
                    // own here, and a refusal on the first line of the body is
                    // the same function the reader is looking at. The checker
                    // reaches for the same statement when it needs to name a
                    // function rather than a line inside one.
                    return Err(match body.stmts.first() {
                        Some(first) => refused_at!(
                            first.span.at(),
                            "`Self::dsl` stands for the function's `...args: Self::dsl` \
                             parameters, but this function has none."
                        ),
                        None => refused!(
                            "`Self::dsl` stands for the function's `...args: Self::dsl` \
                             parameters, but this function has none."
                        ),
                    });
                }
                DSL_PARAMETER.to_string()
            }
            Some(ty) => {
                // The result's lifetimes are not always the parameters': a view
                // handed back by a function that borrowed nothing has to say
                // `'static`, because there is nothing for Rust's elision to take
                // and `-> &str` is a message about the generated file
                // (`Lifetimes::Static`).
                //
                // A receiver counts as something to borrow from, and so does any
                // parameter that is a view or holds one. Where either is there,
                // the position keeps the spelling it had.
                //
                // **And a struct that holds a view is one too**, which is the
                // half `holds_view` alone could not see: `-> Vec[Entry]` where
                // `Entry` holds a `&str` carries a lifetime exactly as `-> &str`
                // does, and a function with nothing to borrow from lowered it to
                // `Vec<Entry<'_>>` — *missing lifetime specifier* about a file
                // nobody wrote. `carries_a_view` asks the question about the
                // whole type, the declaration included, and it is asked on
                // **both** sides so that `fn f(r: Reading) -> Reading` keeps the
                // elision it needs.
                //
                // **And only where the position declares no lifetime of its
                // own**, which is `Lifetimes::Elided` and is D9's own case: *a
                // free function's signature*. Inside an `impl` that declares
                // `'a` the result is the subject's `'a`, and `'static` there
                // says the value outlives the program — which is a promise the
                // `impl` cannot keep. `examples/1brc.nika`'s `Summary()` is
                // what said so: an anonymous constructor has no parameters and
                // no receiver, so the widening reached it.
                // **A parameter lowered as a view counts too**
                // ([ADR-283](../../docs/specification/adr/adr-283.md) D24): a
                // `String` the body only reads is a `&str` below (Part I 6.5),
                // and a view handed back through it borrows from the caller's
                // argument - `'static` there was *lifetime may not live long
                // enough* about a file nobody wrote.
                let borrows_from_something = receiver.is_some()
                    || args
                        .iter()
                        .enumerate()
                        .any(|(at, a)| self.carries_a_view(&a.ty) || lowered_as_a_view(at, a));
                let widens = lifetimes == Lifetimes::Elided
                    && !answers_a_trait
                    && self.carries_a_view(ty)
                    && (!borrows_from_something || nothing_borrowed);
                let result = match (widens, kept) {
                    (_, Some(kept))
                        if self.tethered_position(&key, crate::contracts::tether::RESULT) =>
                    {
                        kept
                    }
                    _ if shared => Lifetimes::Shared,
                    (true, _) => Lifetimes::Static,
                    (false, _) if inner_views.is_some() => Lifetimes::Named,
                    // **A published parameter's `'e` is the one lifetime there
                    // is** (ADR-282 D23): the elision does not read one out of
                    // an `impl` bound, so it is named.
                    (false, _)
                        if lifetimes == Lifetimes::Elided
                            && receiver.is_none()
                            && !self.walk(|w| w.either_prelude.is_empty())
                            && !args.iter().any(|a| {
                                self.carries_a_view(&a.ty)
                                    && !(a.ty.either && a.ty.generics.is_empty())
                            }) =>
                    {
                        Lifetimes::Each
                    }
                    (false, _) => lifetimes,
                };
                let fast = standalone && self.s.fast_slots.contains(&format!("r:{own_name}"));
                let outer = self.walking(|w| std::mem::replace(&mut w.trusting, fast));
                let written = self.ty_counted(ty, result, self.count_at(&key, SHARED_RESULT));
                self.walking(|w| w.trusting = outer);
                written
            }
            None => "()".to_string(),
        };
        // **The failure channel** ([ADR-280](../../docs/specification/adr/adr-280.md)
        // D1): the error type where the ledger names exactly one, the box
        // otherwise. A method that implements a **trait** keeps the box, and
        // has to: the trait's own declaration writes the channel, and an `impl`
        // answering with another type would not satisfy it.
        let channel = match owner.and_then(|o| o.declared_by) {
            Some(_) => "Box<dyn std::error::Error>".to_string(),
            None => self.error_channel(
                &key,
                lifetimes,
                receiver.is_some() || args.iter().any(|a| self.carries_a_view(&a.ty)),
            ),
        };
        // **One lifetime per parameter a view field points into**
        // ([ADR-353](../../docs/specification/adr/adr-353.md) D1): where the
        // ledger's `returns` names one parameter per field, `pair(a, b)` is
        // `fn pair<'r, 'r1>(a: &'r str, b: &'r1 str) -> Pair<'r, 'r1>`, so a
        // caller holding `.left` keeps only `a` borrowed.
        let (returned, declared) = match self.split_lifetimes(
            ret_type
                .as_ref()
                .filter(|_| shared && !source_nested && several_at.len() >= 2),
            lent.map(|c| c.returns.as_str()),
            &several_at
                .iter()
                .map(|at| self.text(args[*at].name).to_string())
                .collect::<Vec<_>>(),
        ) {
            Some((own, fields)) => {
                for (param, lifetime) in &own {
                    let head = format!("{}: ", self.local(param));
                    if let Some(p) = params.iter_mut().find(|p| p.starts_with(&head)) {
                        *p = renamed_lifetime(p, "'r", lifetime);
                    }
                }
                let returned = match ret_type.as_ref().map(|t| t.name) {
                    Some(t) => returned.replacen(
                        &format!(
                            "{}<{}>",
                            self.name(t),
                            self.s.lifetime_args(self.text(t), "'r")
                        ),
                        &format!("{}<{}>", self.name(t), fields.join(", ")),
                        1,
                    ),
                    None => returned,
                };
                let at = declared
                    .iter()
                    .position(|d| d == "'r")
                    .map_or(0, |at| at + 1);
                let mut declared = declared;
                for (offset, (_, lifetime)) in own.iter().filter(|(_, l)| l != "'r").enumerate() {
                    declared.insert(at + offset, lifetime.clone());
                }
                (returned, declared)
            }
            None => (returned, declared),
        };
        let ret = if *throws {
            format!(" -> Result<{returned}, {channel}>")
        } else if ret_type.is_some() {
            format!(" -> {returned}")
        } else {
            String::new()
        };

        self.floor.set(false);
        let hidden = match seeded_fn {
            true => self.place.borrow_mut().finish(base),
            false => 0,
        };
        let declared: Vec<String> = declared
            .into_iter()
            .chain(seeds::Seeds::declared(base, hidden, false))
            .collect();
        // A map made in a body whose positions are parameters is made with the
        // hasher the parameter holds.
        let outer_unpinned =
            self.walking(|w| std::mem::replace(&mut w.unpinned, hidden > 0 || in_seeded_impl));
        let vis = match owner.and_then(|o| o.bridged) {
            Some(true) => "pub ",
            _ if *is_public => "pub ",
            _ => "",
        };
        let name = own_name;
        // ADR-303 D4: `fn main` is the runtime's, and the program's own entry
        // point is called from inside it. Only at the crate root, and only for
        // the shape `entry_point` writes a wrapper for - `user_main` and this
        // ask the same question, so a renamed function always has a caller.
        //
        // The **source** name is kept for the body, because it is what an
        // error raised in here reports as its site (ADR-023 D6, ADR-280): a
        // `throw` in `main` says `main`, and the name this emitter chose for
        // the lowering is not something the author ever wrote.
        let emitted = if depth == 0 && name == MAIN && self.user_main().is_some() {
            PROGRAM_MAIN.to_string()
        } else {
            escaped(&name).into_owned()
        };
        // **One copy per type this was used with**
        // ([ADR-304](../../docs/specification/adr/adr-304.md) D6), named so the
        // call and the definition cannot drift: `check::specialised` writes
        // both.
        let emitted = match self.standing_for() {
            Some(on) => crate::check::specialised(&emitted, &on),
            None => emitted,
        };
        if emitted == PROGRAM_MAIN {
            *self.main_params.borrow_mut() = params.clone();
        }

        // ADR-055 D1: a function that can pause is an `async fn`, and one the
        // ledger's `sync` column says cannot is a plain `fn`. The property is
        // ADR-288 D1's, already inferred; this reads it.
        //
        // **And a trait's declaration is the wider claim**
        // ([ADR-288](../../docs/specification/adr/adr-288.md) D25): a method
        // declared without `sync` is lowered `-> impl Future<…>`, so every
        // implementation of it hands back a future — whether or not its own
        // body pauses. A body that never pauses under such a declaration is
        // correct and says nothing, which is D2's own sentence; what it cannot
        // do is hand back an `i64` where a future was promised.
        let declared_pausing = owner
            .filter(|owner| owner.bridged.is_none())
            .and_then(|owner| owner.declared_by)
            .map(|t| format!("{t}::{name}"))
            .is_some_and(|key| {
                self.s
                    .own_contracts
                    .functions
                    .get(&key)
                    .is_some_and(|c| !c.sync_claim.is_sync())
            });
        // **A function written `sync(f)` is the `async fn` its body is**
        // ([ADR-288](../../docs/specification/adr/adr-288.md) D30, D31): `f` may
        // pause, so the call of it inside is awaited. What the promise changes
        // is the caller's check, and how a caller that cannot pause drives it.
        let pausing = if self.pauses(&key) || declared_pausing || self.lambda_decides(&key) {
            "async "
        } else {
            ""
        };
        // **A name this compiler chose is not one the program wrote**
        // ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)):
        // `describe__User` is a copy's name ([ADR-304](../../docs/specification/adr/adr-304.md)
        // D2) and Rust's own lint would ask the author to rename a function
        // they wrote as `describe`.
        if self.walk(|w| w.specialising.is_some()) {
            out.push("#[allow(non_snake_case)]\n");
            out.push(&pad);
        }
        // **A function takes the parameters the program wrote** (#459): the
        // language below's lint asks for fewer than eight inputs, which is a
        // rule about code written in it, and the warning would be about a file
        // nobody wrote.
        if args.len() + usize::from(receiver.is_some()) > 7 {
            out.push("#[allow(clippy::too_many_arguments)]\n");
            out.push(&pad);
        }
        // **What a fold's step calls runs once per item** (#433): LLVM left
        // 1BRC's `Summary::record` out of line - hashbrown's insert path makes
        // it too big for its heuristic - at 319 instructions a row, and 300
        // with the hint. Only a hint, and only here: every function `#[inline]`
        // would bloat the code and the build.
        //
        // **And what the program wrote wins** ([ADR-370](../../docs/specification/adr/adr-370.md)
        // D1-D2): `@inline`, `@inline(always)` or `@inline(never)` is Rust's
        // attribute of the same name, and a written one replaces the hint
        // above - `@inline(never)` keeps a fold's step's callee out of line.
        let hint = crate::check::attributes::inline_written(self.parsed, &written.attributes)
            .or_else(|| self.s.fold_callees.contains(&name).then_some("#[inline]"));
        if let Some(hint) = hint {
            out.push(hint);
            out.push("\n");
            out.push(&pad);
        }
        // **A function with a precondition has two entries**
        // ([ADR-269](../../docs/specification/adr/adr-269.md) D20): `f` checks
        // it and calls `f__unchecked`, which is the body. A caller the prover
        // saw prove it, or check it at the call, calls the second; every other
        // caller - a function value, another package, the language below -
        // reaches the first. A copy made per type is reached by name from the
        // call that made it, so it checks at the top of its body instead.
        let checks = self.s.facts.entries.get(&key).filter(|c| !c.is_empty());
        let copy = self.standing_for().is_some();
        let signature = |name: &str| {
            format!(
                "{vis}{pausing}fn {name}{}({}){ret} ",
                angled(&declared),
                params.join(", ")
            )
        };
        let check_lines =
            |checks: &[crate::prove::Check]| checks.iter().map(failed_check).collect::<String>();
        let pending = match checks {
            Some(checks) if !copy => {
                out.push("#[allow(non_snake_case)]\n");
                out.push(&pad);
                out.push(&signature(&format!("{emitted}{UNCHECKED}")));
                // The checked entry is written after the body, below.
                // The checked entry carries the written hint too: a call
                // from elsewhere reaches it first.
                let written_hint =
                    crate::check::attributes::inline_written(self.parsed, &written.attributes)
                        .map(|h| format!("{pad}{h}\n"))
                        .unwrap_or_default();
                Some(format!(
                    "{pad}#[allow(dead_code)]\n{written_hint}{pad}{}{{ {}{}{UNCHECKED}({}){} }}\n",
                    signature(&emitted),
                    check_lines(checks),
                    match (receiver.is_some(), owner.is_some()) {
                        (true, _) => format!("self.{emitted}"),
                        (false, true) => format!("Self::{emitted}"),
                        (false, false) => emitted.clone(),
                    },
                    params
                        .iter()
                        .filter(|p| !p.ends_with("self"))
                        .map(|p| p
                            .split(':')
                            .next()
                            .unwrap_or_default()
                            .trim()
                            .trim_start_matches("mut ")
                            .to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    if pausing.is_empty() { "" } else { ".await" },
                ))
            }
            Some(checks) => {
                out.push(&signature(&emitted));
                out.push(&format!("{{ {}", check_lines(checks)));
                Some("}".to_string())
            }
            None => {
                out.push(&signature(&emitted));
                None
            }
        };
        // **The parameters a call has to await**
        // ([ADR-277](../../docs/specification/adr/adr-277.md) D11): those whose
        // type is code and does not say `sync`, which is the default.
        let awaited: Vec<Symbol> = args
            .iter()
            .filter(|arg| (*arg.ty.code).as_ref().is_some_and(|code| !code.is_sync))
            .map(|arg| arg.name)
            .collect();
        let declared = Declared {
            throws: *throws,
            returns_value: ret_type.is_some(),
            awaited: &awaited,
            key: &key,
            channel: &channel,
        };
        match self.s.facts.settle_fns.get(&key) {
            // **A body where a value with a cleanup may die ends at a settle
            // point** ([ADR-297](../../docs/specification/adr/adr-297.md) D2):
            // the body runs to its end as a block of its own - a `return` and
            // a failing call end the block and not the function - and what
            // died in it is settled before the function hands its value back.
            Some(&failing) => {
                let body = {
                    let mut inner_buf = Out::empty();
                    let inner = &mut inner_buf;
                    self.function_body(inner, body, depth, declared)?;
                    inner_buf
                };
                let settle = match failing {
                    true => "nikaia_std::cleanup::settle_after",
                    false => "nikaia_std::cleanup::settle_with",
                };
                let kind = self.settle_kind();
                let rest = match failing {
                    true => "_, _, _",
                    false => "_, _",
                };
                out.push(&format!("{{ {settle}::<{kind}, {rest}>(async "));
                out.append(body);
                // A cleanup's failure is raised where the settle point is
                // ([ADR-280](../../docs/specification/adr/adr-280.md) D29).
                match failing {
                    true => {
                        let origin = key.rsplit("::").next().unwrap_or(&key);
                        out.push(&format!(", &{origin:?}).await }}"));
                    }
                    false => out.push(").await }"),
                }
            }
            None => self.function_body(out, body, depth, declared)?,
        }
        self.walking(|w| w.unpinned = outer_unpinned);
        // The checked entry after the unchecked one's body, or the brace that
        // closes a copy's checks around its body.
        match pending {
            Some(closing) if closing == "}" => out.push(" }\n"),
            Some(entry) => {
                out.push("\n");
                out.push(&entry);
            }
            None => out.push("\n"),
        }
        Ok(())
    }

    /// The body of a function, which differs from any other block in one way:
    /// a `throws` function has to hand back an `Ok`.
    fn function_body(
        &self,
        out: &mut Out,
        body: &Block,
        depth: usize,
        declared: Declared<'_>,
    ) -> Result<()> {
        let Declared {
            throws,
            returns_value,
            awaited,
            key,
            channel,
        } = declared;
        self.walking(|w| w.keep_function = key.to_string());
        let locals: std::collections::BTreeSet<String> = match self
            .nonnegative_bodies
            .get(&(std::ptr::from_ref(body) as usize))
        {
            Some(known) => known.iter().map(|s| self.text(*s).to_string()).collect(),
            None => nikaia_std::tools::emit_locals::nonnegative_locals(body)
                .into_iter()
                .map(|s| self.text(s).to_string())
                .collect(),
        };
        self.walking(|w| w.nonnegative.insert(key.to_string(), locals));
        let flow = Flow {
            changed: &[],
            tasks_channel: "",
            tasks_function: "",
            awaited,
            throws,
            origin: key.rsplit("::").next().unwrap_or(key),
            caught: false,
            // A function body is not a handler, whatever the call to it sits
            // inside: the boundary `in_lambda` does not cross, this does not
            // cross either.
            caught_named: false,
            caught_sum: None,
            caught_member: None,
            in_a_place: false,
            arms_are_statements: false,
            statement: usize::MAX,
            function: key,
            channel,
            // A function body is not a `catch`'s guarded half.
            handled_here: false,
            // A function body was not written inside whatever lambda the call
            // to it sits in: this is the one boundary the flag does not cross.
            in_lambda: false,
            // A function body binds nothing until a statement in it does.
            bound: "",
            // Both are decided per expression, so a body starts with neither.
            widen: false,
            inferred: false,
            // And no loop encloses a function's first statement.
            in_loop: false,
            // Nor a `catch` a call in it could break out to.
            breaks_to: -1,
        };

        // A function body's last statement is the *function's* value, which is
        // the one place a `return x` may be written as `x`.
        let tail = if returns_value {
            Tail::Return
        } else {
            Tail::Statement
        };

        // A published parameter taken as it goes in (ADR-282 D23), first.
        let mut first = self.walking(|w| std::mem::take(&mut w.either_prelude));
        // **A call is a step of a build-time run**, and the function is on its
        // path while it runs (ADR-321 D7).
        if self.build.counts_steps {
            first.insert(
                0,
                format!("let __nikaia_frame = nikaia_std::build_time::enter({key:?});"),
            );
        }
        // **Not after a statement that never comes back** (0.0.240): a body
        // that ends in `panic(…)`, `throw` or `return` has left before the
        // `Ok(())`, and `rustc` said *unreachable expression* about a file
        // nobody wrote ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
        let leaves = body.stmts.last().is_some_and(|stmt| match &stmt.node {
            Stmt::Return(_) => true,
            Stmt::Expr(expr) => jumps(expr) || self.never_returns(expr),
            _ => false,
        });
        // **The program's `main` says where its last statement ended**
        // ([ADR-348](../../docs/specification/adr/adr-348.md) D1): from there,
        // `skip-free-at-exit:on` leaves the memory of what dies to the system.
        let main_ends = (self.build.skip_free_at_exit
            && depth == 0
            && key == MAIN
            && self.user_main().is_some()
            && !leaves)
            .then_some("nikaia_std::rt::exit::main_ends();");
        if !throws {
            // The function's own settle point stands around the body, so the
            // body is not settled a second time as a block (ADR-297 D2).
            let settled = self.s.facts.settle_fns.contains_key(key);
            let body_block = |out: &mut Out| match settled {
                true => self.block_plain(out, body, depth, flow, tail, None),
                false => self.block(out, body, depth, flow, tail),
            };
            if first.is_empty() && main_ends.is_none() {
                return body_block(out);
            }
            // Inside the body's own brace, so there is one block and not two.
            let mut written = Out::empty();
            body_block(&mut written)?;
            if let Some(call) = main_ends {
                written.insert_before_last_brace(&format!(" {call} "));
            }
            if !first.is_empty() {
                written.insert_after_first_brace(&format!(" {}", first.join(" ")));
            }
            out.append(written);
            return Ok(());
        }

        let pad = "    ".repeat(depth);
        let inner_pad = "    ".repeat(depth + 1);

        out.push("{\n");
        for line in &first {
            out.push(&format!("{inner_pad}{line}\n"));
        }
        let last = body.stmts.len().saturating_sub(1);
        let mut i = 0;
        while i < body.stmts.len() {
            let stmt = &body.stmts[i];
            out.push(&inner_pad);
            // A value-returning `throws` function ends in its value; one that
            // returns nothing ends in the `Ok(())` below, so its last statement
            // is a statement like any other.
            let here = tail.at(i as i64, last as i64);
            // **A tail that is a `throw` is not wrapped**
            // ([ADR-292](../../docs/specification/adr/adr-292.md) D20): it writes
            // its own `Err(…)` and leaves, so an `Ok(` around it is
            // `Ok(return Err(…))` — *unreachable call*, about a file nobody
            // wrote. `fn f() -> String throws { throw E() }` is the whole
            // program it takes.
            //
            // A tail `return` is the other way round and stays wrapped: it is
            // rewritten to its bare value here (the one place Part I allows
            // that), and the `Ok(` is what makes that value the function's
            // outcome.
            let wrap = here == Tail::Return && !matches!(&stmt.node, Stmt::Expr(Expr::Throw(_, _)));
            // **And a tail that enters a grammar binds its value first**
            // ([ADR-185](../../docs/specification/adr/adr-185.md) D2). The
            // lowering of an entry is a block holding `let _source = &*data`
            // and a stream over it, and in `Ok({ … }?)` those temporaries live
            // to the end of the enclosing block — past the **local** that owns
            // the text. `rustc` then says *`data` does not live long enough*
            // about a file nobody wrote
            // ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)),
            // and its own hint is this fix: *save the expression's value in a
            // new local variable*.
            //
            // A `let` makes the block a **statement**, so its temporaries drop
            // at the `;` and ahead of the local — which is `ARM_VALUE`'s trick
            // one construct over ([ADR-292](../../docs/specification/adr/adr-292.md)
            // D1), for the same kind of reason.
            //
            // **Only where the lowering writes the borrow**, which is the one
            // shape this emitter can be sure of: it is the emitter's own
            // `_source`, not something a program's expression left behind. Every
            // other tail keeps `Ok(x)`, because a line the generated file does
            // not need is a line a reader has to skip (ADR-296 D17).
            let binds = wrap && self.a_tail_that_enters_a_grammar(&stmt.node);
            // **The keeps a statement needs are declared before it**
            // (ADR-283), and before the `Ok(` a tail is wrapped in: a `let`
            // inside it would not be Rust.
            self.write_keep_prelude(out, key, stmt.span.at(), depth + 1);
            {
                let st = out.mark();
                match (wrap, binds) {
                    (true, true) => out.push(&format!("let {ARM_VALUE} = ")),
                    (true, false) => out.push("Ok("),
                    (false, _) => {}
                }
                self.stmt(out, &stmt.node, &stmt.span, depth + 1, here, flow)?;
                match (wrap, binds) {
                    (true, true) => out.push(&format!(";\n{inner_pad}Ok({ARM_VALUE})")),
                    (true, false) => out.push(")"),
                    (false, _) => {}
                }
                out.record(&stmt.span, st);
            }
            out.push("\n");
            i += 1;
        }
        if let Some(call) = main_ends {
            out.push(&format!("{inner_pad}{call}\n"));
        }
        if !returns_value && !leaves {
            out.push(&format!("{inner_pad}Ok(())\n"));
        }
        out.push(&pad);
        out.push("}");
        Ok(())
    }

    /// **Whether this statement enters a grammar**
    /// ([ADR-185](../../docs/specification/adr/adr-185.md) D2).
    ///
    /// The one lowering that leaves a temporary borrowing a local behind: the
    /// entry's block binds `_source` to a view of its input and builds a stream
    /// over it. Read off the **statement** rather than off the type, because
    /// what has to know is the writer of the `Ok(` around it and this emitter
    /// has no types ([ADR-288](../../docs/specification/adr/adr-288.md)).
    fn a_tail_that_enters_a_grammar(&self, stmt: &Stmt) -> bool {
        self.s.a_tail_that_enters_a_grammar(&self.world, stmt)
    }

    // --- Grammars ---

    /// A grammar, and where a call enters it with a `Bytes`, its copy over
    /// bytes after it ([ADR-366](../../docs/specification/adr/adr-366.md) D1):
    /// the backend reads one input per grammar (winnow-grammar ADR 25).
    fn grammar(&self, out: &mut Out, def: &GrammarDef) -> Result<()> {
        let name = self.text(def.name).to_string();
        self.grammar_over(out, def, &name, false)?;
        if self.s.facts.bytes_grammars.contains(&name) {
            let lowered = nikaia_std::tools::emit_core_leaves::emit_bytes_grammar(&name);
            self.grammar_over(out, def, &lowered, true)?;
        }
        for rule in def.rules.iter().filter(|rule| rule.is_entry) {
            if let Some(spec) = a_par_fold(rule) {
                self.fold_options(out, &name, self.text(rule.name), spec)?;
            }
        }
        Ok(())
    }

    /// **The options a `par_fold` entry names after its `;`**
    /// ([ADR-372](../../docs/specification/adr/adr-372.md) D6), made by a
    /// function beside the grammar that every call of the entry calls: the
    /// grammar's own module is the backend's, and they are evaluated, and
    /// checked, where the parse starts.
    fn fold_options(
        &self,
        out: &mut Out,
        grammar: &str,
        rule: &str,
        spec: &FoldSpec,
    ) -> Result<()> {
        let named = |wanted: &str| {
            spec.config
                .iter()
                .find(|option| self.text(option.name) == wanted)
                .map(|option| &option.value)
        };
        let function = nikaia_std::tools::emit_core_leaves::emit_fold_options(grammar, rule);
        out.push(&format!(
            "\n#[allow(non_snake_case, dead_code)]\npub fn {function}() -> nikaia_std::par::FoldOptions {{\n    nikaia_std::par::FoldOptions::new("
        ));
        match named("order") {
            Some(order) => self.expr(out, order, 0, Flow::PLAIN)?,
            None => out.push("nikaia_std::par::Order::Kept"),
        }
        for option in ["workers", "piece"] {
            match named(option) {
                Some(value) => {
                    out.push(", Some(");
                    self.expr(out, value, 0, Flow::PLAIN)?;
                    out.push(")");
                }
                None => out.push(", None"),
            }
        }
        out.push(")\n}\n");
        Ok(())
    }

    fn grammar_over(&self, out: &mut Out, def: &GrammarDef, name: &str, bytes: bool) -> Result<()> {
        out.push("grammar! {\n");
        out.push(&format!("    grammar {name} {{\n"));
        if bytes {
            out.push("        input bytes;\n");
        }
        self.grammar_bytes.set(bytes);
        let written = self.grammar_body(out, def, bytes);
        self.grammar_bytes.set(false);
        written
    }

    fn grammar_body(&self, out: &mut Out, def: &GrammarDef, bytes: bool) -> Result<()> {
        let types = def
            .rules
            .iter()
            .filter_map(|rule| {
                let ty = rule.ret_type.as_ref()?;
                Some((
                    self.text(rule.name).to_string(),
                    self.ty_filled(ty, Lifetimes::Named),
                ))
            })
            .collect();
        self.walking(|w| w.grammar_rule_types = types);
        for rule in &def.rules {
            out.push("\n");
            {
                let st = out.mark();
                self.grammar_rule(out, rule)?;
                if !bytes {
                    out.record(&rule.span, st);
                }
            }
        }
        // **`list(item, sep)` is a rule the backend is given** (Part II
        // 10.8, issue #165 closed at 0.0.249): the backend has no
        // such element and read `list` as a rule of the grammar's own, so each
        // grammar that writes one gets the two template rules it lowers to -
        // the first item, then every `sep item` after it, or nothing at all.
        let lists = def
            .rules
            .iter()
            .flat_map(|rule| &rule.alts)
            .any(|alt| writes_a_list(self.parsed, &alt.pattern));
        if lists {
            out.push(&format!(
                "\n        rule {LIST_RULE}<T>(item: Rule<T>, sep) -> Vec<T> =\n\
                 \x20         first:item rest:{LIST_RULE}_more<T>(item = item, sep = sep)*\n\
                 \x20           -> {{ let mut all = vec![first]; all.extend(rest); all }}\n\
                 \x20       | \"\"\n\
                 \x20           -> {{ Vec::new() }}\n\
                 \n        rule {LIST_RULE}_more<T>(item: Rule<T>, sep) -> T =\n\
                 \x20         sep x:item\n\
                 \x20           -> {{ x }}\n"
            ));
        }

        out.push("    }\n");
        out.push("}\n");
        Ok(())
    }

    fn grammar_rule(&self, out: &mut Out, rule: &GrammarRule) -> Result<()> {
        if let Some(frame) = &rule.frame {
            out.push(&format!("        {}\n", frame_attribute(frame)));
        }

        // An entry is a `pub` rule to the backend: one function per entry
        // (ADR-296 D25).
        let vis = if rule.is_entry { "pub " } else { "" };
        let ret = match &rule.ret_type {
            Some(ty) => format!(" -> {}", self.ty_filled(ty, Lifetimes::Named)),
            None => String::new(),
        };
        // The label sits between the return type and the `=`, in both
        // languages: `rule expr -> Expr # "expression" = …`.
        let label = match &rule.label {
            Some(text) => format!(" # {text:?}"),
            None => String::new(),
        };
        out.push(&format!(
            "        {vis}rule {}{ret}{label} =\n",
            self.text(rule.name)
        ));

        for (i, alt) in rule.alts.iter().enumerate() {
            let separator = if i == 0 { "  " } else { "| " };

            match &alt.action {
                Some(action) => {
                    out.push(&format!("          {separator}"));
                    self.pattern(out, &alt.pattern)?;
                    out.push("\n            -> ");
                    // In a text grammar the backend has no `bytes(p)`: it is
                    // `text(p)` there, and the action sees its bytes (ADR-366
                    // D4).
                    let mut as_bytes = Vec::new();
                    if !self.grammar_bytes.get() {
                        bytes_bindings(
                            &alt.pattern.node,
                            &|s| self.text(s) == "bytes",
                            &mut as_bytes,
                        );
                    }
                    if !as_bytes.is_empty() {
                        out.push("{ ");
                        for name in &as_bytes {
                            let name = self.text(*name);
                            out.push(&format!("let {name} = {name}.as_bytes(); "));
                        }
                    }
                    // A grammar action is the rule's own body: what it ends
                    // in is what the rule hands back, so a `return` there is
                    // that value (ADR-296).
                    self.block(out, action, 3, Flow::PLAIN, Tail::Return)?;
                    if !as_bytes.is_empty() {
                        out.push(" }");
                    }
                    out.push("\n");
                }
                None => {
                    // The one body that needs no action: a `par_fold` is the
                    // whole rule (ADR-296 D10), so the value of the rule is the
                    // value of the fold. The binding the backend wants is
                    // supplied here rather than demanded from the user.
                    if rule.ret_type.is_some() {
                        out.push(&format!("          {separator}folded:"));
                        self.pattern(out, &alt.pattern)?;
                        out.push("\n            -> { folded }\n");
                    } else {
                        out.push(&format!("          {separator}"));
                        self.pattern(out, &alt.pattern)?;
                        out.push("\n");
                    }
                }
            }
        }

        Ok(())
    }

    fn pattern(&self, out: &mut Out, pattern: &Spanned<Pattern>) -> Result<()> {
        let st = out.mark();
        self.pattern_body(out, pattern)?;
        out.record(&pattern.span, st);
        Ok(())
    }

    fn pattern_body(&self, out: &mut Out, pattern: &Spanned<Pattern>) -> Result<()> {
        match &pattern.node {
            Pattern::Seq(parts) => self.patterns(out, parts, " "),
            Pattern::Choice(parts) => self.patterns(out, parts, " | "),
            Pattern::Bind { name, pat } => {
                out.push(&format!("{}:", self.text(*name)));
                // A literal's capture is text in Nikaia; over bytes the backend
                // hands back bytes, so it is checked (ADR-366 D2).
                if self.grammar_bytes.get() && matches!(pat.node, Pattern::Literal(_)) {
                    out.push("text(");
                    self.pattern(out, pat)?;
                    out.push(")");
                    return Ok(());
                }
                self.pattern(out, pat)
            }
            Pattern::Literal(text) => {
                out.push(&format!("\"{text}\""));
                Ok(())
            }
            Pattern::Ref {
                name,
                generics,
                args,
            } if self.text(*name) == "list" && generics.is_empty() && args.len() == 2 => {
                out.push(LIST_RULE);
                // The element type is the item rule's where this grammar
                // declares it; a built-in is left to the backend to infer.
                if let Pattern::Ref {
                    name: item,
                    args: none,
                    ..
                } = &args[0].node
                    && none.is_empty()
                    && let Some(ty) =
                        self.walk(|w| w.grammar_rule_types.get(self.text(*item)).cloned())
                {
                    out.push(&format!("<{ty}>"));
                }
                out.push("(item = ");
                self.pattern(out, &args[0])?;
                out.push(", sep = ");
                self.pattern(out, &args[1])?;
                out.push(")");
                Ok(())
            }
            Pattern::Ref {
                name,
                generics,
                args,
            } if generics.is_empty()
                && !args.is_empty()
                && (self.text(*name) == "bytes"
                    || self.grammar_bytes.get()
                        && matches!(self.text(*name), "until" | "text")) =>
            {
                // Over bytes `until(t)` is bytes to the backend and text to
                // Nikaia: it is checked where it becomes text (ADR-366 D2).
                // Inside `bytes(...)` or `text(...)` what is captured is the
                // outer one's, so nothing below is wrapped again.
                let called = self.text(*name);
                let wrap = called == "until";
                let called = if called == "bytes" && !self.grammar_bytes.get() {
                    "text"
                } else {
                    called
                };
                out.push(if wrap { "text(until(" } else { called });
                if !wrap {
                    out.push("(");
                }
                let bytes = self.grammar_bytes.replace(false);
                let raw = self
                    .grammar_raw
                    .replace(self.grammar_raw.get() || bytes && called == "bytes");
                let written = self.patterns(out, args, ", ");
                self.grammar_raw.set(raw);
                self.grammar_bytes.set(bytes);
                written?;
                out.push(if wrap { "))" } else { ")" });
                Ok(())
            }
            Pattern::Ref {
                name,
                generics,
                args,
            } => {
                // Over bytes the backend's `any` is a checked `char`, as in
                // Nikaia; under `bytes(...)` it is the raw `byte`.
                let called = self.text(*name);
                let raw_any = self.grammar_raw.get() && called == "any" && args.is_empty();
                out.push(if raw_any { "byte" } else { called });
                // `dec[i32](…)` -> `dec::<i32>(…)`? No: the backend's builtin
                // takes its type between angles, not after a turbofish, and a
                // grammar body is the backend's syntax once it is emitted.
                if !generics.is_empty() {
                    let params: Vec<String> = generics
                        .iter()
                        .map(|g| self.ty(g, Lifetimes::Named))
                        .collect();
                    out.push(&format!("<{}>", params.join(", ")));
                }
                if !args.is_empty() {
                    out.push("(");
                    self.patterns(out, args, ", ")?;
                    out.push(")");
                }
                Ok(())
            }
            Pattern::Repeat { pat, rep } => {
                // A sequence or a choice has to be grouped before a suffix can
                // apply to all of it rather than to its last element.
                let group = matches!(pat.node, Pattern::Seq(_) | Pattern::Choice(_));
                if group {
                    out.push("(");
                }
                self.pattern(out, pat)?;
                if group {
                    out.push(")");
                }
                out.push(&repeat_suffix(*rep));
                Ok(())
            }
            Pattern::Group(inner) => {
                out.push("(");
                self.pattern(out, inner)?;
                out.push(")");
                Ok(())
            }
            Pattern::Cut => {
                out.push("=>");
                Ok(())
            }
            Pattern::Fold(spec) => self.fold(out, spec),
        }
    }

    fn patterns(
        &self,
        out: &mut Out,
        patterns: &[Spanned<Pattern>],
        separator: &str,
    ) -> Result<()> {
        for (i, pattern) in patterns.iter().enumerate() {
            if i > 0 {
                out.push(separator);
            }
            self.pattern(out, pattern)?;
        }
        Ok(())
    }

    fn fold(&self, out: &mut Out, spec: &FoldSpec) -> Result<()> {
        let rule = self.text(spec.rule);
        let name = if spec.merge.is_some() {
            "par_fold"
        } else {
            "fold"
        };

        out.push(&format!("{name}({rule}, "));
        // **`init` makes the accumulator** (#704 item 7): the backend calls
        // it once per accumulator. A function's or a type's name, or a
        // lambda, is that maker as written; any other expression is the
        // starting value, and is handed over as a lambda that makes it.
        let maker = matches!(
            &spec.init,
            Expr::Variable(..) | Expr::Path(..) | Expr::Closure { .. }
        );
        if !maker {
            out.push("|| ");
        }
        self.expr(out, &spec.init, 0, Flow::PLAIN)?;
        out.push(", ");
        self.fold_step(out, &spec.step)?;
        if let Some(merge) = &spec.merge {
            out.push(", ");
            self.fold_merge(out, merge)?;
        }
        out.push(")");
        Ok(())
    }

    /// A fold's step, adapted where the program's own declarations say it has
    /// to be.
    ///
    /// ADR-296 D17 refused to infer this from the shape of the body, and was
    /// right to: `acc.record(m)` and `acc.merged(m)` look identical. What
    /// changed is that `impl` blocks are lowered now, so this is a lookup in
    /// what the program declares - `record` takes `&mut self` and returns
    /// nothing, so the accumulator is changed in place; a method that returns a new
    /// accumulator is left exactly as written.
    ///
    /// A step whose last statement assigns to a part of the accumulator,
    /// `acc.lines += 1`, hands back nothing either: it changes the
    /// accumulator in place too (#704 item 7).
    fn fold_step(&self, out: &mut Out, step: &Expr) -> Result<()> {
        if let Expr::Closure { params, body, .. } = step
            && let [accumulator, item] = params.as_slice()
        {
            let (on_accumulator, mutates) = match body.stmts.last().map(|s| &s.node) {
                Some(Stmt::Expr(Expr::MethodCall {
                    receiver, method, ..
                })) => (
                    matches!(&**receiver, Expr::Variable(name, _) if name == accumulator),
                    self.s
                        .by_name
                        .get(self.text(*method))
                        .is_some_and(Method::mutates_in_place),
                ),
                Some(Stmt::Assign { target, .. }) => (a_part_of(target, *accumulator), true),
                _ => (false, false),
            };

            // **Changed in place, not threaded** (`winnow-grammar` ADR 24
            // §11): a step whose accumulator is written `&mut` is run by a
            // fold that never moves it. Handed in and back by value, an
            // accumulator that holds a table was copied in and out once per
            // item - 7 instructions a row of 1BRC. `&mut _` is all the
            // generator needs to pick that fold; the type is the init's.
            if on_accumulator && mutates {
                let accumulator = self.text(*accumulator);
                out.push(&format!(
                    "|{accumulator}: &mut _, {}| {{ ",
                    self.text(*item)
                ));
                for stmt in &body.stmts {
                    self.stmt(out, &stmt.node, &stmt.span, 0, Tail::Statement, Flow::PLAIN)?;
                    out.push(" ");
                }
                out.push("}");
                return Ok(());
            }
        }

        // **Any other step that changes the accumulator in place** (ADR-372
        // D9): its result type said so, and it is lent the accumulator.
        if self.in_place_fold(step).is_some() {
            out.push("|__nk_acc: &mut _, __nk_x| { nikaia_std::par::in_place(__nk_acc, __nk_x, ");
            self.expr(out, step, 0, Flow::PLAIN)?;
            out.push("); }");
            return Ok(());
        }

        self.expr(out, step, 0, Flow::PLAIN)
    }

    /// How a fold's step or merge changes the accumulator in place, where its
    /// result type says it does (ADR-372 D9): `1` takes the second argument by
    /// value, `2` lent.
    fn in_place_fold(&self, part: &Expr) -> Option<i64> {
        self.s
            .facts
            .in_place_folds
            .get(&(crate::ast::id_of(part) as i64))
            .copied()
    }

    /// A fold's merge. `Summary::merge` names a method, and the `impl` says
    /// whether it mutates its subject and whether it takes the other by view.
    fn fold_merge(&self, out: &mut Out, merge: &Expr) -> Result<()> {
        if let Expr::Path(segments, _) = merge
            && let [owner, name] = segments.as_slice()
            && let Some(method) =
                self.s
                    .methods
                    .get(&format!("{}::{}", self.text(*owner), self.text(*name)))
            && method.mutates_in_place()
            && method.args.len() == 1
        {
            let by_view = if method.args[0].ty.is_view { "&" } else { "" };
            // The accumulator's type is annotated because the path
            // names it: without that, the backend's `let merge = …`
            // leaves the closure with nothing to infer from.
            let owner = self.ty_filled(
                &Type {
                    name: *owner,
                    generics: Vec::new(),
                    is_view: false,
                    is_tuple: false,
                    is_nullable: false,
                    code: Box::new(None),
                    count: None,
                    is_mut: false,
                    is_slice: false,
                    either: false,
                    lasting: false,
                },
                Lifetimes::Named,
            );
            out.push(&format!(
                "|mut a: {owner}, b| {{ a.{}({by_view}b); a }}",
                self.text(*name)
            ));
            return Ok(());
        }

        // **A declared function that changes its first argument in place**
        // (ADR-372 D9): lent the accumulator, which its own signature types.
        if let Some(form) = self.in_place_fold(merge)
            && !matches!(merge, Expr::Closure { .. })
        {
            let second = if form == 2 { "&__nk_b" } else { "__nk_b" };
            out.push(&format!(
                "|mut __nk_a, __nk_b| {{ nikaia_std::par::in_place(&mut __nk_a, {second}, "
            ));
            self.expr(out, merge, 0, Flow::PLAIN)?;
            out.push("); __nk_a }");
            return Ok(());
        }

        self.expr(out, merge, 0, Flow::PLAIN)
    }

    // --- Types ---

    /// The shadow type of every deferred-parameter DSL in this unit
    /// (ADR-296 D5).
    ///
    /// One struct per parameter list, a field per `:name`, and **generic in
    /// every field**: nothing in `… WHERE id = :id …` says what `:id` is, so
    /// the type comes from the argument at the call site and is monomorphised
    /// there. Inventing `i32` here is exactly the guess ADR-296 D17 forbids an
    /// emitter to make.
    ///
    /// A plain struct, passed by value: D5 asks for the parameters on the
    /// stack, and a struct of the caller's own values is what that is.
    fn shadow_types(&self, out: &mut Out) {
        for (name, parameters) in crate::dsl::shadow_types(self.parsed) {
            let generics: Vec<String> = (0..parameters.len()).map(|i| format!("P{i}")).collect();
            let fields: Vec<String> = parameters
                .iter()
                .zip(&generics)
                .map(|(field, ty)| format!("    pub {field}: {ty},"))
                .collect();
            out.push(&format!(
                "// ADR-296 D5: the parameters of a `dsl … {{ … }} eod` statement, \
                 as a type.\n\
                 #[derive(Debug, Clone, Copy, PartialEq)]\n\
                 #[allow(non_camel_case_types, dead_code)]\n\
                 pub struct {name}<{}> {{\n{}\n}}\n\n",
                generics.join(", "),
                fields.join("\n"),
            ));
        }
    }

    /// A `dsl … { … } eod` body: the `html` template compiled here (ADR-017),
    /// or a statement whose holes the call site fills (ADR-296 D5).
    ///
    /// Which it is follows from the target and the holes, and not from anything
    /// inferred about the body. `html` is the one grammar this compiler *is*,
    /// so `:name` there is an immediate capture (ADR-296 D4); anywhere else a
    /// `:name` is a deferred parameter and the body reaches its driver intact.
    /// A body with neither is refused, because nothing here knows what it
    /// means.
    #[allow(clippy::too_many_arguments)]
    fn template(
        &self,
        out: &mut Out,
        name: &str,
        context: Option<&Symbol>,
        content: &str,
        site: u32,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_template(
                &self.world,
                walk,
                out,
                self,
                name,
                context.copied(),
                content,
                site as i64,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// A path, with the map it names chosen the same way its type is.
    ///
    /// `HashMap::new` has no counterpart on a map with a hasher of its own -
    /// `new` exists only for the default one - so the trusted map is built with
    /// `default`, which is what every `HashMap<_, _, S>` is built with.
    fn path(&self, segments: &[&str]) -> String {
        let segments: Vec<String> = segments.iter().map(|s| s.to_string()).collect();
        self.s.path(
            &self.world,
            self.walk(|w| w.trusting),
            self.walk(|w| w.unpinned),
            &segments,
        )
    }

    /// The count the analysis gave the `Shared` written at one position, with the
    /// atomic floor where it has no answer (ADR-037 D9).
    fn count_at(&self, function: &str, value: &str) -> crate::contracts::sharing::Count {
        self.s
            .count_at(self.build.crossings_are_possible(), function, value)
    }

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
                lifetimes,
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

    /// The same, where the count a `Shared` in this position was given is known.
    ///
    /// The count rides down through the arguments, because `Vec[Shared[i64]]` in
    /// a parameter is the parameter's class and not a class of its own - a count
    /// belongs to the allocation, and the analysis keys it by the position the
    /// source wrote.
    fn ty_counted(
        &self,
        ty: &Type,
        lifetimes: Lifetimes,
        count: crate::contracts::sharing::Count,
    ) -> String {
        let written = self.nikaia(|walk| {
            self.s.emit_core_ty_counted(
                &self.world,
                walk,
                self.build.crossings_are_possible(),
                self.build.overlaps_user_code(),
                ty,
                lifetimes,
                count,
            )
        });
        self.seeded(written)
    }

    /// **A written type, with the hidden map parameters in** (ADR-010 D9).
    fn seeded(&self, written: String) -> String {
        // **A barrier's signature is the keyed map** (D9): std's `HashMap`,
        // whatever the package, and the keyed hasher in a library type's
        // arguments.
        let floor = self.floor.get();
        let written = match floor {
            true => written
                .replace("TrustedMap", "HashMap")
                .replace("TrustedSet", "HashSet"),
            false => written,
        };
        if self.seeds.is_empty() {
            return written;
        }
        let mut place = self.place.borrow_mut();
        self.seeds
            .write(&written, &mut place, self.s.trusted_input && !floor)
    }

    /// The queues a settle point drains: at `user_parallelism = yes` only the
    /// one whose work may cross threads, so the settle point may too
    /// (ADR-297 D2).
    fn settle_kind(&self) -> String {
        nikaia_std::tools::emit_core_types::emit_settle_kind(self.build.overlaps_user_code())
    }

    // --- Statements and expressions ---

    /// `if c { … } else { … }`, where `tail` says what the whole thing is in
    /// the position of ([`Tail`]).
    ///
    /// It decides what the last statement of each branch means, and the branches
    /// are in the same position the `if` is: as the tail of a function body they
    /// carry the function's value, as `let x = if c { a } else { b }` they carry
    /// the `let`'s, and as a statement they carry nothing.
    #[allow(clippy::too_many_arguments)]
    fn if_expr(
        &self,
        out: &mut Out,
        cond: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
        depth: usize,
        flow: Flow<'_>,
        tail: Tail,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_if_expr(
                &self.world,
                walk,
                out,
                self,
                cond,
                then_branch,
                else_branch,
                depth as i64,
                flow,
                &scope,
                tail,
            )
        });
        self.crossed(crossed)
    }

    /// A cast's operand, through the view a `for` binding is
    /// ([ADR-182](../../docs/specification/adr/adr-182.md) D1).
    ///
    /// **`num::value` and not a `*`**, which is `nikaia_std::index::at`'s own
    /// reasoning one construct over: for a number that is already a number it
    /// is the identity, so it cannot be written onto the wrong operand — and a
    /// `*` written onto one would be a `rustc` error about the generated file,
    /// which is the very thing this closes.
    ///
    /// **Inside the checked conversion and not around it**, because a
    /// narrowing cast over a lent binding is still a narrowing cast: `i8::from`
    /// of a `&i64` does not exist either, and the abort ADR-285 D12 puts there
    /// is not a thing a view may skip.
    fn operand(
        &self,
        out: &mut Out,
        expr: &Expr,
        viewed: bool,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_operand(
                &self.world,
                walk,
                out,
                self,
                expr,
                viewed,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// **A turn of a loop is a step of a build-time run** (ADR-321 D7): the
    /// count, in front of whatever else a loop's body opens with.
    fn a_counted_turn(&self, opening: Option<String>) -> Option<String> {
        nikaia_std::tools::emit_core_types::emit_counted_turn(self.build.counts_steps, opening)
    }

    fn block(
        &self,
        out: &mut Out,
        block: &Block,
        depth: usize,
        flow: Flow<'_>,
        tail: Tail,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_block(
                &self.world,
                walk,
                out,
                self,
                block,
                depth as i64,
                flow,
                &scope,
                tail,
            )
        });
        self.crossed(crossed)
    }

    /// The same, with one line written before the statements.
    ///
    /// One caller: a `for` whose step can fail opens its body by unwrapping the
    /// step (ADR-025 D7). It is a parameter rather than a wrapping block
    /// because an extra brace level in the emitted Rust is a level the source
    /// map has to explain and the reader has to skip.
    fn block_opening_with(
        &self,
        out: &mut Out,
        block: &Block,
        depth: usize,
        flow: Flow<'_>,
        tail: Tail,
        opening: Option<&str>,
    ) -> Result<()> {
        // **A block where a value with a cleanup dies is settled where it
        // ends** ([ADR-297](../../docs/specification/adr/adr-297.md) D2): the
        // value dies at its closing brace and parks its cleanup, and the
        // settle point right after runs it. Not inside a closure that cannot
        // pause - there the function's own settle point runs it.
        let settles = block
            .stmts
            .iter()
            .filter_map(|stmt| {
                self.s
                    .facts
                    .settle_lets
                    .get(&(stmt.span.at() as i64))
                    .copied()
            })
            .reduce(|a, b| a || b);
        // **A failure is never settled away**: where a cleanup that can fail
        // has no channel to fail into here - a lambda's body, a branch that is
        // not wrapped, a guarded half - the value stays parked for the next
        // settle point out, which has one (ADR-297 D4).
        let propagates = flow.throws && !flow.caught;
        if let Some(fails) = settles
            && !flow.in_lambda
            && (!fails || propagates)
        {
            let inner = {
                let mut inner_buf = Out::empty();
                let inner = &mut inner_buf;
                self.block_plain(inner, block, depth, flow, tail, opening)?;
                inner_buf
            };
            let kind = self.settle_kind();
            let settle = match fails {
                true => format!(
                    "nikaia_std::cleanup::settle_at::<{kind}>(&{:?}).await?",
                    flow.origin
                ),
                false => format!("nikaia_std::cleanup::settle_quietly::<{kind}>().await"),
            };
            out.push("{ let __nikaia_block = ");
            out.append(inner);
            out.push(&format!("; {settle}; __nikaia_block }}"));
            return Ok(());
        }
        self.block_plain(out, block, depth, flow, tail, opening)
    }

    /// A block as it is written, with no settle point after it.
    fn block_plain(
        &self,
        out: &mut Out,
        block: &Block,
        depth: usize,
        flow: Flow<'_>,
        tail: Tail,
        opening: Option<&str>,
    ) -> Result<()> {
        if block.stmts.is_empty() {
            match opening {
                Some(line) => {
                    out.push(&format!("{{ {line} }}"));
                    return Ok(());
                }
                None => {
                    out.push("{ }");
                    return Ok(());
                }
            }
        }

        // A one-statement block stays on its line. Most action blocks are one
        // expression, and a grammar reads better when its actions do not push
        // the pattern three lines apart. Whether it fits is decided by
        // rendering it, not by guessing from the shape.
        if block.stmts.len() == 1 && opening.is_none() {
            let only = block.stmts.first().expect("one statement");
            // **A try that is thrown away leaves nothing behind**: the keeps
            // it declared (ADR-283 D10) are declared again by the rendering
            // that is kept - `println(d())` alone in `main` lost its keep.
            let declared = self.walk(|w| w.preluded.clone());
            let rendered = {
                let mut rendered = Out::empty();
                let st = rendered.mark();
                self.stmt(&mut rendered, &only.node, &only.span, depth, tail, flow)?;
                rendered.record(&only.span, st);
                rendered
            };
            if !rendered.buf.contains('\n') {
                out.push("{ ");
                out.append(rendered);
                out.push(" }");
                return Ok(());
            }
            self.walking(|w| w.preluded = declared);
        }

        let pad = "    ".repeat(depth);
        let inner_pad = "    ".repeat(depth + 1);

        out.push("{\n");
        if let Some(line) = opening {
            out.push(&inner_pad);
            out.push(line);
            out.push("\n");
        }
        let last = block.stmts.len() - 1;
        let mut i = 0;
        while i < block.stmts.len() {
            // **Statements run in the order they are written**
            // ([ADR-292](../../../docs/specification/adr/adr-292.md) D1), and no
            // analysis stands between the source and the schedule. A program
            // that wants overlap writes `overlap { … }` and has the claim
            // checked (D3).

            let stmt = &block.stmts[i];
            // **An empty list made `n` long at once is `vec![v; n]`**
            // ([ADR-270](../../docs/specification/adr/adr-270.md) D8 step 1):
            // `let mut xs: Vec[T] = []` followed by `xs.resize(n, v)` is one
            // allocation of a known length - zeroed by the system for zeros -
            // which is what Rust writes, and what lets the optimiser drop the
            // bounds checks of every read below that it can see is in range.
            if i < last
                && let Some((fill, length)) = self.filled_at_once(stmt, &block.stmts[i + 1])
            {
                let declared = {
                    let mut declared = Out::empty();
                    let st = declared.mark();
                    self.stmt(
                        &mut declared,
                        &stmt.node,
                        &stmt.span,
                        depth + 1,
                        tail.at(i as i64, last as i64),
                        flow,
                    )?;
                    declared.record(&stmt.span, st);
                    declared
                };
                if let Some(head) = declared.buf.strip_suffix("vec![];") {
                    let next = &block.stmts[i + 1];
                    let flow = flow.at(next.span.at());
                    let value = {
                        let mut v_buf = Out::empty();
                        let v = &mut v_buf;
                        self.expr(v, fill, depth + 1, flow)?;
                        v_buf
                    };
                    let count = {
                        let mut c_buf = Out::empty();
                        let c = &mut c_buf;
                        let done = match only_literals(length) {
                            true => self.expr(c, length, depth + 1, flow.inferred()),
                            false => {
                                c.push("nikaia_std::count::of(");
                                self.expr(c, length, depth + 1, flow)?;
                                c.push(")");
                                Ok(())
                            }
                        };
                        done?;
                        c_buf
                    };
                    out.push(&inner_pad);
                    {
                        let st = out.mark();
                        out.push(&format!("{head}vec![{}; {}];", value.buf, count.buf));
                        out.record(&stmt.span, st);
                    }
                    out.push("\n");
                    i += 2;
                    continue;
                }
            }
            out.push(&inner_pad);
            {
                let st = out.mark();

                self.stmt(
                    out,
                    &stmt.node,
                    &stmt.span,
                    depth + 1,
                    tail.at(i as i64, last as i64),
                    flow,
                )?;
                out.record(&stmt.span, st);
            }
            out.push("\n");
            i += 1;
        }
        out.push(&pad);
        out.push("}");
        Ok(())
    }

    /// Whether `expr` is never negative here: a non-negative literal, a
    /// counter of [`Emitter::nonnegative`], or a sum or product of those.
    fn never_negative(&self, flow: Flow<'_>, expr: &Expr) -> bool {
        self.walk(|w| w.never_negative(&self.world, flow.function, expr))
    }

    /// `let mut xs = []` and, right after it, `xs.resize(n, v)` with `std`'s
    /// `resize`: the value and the length, for `vec![v; n]`.
    fn filled_at_once<'s>(
        &self,
        declared: &'s Spanned<Stmt>,
        next: &'s Spanned<Stmt>,
    ) -> Option<(&'s Expr, &'s Expr)> {
        use nikaia_std::tools::emit_core_below as below;
        if !below::emit_filled_at_once(
            &self.world,
            &self.s.own_contracts,
            &self.s.own_methods,
            &declared.node,
            &next.node,
        ) {
            return None;
        }
        match below::emit_resize_args(&next.node).as_slice() {
            [length, fill] => Some((fill, length)),
            _ => None,
        }
    }

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
    fn branch_pauses(&self, stmt: &Spanned<Stmt>, _flow: Flow<'_>) -> bool {
        self.s
            .branch_pauses(&self.world, &stmt.node, stmt.span.at() as i64)
    }

    /// `tail` says what this statement is in the position of ([`Tail`]):
    /// Nikaia's blocks are expressions (Part I, 3.1), so a block's last
    /// statement is its value and keeps no semicolon - and where that value is
    /// the *function's*, a `return x` is written as `x`.
    /// **A `?.` view out of a temporary is held for the rest of the block**
    /// ([ADR-278](../../docs/specification/adr/adr-278.md) D21).
    ///
    /// `let a = find(1)?.label() ?? "none"` hands back a view of the `User`
    /// `find(1)` made, and that value dies at the `;` - `rustc`'s *temporary
    /// value dropped while borrowed* about a file nobody wrote. So the receiver
    /// is bound first, `let __nikaia_held_0 = find(1);`, and the reach reads
    /// the binding. Nothing is copied and nothing runs that did not run.
    ///
    /// **Only where binding it first changes nothing about what runs** (D2):
    /// not on the lazy side of `??`, `&&` or `||`, not in a `match` arm, not
    /// after a call earlier in the same statement, and not where the view is
    /// the block's value, which would outlive the binding. There the program is
    /// refused, with the `let` it can write.
    fn hold_temporaries(
        &self,
        out: &mut Out,
        stmt: &Stmt,
        span: &Span,
        depth: usize,
        tail: Tail,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_hold_temporaries(
                &self.world,
                walk,
                out,
                self,
                stmt,
                span,
                depth as i64,
                tail,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// A statement; a `let` of a task whose body cannot fail is remembered,
    /// since its `join` throws `task::Crashed` alone (ADR-328 D8).
    fn stmt(
        &self,
        out: &mut Out,
        stmt: &Stmt,
        span: &Span,
        depth: usize,
        tail: Tail,
        flow: Flow<'_>,
    ) -> Result<()> {
        let mark = out.buf.len();
        self.stmt_written(out, stmt, span, depth, tail, flow)?;
        if let Stmt::Let {
            names,
            value: Expr::Spawn { .. },
            ..
        } = stmt
            && let [name] = names.as_slice()
        {
            let written = &out.buf[mark..];
            let first = |what: &str| written.find(what).unwrap_or(usize::MAX);
            let plain = first("task::TaskHandle::start")
                < first("task::FailingTask::").min(first("task::BoxedTask::"));
            let name = self.text(*name).to_string();
            self.walking(|w| {
                if plain {
                    w.plain_tasks.insert(name);
                } else {
                    w.plain_tasks.remove(&name);
                }
            });
        }
        Ok(())
    }

    /// A statement, and the map a `let` of [`Emitter::trusted_lets`] makes
    /// under its fast hash (#431).
    fn stmt_written(
        &self,
        out: &mut Out,
        stmt: &Stmt,
        span: &Span,
        depth: usize,
        tail: Tail,
        flow: Flow<'_>,
    ) -> Result<()> {
        let trusted =
            matches!(stmt, Stmt::Let { .. }) && self.s.trusted_lets.contains(&(span.at() as i64));
        let outer = self.walking(|w| std::mem::replace(&mut w.trusting, trusted));
        // **A `let` that makes a map is a position of its own**: its hasher is
        // the package's, as it was before a body could hold parameters.
        let pinned = match stmt {
            Stmt::Let { value, .. } => {
                self.makes_a_map(value) && !self.s.escaped_lets.contains(&(span.at() as i64))
            }
            _ => false,
        };
        let loose = self.walk(|w| w.unpinned);
        if pinned {
            self.walking(|w| w.unpinned = false);
        }
        let written = self.a_stmt(out, stmt, span, depth, tail, flow);
        self.walking(|w| w.trusting = outer);
        self.walking(|w| w.unpinned = loose);
        written
    }

    /// Whether an expression is `HashMap()` or `HashSet()`, made and nothing else.
    fn makes_a_map(&self, expr: &Expr) -> bool {
        let Expr::Call { func, args, .. } = expr else {
            return false;
        };
        if !args.is_empty() {
            return false;
        }
        let name = match &**func {
            Expr::Variable(name, _) => self.text(*name).to_string(),
            Expr::Path(segments, _) => segments
                .iter()
                .map(|s| self.text(*s))
                .collect::<Vec<_>>()
                .join("::"),
            _ => return false,
        };
        matches!(
            name.rsplit("::").next().unwrap_or(&name),
            "HashMap" | "HashSet"
        )
    }

    fn a_stmt(
        &self,
        out: &mut Out,
        stmt: &Stmt,
        span: &Span,
        depth: usize,
        tail: Tail,
        flow: Flow<'_>,
    ) -> Result<()> {
        // Which statement this is, for the one thing that has to look itself up
        // by it: a method call that can fail (ADR-023 D8). Shadowed rather than
        // passed on separately, so that everything emitted from here - an
        // expression, a nested block, a `catch` handler - sees the statement it
        // is actually in.
        let flow = flow.at(span.at());
        self.write_keep_prelude(out, flow.function, span.at(), depth);
        self.hold_temporaries(out, stmt, span, depth, tail, flow)?;
        match stmt {
            Stmt::Let {
                names,
                shape,
                mutable,
                ty,
                value,
            } => {
                let unmuted = self.give_ups.unmuted.contains(&(span.at() as i64));
                let mutable = if *mutable && !unmuted { "mut " } else { "" };
                // **A tuple of names is Rust's own tuple pattern**
                // ([ADR-291](../../docs/specification/adr/adr-291.md)): the
                // language below takes a tuple apart by position exactly as
                // this one does, so there is nothing to translate. No count is
                // looked up and no annotation is written, because a hull's
                // count is decided per **value** and a destructure names none -
                // which is the same answer `contracts::sharing` gives, and the
                // two have to agree.
                if names.len() > 1 || !shape.is_empty() {
                    let bound: Vec<String> =
                        names.iter().map(|n| self.local(self.text(*n))).collect();
                    let pattern = tuple_pattern(&bound, shape);
                    out.push(&format!("let {mutable}{pattern} = "));
                    let (before, after) = Self::around_value(
                        self.s.facts.nullable_sites.get(&(span.at() as i64)),
                        value,
                    );
                    out.push(&before);
                    // **Out of a place, the tuple is lent** (#465), as one
                    // name's `let` over a place is.
                    if mutable.is_empty() && self.s.facts.lent_lets.contains(&(span.at() as i64)) {
                        out.push("&");
                    }
                    self.expr(out, value, depth, flow)?;
                    out.push(&after);
                    out.push(";");
                    // A number among its parts is read out of its view.
                    for name in names {
                        if self
                            .s
                            .facts
                            .copied_tuple_parts
                            .contains(&((span.at() as i64), self.text(*name).to_string()))
                        {
                            let name = self.local(self.text(*name));
                            out.push(&format!(" let {name} = nikaia_std::num::value({name});"));
                        }
                    }
                    return Ok(());
                }
                let bound = self.text(names[0]);
                let count = self.count_at(flow.function, bound);
                // A hull written by a call inside this value looks its count up
                // by the name being bound (ADR-281 D2), and an expression has
                // none of its own.
                let flow = flow.binding(bound);
                let plan = self.keep_plan(flow.function);
                // **A buffer whose views outlive this scope goes into a keep**
                // (ADR-283 D9), and the binding is where it now lives.
                let put = plan.and_then(|p| p.puts.get(&(span.at() as i64))).copied();
                // **A binding a task takes with it is packed with the task's
                // keep** (D3), and every use of it reads through the handle.
                let packed = plan
                    .and_then(|p| p.tethered.get(bound))
                    .filter(|at| **at == span.at() as i64)
                    .is_some();
                // **A keeper that drops entries holds each text view with its
                // own handle** (D4): `ref String` is `Held` in its type.
                let holding = plan
                    .filter(|p| p.element_keepers.contains(bound))
                    .map(|p| p.widths.get(bound).copied().unwrap_or(1));
                self.walking(|w| w.holding = holding);
                let annotation = match ty {
                    Some(_) if put.is_some() || packed => String::new(),
                    Some(ty) => format!(": {}", self.ty_counted(ty, Lifetimes::Elided, count)),
                    // **A number its uses typed is written with the type**
                    // ([ADR-285](../../docs/specification/adr/adr-285.md) D27):
                    // this compiler decided it, and the language below's own
                    // inference is not asked a second time - which is what left
                    // `v[i].push(x)` over a bare `let i = 0` without an answer.
                    None => match self.s.facts.number_lets.get(&(span.at() as i64)) {
                        Some(ty) => format!(": {ty}"),
                        None => String::new(),
                    },
                };
                self.walking(|w| w.holding = None);
                if packed {
                    let Some(held) = self.tethered_type(ty.as_ref(), value) else {
                        return Err(refused_at!(
                            span.at(),
                            "`{bound}` is passed to a task and keeps data alive, so it needs \
                             its type written out: `let {bound}: … = …`."
                        ));
                    };
                    // **Held by the task's keep** (ADR-218 D4): the value is
                    // built first, and every view in it is found again in the
                    // keep - which is where the buffers it was cut from were
                    // put - over a lifetime the `Holding` alone decides.
                    let wrapper = self.tether_wrapper(&held);
                    out.push(&format!(
                        "let {mutable}{} = {{ let __packed = ",
                        self.local(bound)
                    ));
                    self.expr(out, value, depth, flow)?;
                    out.push(&format!(
                        "; nikaia_std::tether::Holding::<{wrapper}, 1>::new([std::sync::Arc::clone(&{KEEP_TASK})], \
                         move |keeps| nikaia_std::tether::Rebase::rebase(__packed, &keeps)) }};"
                    ));
                    return Ok(());
                }
                if let Some(keep) = put {
                    // A buffer whose views are held one by one is one they
                    // are found in again (ADR-283 D14) - and so is one a task's
                    // `Holding` finds them in, whether the task's keep is this
                    // function's or a caller's handed in (ADR-218 D4).
                    let put = match keep {
                        nikaia_std::tools::buffers::KeepAt::Element(_)
                        | nikaia_std::tools::buffers::KeepAt::Task
                        | nikaia_std::tools::buffers::KeepAt::Param => "put_viewed",
                        _ => "put",
                    };
                    out.push(&format!(
                        "let {mutable}{} = {}.{put}(",
                        self.local(bound),
                        Self::keep_expr(keep, false)
                    ));
                    self.expr(out, value, depth, flow)?;
                    out.push(");");
                    return Ok(());
                }
                // **The annotation is no longer the constructor**
                // ([ADR-281](../../../docs/specification/adr/adr-281.md) D2). What
                // used to be allocated here, out of an answer the checker had to
                // compute and hand over, is now written where it happens - and the
                // two positions that could carry one stopped being a list.
                //
                // Part I 2.3: a plain value standing in a nullable slot. That hull
                // stays the compiler's, because it is one a program cannot observe
                // - the same value, possibly absent (ADR-281 D2's own line).
                let (before, after) =
                    Self::around_value(self.s.facts.nullable_sites.get(&(span.at() as i64)), value);
                out.push(&format!(
                    "let {mutable}{}{annotation} = ",
                    self.local(bound)
                ));
                out.push(&before);
                // **A `let` over a place is a view of it**
                // ([ADR-094](../../docs/specification/adr/adr-094.md) D4).
                // `let s = totals.stations[name]` and `let name = config.name`
                // were never moves: the language below refuses to move a value
                // out of a container or out of a borrowed field, so what the
                // line meant is the `&` it would otherwise have been refused
                // for not writing.
                //
                // **`let y = x` over a whole variable stays a move** — it is a
                // rename — and a `let` whose initialiser is a call or a literal
                // owns what it is given. So this is narrower than `for`'s rule
                // by exactly one shape, and that shape is the reason both are
                // written here rather than in one test.
                if self.s.facts.lent_lets.contains(&(span.at() as i64)) {
                    out.push("&");
                }
                // **A `let` whose annotation is a number and whose value is a
                // `for` binding reads the number through the view**
                // ([ADR-182](../../docs/specification/adr/adr-182.md) D5):
                // `let q: i64 = m` over a lent `m` is a
                // `&i64` where an `i64` was declared, and *consider
                // dereferencing the borrow* is a way out the source cannot
                // take, because there is no borrow in it.
                let viewed = match value {
                    Expr::Variable(name, _) => self
                        .s
                        .facts
                        .viewed_numbers
                        .contains(&((flow.statement as i64), self.text(*name).to_string())),
                    _ => false,
                };
                self.operand(out, value, viewed, depth, flow)?;
                out.push(&after);
                out.push(";");
            }
            // **The folded value is what is written**, not the expression that
            // folded ([ADR-287](../../docs/specification/adr/adr-287.md) D4).
            // That is the visible half of the demand: `const LIMIT = 4 * 1024`
            // reaches the language below as `4096`, so a reader of the generated
            // file can see that the arithmetic did not survive into the program.
            //
            // The type comes from the checker, because Rust's `const` takes one
            // and this has no types (ADR-288). A `comptime` with no entry there was
            // refused as `NK1127` and never arrives.
            Stmt::Comptime { name, .. } => {
                let bound = self.text(*name);
                let Some((below, written)) = self
                    .s
                    .facts
                    .comptime_values
                    .get(&(span.at() as i64))
                    .cloned()
                else {
                    return Err(refused_at!(
                        span.at(),
                        "`{bound}` has no value computed at build time. This is a compiler \
                         bug: the check should have reported it first."
                    ));
                };
                out.push(self.folded(*name));
                out.push(&format!("const {bound}: {below} = {written};"));
            }

            // **A write through the brackets is not an index**
            // ([ADR-080](../../docs/specification/adr/adr-080.md) D2). Rust's
            // `Index` for a map is over whatever the key *borrows* as, so
            // `scores["Player1"] = 100` left the key type unpinned and the
            // program failed with `E0282` in a message naming `TrustedMap` and
            // a type parameter `K` - three spellings the program never wrote.
            // `index::set` is `insert` for a map and an indexed assignment for a
            // sequence, chosen by the language below on the container's type,
            // because this emitter does not know it (ADR-296 D17).
            //
            // Only the plain `=`: a compound `m[k] += 1` reads the slot as well
            // as writing it, so it is an `Index` either way and a second rule
            // would be needed to say what reading an absent key means. That is
            // a question of its own and not this one.
            Stmt::Assign {
                target: box_index @ Expr::Index { .. },
                op: None,
                value,
            } => {
                let Expr::Index { base, index, .. } = box_index else {
                    unreachable!("matched as an index")
                };
                let (before, after) =
                    Self::around_value(self.s.facts.nullable_sites.get(&(span.at() as i64)), value);
                // **The value first, and then the write**
                // ([ADR-293](../../../docs/specification/adr/adr-293.md) D2).
                // Since D1 a read is `index::get(&m, …)`, so the written-out
                // counter D2 hands a reader — `m[k] = (m[k] ?? 0) + 1` — has a
                // `&m` inside the arguments of a `set(&mut m, …)`, which is
                // *cannot borrow as immutable because it is also borrowed as
                // mutable* about a file nobody wrote
                // ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
                // A `let` ends the read before the write begins, which is what
                // a reader would write by hand.
                out.push(&format!("{{ let {STORED} = "));
                out.push(&before);
                self.expr(out, value, depth, flow)?;
                out.push(&after);
                // **A `mut` parameter is already a `&mut`** (ADR-094 D3), so
                // it is lent on again rather than borrowed a second time:
                // `set(&mut out, …)` asks for the binding itself to be `mut`,
                // which a parameter is not ([ADR-270](../../docs/specification/adr/adr-270.md)
                // D8 step 1).
                let lent_on = matches!(&**base, Expr::Variable(name, _)
                    if self.changes_in_place(flow, self.text(*name)));
                // **A position proved inside is written without its check**
                // (ADR-306 D5), as it is read.
                if self
                    .s
                    .proven_indices
                    .contains(&((crate::check::value_node(base) as usize) as i64))
                    && self.map_key(index, true).is_none()
                {
                    out.push(match lent_on {
                        true => "; unsafe { nikaia_std::proven::write(&mut *",
                        false => "; unsafe { nikaia_std::proven::write(&mut ",
                    });
                    self.expr(out, base, depth, flow.place())?;
                    out.push(", (");
                    match only_literals(index) {
                        true => self.index_expr(out, index, depth, flow.inferred())?,
                        false => self.index_expr(out, index, depth, flow)?,
                    }
                    out.push(&format!(") as usize, {STORED}) }}; }}"));
                    return Ok(());
                }
                out.push(match lent_on {
                    true => "; nikaia_std::index::set(&mut *",
                    false => "; nikaia_std::index::set(&mut ",
                });
                // **The container written into is a place** (0.0.238):
                // `d[i][j] = v` writes into the list `d[i]`, and read as
                // `*index::get(&d, …)` that list is behind a shared reference -
                // `rustc`'s *cannot borrow data in a `&` reference as mutable*
                // about a file nobody wrote. As a place it is `d[at(i)]`, which
                // the language below lends mutably.
                self.expr(out, base, depth, flow.place())?;
                // **A key the map keeps goes in as it is** (ADR-293 D27): it is
                // not a position, and `at` is for positions.
                match self.map_key(index, true) {
                    Some(form) => {
                        out.push(", ");
                        self.key(out, form, index, depth, flow)?;
                        out.push(&format!(", {STORED}); }}"));
                    }
                    // A literal position is written as the `usize` it is.
                    None if let Expr::LitInt {
                        value,
                        negative: false,
                        ..
                    } = &**index =>
                    {
                        let at = crate::ast::int_value(*value, false);
                        out.push(&format!(", {at}usize, {STORED}); }}"));
                    }
                    None if self.never_negative(flow, index) => {
                        out.push(", (");
                        self.index_expr(out, index, depth, flow)?;
                        out.push(&format!(") as usize, {STORED}); }}"));
                    }
                    None => {
                        out.push(", nikaia_std::index::at(");
                        self.index_expr(out, index, depth, flow)?;
                        out.push(&format!("), {STORED}); }}"));
                    }
                }
            }
            // **A field of a held struct is written through its handle**
            // ([ADR-283](../../docs/specification/adr/adr-283.md) D17): the
            // value first, as everywhere, and then a closure that is handed the
            // struct and its keeps - where a view made outside is found again
            // in them, or copied into them, before it is stored.
            Stmt::Assign {
                target:
                    Expr::Field {
                        base: element,
                        name: field,
                        ..
                    },
                op,
                value,
            } if self.held_element(flow, element).is_some() => {
                let (keeper, index) = self
                    .held_element(flow, element)
                    .expect("matched as a held element");
                let (before, after) =
                    Self::around_value(self.s.facts.nullable_sites.get(&(span.at() as i64)), value);
                out.push(&format!("{{ let {STORED} = "));
                out.push(&before);
                self.expr(out, value, depth, flow)?;
                out.push(&after);
                out.push("; ");
                self.expr(out, keeper, depth, flow.place())?;
                match only_literals(index) {
                    true => {
                        out.push("[");
                        self.index_expr(out, index, depth, flow.inferred())?;
                        out.push("]");
                    }
                    false => {
                        out.push("[nikaia_std::index::at(");
                        self.index_expr(out, index, depth, flow)?;
                        out.push(")]");
                    }
                }
                let (keeps, stored) = match self.held_field_carries_view(flow, keeper, *field) {
                    true => (
                        "keeps",
                        format!("nikaia_std::tether::Rebase::rebase({STORED}, &keeps)"),
                    ),
                    false => ("_", STORED.to_string()),
                };
                let assign = match op {
                    Some(op) => format!("{}=", binary_op(*op)),
                    None => "=".to_string(),
                };
                out.push(&format!(
                    ".with_mut(|held, {keeps}| held.{} {assign} {stored}); }}",
                    self.name(*field)
                ));
            }
            // **`xs[i] += d` at a position proved inside** (#387): the slot
            // is a number, so it is read once and written back, neither
            // without its check - the value first, then the position, as the
            // plain write runs them (ADR-293 D2). Where the operation is
            // proved too it is the operation alone.
            Stmt::Assign {
                target: Expr::Index { base, index, .. },
                op: Some(op),
                value,
            } if self.s.facts.copied_slots.contains(&(span.at() as i64))
                && self
                    .s
                    .proven_indices
                    .contains(&((crate::check::value_node(base) as usize) as i64))
                && self.map_key(index, true).is_none() =>
            {
                let lent_on = matches!(&**base, Expr::Variable(name, _)
                    if self.changes_in_place(flow, self.text(*name)));
                out.push(&format!("{{ let {STORED} = "));
                self.expr(out, value, depth, flow)?;
                out.push("; let __nikaia_at = (");
                match only_literals(index) {
                    true => self.index_expr(out, index, depth, flow.inferred())?,
                    false => self.index_expr(out, index, depth, flow)?,
                }
                out.push(") as usize; ");
                let read = |emitter: &Self, out: &mut Out| -> Result<()> {
                    out.push("*unsafe { nikaia_std::proven::read(&");
                    emitter.expr(out, base, depth, flow.place())?;
                    out.push(", __nikaia_at) }");
                    Ok(())
                };
                out.push(&format!("let {STORED} = "));
                match (self.s.proven_arithmetic.get(&(span.at() as i64)), op) {
                    (Some(ty), BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul) => {
                        let name = match op {
                            BinaryOp::Add => "wrapping_add",
                            BinaryOp::Sub => "wrapping_sub",
                            _ => "wrapping_mul",
                        };
                        out.push(&format!("<{ty}>::{name}("));
                        read(self, out)?;
                        out.push(&format!(", {STORED})"));
                    }
                    _ => {
                        read(self, out)?;
                        out.push(&format!(" {} {STORED}", binary_op(*op)));
                    }
                }
                out.push(match lent_on {
                    true => "; unsafe { nikaia_std::proven::write(&mut *",
                    false => "; unsafe { nikaia_std::proven::write(&mut ",
                });
                self.expr(out, base, depth, flow.place())?;
                out.push(&format!(", __nikaia_at, {STORED}) }}; }}"));
            }
            // **`x += d` proved to stay inside its type** (#387) is written as
            // the operation it is, without its check, as `x = x + d` would be
            // (ADR-306 D10); so is `x.f += d`, a fold's counter (ADR-367 D3).
            Stmt::Assign {
                target: target @ (Expr::Variable(_, _) | Expr::Field { .. }),
                op: Some(op @ (BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul)),
                value,
            } if self.s.proven_arithmetic.contains_key(&(span.at() as i64)) => {
                let ty = &self.s.proven_arithmetic[&(span.at() as i64)];
                let name = match op {
                    BinaryOp::Add => "wrapping_add",
                    BinaryOp::Sub => "wrapping_sub",
                    _ => "wrapping_mul",
                };
                self.expr(out, target, depth, flow.place())?;
                out.push(&format!(" = <{ty}>::{name}("));
                self.expr(out, target, depth, flow)?;
                out.push(", ");
                self.expr(out, value, depth, flow)?;
                out.push(");");
            }
            Stmt::Assign { target, op, value } => {
                let (before, after) =
                    Self::around_value(self.s.facts.nullable_sites.get(&(span.at() as i64)), value);
                // **A `mut` parameter assigned whole is written through**
                // (ADR-094 D3, #576): below it is a `&mut T`, and `n = n + 1`
                // is the caller's value changed, `*n = *n + 1`.
                if matches!(target, Expr::Variable(name, _)
                    if self.changes_in_place(flow, self.text(*name)))
                {
                    out.push("*");
                }
                // **Given up while a view of it is used** (ADR-353 D3): the
                // new value is a binding of its own, and the old one lives
                // on, unchanged, to the end of the block.
                match self.give_ups.rebound.get(&(span.at() as i64)) {
                    Some(true) => out.push("let mut "),
                    Some(false) => out.push("let "),
                    None => {}
                }
                self.expr(out, target, depth, flow.place())?;
                match op {
                    Some(op) => out.push(&format!(" {}= ", binary_op(*op))),
                    None => out.push(" = "),
                }
                // **A kept buffer assigned again goes into its keep too**
                // (#293): the binding is a place in the keep, and the views of
                // the old value still point into the old one.
                let reput = self
                    .keep_plan(flow.function)
                    .and_then(|p| p.reputs.get(&(span.at() as i64)))
                    .copied();
                if let Some(keep) = reput {
                    let put = match keep {
                        nikaia_std::tools::buffers::KeepAt::Element(_)
                        | nikaia_std::tools::buffers::KeepAt::Task
                        | nikaia_std::tools::buffers::KeepAt::Param => "put_viewed",
                        _ => "put",
                    };
                    out.push(&format!("{}.{put}(", Self::keep_expr(keep, false)));
                }
                out.push(&before);
                // **A view of the place being assigned is copied first**
                // (#563): `rest = rest.strip_prefix(">") ?? ""` into a mixed
                // `String` hands `EitherText` a borrow of the very buffer the
                // assignment replaces (E0506). The text is made its own, and
                // the value that goes in is the owned kind.
                let mut own_view = false;
                if op.is_none()
                    && let Expr::Variable(assigned, _) = target
                    && let Expr::MethodCall {
                        receiver,
                        method,
                        args,
                        ..
                    } = value
                    && args.is_empty()
                    && self.text(*method) == "into_either"
                {
                    let mut mentions = false;
                    visit_expr(receiver, &mut |inner| {
                        mentions |= matches!(inner, Expr::Variable(name, _) if name == assigned);
                    });
                    if mentions {
                        self.postfix_base(out, receiver, depth, flow)?;
                        out.push(".to_owned().into_either()");
                        own_view = true;
                    }
                }
                if !own_view {
                    self.expr(out, value, depth, flow)?;
                }
                out.push(&after);
                if reput.is_some() {
                    out.push(")");
                }
                out.push(";");
            }
            // Kap 3.3. Name for name (ADR-296 D17): the language below spells
            // this the same way, so there is nothing to decide here.
            Stmt::While { cond, body } => {
                // **`while true` is emitted as `loop`**, and this is the one
                // place the lowering translates what a loop *means* rather than
                // how it is spelled.
                //
                // It is not a departure from name for name (ADR-296 D17) but the
                // decided case of it:
                // [ADR-276](../../../docs/specification/adr/adr-276.md) D1 says
                // `while true { … }` **is** this language's unconditional loop
                // and that the absence of a second spelling is a decision. Rust's
                // name for the unconditional loop is `loop`. So `loop` is the
                // translation of the program and `while true` is a transcription
                // of its letters - and the letters are what the language below
                // objects to:
                //
                //     warning: denote infinite loops with `loop { ... }`
                //
                // a warning on a line nobody wrote, which is Part III C.1's class
                // one severity down. **The alternative was an `#![allow]` in the
                // preamble**, and it is worse twice over: it silences a symptom
                // on every file this compiler will ever write, and it leaves the
                // second reason below unbuilt.
                //
                // **The second reason, which is why this is not cosmetic.** The
                // two forms do not have the same *type* below. `while true { }`
                // is `()`; `loop { }` diverges and is `!`, so
                // `fn f() -> i32 { loop { } }` compiles and the `while` form is
                // an `E0308`. [ADR-276](../../docs/specification/adr/adr-276.md)
                // wants a function that never
                // returns to stop needing an unreachable `return`, and no
                // checker change can deliver that while the lowering emits the
                // form the language below refuses. This is that prerequisite.
                //
                // **The literal only**, never a name that happens to be true:
                // the equivalence is ADR-276 D1's and it is about the written
                // form, and the polarity is the usual one - claim it where it is
                // certain and nowhere else.
                match cond {
                    Expr::LitBool(true, _) => out.push("loop "),
                    _ => {
                        out.push("while ");
                        self.expr(out, cond, depth, flow)?;
                        out.push(" ");
                    }
                }
                self.block_opening_with(
                    out,
                    body,
                    depth,
                    flow.inside_a_loop(),
                    Tail::Statement,
                    self.a_counted_turn(None).as_deref(),
                )?;
            }

            Stmt::For {
                bindings,
                shape,
                iter,
                body,
            } => {
                // **A loop over a type's fields is not a loop below**
                // ([ADR-304](../../docs/specification/adr/adr-304.md) D5,
                // [ADR-304](../../docs/specification/adr/adr-304.md) D6): it is
                // known while the program is built, so what is emitted is the
                // block once per field — the code somebody would have written
                // by hand, with no loop and no dispatch (D5's *at run time:
                // nothing*).
                if let Some(fields) = self.unrolls_here(iter) {
                    let bound = bindings
                        .first()
                        .map(|b| self.text(*b).to_string())
                        .unwrap_or_default();
                    for field in fields {
                        let pair = EmitPair {
                            first: bound.clone(),
                            second: field.name.clone(),
                        };
                        let outer = self.walking(|w| w.at_field.replace(pair));
                        let written = self.block(out, body, depth, flow, Tail::Statement);
                        self.walking(|w| w.at_field = outer);
                        written?;
                    }
                    return Ok(());
                }
                let each: Vec<String> =
                    bindings.iter().map(|b| self.local(self.text(*b))).collect();
                let names = each.join(", ");
                let bound = tuple_pattern(&each, shape);
                // **A step that pauses is a loop that gives its thread up**
                // ([ADR-172](../../docs/specification/adr/adr-172.md) D1). The
                // language below has no `for` that awaits, so the shape is the
                // one a Rust programmer writes by hand for the same thing: the
                // sequence is bound once and stepped with a `.next().await`.
                //
                // **Off the checker's set and not off the shape**, exactly as
                // the `?` below is: which sequences pause is a claim in the
                // ledger about a *type*, and this emitter has none
                // ([ADR-288](../../docs/specification/adr/adr-288.md)).
                if self.s.facts.pausing_loops.contains(&(span.at() as i64)) {
                    // ADR-025 D1's `?`, unchanged: a step that can fail fails
                    // the enclosing function, and the checker has already made
                    // `throws` be there (`NK2701`).
                    let unwrap = self
                        .s
                        .facts
                        .fallible_loops
                        .contains(&(span.at() as i64))
                        .then(|| format!("let {bound} = {bound}?;"));
                    let pad = "    ".repeat(depth);
                    let inner = "    ".repeat(depth + 1);
                    out.push("{\n");
                    out.push(&inner);
                    out.push(&format!("let mut {SEQUENCE} = "));
                    self.expr(out, iter, depth + 1, flow)?;
                    // A produced sequence is walked **by value**
                    // ([ADR-105](../../docs/specification/adr/adr-105.md) D2),
                    // so nothing is lent here and there is no `.iter()`.
                    out.push(";\n");
                    out.push(&inner);
                    out.push(&format!(
                        "while let Some({bound}) = {SEQUENCE}.next().await "
                    ));
                    self.block_opening_with(
                        out,
                        body,
                        depth + 1,
                        flow.inside_a_loop(),
                        Tail::Statement,
                        self.a_counted_turn(unwrap).as_deref(),
                    )?;
                    out.push("\n");
                    out.push(&pad);
                    out.push("}");
                    return Ok(());
                }
                out.push(&format!("for {bound} in "));
                // **A `for` lends** ([ADR-094](../../docs/specification/adr/adr-094.md)
                // D4). `for e in entries { … }` leaves `entries` where it was,
                // so `entries.len()` on the next line is a program rather than
                // `rustc`'s *use of moved value* about a file nobody wrote.
                // Iteration that takes the elements away is **written** —
                // `for x in xs.drain()` — because removing a name from scope is
                // the rare case and the one worth a word.
                //
                // Off the shape of the expression and not off a column: a
                // **place** is lent, and a call, a range or a literal owns what
                // it made.
                //
                // **`.iter()` and not `&`**, which is one measurement rather
                // than a preference: `entries` may already *be* a view — a
                // parameter declared `&Vec[Entry]` — and `&entries` is then a
                // `&&Vec<Entry>`, which Rust does not iterate. `.iter()` reads
                // the same through any number of references, and this emitter
                // has no types to tell the two apart with (ADR-288).
                //
                // **A name holding a sequence is not a place that lends**
                // (ADR-293 D19): it is walked by value, and a range walks a copy
                // of itself. The checker says which loops those are.
                let lends =
                    is_a_place(iter) && !self.s.facts.owned_loops.contains(&(span.at() as i64));
                match iter {
                    // **A range written into the `for` stays the language
                    // below's own** (ADR-293 D18): it is walked once, where it
                    // stands, and needs to be nothing more.
                    Expr::Range { .. } => self.bare_range(out, iter, depth, flow)?,
                    // **So does a list written into it**: walked once and
                    // dropped, it is an array, which Rust walks by value -
                    // `vec![..]` there is an allocation `clippy` calls useless
                    // (found moving `contracts::locks` into Nikaia). Not an
                    // empty one, whose element type nothing would say.
                    Expr::ListLit { items, .. } if !lends && !items.is_empty() => {
                        out.push("[");
                        for (i, item) in items.iter().enumerate() {
                            if i > 0 {
                                out.push(", ");
                            }
                            self.expr(out, item, depth, flow)?;
                        }
                        out.push("]");
                    }
                    // **An element walked is a read with a postfix after it**
                    // (0.0.250): `(*get(&v, i)).iter()`, parenthesised, where
                    // `*get(&v, i).iter()` dereferenced the iterator.
                    _ if lends => self.postfix_base(out, iter, depth, flow)?,
                    _ => self.expr(out, iter, depth, flow)?,
                }
                if lends {
                    out.push(".iter()");
                    if self.reads_through_handle(flow, iter) {
                        out.push(HELD_READ);
                    }
                }
                out.push(" ");

                // ADR-025 D1: a step that can fail fails the enclosing
                // function, exactly as a written call would. This is that,
                // and it is the line a Rust programmer writes by hand. The
                // compiler has already refused the program if the function
                // does not declare `throws` (`NK2701`), so the `?` always has
                // somewhere to go.
                let unwrap = self
                    .s
                    .facts
                    .fallible_loops
                    .contains(&(span.at() as i64))
                    .then(|| format!("let {names} = {names}?;"));
                // **A number, a `bool` or a `char` is read out of the view
                // once, where the body opens** (`Checked::copied_loop_bindings`):
                // then the name is the element in every position below, and
                // not only at a cast and an annotated `let`
                // ([ADR-182](../../docs/specification/adr/adr-182.md) D1, D5).
                let copied = bindings
                    .iter()
                    .filter(|b| {
                        self.s
                            .facts
                            .copied_loop_bindings
                            .contains(&((span.at() as i64), self.text(**b).to_string()))
                    })
                    .map(|b| {
                        let name = self.local(self.text(*b));
                        format!("let {name} = nikaia_std::num::value({name});")
                    })
                    .chain(
                        bindings
                            .iter()
                            .filter(|b| {
                                self.s
                                    .facts
                                    .copied_view_bindings
                                    .contains(&((span.at() as i64), self.text(**b).to_string()))
                            })
                            .map(|b| {
                                let name = self.local(self.text(*b));
                                format!("let {name} = *{name};")
                            }),
                    )
                    .collect::<String>();
                let opening = match (unwrap, copied.is_empty()) {
                    (unwrap, true) => unwrap,
                    (None, false) => Some(copied),
                    (Some(unwrap), false) => Some(format!("{unwrap} {copied}")),
                };
                self.block_opening_with(
                    out,
                    body,
                    depth,
                    flow.inside_a_loop(),
                    Tail::Statement,
                    self.a_counted_turn(opening).as_deref(),
                )?;
            }
            // A `return` that ends a *function body* is that function's
            // value, and is written as one: `fn f() -> T { return x }` is
            // `{ x }`. In a `throws` function the caller of this wraps the tail
            // in `Ok`, so the value is emitted bare here in both cases.
            //
            // `Tail::Return` and not merely "last": the last statement of a
            // `match` arm, a `catch` handler or a block used as a value is the
            // value of the expression around it, and a `return` there leaves
            // the function past that expression. It falls through to the arm
            // below and stays a `return`.
            // Kap 3.3 and [ADR-276](../../../docs/specification/adr/adr-276.md).
            // Name for name, like `while` above and for the same reason
            // (ADR-296 D17): the language below spells both of these the same way
            // and gives them the same meaning, so the lowering is a
            // transcription - which is also what makes the construct cost
            // nothing at run time (`docs/history/break-continue-cost.md` §2).
            //
            // **No `Tail` arm**, and the semicolon stands even where the
            // statement is a block's last: a block that ends in a jump diverges,
            // and Rust reads `{ break; }` in value position as the `!` it is -
            // the same way `{ return x; }` is read there.
            //
            // **And the `in_loop` test is D6's guarantee**, not a second opinion
            // on the checker's: `NK1132` is the message a program meets, and
            // this is what makes *"the checker's walk is complete"* something
            // that holds rather than something to hope for.
            Stmt::Break | Stmt::Continue => {
                let word = match stmt {
                    Stmt::Break => "break",
                    _ => "continue",
                };
                if !flow.in_loop {
                    return Err(refused_at!(
                        flow.statement,
                        "`{word}` only works inside a loop, and there's none here."
                    ));
                }
                out.push(&format!("{word};"));
            }
            Stmt::Return(Some(value)) if tail == Tail::Return => {
                self.handed_back(out, value, span, depth, flow)?;
            }
            Stmt::Return(value) => {
                // Kap 7.1: a `throws` function returns a `Result`, so what the
                // source hands back is what goes inside the `Ok`.
                match (value, flow.throws) {
                    (Some(value), true) => {
                        out.push("return Ok(");
                        self.handed_back(out, value, span, depth, flow)?;
                        out.push(");");
                    }
                    (Some(value), false) => {
                        out.push("return ");
                        self.handed_back(out, value, span, depth, flow)?;
                        out.push(";");
                    }
                    (None, true) => out.push("return Ok(());"),
                    (None, false) => out.push("return;"),
                }
            }
            // An `if` in *statement* position is not a value, and its branches
            // are not tails. Emitting them as tails is right for
            // `let x = if c { a } else { b }` and wrong here: a `return` at the
            // end of a branch would be written as the branch's value and stop
            // returning - `if seq.len() < k { return counts }` came out as
            // `if seq.len() < k { counts }`, which is a different program.
            Stmt::Expr(Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            }) => self.if_expr(
                out,
                cond,
                then_branch,
                else_branch.as_ref(),
                depth,
                flow,
                tail,
            )?,
            // A `match` in *statement* position, for the `if`'s reason above:
            // its arms are not the block's value.
            Stmt::Expr(expr @ Expr::Match { .. }) if tail == Tail::Statement => {
                self.expr(
                    out,
                    expr,
                    depth,
                    Flow {
                        arms_are_statements: true,
                        ..flow
                    },
                )?;
            }
            Stmt::Expr(expr) => {
                // **A tail is a `return` written without the word**, so the `&`
                // it may owe is the same one — `fn text(ref self) -> ref String
                // { self.text }` and the `return` form are one program.
                // A `match` or `if` whose arms all leave has no value to wrap
                // (#696): only the arms that come to one are handed back.
                let either = tail == Tail::Return
                    && self.returns_either(flow.function)
                    && !matches!(expr, Expr::LitNull(_))
                    && !nikaia_std::tools::emit_ops::never_a_value(expr)
                    && !self
                        .s
                        .facts
                        .mixed_choices
                        .contains(&crate::check::value_node(expr))
                    && !self
                        .s
                        .facts
                        .mixed_leaves
                        .contains_key(&crate::check::value_node(expr));
                if either {
                    out.push("nikaia_std::either_text::either(");
                }
                if tail == Tail::Return && self.s.facts.lent_returns.contains(&(span.at() as i64)) {
                    out.push("&");
                }
                self.expr(out, expr, depth, flow)?;
                if either {
                    out.push(")");
                }
                // `if x { … };` is legal and noisy; a block-shaped statement
                // ends where its brace does.
                let block_shaped = matches!(
                    expr,
                    Expr::If { .. } | Expr::Block(_, _) | Expr::Unsafe(_, _) | Expr::Overlap(_, _)
                );
                if !tail.is_value() && !block_shaped {
                    out.push(";");
                }
            }
        }
        Ok(())
    }

    /// **What goes before a wrapped value, and what goes after**
    /// ([ADR-278](../../../docs/specification/adr/adr-278.md)).
    ///
    /// One function for all four of Part I 2.3's positions, so the two forms are
    /// written in one place: `Some(` … `)` where the checker knows the value is
    /// a plain `T`, and `` … `.into()` where it could not work the type out —
    /// which is right whether the value turns out to be a `T` or already a `T?`.
    ///
    /// The target is pinned in every one of the four — a declared result, an
    /// annotation, an assignment's left side, a field's or a parameter's
    /// declared type — so the conversion has something to resolve against and
    /// never has to be inferred from the value alone.
    /// The same, around a value written out: a conversion is a method call, so
    /// a value an operator makes is held in parentheses - `a - 1` into an
    /// `i64?` went below as `a - 1.into()`, the conversion taking the `1` alone
    /// (found moving the build-time integers into Nikaia, #125).
    fn around_value(how: Option<&crate::check::Wrap>, value: &Expr) -> (String, String) {
        nikaia_std::tools::emit_exprs::emit_around_value(how, value)
    }

    /// What a `return` hands back: the nullable wrap, and **the `&` the compiler
    /// owes** where the value is a view of the subject
    /// ([ADR-094](../../../docs/specification/adr/adr-094.md) D1's third
    /// position).
    ///
    /// The `&` goes **inside** the wrap, because `Some(&self.text)` is an
    /// `Option<&String>` and `&Some(self.text)` is a view of a value built
    /// here — which also moves the field, the very thing the reference is for.
    fn handed_back(
        &self,
        out: &mut Out,
        value: &Expr,
        span: &Span,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_handed_back(
                &self.world,
                walk,
                out,
                self,
                value,
                span.at() as i64,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// Whether the function being written hands back text both kinds of
    /// which flow into its result (ADR-282 D14).
    fn returns_either(&self, function: &str) -> bool {
        nikaia_std::tools::emit_calls::call_returns_either(
            &self.parsed.interner,
            &self.parsed.program.items,
            function,
        )
    }

    /// Whether what stands in the brackets is a **run**: a range written there,
    /// or one the checker says is kept in a name (ADR-293 D23).
    fn slices(&self, index: &Expr) -> bool {
        self.s.slices(index)
    }

    /// How the checker said this key goes into a map's brackets, where the
    /// map's keys are owned ([ADR-293](../../docs/specification/adr/adr-293.md)
    /// D1).
    fn map_key(&self, index: &Expr, written: bool) -> Option<crate::check::KeyForm> {
        self.s.map_key(index, written)
    }

    /// A key, handed over as it is or lent with a `&`.
    fn key(
        &self,
        out: &mut Out,
        form: crate::check::KeyForm,
        index: &Expr,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_key(
                &self.world,
                walk,
                out,
                self,
                form,
                index,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// A range as the language below writes one, `a..b`, for the two places
    /// that walk or slice it where it stands
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D18).
    fn bare_range(&self, out: &mut Out, expr: &Expr, depth: usize, flow: Flow<'_>) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_bare_range(
                &self.world,
                walk,
                out,
                self,
                expr,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// What stands inside brackets: a range there is a **slice**, and stays
    /// the language below's own (ADR-293 D18).
    /// Whether `name` is a parameter of the function being emitted that it
    /// changes in place - a `mut` parameter, lowered as `&mut T` (ADR-094 D3).
    fn changes_in_place(&self, flow: Flow<'_>, name: &str) -> bool {
        self.s.changes_in_place(flow.function, name)
    }

    /// Whether `name` is a `mut` parameter whose value **copies** - a number,
    /// a `bool`, a `char`, a `scalar` (Part I 2.2): below it is a `&mut i64`,
    /// and a read of it is the number, `*n`, as `n + 1` and `n == 0` need
    /// (#576). A parameter of any other type is read through methods and
    /// handed on, which the language below dereferences itself.
    fn copied_in_place(&self, flow: Flow<'_>, name: &str) -> bool {
        self.s.copied_in_place(flow.function, name)
    }

    fn index_expr(&self, out: &mut Out, index: &Expr, depth: usize, flow: Flow<'_>) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_index_expr(
                &self.world,
                walk,
                out,
                self,
                index,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    fn expr(&self, out: &mut Out, expr: &Expr, depth: usize, flow: Flow<'_>) -> Result<()> {
        // **A receiver bound before the statement** reads as its name
        // (`hold_owned_receiver`).
        if let Some(name) = self.walk(|w| w.held.get(&(crate::check::value_node(expr))).cloned()) {
            out.push(&name);
            return Ok(());
        }
        // **A `T?` a test proved not `null` is read as the `T`** (ADR-350):
        // the test runs before the read on every path, so the unwrap never
        // fails.
        if let (Some(how), Expr::Variable(name, _)) = (
            self.s
                .facts
                .narrowed_reads
                .get(&crate::check::value_node(expr))
                .copied(),
            expr,
        ) {
            // **Put back into an option, it stays the option it is**: the
            // wrap the slot asked for is already written, and its `)` follows,
            // so the read is the option itself, as it was before the test.
            let copied = how & 1 == 1;
            let operand = how & 2 == 2;
            // **A struct or a list is lent out of the option** (#621), and
            // given up where the read hands it to something that keeps it.
            let other = how & 4 == 4;
            let given_up = how & 8 == 8;
            let lent = match copied || other {
                true => "Option::as_ref(&",
                false => "Option::as_deref(&",
            };
            for (wrap, instead) in [("Some(&", lent), ("Some(", "std::convert::identity(")] {
                if !operand && out.buf.ends_with(wrap) {
                    out.buf.truncate(out.buf.len() - wrap.len());
                    out.push(instead);
                    out.push(self.parsed.text(*name));
                    return Ok(());
                }
            }
            // Lent where a `&` asked for a reference: `as_ref` is that
            // reference already.
            if other && !given_up && out.buf.ends_with('&') && !out.buf.ends_with("&&") {
                out.buf.truncate(out.buf.len() - 1);
            }
            out.push(self.parsed.text(*name));
            out.push(match copied || given_up {
                true => ".unwrap()",
                false => ".as_ref().unwrap()",
            });
            return Ok(());
        }
        // **An arm of a choice that goes into a mixed position is wrapped on
        // its own** (ADR-282 D14, `check::Checked::mixed_leaves`, #562): a
        // view and text of its own are two types, and only `EitherText` holds
        // both.
        if let Some(nullable) = self
            .s
            .facts
            .mixed_leaves
            .get(&crate::check::value_node(expr))
            .copied()
        {
            out.push(match nullable {
                true => "nikaia_std::either_text::either_maybe(",
                false => "nikaia_std::either_text::either(",
            });
            // `either_maybe` hands a present value in as `Some` itself, and an
            // arm that is a `String?` already as it is: the nullable wrap of a
            // plain arm would make the one an `Option` too many and wrap the
            // other twice (#696).
            let node = crate::check::value_node(expr);
            if nullable
                && (self.s.facts.some_tails.contains(&node)
                    || self.s.facts.wrapped.contains_key(&node))
            {
                self.expr_as_written(out, expr, depth, flow)?;
            } else {
                self.expr_unmixed(out, expr, depth, flow)?;
            }
            out.push(")");
            return Ok(());
        }
        self.expr_unmixed(out, expr, depth, flow)
    }

    /// [`Emitter::expr`], past the wrap of an arm in a mixed position.
    fn expr_unmixed(&self, out: &mut Out, expr: &Expr, depth: usize, flow: Flow<'_>) -> Result<()> {
        // **A plain arm beside a `null` is `Some`** (`check::Checked::some_tails`).
        if self
            .s
            .facts
            .some_tails
            .contains(&(crate::check::value_node(expr)))
        {
            out.push("Some(");
            self.expr_as_written(out, expr, depth, flow)?;
            out.push(")");
            return Ok(());
        }
        // **And a plain value in a nullable slot** (`check::Checked::wrapped`).
        if let Some(how) = self.s.facts.wrapped.get(&(crate::check::value_node(expr))) {
            let (before, after) = Self::around_value(Some(how), expr);
            out.push(&before);
            self.expr_as_written(out, expr, depth, flow)?;
            out.push(&after);
            return Ok(());
        }
        // **A map's read copied out where a `T?` of its own is kept**
        // ([ADR-293](../../docs/specification/adr/adr-293.md) D15). A read
        // through the brackets is `*get(…)`, whose `*` would take the whole
        // chain, so it is held in parentheses.
        let shape = crate::check::value_node(expr);
        if self.s.facts.copied_reads.contains(&shape) {
            let held =
                matches!(expr, Expr::Index { .. }) && !self.s.facts.flat_reads.contains(&shape);
            if held {
                out.push("(");
            }
            self.expr_after_the_copy(out, expr, depth, flow)?;
            if held {
                out.push(")");
            }
            out.push(".copied()");
            return Ok(());
        }
        self.expr_after_the_copy(out, expr, depth, flow)
    }

    /// [`Emitter::expr`]'s last step, after a copy out of a map's read.
    fn expr_after_the_copy(
        &self,
        out: &mut Out,
        expr: &Expr,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        // **`get` on a map of `T?`** is one `T?`, as the brackets are
        // ([ADR-293](../../docs/specification/adr/adr-293.md) D14).
        if matches!(expr, Expr::MethodCall { .. })
            && self
                .s
                .facts
                .flat_reads
                .contains(&crate::check::value_node(expr))
        {
            out.push("nikaia_std::index::flat(");
            self.expr_as_written(out, expr, depth, flow)?;
            out.push(")");
            return Ok(());
        }
        self.expr_as_written(out, expr, depth, flow)
    }

    /// **A call that can fail inside the expression a `catch` guards leaves
    /// it with its error** (#721, Part I 7.1: the handler gets every error
    /// that arrives there): `match call { Ok(value) => value, Err(error) =>
    /// break 'label Err(error.into()) }`, to the labelled block the `catch`
    /// opened (`emit_core_try_catch`).
    fn breaking_out(
        &self,
        out: &mut Out,
        expr: &Expr,
        flow: Flow<'_>,
        write: impl FnOnce(&mut Out) -> Result<()>,
    ) -> Result<()> {
        if flow.breaks_to < 0 || !self.s.emit_call_fails(&self.world, expr) {
            return write(out);
        }
        out.push("match ");
        write(out)?;
        out.push(&format!(
            " {{ Ok(value) => value, Err(error) => break '__nikaia_caught_{} Err(error.into()) }}",
            flow.breaks_to
        ));
        Ok(())
    }

    /// [`Emitter::expr`], without the wrap an arm beside a `null` takes.
    fn expr_as_written(
        &self,
        out: &mut Out,
        expr: &Expr,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        // **A text's own bytes kept beyond the text are copied** into a `Bytes`
        // of their own (ADR-320 D11, `kept_text_bytes`): `s.bytes`, or a name
        // a `let` bound to it, handed to what keeps it or handed back.
        if self
            .s
            .facts
            .kept_text_bytes
            .contains(&(crate::check::value_node(expr)))
        {
            out.push("nikaia_std::bytes::Bytes::from(");
            self.expr_written(out, expr, depth, flow)?;
            out.push(")");
            return Ok(());
        }
        self.expr_written(out, expr, depth, flow)
    }

    /// [`Emitter::expr_as_written`], past the copy of a text's bytes.
    fn expr_written(&self, out: &mut Out, expr: &Expr, depth: usize, flow: Flow<'_>) -> Result<()> {
        // **An arm read off a place under a `ref T` is lent** (ADR-354 D1).
        if self
            .s
            .facts
            .lent_reads
            .contains(&crate::check::value_node(expr))
        {
            out.push("&");
        }
        match expr {
            // **A literal in an unsigned place carries that type's suffix**
            // (ADR-285 D3), ahead of the widening a bare one gets.
            Expr::LitInt {
                value, negative, ..
            } => {
                let v = crate::ast::int_value(*value, *negative);
                match self
                    .s
                    .facts
                    .unsigned_literals
                    .get(&crate::check::value_node(expr))
                {
                    Some(ty) => out.push(&format!("{v}{ty}")),
                    None => out.push(&integer_literal(v, flow.widen)),
                }
            }
            Expr::LitFloat(v, _) => out.push(v),
            Expr::LitStr { .. } | Expr::LitInterpolated { .. } => {
                self.string(out, expr, depth, flow)?
            }
            Expr::LitChar(c, _)
                if self
                    .s
                    .facts
                    .byte_literals
                    .contains(&crate::check::value_node(expr)) =>
            {
                out.push(&crate::check::byte_literal(c))
            }
            Expr::LitChar(c, _) => out.push(&crate::check::char_literal(c)),
            // **A range is a value**
            // ([ADR-293](../../docs/specification/adr/adr-293.md) D18): two
            // numbers, `Copy`, and walked as often as a program likes - which
            // the language below's `Range` is not, being its own iterator. So
            // one that is kept, handed on or called on is `nikaia_std`'s, and
            // only one written straight into a `for` or into brackets is Rust's
            // (`bare_range`).
            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                out.push(match inclusive {
                    true => "nikaia_std::range::through(",
                    false => "nikaia_std::range::span(",
                });
                self.expr(out, start, depth, flow)?;
                out.push(", ");
                self.expr(out, end, depth, flow)?;
                out.push(")");
            }
            Expr::LitBool(b, _) => out.push(&b.to_string()),
            // Part I 2.3. `None` and nothing around it: `null` has no type of
            // its own, so the type beside it is what says what it is the
            // absence of - and a `None` the language below cannot type is a
            // program `rustc` asks an annotation for, which is the honest
            // answer rather than one this compiler invented.
            Expr::LitNull(_) => out.push("None"),
            // **A `mut` lambda parameter is an address** (ADR-281 D12), so
            // every mention of it is dereferenced.
            Expr::Variable(name, _) if flow.changed.contains(name) => {
                out.push(&format!("(*{})", self.read_name(*name, expr)))
            }
            // **A constructor handed over as a value**
            // ([ADR-140](../../docs/specification/adr/adr-140.md) D2). `Summary`
            // in value position is the anonymous constructor, and the language
            // below calls it `Summary::new` — the same two spellings the call
            // path reconciles, one position over. `par_fold(M, Summary, …)` is
            // where it is written.
            //
            // **Only a name this file declares as a type**, or one the library
            // publishes with a constructor: anything else is an ordinary name,
            // and a local that shadows a type is refused a declaration by
            // [ADR-286](../../docs/specification/adr/adr-286.md) rather than
            // guessed at here.
            Expr::Variable(name, _) if self.constructs_by_name(self.text(*name)) => {
                out.push(&self.path(&[self.text(*name), "new"]))
            }
            Expr::Variable(_, _) | Expr::Path(_, _)
                if !self.walk(|w| w.in_a_kept_call)
                    && self
                        .s
                        .facts
                        .kept_functions
                        .contains_key(&crate::check::value_node(expr)) =>
            {
                // **A named function, or a method named by its path, where a
                // function value is kept** is the closure that calls it,
                // written as the checker typed it.
                let (pauses, fails, arity) =
                    self.s.facts.kept_functions[&crate::check::value_node(expr)];
                // The call the checker built and typed, read back by its key
                // ([`crate::parser::Parsed::kept_function_call`]): only the
                // checker knows whether a method takes a `self`.
                let key = (self.parsed.kept_function_text(expr), arity);
                let Some(kept) = self.s.kept_calls.get(&key) else {
                    return Err(anyhow!("no kept call was built for `{}`", key.0));
                };
                let (names, call) = (kept.names.clone(), kept.call.clone());
                out.push(&format!(
                    "nikaia_std::func::Kept(std::sync::Arc::new(move |{}| ",
                    names
                        .iter()
                        .map(|n| self.text(*n).to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                let inside = Flow {
                    throws: fails,
                    statement: flow.statement,
                    ..Flow::PLAIN
                };
                if pauses {
                    out.push("Box::pin(async move ");
                }
                out.push(match fails {
                    true => "{ Ok(",
                    false => "{ ",
                });
                let outer = self.walking(|w| std::mem::replace(&mut w.in_a_kept_call, true));
                let written = self.expr(out, &call, depth, inside);
                self.walking(|w| w.in_a_kept_call = outer);
                written?;
                out.push(match fails {
                    true => ") }",
                    false => " }",
                });
                if pauses {
                    out.push(&format!(") as {}<'_, _>", self.boxed_future_in()));
                }
                out.push("))");
            }
            // **An integer `comptime` where a use asks a type is its value in
            // that type** (ADR-287 D21, `Checked::comptime_uses`).
            Expr::Variable(_, _)
                if self
                    .s
                    .facts
                    .comptime_uses
                    .contains_key(&crate::check::value_node(expr)) =>
            {
                out.push(&self.s.facts.comptime_uses[&crate::check::value_node(expr)]);
            }
            Expr::Variable(name, _)
                if !flow.in_a_place && self.copied_in_place(flow, self.text(*name)) =>
            {
                out.push(&format!("*{}", self.read_name(*name, expr)));
            }
            Expr::Variable(name, _) => {
                out.push(&self.read_name(*name, expr));
                // **A binding packed for a task is read through its handle**
                // (ADR-283 D11), for as long as the read borrows it - and so is
                // an element taken out of a keeper that holds structs of views
                // ([ADR-283](../../docs/specification/adr/adr-283.md) D16): it
                // carries its handles with it.
                let packed = self.keep_plan(flow.function).is_some_and(|p| {
                    p.tethered.contains_key(self.text(*name))
                        || p.held_locals.contains(self.text(*name))
                });
                if packed && !flow.in_a_place {
                    out.push(".get()");
                }
            }
            // ADR-017: the template is compiled where it is written. What comes
            // out is the string building a hand-written renderer would do, with
            // `html::Render` at every hole - which is what makes the escaping a
            // property of the template rather than of whoever filled it in.
            // **A block over a grammar of this program is the value its
            // entry computed while the program was built** (#518): the
            // checker recorded it under the statement, as a `comptime`'s.
            Expr::Dsl {
                target,
                package: None,
                context: None,
                ..
            } if self.s.grammars.contains_key(self.text(*target))
                && self
                    .s
                    .facts
                    .dsl_values
                    .contains_key(&crate::check::value_node(expr)) =>
            {
                // Not an `if let` guard: the toolchain floor (1.88) has none.
                let (below, written) =
                    self.s.facts.dsl_values[&crate::check::value_node(expr)].clone();
                out.push(&format!(
                    "{{ const __NIKAIA_DSL: {below} = {written}; __NIKAIA_DSL }}"
                ));
            }
            Expr::Dsl {
                target,
                package,
                content,
                context,
                ..
            } => {
                let name = match package {
                    Some(package) => format!("{}::{}", self.text(*package), self.text(*target)),
                    None => self.text(*target).to_string(),
                };
                self.template(
                    out,
                    &name,
                    context.as_ref(),
                    content,
                    crate::ast::id_of(expr),
                    depth,
                    flow,
                )?
            }
            Expr::Path(segments, _) => {
                let path: Vec<&str> = segments.iter().map(|s| self.text(*s)).collect();
                out.push(&self.path(&path));
            }
            // A block in *expression* position: its last statement is its
            // own value, and a `return` inside it leaves the function around
            // it - so it is not written as the block's value (`Tail`).
            Expr::Block(block, _) => self.block(out, block, depth, flow, Tail::Value)?,
            // **Rust's own `unsafe`**
            // ([ADR-302](../../docs/specification/adr/adr-302.md) D3), which is
            // the whole of the lowering: the word means the same thing on both
            // sides and the block inside it is an ordinary one.
            Expr::Unsafe(block, _) => {
                out.push("unsafe ");
                self.block(out, block, depth, flow, Tail::Value)?;
            }

            // **Part I 8.1.2: every branch in flight, and the value is their
            // results in written order** (ADR-292 D2).
            Expr::Overlap(block, _) => self.overlap(out, block, depth, flow)?,
            // **Part II 12.4: every arm in flight, and the first to finish is
            // the one that is kept** (ADR-292 D12).
            Expr::Select(arms, _) => self.select(out, arms, depth, flow)?,
            // An `if` in *expression* position - `let x = if c { a } else { b }`
            // - hands its branch's value to whoever asked for it, so a `return`
            // in a branch is the function's and stays one.
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => self.if_expr(
                out,
                cond,
                then_branch,
                else_branch.as_ref(),
                depth,
                flow,
                Tail::Value,
            )?,
            Expr::Call {
                func, args, config, ..
            } => self.breaking_out(out, expr, flow, |out| {
                self.call(out, func, args, config, depth, flow)
            })?,
            // **A `?.` after a value that cannot be absent is the plain call**
            // (ADR-279 D13, `check::Checked::plain_reaches`).
            Expr::MethodCall {
                receiver,
                method,
                args,
                config,
                ..
            }
            | Expr::SafeMethod {
                receiver,
                method,
                args,
                config,
                ..
            } if matches!(expr, Expr::MethodCall { .. })
                || self
                    .s
                    .facts
                    .plain_reaches
                    .contains(&crate::check::value_node(expr)) =>
            {
                self.breaking_out(out, expr, flow, |out| {
                    self.method_arm(out, expr, receiver, *method, args, config, depth, flow)
                })?
            }
            // Part I 3.5 onto a **method** (ADR-278): `emit_core_safe_method`.
            Expr::SafeMethod {
                receiver,
                method,
                args,
                config,
                ..
            } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_safe_method(
                    &self.world,
                    walk,
                    out,
                    self,
                    receiver,
                    *method,
                    args,
                    config,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Match { value, arms, .. } => self.match_expr(out, value, arms, depth, flow)?,
            Expr::Tuple(parts, _) => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_tuple(
                    &self.world,
                    walk,
                    out,
                    self,
                    parts,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::ListLit { items, .. } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_list_lit(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    items,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::ListRepeat { value, count, .. } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_list_repeat(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    value,
                    count,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Field { base, name, .. } | Expr::SafeField { base, name, .. }
                if matches!(expr, Expr::Field { .. })
                    || self
                        .s
                        .facts
                        .plain_reaches
                        .contains(&crate::check::value_node(expr)) =>
            {
                self.cross(flow, |walk, flow, scope| {
                    self.s.emit_core_field(
                        &self.world,
                        walk,
                        out,
                        self,
                        expr,
                        base,
                        *name,
                        depth as i64,
                        flow,
                        scope,
                    )
                })?
            }
            Expr::SafeField { base, name, .. } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_safe_field(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    base,
                    *name,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Index { base, index, .. } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_index(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    base,
                    index,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Cast {
                expr: operand, ty, ..
            } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_cast(
                    &self.world,
                    walk,
                    out,
                    self,
                    operand,
                    ty,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::StructLit { name, fields, .. } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_struct_lit(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    *name,
                    fields,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::With {
                base, fields, at, ..
            } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_with(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    base,
                    fields,
                    *at,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Closure {
                params,
                mutable,
                body,
                ..
            } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_closure(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    params,
                    mutable,
                    body,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            whole @ Expr::Unary { op, expr, .. } => {
                // **`-2147483648` is an `i32`**, and its digits are not
                // ([ADR-285](../../../docs/specification/adr/adr-285.md) D23): the
                // question is about the value, so a negation is folded here
                // rather than left for `integer_literal` to answer about a number
                // that is one too large.
                if let (
                    UnaryOp::Neg,
                    Expr::LitInt {
                        value, negative, ..
                    },
                ) = (op, &**expr)
                    && let v = &crate::ast::int_value(*value, *negative)
                    && i32::try_from(-*v).is_ok()
                {
                    // The suffix still applies: a small negative number
                    // inside a constant written wide is written wide too,
                    // or the operands of one sum disagree.
                    let wide = match flow.widen {
                        true => "i64",
                        false => "",
                    };
                    out.push(&format!("-{v}{wide}"));
                    return Ok(());
                }
                // **A `&` over a slice read is the view it already is**
                // ([ADR-182](../../docs/specification/adr/adr-182.md) D2). The read
                // answers a `&str` or a `&[T]` since the `*` came off it, so
                // writing the `&` as well would make `&dna[i..<i + k]` a
                // `&&str` — which coerces in most places and is a type error
                // in the ones that matter, and is not what the source says
                // either: the source's `&` and the read's own view are one
                // claim written twice.
                if let (UnaryOp::Ref, Expr::Index { index, .. }) = (op, &**expr)
                    && self.slices(index)
                    && !flow.in_a_place
                {
                    return self.expr(out, expr, depth, flow);
                }
                // **`-x` proved to stay inside its type** (ADR-314 D5) is
                // written without its check.
                if let (UnaryOp::Neg, Some(ty)) = (
                    op,
                    self.s
                        .proven_negations
                        .get(&((crate::check::value_node(whole) as usize) as i64)),
                ) {
                    out.push(&format!("<{ty}>::wrapping_neg("));
                    self.expr(out, expr, depth, flow)?;
                    out.push(")");
                    return Ok(());
                }
                out.push(&unary_op(*op));
                self.nested(out, expr, u8::MAX, depth, flow)?;
            }
            Expr::Binary {
                op,
                lhs,
                rhs,
                span: at,
                ..
            } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_binary(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    *op,
                    lhs,
                    rhs,
                    at,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            // The four arms of `??`, in their original order, are one function.
            Expr::Coalesce {
                value, fallback, ..
            } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_coalesce(
                    &self.world,
                    walk,
                    out,
                    self,
                    expr,
                    value,
                    fallback,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::TryCatch {
                expr: guarded,
                handler,
                ..
            } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_try_catch(
                    &self.world,
                    walk,
                    out,
                    self,
                    guarded,
                    handler,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Try(inner, _) => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_try(
                    &self.world,
                    walk,
                    out,
                    self,
                    inner,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Return(value, _) => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_return(
                    &self.world,
                    walk,
                    out,
                    self,
                    (**value).as_ref(),
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Break(_) | Expr::Continue(_) => self.cross(flow, |_, flow, _| {
                self.s
                    .emit_core_jump(out, matches!(expr, Expr::Break(_)), flow)
                    .map_err(Into::into)
            })?,
            Expr::Throw(inner, _) => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_throw(
                    &self.world,
                    walk,
                    out,
                    self,
                    inner,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            Expr::Spawn { body, .. } => self.cross(flow, |walk, flow, scope| {
                self.s.emit_core_spawn(
                    &self.world,
                    walk,
                    out,
                    self,
                    body,
                    depth as i64,
                    flow,
                    scope,
                )
            })?,
            other => {
                return Err(refused_at!(
                    flow.statement,
                    "The compiler can't translate this expression yet: {other:?}"
                ));
            }
        }
        Ok(())
    }

    /// A call. `Stats(temp)` is the anonymous constructor of Kap 4.2 - Rust has
    /// no such thing, so the declaration decides: a name that is a struct is a
    /// call to the `new` its `impl` provides.
    fn call(
        &self,
        out: &mut Out,
        func: &Expr,
        args: &[Expr],
        config: &[crate::ast::ConfigArg],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_call(
                &self.world,
                walk,
                out,
                self,
                func,
                args,
                config,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// **`overlap { … }`** — Part I 8.1.2,
    /// [ADR-292](../../../docs/specification/adr/adr-292.md) D2 and D6.
    ///
    /// Each statement is a branch, each branch becomes an `async` block, and
    /// `task::overlap<n>` polls all of them in one pass. An `async` **block**
    /// and not a closure, for the reason a task's body is one: a branch is a
    /// block, and a closure around it would only be called once
    /// ([ADR-277](../../../docs/specification/adr/adr-277.md) D16 - the reason
    /// this line used to give, that Rust has no `async` closure, is false).
    /// No `move`, because D4 says
    /// nothing outlives the block — a branch borrows what is around it exactly
    /// as an ordinary statement does, and that is what makes the form lighter
    /// than two `spawn`s.
    ///
    /// **D6 is the argument order.** *"A branch is started up to its first
    /// suspension point before any branch that cannot suspend is run"* — so the
    /// branches that can pause are handed over first, and the join polls in the
    /// order it is given. Which those are is the ledger's `sync` column, read
    /// the same way [`Emitter::branch_pauses`] reads it for ADR-292's pairs.
    ///
    /// **And the value goes back into written order**, because D2 says it is the
    /// tuple in written order and the reordering above is a schedule. Where the
    /// two differ the tuple is rebuilt, which costs a move per branch and is
    /// visible in the emitted Rust as the permutation it is.
    fn overlap(&self, out: &mut Out, block: &Block, depth: usize, flow: Flow<'_>) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_overlap(
                &self.world,
                walk,
                out,
                self,
                block,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// **`select { … }`** — Part II 12.4,
    /// [ADR-292](../../../docs/specification/adr/adr-292.md) D12 and D13.
    ///
    /// Each arm's expression becomes an `async` block, `task::race<n>` polls all
    /// of them in one pass, and what comes back says **which** arm won. So the
    /// construct lowers to a `match` over a sum, which is what an arm binding a
    /// name and then running a block *is* in the language below.
    ///
    /// **The losers are cancelled, and nothing here writes that** (D2). The
    /// losing futures are dropped when `race<n>` returns; a future dropped at
    /// its suspension point tears its values down, and a `cleanup` that pauses
    /// is adopted by the runtime and bounded by the `cleanup-deadline`
    /// ([ADR-297](../../../docs/specification/adr/adr-297.md) D5). That is the
    /// mechanism this runtime already had, which is the whole argument of D3.
    ///
    /// **A failing arm**, where the enclosing function is `throws`: every arm is
    /// wrapped in `Ok` so the vehicle sees one shape, and the winner is
    /// unwrapped at the top of its own body — `?` there and not at the `match`,
    /// because only the arm that won has a value to propagate.
    fn select(
        &self,
        out: &mut Out,
        arms: &[SelectArm],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_select(
                &self.world,
                walk,
                out,
                self,
                arms,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// Whether a type named here holds a view, and so carries the input lifetime
    /// wherever it is written (Part II, 10.6).
    ///
    /// Two sources and one answer: the file's own declarations, and the
    /// package's ledger for the types another file declares
    /// ([`Emitter::tethered`]).
    /// Each source parameter's own lifetime and the result's lifetime
    /// arguments, where the result is a struct of several view fields and
    /// `returns` names one of `sources` for each (ADR-353 D1, D5).
    #[allow(clippy::type_complexity)]
    fn split_lifetimes(
        &self,
        ty: Option<&Type>,
        returns: Option<&str>,
        sources: &[String],
    ) -> Option<(Vec<(String, String)>, Vec<String>)> {
        let ty = ty.filter(|t| t.generics.is_empty() && !t.is_view)?;
        let slots = self.s.view_slots.get(self.text(ty.name))?;
        let returns = returns.filter(|r| !r.is_empty())?;
        // Each field's sources, by its path. A union (`ref(a | b)`) is one
        // lifetime over its group: every parameter a field may view is tied
        // to the others it may view, and to nothing else (ADR-353 D2).
        let mut of_field: HashMap<&str, Vec<&str>> = HashMap::new();
        for entry in returns.split("),") {
            let (path, written) = entry.split_once(':')?;
            let inside = written.trim().strip_prefix("ref(")?.trim_end_matches(')');
            let params: Vec<&str> = inside
                .split(['|', ','])
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .collect();
            if params.is_empty() || params.iter().any(|p| !sources.iter().any(|s| s == p)) {
                return None;
            }
            of_field.insert(path.trim(), params);
        }
        // The groups, joined by each field's union; a group is named by its
        // lowest parameter.
        fn find(group: &[usize], mut at: usize) -> usize {
            while group[at] != at {
                at = group[at];
            }
            at
        }
        let index = |p: &str| sources.iter().position(|s| s == p);
        let mut group: Vec<usize> = (0..sources.len()).collect();
        for params in of_field.values() {
            for p in &params[1..] {
                let first = find(&group, index(params[0])?);
                let other = find(&group, index(p)?);
                let (low, high) = (first.min(other), first.max(other));
                group[high] = low;
            }
        }
        let mut named: HashMap<usize, String> = HashMap::new();
        let mut own: Vec<(String, String)> = Vec::new();
        for (at, p) in sources.iter().enumerate() {
            let root = find(&group, at);
            let next = named.len();
            let lifetime = named
                .entry(root)
                .or_insert_with(|| match next {
                    0 => "'r".to_string(),
                    _ => format!("'r{next}"),
                })
                .clone();
            own.push((p.clone(), lifetime));
        }
        let fields = slots
            .iter()
            .map(|f| {
                let param = of_field.get(f.as_str())?[0];
                own.iter().find(|(p, _)| p == param).map(|(_, l)| l.clone())
            })
            .collect::<Option<Vec<String>>>()?;
        Some((own, fields))
    }

    fn borrows(&self, name: Symbol) -> bool {
        self.s.borrows(&self.world, name)
    }

    /// Whether a written type **carries** a view: it is one, it names a struct
    /// that holds one, or one of its arguments does
    /// ([ADR-283](../../docs/specification/adr/adr-283.md) D1, D7).
    ///
    /// [`holds_view`] asks the first of the three, off the written type alone,
    /// and that is all a walk without the declarations can do. This one has them:
    /// `Vec[Entry]` carries a view exactly when `Entry` does, and D9's
    /// derivation — *a function with nothing to borrow from can only write
    /// `'static`* — is about the **lifetime** in the signature rather than about
    /// the `&`, so it is this question it wants answered.
    fn carries_a_view(&self, ty: &Type) -> bool {
        self.s.carries_a_view(&self.world, ty)
    }

    /// The type a function's failures travel in.
    ///
    /// `borrows` is the same question the result position asks
    /// ([ADR-283](../../docs/specification/adr/adr-283.md) D7): an error that
    /// carries a view — `ConfigError::NotFound(path)`, Part I 7.1's own — is a
    /// type with a lifetime, and a function with nothing to borrow from can
    /// only write `'static` for it.
    fn error_channel(&self, key: &str, lifetimes: Lifetimes, borrows: bool) -> String {
        self.s.error_channel(
            &self.world,
            key,
            lifetimes == Lifetimes::Elided,
            &lifetimes.params(),
            &Lifetimes::Static.params(),
            borrows,
        )
    }

    /// The generated sums, written into the file
    /// ([ADR-280](../../docs/specification/adr/adr-280.md) D15).
    ///
    /// One `enum` per distinct set, the `From` each member needs so that a `?`
    /// converts on its own, and `Display`/`Error` so that the sum can still go
    /// into the opaque channel where a caller has one.
    fn write_error_sums(&self, out: &mut Out) {
        self.s.write_error_sums(&self.world, out)
    }

    /// A `match`: the arms as Rust's own, a `match` that only answers `true` or
    /// `false` as a `matches!`, and one over the error a `catch` binds, where that
    /// travels in a sum, taken apart by member (ADR-280 D17).
    fn match_expr(
        &self,
        out: &mut Out,
        value: &Expr,
        arms: &[crate::ast::MatchArm],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_match(
                &self.world,
                walk,
                out,
                self,
                value,
                arms,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// Whether an expression **never comes back**, so that nothing joins with
    /// it ([Part III A.2](../../docs/specification/30-nikaia-tooling.md)).
    ///
    /// `panic(…)` is the one call of that shape, and it is
    /// [ADR-293](../../docs/specification/adr/adr-293.md) D1's own written way
    /// out for a key the program knows is present: `m[k] ?? panic(f"…")`. It
    /// belongs beside [`jumps`] rather than inside it, because telling it apart
    /// needs the name and a free function has no parse to read one from.
    fn never_returns(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Call { func, .. }
            if matches!(func.as_ref(), Expr::Variable(name, _) if self.text(*name) == PANIC))
    }

    /// Whether a function of **this program** can pause, and is therefore an
    /// `async fn` ([ADR-055](../../../docs/specification/adr/adr-055.md) D1).
    ///
    /// **The answer is the ledger's `sync` column and nothing computed here.**
    /// [ADR-288](../../../docs/specification/adr/adr-288.md) D1 infers it per
    /// function over the call graph, and its conservatism is already the safe
    /// direction (D2): an unresolved call costs a function its claim, which here
    /// makes it `async` - and an `async fn` that never awaits finishes on its
    /// first poll, while a plain `fn` that needed to pause does not compile.
    fn pauses(&self, key: &str) -> bool {
        nikaia_std::tools::emit_calls::call_pauses(&self.s.own_contracts, key)
    }

    /// Whether a call to this callee carries an `.await` (ADR-055 D2).
    ///
    /// The same resolution [`Emitter::can_fail`] does, one ledger column over:
    /// `throws` becomes a `?` and *can pause* becomes an `.await`, both read off
    /// the callee's contract and neither resolving a name
    /// ([ADR-296](../../../docs/specification/adr/adr-296.md) D17).
    ///
    /// The ledger key a call resolves to, when that key pauses - `None` when the
    /// call takes no `.await` at all.
    ///
    /// D6's boxing needs to know *which* function is being called and not just
    /// that it pauses, so the answer is the name rather than a yes.
    ///
    /// **Both ledgers**, since §6 step 3 made `std`'s own pausing entries
    /// `async fn`. Step 2 asked only this program's own, because a `std` entry
    /// blocked its thread then and awaiting one would have been awaiting a value
    /// rather than a future - which is what let step 2 land on its own.
    /// Whether the own function under `key` was written `sync(f)`: the only
    /// way an own entry says `Sync::From`, since the inference never writes it
    /// ([ADR-288](../../docs/specification/adr/adr-288.md) D31).
    fn lambda_decides(&self, key: &str) -> bool {
        self.s.lambda_decides(key)
    }

    /// Whether a bare name in **value** position is a type's anonymous
    /// constructor ([ADR-140](../../docs/specification/adr/adr-140.md) D2).
    ///
    /// A type this file declares, or one the library publishes with a `::new`
    /// entry — read off the ledger rather than from a list of three names, for
    /// the reason the call path gives.
    fn constructs_by_name(&self, name: &str) -> bool {
        self.s.constructs_by_name(&self.world, name)
    }

    /// A string literal. Which of the two it is decides what comes out, and
    /// that is read off the syntax rather than the text (ADR-309 D2).
    fn string(&self, out: &mut Out, expr: &Expr, depth: usize, flow: Flow<'_>) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_string(
                &self.world,
                walk,
                out,
                self,
                expr,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// The future a kept function that may pause hands back: one that crosses
    /// threads where tasks run on them.
    fn boxed_future_in(&self) -> String {
        nikaia_std::tools::emit_core_types::emit_boxed_future_in(self.build.overlaps_user_code())
    }

    /// A member's lowered type, in its box where it has one.
    fn boxed_ty(&self, owner: &str, member: &str, lowered: String) -> String {
        self.s.boxed_ty(owner, member, lowered)
    }

    /// The `MethodCall` arm of `expr_as_written`, and the `?.` that is a plain
    /// call (ADR-279 D13).
    #[allow(clippy::too_many_arguments)]
    fn method_arm(
        &self,
        out: &mut Out,
        node: &Expr,
        receiver: &Expr,
        method: Symbol,
        args: &[Expr],
        config: &[crate::ast::ConfigArg],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_method_arm(
                &self.world,
                walk,
                out,
                self,
                node,
                receiver,
                method,
                args,
                config,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// The thing a `.` or a `[` is applied to, parenthesised where it binds
    /// looser than the postfix does.
    ///
    /// A postfix binds tighter than everything except another postfix in both
    /// languages, so `(a as f64).sqrt()` and `(a + b).len()` need their
    /// parentheses back: the parser drops them - a group is not a node, it is
    /// how the tree was written - and without them the emitted Rust means
    /// something else and often still compiles.
    fn postfix_base(&self, out: &mut Out, expr: &Expr, depth: usize, flow: Flow<'_>) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_postfix_base(
                &self.world,
                walk,
                out,
                self,
                expr,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// An operand of an operator, parenthesised only where it binds looser
    /// than the position it stands in.
    fn nested(
        &self,
        out: &mut Out,
        expr: &Expr,
        needs: u8,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_nested(
                &self.world,
                walk,
                out,
                self,
                expr,
                needs as i64,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }
}

// --- Free helpers ---

/// ADR-296 D9: the attribute is keyed in both languages, and the lowering is
/// one-to-one. A bare `@frame` stays bare - the boundary is then the rule's
/// trailing literal, which the backend infers and checks.
fn frame_attribute(frame: &FrameAttr) -> String {
    nikaia_std::tools::emit_ops::frame_attribute(frame)
}

/// The template rule `list(item, sep)` lowers to (Part II 10.8).
const LIST_RULE: &str = "__nikaia_list";

/// Whether a pattern writes `list(item, sep)` anywhere in it.
fn writes_a_list(parsed: &Parsed, pattern: &Spanned<Pattern>) -> bool {
    match &pattern.node {
        Pattern::Seq(parts) | Pattern::Choice(parts) => {
            parts.iter().any(|p| writes_a_list(parsed, p))
        }
        Pattern::Bind { pat, .. } | Pattern::Repeat { pat, .. } | Pattern::Group(pat) => {
            writes_a_list(parsed, pat)
        }
        Pattern::Ref { name, args, .. } => {
            (parsed.text(*name) == "list" && args.len() == 2)
                || args.iter().any(|p| writes_a_list(parsed, p))
        }
        Pattern::Literal(_) | Pattern::Cut | Pattern::Fold(_) => false,
    }
}

/// **The names bound to a `bytes(p)`** in a pattern, for a text grammar's
/// action to see as bytes (ADR-366 D4).
fn bytes_bindings(pattern: &Pattern, is_bytes: &dyn Fn(Symbol) -> bool, found: &mut Vec<Symbol>) {
    match pattern {
        Pattern::Bind { name, pat } => {
            if let Pattern::Ref { name: called, .. } = &pat.node
                && is_bytes(*called)
            {
                found.push(*name);
            }
            bytes_bindings(&pat.node, is_bytes, found);
        }
        Pattern::Seq(parts) | Pattern::Choice(parts) => {
            for part in parts {
                bytes_bindings(&part.node, is_bytes, found);
            }
        }
        Pattern::Repeat { pat, .. } | Pattern::Group(pat) => {
            bytes_bindings(&pat.node, is_bytes, found)
        }
        _ => {}
    }
}

fn repeat_suffix(rep: Repeat) -> String {
    nikaia_std::tools::emit_ops::repeat_suffix(rep)
}

/// **An integer literal, and the one case it has to carry its own type**
/// ([ADR-285](../../../docs/specification/adr/adr-285.md) D21).
///
/// Part I 2.4 says a number takes the type its use asks for, and `i32` where
/// nothing asks. The first half is the language below's inference and works;
/// the second half was that language's *default*, inherited rather than chosen,
/// and it is what refused `let big = 3000000000` in the backend's words about a
/// type the program never wrote (Part III C.1).
///
/// So a literal an `i32` does not hold is written as an `i64`, and D3 is why
/// that needs no analysis at all: the integer types a program may write are
/// `i32` and `i64` ([ADR-285](../../../docs/specification/adr/adr-285.md)), an
/// integer literal is never a float, and `u8` is narrower — so a value an `i32`
/// cannot hold has no second answer a use could ask for. There is nothing to
/// find out.
///
/// **A literal that fits an `i32` is untouched**, which is what keeps every
/// program that compiles today compiling: the suffix would pin what the use is
/// supposed to decide, and `let small = 42` passed to a parameter taking an
/// `i64` is Part I 2.4's own example.
///
/// The value and not the digits, which is why `-2147483648` never reaches here
/// as `2147483648`: the negation is folded at the `Unary` arm above.
fn integer_literal(value: i128, widen: bool) -> String {
    match (i32::try_from(value), i64::try_from(value)) {
        (Ok(_), _) if !widen => value.to_string(),
        (_, Ok(_)) => format!("{value}i64"),
        // **Above an `i64` only a `u64` holds it** (ADR-285 D19), and the
        // checker has already refused it anywhere else.
        _ => format!("{value}u64"),
    }
}

fn unary_op(op: UnaryOp) -> String {
    nikaia_std::tools::emit_ops::unary_op(op)
}

/// Rust's binding strength, which Nikaia shares.
/// Whether an index is written **only in literals**, and therefore needs no
/// conversion and can take none.
///
/// `xs[0]`, `xs[-1]`, `&text[1..3]`, `xs[2 * 3]`: Rust's own inference gives each
/// of these the `usize` a sequence wants. And `nikaia_std::index::at` cannot help
/// here even where it would be harmless, because there is nothing to infer its
/// argument type *from* - every integer type answers with the same `usize`, so the
/// call is ambiguous and `cannot infer type` about a generated file is what
/// Part III C.1 forbids.
/// Whether this expression **leaves** rather than coming to a value
/// ([ADR-276](../../docs/specification/adr/adr-276.md) D20).
///
/// What it decides is where the jump is *written*: a closure is a function
/// boundary ([ADR-276](../../docs/specification/adr/adr-276.md) D16), so a
/// lowering that puts an expression inside one has to know whether that
/// expression is a jump before it does.
fn jumps(expr: &Expr) -> bool {
    nikaia_std::tools::emit_ops::jumps(expr)
}

/// `const` or `mut`, the word Rust puts between the `*` and the type.
fn pointing(mutable: bool) -> String {
    nikaia_std::tools::emit_ops::pointing(mutable)
}

fn only_literals(index: &Expr) -> bool {
    nikaia_std::tools::emit_ops::only_literals(index)
}

fn binary_op(op: BinaryOp) -> String {
    nikaia_std::tools::emit_ops::binary_op(op)
}

type HoleTables = (
    std::collections::BTreeMap<(i64, String), Expr>,
    std::collections::BTreeMap<(i64, String), String>,
);

/// Every hole of every template in the file, by the template's site and the
/// hole's text - the trees every reader of them shares - and what was said of
/// the ones that did not parse.
fn template_hole_tables(parsed: &Parsed) -> HoleTables {
    let mut errors = std::collections::BTreeMap::new();
    let mut ask = |expr: &Expr| {
        let Expr::Dsl {
            target,
            package,
            content,
            ..
        } = expr
        else {
            return;
        };
        if crate::dsl::is_deferred(&dsl_name(parsed, *package, *target), content) {
            return;
        }
        let Ok(segments) = template::split(content.trim()) else {
            return;
        };
        let site = crate::ast::id_of(expr);
        for (hole, _) in template_holes_bound(&segments) {
            if let Err(error) = parsed.hole(site, &hole) {
                errors.insert((site as i64, hole), error.to_string());
            }
        }
    };
    for item in &parsed.program.items {
        match &item.node {
            Item::Fn { body, .. } => visit_block(body, &mut ask),
            Item::Impl { methods, .. } => {
                for method in methods {
                    if let Item::Fn { body, .. } = &method.node {
                        visit_block(body, &mut ask);
                    }
                }
            }
            _ => {}
        }
    }
    let holes = parsed
        .cached_holes()
        .into_iter()
        .map(|((site, text), expr)| ((site as i64, text), expr))
        .collect();
    (holes, errors)
}

/// Whether `target` is a field or an element of the name `root`, at any
/// depth: `acc.lines`, `acc.rows[i].n`. The name itself is not a part.
fn a_part_of(target: &Expr, root: Symbol) -> bool {
    match target {
        Expr::Field { base, .. } | Expr::Index { base, .. } => {
            matches!(&**base, Expr::Variable(name, _) if *name == root) || a_part_of(base, root)
        }
        _ => false,
    }
}

/// **The names a `fold`'s or a `par_fold`'s step calls directly** (#433): a
/// function it calls and a method it calls, by name, in any rule of any
/// grammar this unit declares. By name, because the step's receiver is the
/// accumulator, whose type this emitter does not know (ADR-296 D17) - and a
/// method of another type with the same name gains a hint, nothing more.
fn fold_callees(
    parsed: &Parsed,
    grammars: &std::collections::BTreeMap<String, GrammarDef>,
) -> HashSet<String> {
    fn steps<'a>(pattern: &'a Pattern, found: &mut Vec<&'a Expr>) {
        match pattern {
            Pattern::Fold(spec) => found.push(&spec.step),
            Pattern::Seq(all) | Pattern::Choice(all) => {
                all.iter().for_each(|p| steps(&p.node, found));
            }
            Pattern::Bind { pat, .. } | Pattern::Repeat { pat, .. } => steps(&pat.node, found),
            Pattern::Group(inner) => steps(&inner.node, found),
            Pattern::Ref { args, .. } => args.iter().for_each(|p| steps(&p.node, found)),
            Pattern::Literal(_) | Pattern::Cut => {}
        }
    }
    let mut found = Vec::new();
    for def in grammars.values() {
        for rule in &def.rules {
            for alt in &rule.alts {
                steps(&alt.pattern.node, &mut found);
            }
        }
    }
    let mut names = HashSet::new();
    for step in found {
        let Expr::Closure { body, .. } = step else {
            continue;
        };
        for stmt in &body.stmts {
            crate::contracts::sync::visit_stmt(parsed, &stmt.node, &mut |e| match e {
                Expr::MethodCall { method, .. } => {
                    names.insert(parsed.text(*method).to_string());
                }
                Expr::Call { func, .. } => {
                    if let Expr::Variable(name, _) = &**func {
                        names.insert(parsed.text(*name).to_string());
                    }
                }
                _ => {}
            });
        }
    }
    names
}

/// Whether an expression names a **place** — something that already exists and
/// can be pointed at — rather than a value this expression makes
/// ([ADR-094](../../../docs/specification/adr/adr-094.md) D4).
///
/// A name, a field of one, an element of one. A call, a range, a literal and a
/// method call are not: `for i in 0..n` counts, `for line in io::lines()` reads
/// a stream, and `for x in xs.drain()` is the written form of taking the
/// elements away — none of the three has anything to lend.
pub(crate) fn is_a_place(expr: &Expr) -> bool {
    nikaia_std::tools::emit_ops::is_a_place(expr)
}

/// The types a ledger records a view-holding field for.
///
/// The package-wide half of [`Emitter::borrows`]. `tethered` is non-empty for
/// exactly the structs [`borrowing_structs`] finds in the file that declares
/// them (`Ledger::infer_checked` computes it from that set), so this carries the
/// same fact across a file boundary rather than recomputing a different one.
/// The types this unit declares an `impl Error for …` for.
///
/// The trait's name and nothing more: `Error` is the one trait this compiler
/// reads by name (`NK1130`'s note says so), and Part I 7.1 makes the `impl`
/// line the marker — *what is thrown implements `Error`, and the `impl` line
/// says so*.
fn declared_errors(parsed: &Parsed) -> std::collections::BTreeSet<String> {
    nikaia_std::tools::emit_items::declared_errors(&parsed.interner, &parsed.program.items)
}

fn tethered_types(contracts: &crate::contracts::Ledger) -> std::collections::BTreeSet<String> {
    contracts
        .types
        .iter()
        .filter(|(_, contract)| !contract.tethered.is_empty())
        .map(|(name, _)| name.clone())
        .collect()
}

/// `text` with the lifetime `from` written `to`, where it stands as a whole name.
fn renamed_lifetime(text: &str, from: &str, to: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(from) {
        let after = &rest[at + from.len()..];
        out.push_str(&rest[..at]);
        match after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            true => out.push_str(from),
            false => out.push_str(to),
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// **The places of each type another package declares** (ADR-353 D7): the
/// ledger's `tethered` lists its view fields in the order written, one
/// lifetime each where there are two or more. An enum, and a type a function
/// there throws, keeps one place.
fn read_view_slots(
    ledger: &crate::contracts::Ledger,
) -> std::collections::BTreeMap<String, Vec<String>> {
    let thrown: std::collections::BTreeSet<&String> = ledger
        .functions
        .values()
        .flat_map(|f| f.fails_with.iter())
        .collect();
    ledger
        .types
        .iter()
        .filter(|(name, contract)| {
            contract.tethered.len() >= 2 && contract.variants.is_empty() && !thrown.contains(name)
        })
        .map(|(name, contract)| (name.clone(), contract.tethered.clone()))
        .collect()
}

/// **The view-holding fields of each struct that holds two or more**
/// (ADR-353 D1, D7): one lifetime each, `pub` or not, in the order written.
/// An error keeps one place.
pub(crate) fn view_slots(
    units: &[&Parsed],
    borrowing: &std::collections::BTreeSet<String>,
) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut slots = std::collections::BTreeMap::new();
    for parsed in units {
        let errors = declared_errors(parsed);
        for item in &parsed.program.items {
            if let Item::Struct {
                name,
                fields,
                is_extern: false,
                ..
            } = &item.node
            {
                let text = parsed.text(*name).to_string();
                if errors.contains(&text) {
                    continue;
                }
                let held: Vec<String> = fields
                    .iter()
                    .filter(|f| holds_view(&f.ty) || names_outside(parsed, &f.ty, borrowing))
                    .map(|f| parsed.text(f.name).to_string())
                    .collect();
                if held.len() >= 2 {
                    slots.insert(text, held);
                }
            }
        }
    }
    slots
}

/// Which structs hold a view into the parser's input.
///
/// Part II, 10.6: a view is a slice of the input, so a struct holding one is
/// tied to the input as well - transitively, which is why this is a fixpoint
/// and not one pass.
pub(crate) fn borrowing_structs(parsed: &Parsed) -> HashSet<Symbol> {
    borrowing_structs_beside(parsed, &std::collections::BTreeSet::new())
}

/// **The same, with the types another file of the package declares**
/// (Part I 9.1, #544): `outside` names the ones known to hold a view, so a
/// struct here that holds an `enum` declared beside this file holds a view
/// too. By name, because a `Symbol` belongs to the parse that interned it.
pub(crate) fn borrowing_structs_beside(
    parsed: &Parsed,
    outside: &std::collections::BTreeSet<String>,
) -> HashSet<Symbol> {
    let mut fields_of: HashMap<Symbol, Vec<&Type>> = HashMap::new();
    let mut borrowing = HashSet::new();
    let named_outside = |t: &Type| names_outside(parsed, t, outside);

    for item in &parsed.program.items {
        // An enum holds views the same way a struct does - in the types of
        // what its variants carry - so it is walked the same way.
        if let Item::Enum { name, variants, .. } = &item.node {
            let types: Vec<&Type> = variants
                .iter()
                .flat_map(|v| match &v.fields {
                    VariantFields::Unit => Vec::new(),
                    VariantFields::Tuple(types) => types.iter().collect(),
                    VariantFields::Named(fields) => fields.iter().map(|f| &f.ty).collect(),
                })
                .collect();
            if types.iter().any(|t| holds_view(t) || named_outside(t)) {
                borrowing.insert(*name);
            }
            fields_of.insert(*name, types);
        }
        if let Item::Struct { name, fields, .. } = &item.node {
            let types: Vec<&Type> = fields.iter().map(|f| &f.ty).collect();
            if types.iter().any(|t| holds_view(t) || named_outside(t)) {
                borrowing.insert(*name);
            }
            fields_of.insert(*name, types);
        }
    }

    loop {
        let mut changed = false;
        for (name, types) in &fields_of {
            if borrowing.contains(name) {
                continue;
            }
            if types.iter().any(|t| names_borrowing(t, &borrowing)) {
                borrowing.insert(*name);
                changed = true;
            }
        }
        if !changed {
            return borrowing;
        }
    }
}

/// **A function type holds what its result holds, and nothing it is
/// handed**: a kept function's parameters are borrowed for the call alone and
/// written without a lifetime ([`Emitter::ty_counted`]), so a struct whose
/// only field is `fn(ref String) -> i64` names no lifetime either - with one,
/// it was *lifetime parameter `'a` is never used*.
pub(crate) fn holds_view(ty: &Type) -> bool {
    nikaia_std::tools::views::holds_view(ty)
}

/// Every view-holding type the files of a package declare, by name: each
/// file's [`borrowing_structs_beside`] read against what the others found,
/// until nothing more is found - a struct in one file can hold an `enum` of a
/// second that holds a struct of a third.
pub(crate) fn package_borrowing(units: &[&Parsed]) -> std::collections::BTreeSet<String> {
    let mut found = std::collections::BTreeSet::new();
    loop {
        let before = found.len();
        for parsed in units {
            let more: Vec<String> = borrowing_structs_beside(parsed, &found)
                .into_iter()
                .map(|s| parsed.text(s).to_string())
                .collect();
            found.extend(more);
        }
        if found.len() == before {
            return found;
        }
    }
}

fn names_outside(parsed: &Parsed, ty: &Type, outside: &std::collections::BTreeSet<String>) -> bool {
    if outside.is_empty() {
        return false;
    }
    if let Some(code) = &*ty.code {
        return (*code.result)
            .as_ref()
            .is_some_and(|result| names_outside(parsed, result, outside));
    }
    outside.contains(parsed.text(ty.name))
        || ty
            .generics
            .iter()
            .any(|g| names_outside(parsed, g, outside))
}

pub(crate) fn names_borrowing(ty: &Type, borrowing: &HashSet<Symbol>) -> bool {
    if let Some(code) = &*ty.code {
        // A result that is one of two views handed in is the call's (#730).
        let named = ty
            .generics
            .iter()
            .filter(|g| !holds_view(g) && names_borrowing(g, borrowing))
            .count();
        if nikaia_std::tools::views::views_the_call_s(ty, named as i64) {
            return false;
        }
        return (*code.result)
            .as_ref()
            .is_some_and(|result| names_borrowing(result, borrowing));
    }
    borrowing.contains(&ty.name) || ty.generics.iter().any(|g| names_borrowing(g, borrowing))
}

/// Walk every expression in a block, including the ones inside statements.
/// The functions of this program that can pause **and take part in a cycle**
/// ([ADR-055](../../../docs/specification/adr/adr-055.md) D6's first sharp
/// edge).
///
/// A recursive `async fn` has an infinitely sized future, and `rustc` says so:
/// *"recursion in an async fn requires boxing"*. `examples/json/src/main.nika` has two of
/// them - `show` and `longest` both walk a tree - so this is not a case to meet
/// later, it is the corpus on day one.
///
/// **The graph is syntactic, and over-approximating is the safe direction.** A
/// free call names its callee outright; a method call does not name its receiver's
/// type, so an edge is drawn to *every* pausing method of that name. Boxing a
/// call that did not need it costs one allocation and nothing else, while missing
/// one is a program that does not compile - so where the two readings differ this
/// takes the wider.
///
/// **The cycle test is reachability from a function to itself**, computed to a
/// fixpoint. `n` is the number of functions in one program, and the obvious
/// algorithm is the one whose correctness a reader can check.
fn pausing_reach(
    parsed: &Parsed,
    contracts: &crate::contracts::Ledger,
) -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
    nikaia_std::tools::emit_reach::pausing_reach(
        &parsed.interner,
        &parsed.aliases,
        &parsed.program.items,
        contracts,
    )
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
    move |stmt| emitter.branch_pauses(stmt, Flow::PLAIN)
}

pub(crate) fn visit_block<'a>(block: &'a Block, f: &mut impl FnMut(&'a Expr)) {
    for expr in nikaia_std::tools::emit_walk::visited_in_block(block) {
        f(expr);
    }
}

/// Every expression inside this one, this one included.
///
/// `pub(crate)` because the checker reads it too
/// ([ADR-281](../../docs/specification/adr/adr-281.md)): `NK2205` asks whether
/// a `get` is written anywhere inside a `set`'s argument, and a second walk
/// over the same shape is a second thing to keep in step with the AST.
pub(crate) fn visit_expr<'a>(expr: &'a Expr, f: &mut impl FnMut(&'a Expr)) {
    for each in nikaia_std::tools::emit_walk::visited_in_expr(expr) {
        f(each);
    }
}

/// Split an interpolated string into a Rust format string and its holes.
///
/// `"{a}={s.mean()}"` becomes `("{}={}", ["a", "s.mean()"])`. Braces are
/// doubled to be literal, as in every format string; the holes themselves are
/// Nikaia expressions and are parsed as such by the caller.
/// Every expression a literal holds - the holes of an interpolated string, and
/// the holes of a template.
///
/// **The emitter is not the only thing that needs these.** A hole is Nikaia
/// source, and until this existed it was source no analysis could see: the
/// splitting happened here, on the way out, so the type checker, the `sync`
/// check and the provenance analysis all walked past a `LitStr` as if it were
/// a string.
///
/// That was not only a missed diagnostic. `sync` is *inferred* from what a body
/// calls (ADR-288), so a function whose only pausing call sat inside a hole was
/// recorded `sync = "inferred"` - a claim, in a file a library ships, that it
/// cannot pause. ADR-288 D2 and ADR-010 D1 both say the same thing about that
/// direction: an analysis that fails open is a vulnerability generator.
///
/// A hole that does not parse yields nothing here. The emitter reports it, in
/// its own words, at the place it happens; an analysis has nothing to add.
/// The same as [`literal_expressions`], with the names a template's `<for>`
/// binds around each hole. Empty for anything that is not a template.
pub(crate) fn literal_expressions_bound(
    parsed: &Parsed,
    expr: &Expr,
) -> Vec<(Expr, Vec<(String, String)>)> {
    match expr {
        Expr::Dsl {
            target,
            package,
            content,
            ..
        } if crate::dsl::is_deferred(&dsl_name(parsed, *package, *target), content) => Vec::new(),
        Expr::Dsl { content, .. } => match template::split(content.trim()) {
            Ok(segments) => template_holes_bound(&segments)
                .into_iter()
                .filter_map(|(hole, bound)| {
                    parsed
                        .hole(crate::ast::id_of(expr), &hole)
                        .ok()
                        .map(|expr| (expr, bound))
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        _ => literal_expressions(parsed, expr)
            .into_iter()
            .map(|hole| (hole, Vec::new()))
            .collect(),
    }
}

/// **One hole of an `f"…"` as its readers read it**: the node in the tree,
/// or a copy with what the tier pass hands over inside it (ADR-229 D1). The
/// node itself wherever nothing is wrapped, so that what the checker records
/// against a node's address is found by the emitter at the same address.
pub(crate) fn hole_as_read<'a>(
    parsed: &Parsed,
    at: u32,
    hole: &'a Expr,
) -> std::borrow::Cow<'a, Expr> {
    match parsed.hole_wraps.get(&at) {
        Some(wraps) => {
            let mut wrapped = hole.clone();
            crate::text_tiers::wrap_hole(&mut wrapped, wraps);
            std::borrow::Cow::Owned(wrapped)
        }
        None => std::borrow::Cow::Borrowed(hole),
    }
}

/// **An `f"…"`'s holes as the grammar parsed them**
/// ([ADR-309](../../docs/specification/adr/adr-309.md) D6), each beside the
/// byte its `{` stands at - the key the tier pass's wraps are recorded by
/// ([ADR-229](../../docs/specification/adr/adr-229.md) D1).
pub(crate) fn interpolated_holes(parts: &[crate::ast::FPart]) -> Vec<(u32, &Expr)> {
    parts
        .iter()
        .filter_map(|part| match part {
            crate::ast::FPart::Hole { expr, at, .. } => Some((*at, expr)),
            crate::ast::FPart::Text(_) => None,
        })
        .collect()
}

/// **An `f"…"` as the format string the language below reads**
/// ([ADR-309](../../docs/specification/adr/adr-309.md) D9): each run of text
/// as written - an escape and a doubled brace mean there what they mean here
/// - and each hole as `{}`, or `{:spec}` where a format follows its `:`.
pub(crate) fn format_of(parts: &[crate::ast::FPart]) -> String {
    nikaia_std::tools::emit_text::format_of(parts)
}

pub(crate) fn literal_expressions(parsed: &Parsed, expr: &Expr) -> Vec<Expr> {
    match expr {
        // **The grammar parsed them** ([ADR-309](../../docs/specification/adr/adr-309.md)
        // D2): the holes are the literal's parts, read and not parsed again.
        // Each with what the tier pass hands over inside it (ADR-229 D1),
        // which is keyed by the hole's text - the hole at the same place among
        // the text's holes, so nothing is parsed to find it.
        Expr::LitInterpolated { parts, .. } => interpolated_holes(parts)
            .into_iter()
            .map(|(at, hole)| {
                let mut expr = hole.clone();
                if let Some(wraps) = parsed.hole_wraps.get(&at) {
                    crate::text_tiers::wrap_hole(&mut expr, wraps);
                }
                expr
            })
            .collect(),
        // A template's holes are Nikaia too (ADR-017), and reach the emitter by
        // the same route: text, split on the way out. A deferred-parameter
        // statement has none: its braces are the foreign syntax's, and reading
        // them as holes would be this compiler speaking for a grammar it does
        // not have (ADR-296 D5).
        Expr::Dsl {
            target,
            package,
            content,
            ..
        } if crate::dsl::is_deferred(&dsl_name(parsed, *package, *target), content) => Vec::new(),
        Expr::Dsl { content, .. } => match template::split(content.trim()) {
            Ok(segments) => template_holes(&segments)
                .iter()
                .filter_map(|hole| parsed.hole(crate::ast::id_of(expr), hole).ok())
                .collect(),
            Err(_) => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Every hole in a template, including the ones inside a `<for>` body - a hole
/// does not become invisible by being repeated, any more than it becomes safe
/// by it (ADR-017 D3).
fn template_holes(segments: &[template::Segment]) -> Vec<String> {
    template_holes_bound(segments)
        .into_iter()
        .map(|(hole, _)| hole)
        .collect()
}

/// The same, with **what a `<for>` binds around each hole**.
///
/// `<for r in :rows>{r.name}</for>`: `r` is declared by the template and named
/// in the hole, so a reader of a hole that does not know that sees a name
/// nothing declares. Which is what a checker asking *"does anything declare
/// this?"* needs, and nothing else does - the emitter only needs the text
/// ([ADR-017](../../../docs/specification/adr/adr-017.md)).
///
/// Innermost last, so a nested `<for>` shadowing the same word behaves the way
/// a nested block does.
/// Each name beside the collection it walks, as written after its `:` (#524).
pub(crate) fn template_holes_bound(
    segments: &[template::Segment],
) -> Vec<(String, Vec<(String, String)>)> {
    nikaia_std::tools::emit_walk::template_holes_bound(segments)
}

// ---------------------------------------------------------------------------
// The tether: where a buffer lives, written out
// ([ADR-283](../../docs/specification/adr/adr-283.md))
// ---------------------------------------------------------------------------

/// The name of the keep a function was given.
const KEEP_PARAM: &str = "__keep";
/// A held struct's element, read through its handle (ADR-283 D17): appended to
/// what walks such a keeper.
const HELD_READ: &str = ".map(|held| held.get())";
/// The keep a function's tasks share (D3).
const KEEP_TASK: &str = "__keep_task";

impl Emitter<'_> {
    /// The keep plan of the function being written.
    fn keep_plan(&self, function: &str) -> Option<&nikaia_std::tools::buffers::BufferPlan> {
        match self.s.keep_plans.get(function) {
            Some(plan) => Some(plan),
            None => self
                .s
                .keep_plans
                .get(self.walk(|w| w.keep_function.clone()).as_str()),
        }
    }

    /// How a keep is reached where a buffer is put into it or a call is given
    /// it.
    fn keep_expr(keep: nikaia_std::tools::buffers::KeepAt, lent: bool) -> String {
        nikaia_std::tools::emit_keeps::emit_keep_expr(&keep, lent)
    }

    /// Declare the keeps a statement needs, once.
    fn write_keep_prelude(&self, out: &mut Out, function: &str, at: usize, depth: usize) {
        self.nikaia(|walk| {
            self.s
                .write_keep_prelude(walk, out, function, at as i64, depth as i64)
        })
    }

    /// The lifetimes a function that takes a keep writes its tethered
    /// positions with, where it takes one.
    fn kept_lifetimes(&self, key: &str, lifetimes: Lifetimes) -> Option<Lifetimes> {
        self.s.kept_lifetimes(key, lifetimes)
    }

    /// Whether this position of a function is one its views leave through.
    fn tethered_position(&self, key: &str, position: &str) -> bool {
        self.s.tethered_position(key, position)
    }

    /// The type a task's tethered binding holds, as the source wrote it: the
    /// `let`'s annotation, or the declared result of the function it calls.
    fn tethered_type(&self, ty: Option<&Type>, value: &Expr) -> Option<Type> {
        nikaia_std::tools::emit_keeps::emit_tethered_type(&self.world, ty, value)
    }

    /// The `Views` type one tethered binding is held by (D3, and
    /// [ADR-218](../../docs/specification/adr/adr-218.md) D4), declared once
    /// per type: the value over any lifetime, and `shorten`, which `rustc`
    /// only accepts where the type is covariant - so the shortening is checked
    /// rather than trusted. The binding is a `Holding` over the task's keep,
    /// and nothing about it is `unsafe`.
    fn tether_wrapper(&self, ty: &Type) -> String {
        self.nikaia(|walk| {
            self.s.emit_core_tether_wrapper(
                &self.world,
                walk,
                self.build.crossings_are_possible(),
                self.build.overlaps_user_code(),
                ty,
            )
        })
    }

    /// The handle types, at the end of the unit.
    fn tether_wrappers(&self, out: &mut Out) {
        for wrapper in self.walk(|w| w.wrappers.clone()) {
            out.push("\n");
            out.push(&wrapper);
        }
        self.held_structs(out);
    }

    /// `kept[i]`, where `kept` holds structs of views one handle per buffer:
    /// the keeper and the index, for a write through the handle (ADR-283 D17).
    fn held_element<'e>(&self, flow: Flow<'_>, element: &'e Expr) -> Option<(&'e Expr, &'e Expr)> {
        let Expr::Index { base, index, .. } = element else {
            return None;
        };
        self.reads_through_handle(flow, base)
            .then_some((base.as_ref(), index.as_ref()))
    }

    /// Whether the field of a held struct being written holds a view, which
    /// is then found again in the element's keeps before it is stored.
    fn held_field_carries_view(&self, flow: Flow<'_>, keeper: &Expr, field: Symbol) -> bool {
        self.walk(|w| {
            self.s
                .held_field_carries_view(&self.world, w, flow.function, keeper, field)
        })
    }

    /// Whether `expr` names a keeper whose elements are structs of views held
    /// one handle per buffer, so that an element is read through the handle
    /// ([ADR-283](../../docs/specification/adr/adr-283.md) D17).
    fn reads_through_handle(&self, flow: Flow<'_>, expr: &Expr) -> bool {
        self.walk(|w| {
            self.s
                .reads_through_handle(&self.world, w, flow.function, expr)
        })
    }

    /// **What `tether` needs to hold a struct of views**, once per struct, at
    /// the end of a unit that holds any
    /// ([ADR-283](../../docs/specification/adr/adr-283.md) D14, D16): the struct
    /// over any lifetime (`Views`, whose `shorten` compiles only where the
    /// struct is covariant), each of its views found again in the keeps
    /// (`Rebase`, one call per field that holds a view), and the value put
    /// into a `Holding` (`Hold`). Written by the compiler, and none of it
    /// `unsafe`.
    fn held_structs(&self, out: &mut Out) {
        out.push(&self.s.held_structs(&self.world));
    }
}

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

/// What a `dsl` block names: `pkg::Grammar` for a package's, the bare name
/// for one the program declares and for `html`
/// ([ADR-299](../../docs/specification/adr/adr-299.md) D20).
pub(crate) fn dsl_name(parsed: &Parsed, package: Option<Symbol>, target: Symbol) -> String {
    nikaia_std::tools::emit_items::dsl_name(&parsed.interner, package, target)
}

/// **Rust's own tuple pattern for a head** (ADR-291, ADR-356): one name as it
/// is, a flat tuple in parentheses, and a nested one rebuilt from its shape -
/// a tuple's width, `0` for a name, in prefix order.
fn tuple_pattern(names: &[String], shape: &[i64]) -> String {
    fn one(names: &[String], shape: &[i64], at: &mut usize, next: &mut usize) -> String {
        let width = shape.get(*at).copied().unwrap_or(0);
        *at += 1;
        if width <= 0 {
            let name = names.get(*next).cloned().unwrap_or_else(|| "_".to_string());
            *next += 1;
            return name;
        }
        let parts: Vec<String> = (0..width).map(|_| one(names, shape, at, next)).collect();
        format!("({})", parts.join(", "))
    }
    match (names, shape.first()) {
        ([name], None) => name.clone(),
        (_, None) | (_, Some(-1)) => format!("({})", names.join(", ")),
        _ => one(names, shape, &mut 0, &mut 0),
    }
}

/// The `par_fold` a rule's body is, written bare or bound to a name.
fn a_par_fold(rule: &GrammarRule) -> Option<&FoldSpec> {
    rule.alts.iter().find_map(|alt| {
        let pattern = match &alt.pattern.node {
            Pattern::Bind { pat, .. } => &pat.node,
            other => other,
        };
        match pattern {
            Pattern::Fold(spec) if spec.parallel => Some(spec),
            _ => None,
        }
    })
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

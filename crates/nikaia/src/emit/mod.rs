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
mod rust_targets;
pub(crate) mod seeds;
pub mod template;
use winnow_grammar::Symbol;

use crate::assets::Reads;
use crate::ast::{Block, Expr, GrammarDef, Item, Pattern, Span, Spanned, Stmt, Type};
use crate::parser::Parsed;
use crate::refused;

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

impl Build {
    pub fn parse(target: &str, user_parallelism: &str, reentrancy_check: &str) -> Result<Build> {
        Ok(Build {
            target: Target::parse(target)?,
            user_parallelism: UserParallelism::parse(user_parallelism)?,
            reentrancy_check: ReentrancyCheck::parse(reentrancy_check)?,
            bounds: crate::bounds::BoundsChecks::default(),
            overflow: crate::bounds::OverflowChecks::default(),
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
    /// **What a give-up lowers to** (ADR-353 D3, `view_changes`): an
    /// assignment that becomes a binding of its own, and the `let`s that need
    /// no `mut` because of it.
    give_ups: crate::view_changes::GiveUps,
    /// Where the parameters of the type being written come from.
    place: std::cell::RefCell<nikaia_std::tools::emit_seeds::EmitSeedsPlace>,
    /// A signature of a barrier is being written: its maps are the keyed ones.
    floor: std::cell::Cell<bool>,
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

/// What an `impl` says about a method. Enough to adapt a `&mut self` method to
/// the monoid a `par_fold` needs, and never more than the declaration states.
use nikaia_std::tools::emit_kinds::EmitMethod as Method;

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

/// The generated name of [ADR-300](../../../docs/specification/adr/adr-300.md)
/// D1's table. `__nikaia_` for the same reason `__nikaia_main` is: a name a
/// program could also have chosen would be a name this compiler took from it.
const ABORT_TABLE: &str = "__NIKAIA_SITES";

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
        let published_contracts =
            crate::prove::with_published(&own_contracts, &propagation.published);
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
                simd_loads: &propagation.simd_loads,
                comptime_values: &propagation.comptime_values,
            },
            &[&published_contracts, std_ledger()],
            &std::collections::BTreeSet::new(),
            build.prove_through_loops,
        );

        // **Which checks a statement holds and which are proved**, whatever
        // the build writes of them: the order of a block's statements reads
        // it (ADR-379).
        let order_proofs = nikaia_std::tools::emit_order::EmitOrderProofs {
            indices: propagation.list_indices.iter().copied().collect(),
            proved_indices: proven.indices.iter().copied().collect(),
            operations: propagation
                .arithmetic
                .keys()
                .chain(propagation.shifts.keys())
                .copied()
                .collect(),
            proved_operations: proven.arithmetic.iter().copied().collect(),
            negations: propagation.negations.keys().copied().collect(),
            proved_negations: proven.negations.iter().copied().collect(),
        };

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
            unchecked_assumes: build.bounds == crate::bounds::BoundsChecks::Removed
                || build.overflow == crate::bounds::OverflowChecks::Removed,
            // **The arithmetic of a believed `assume` is exact** (ADR-376 D2).
            believed_arithmetic: match build.bounds == crate::bounds::BoundsChecks::Removed
                || build.overflow == crate::bounds::OverflowChecks::Removed
            {
                true => propagation.arithmetic.clone(),
                false => Default::default(),
            },
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
            // **A method call whose precondition is proved where it stands**
            // calls the entry that does not check it (ADR-269 D20, #776), as
            // a call the prover proved does, whatever the build removes.
            proven_calls: proven.calls.clone(),
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
            place: std::cell::RefCell::new(nikaia_std::tools::emit_seeds::emit_seeds_place()),
            floor: std::cell::Cell::new(false),
            nonnegative_bodies: std::mem::take(&mut proven.nonnegative_locals),
            main_params: std::cell::RefCell::new(Vec::new()),
            order_proofs,
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
        if whole && self.user_main().is_some() {
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
}

// --- Free helpers ---

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
    units
        .iter()
        .flat_map(|parsed| {
            nikaia_std::tools::emit_tables::emit_tables_view_slots(
                &parsed.interner,
                &parsed.program.items,
                borrowing,
            )
        })
        .collect()
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
    nikaia_std::tools::emit_tables::emit_tables_borrowing_structs_beside(
        &parsed.interner,
        &parsed.program.items,
        outside,
    )
    .into_iter()
    .collect()
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
    move |stmt| emitter.branch_pauses(stmt)
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

/// What a `dsl` block names: `pkg::Grammar` for a package's, the bare name
/// for one the program declares and for `html`
/// ([ADR-299](../../docs/specification/adr/adr-299.md) D20).
pub(crate) fn dsl_name(parsed: &Parsed, package: Option<Symbol>, target: Symbol) -> String {
    nikaia_std::tools::emit_items::dsl_name(&parsed.interner, package, target)
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

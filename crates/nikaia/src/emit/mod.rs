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
pub mod template;
use winnow_grammar::Symbol;

use crate::assets::Reads;
use crate::ast::{
    BinaryOp, Block, Expr, FoldSpec, FrameAttr, GrammarDef, GrammarRule, Item, Pattern, Repeat,
    SelectArm, Span, Spanned, Stmt, Type, UnaryOp, VariantFields,
};
use crate::check::PausingEntry;
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

    pub fn parse(name: &str) -> Result<Target> {
        match Target::ALL.into_iter().find(|target| target.name() == name) {
            Some(target) => Ok(target),
            None => Err(refused!(
                "There's no target called `{name}`. Use x86_64-linux, aarch64-linux or \
                 wasm32-unknown."
            )),
        }
    }

    /// The word a manifest and `--target` use for it.
    pub fn name(self) -> &'static str {
        match self {
            Target::X86_64Linux => "x86_64-linux",
            Target::Aarch64Linux => "aarch64-linux",
            Target::Wasm32Unknown => "wasm32-unknown",
        }
    }

    /// The triple handed to the backend: Cargo's `--target`.
    pub fn triple(self) -> &'static str {
        match self {
            Target::X86_64Linux => "x86_64-unknown-linux-gnu",
            Target::Aarch64Linux => "aarch64-unknown-linux-gnu",
            Target::Wasm32Unknown => "wasm32-unknown-unknown",
        }
    }

    /// The machine this compiler is running on, where it is one of the
    /// targets. The compiler is a native program, so the machine it was built
    /// for is the machine it runs on.
    ///
    /// `None` on any other (macOS, a 32-bit ARM board, RISC-V): there is no
    /// name for it, and a build there has to name one of the targets and have
    /// the toolchain for it.
    pub fn host() -> Option<Target> {
        if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
            Some(Target::X86_64Linux)
        } else if cfg!(all(target_arch = "aarch64", target_os = "linux")) {
            Some(Target::Aarch64Linux)
        } else {
            None
        }
    }

    /// Whether this machine has threads at all. A target without them bounds
    /// `user_parallelism` to `0` however it is set.
    pub fn has_threads(self) -> bool {
        matches!(self, Target::X86_64Linux | Target::Aarch64Linux)
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
        matches!(self, Target::X86_64Linux | Target::Aarch64Linux)
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
            Target::X86_64Linux | Target::Aarch64Linux => None,
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

/// The name a `?.m()`'s reached value is bound to inside the `match`.
///
/// Prefixed like every other name this emitter invents, so it can never be one
/// a program wrote: `__nikaia_` is reserved by the same rule that reserves the
/// branch names an `overlap` uses.
const REACHED: &str = "__nikaia_it";

/// Kap 7.1: the name a `catch` handler sees the failure under.
///
/// One place, because the emitter writes it and
/// [`contracts::order::block_mentions`](crate::contracts::order::block_mentions)
/// is asked about it — and a second spelling of one name is a binding that goes
/// unread while the walk looks for the other
/// ([ADR-308](../../docs/specification/adr/adr-308.md)).
const CAUGHT: &str = "error";

/// Where the local holding a caught error's **site** is kept
/// ([ADR-280](../../docs/specification/adr/adr-280.md) D10).
///
/// A named channel's envelope is opened at the handler's binding, so the error
/// the source names is the author's own value — and `throw error` needs the
/// site back to pass the error on with the place it was **raised** rather than
/// the place it was caught ([ADR-023](../../docs/specification/adr/adr-023.md)
/// D6). Spelled so that no Nikaia name collides with it.
const SITE: &str = "__nikaia_site";

/// What `std` has no entry for yet: `String::pop` and `String::insert_str`.
trait OutText {
    fn take_lend(&mut self) -> bool;
    fn insert_after_first_brace(&mut self, text: &str);
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
    let trust = crate::contracts::trust::analyse(parsed, &library);
    Emitter::with_contracts(
        parsed,
        beside,
        build,
        trust.provenance,
        own.clone(),
        described,
        &Reads::none(),
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
    emit_std_items_against_at(parsed, beside, own, described, build)
}

/// The same at a given build, for the test that holds the package to lowering
/// the same at both settings of `user_parallelism` (ADR-002 D4).
pub fn emit_std_items_against_at(
    parsed: &Parsed,
    beside: &[&Parsed],
    own: &crate::contracts::Ledger,
    described: &crate::contracts::Ledger,
    build: Build,
) -> Result<Lowered> {
    let mut library = std_ledger().clone();
    library.types.extend(described.types.clone());
    library.functions.extend(described.functions.clone());
    let trust = crate::contracts::trust::analyse(parsed, &library);
    Emitter::with_contracts(
        parsed,
        beside,
        build,
        trust.provenance,
        own.clone(),
        described,
        &Reads::none(),
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
) -> Result<Lowered> {
    Emitter::with_contracts(
        parsed,
        beside,
        build,
        provenance,
        contracts.clone(),
        described,
        reads,
    )
    .for_entry(entry)
    .items_only()
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
    pub fn of(parsed: &Parsed, build: Build) -> Needs {
        // What a program needs does not depend on which checks were proved,
        // and proving them is the slowest thing an emitter is built with.
        let build = Build {
            bounds: crate::bounds::BoundsChecks::Kept,
            overflow: crate::bounds::OverflowChecks::Kept,
            leaves_values: true,
            ..build
        };
        let emitter = Emitter::new(parsed, build, crate::contracts::Provenance::Trusted);
        Needs {
            grammar: parsed
                .program
                .items
                .iter()
                .any(|i| matches!(i.node, Item::Grammar(_))),
            driver: emitter.uses_driver(),
            std: emitter.s.uses_std,
            fails: emitter.s.fails,
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
    world: EmitWorld<'p>,
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

const SHARED: &str = "Shared";
/// Part I 6.3's lock, whose shape is decided per value
/// ([ADR-281](../../../docs/specification/adr/adr-281.md)).
const LOCKED: &str = "Locked";

/// The shared mutable type, which is **one name here and two hulls below**
/// ([ADR-281](../../../docs/specification/adr/adr-281.md) D1).
///
/// It is a name the checker carries and only this module expands, which is the
/// whole of D1: a name that resolved away early would put a spelling nobody wrote
/// into every message about it ([ADR-281](../../../docs/specification/adr/adr-281.md)
/// D9 named that as what one name buys).
const SHARED_MUT: &str = "SharedMut";

/// Whose the one error type in a function's set is
/// ([ADR-280](../../docs/specification/adr/adr-280.md) D9, D13).
///
/// The difference decides the **envelope**. A type this unit declares is one
/// the program `throw`s, so the failure has a site and travels in a
/// `Thrown[E]`. A type a **library** declares arrives from a call, with no
/// `throw` in this program to have a site — so it travels as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Named<'a> {
    Own(&'a str),
    Library(&'a str),
}

/// A member's variant in the sum.
pub(crate) fn sum_variant(member: &str) -> String {
    nikaia_std::tools::emit_items::sum_variant(member)
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
}

impl<'a> Flow<'a> {
    const PLAIN: Flow<'static> = Flow {
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
    };

    /// The same surroundings, for the body of a loop.
    fn inside_a_loop(self) -> Self {
        Flow {
            in_loop: true,
            ..self
        }
    }

    /// The same surroundings, for the expression a `catch` guards.
    fn guarded(self) -> Self {
        Flow {
            caught: true,
            ..self
        }
    }

    /// The guarded half of a `catch`, where that half is a block that **joins**
    /// ([ADR-292](../../../docs/specification/adr/adr-292.md) D18).
    ///
    /// The `match` is written around this expression, so its outcome has to be
    /// the `Result` the handler takes apart rather than the value a
    /// propagating block hands on.
    fn joined_here(self) -> Self {
        Flow {
            caught: true,
            handled_here: true,
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

    /// The surroundings a `catch`'s **handler** is emitted in
    /// ([ADR-280](../../../docs/specification/adr/adr-280.md) D10).
    ///
    /// The handler runs outside the guard — a failure raised *there* leaves
    /// the function like any other — so this is the flow the `catch` was
    /// written in, plus what the handler alone knows: whether the error it was
    /// handed came through a named channel.
    fn handling(self, named: bool) -> Self {
        Flow {
            caught_named: named,
            ..self
        }
    }

    /// The same, for a handler whose error travels in a **sum**
    /// ([ADR-280](../../../docs/specification/adr/adr-280.md) D17).
    ///
    /// A shorter lifetime than the flow it came from, and that is the point:
    /// the sum's name is the emitter's and outlives the handler being written,
    /// which is all this needs.
    fn catching<'b>(self, sum: Option<&'b str>) -> Flow<'b>
    where
        'a: 'b,
    {
        Flow {
            caught_sum: sum,
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

    /// The same surroundings, inside a constant written in the wider type.
    fn widened(self) -> Self {
        Flow {
            widen: true,
            ..self
        }
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
        )
    }

    /// The same, against contracts that already exist - a program's rather than
    /// a file's.
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
    ) -> Self {
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
        let propagation = match build.leaves_values {
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
        let proven = crate::bounds::proven(
            parsed,
            &crate::bounds::Sites {
                lengths: &propagation.std_lengths,
                sized: &propagation.sized_lengths,
                char_codes: &propagation.char_codes,
                arithmetic: &propagation.arithmetic,
                negations: &propagation.negations,
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

        let state = EmitState {
            described: described.clone(),
            borrowing: borrowing_structs_beside(
                parsed,
                &package_borrowing(
                    &std::iter::once(parsed)
                        .chain(beside.iter().copied())
                        .collect::<Vec<_>>(),
                ),
            )
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
            keep_plans: crate::contracts::keep::buffer_plans(parsed, &own_contracts, library)
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
            trusted_lets: match provenance {
                crate::contracts::Provenance::Trusted => std::collections::BTreeSet::new(),
                crate::contracts::Provenance::Untrusted => {
                    crate::contracts::trust::trusted_maps(parsed, library)
                }
            }
            .into_iter()
            .map(|at| at as i64)
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
            proven_arithmetic: propagation
                .arithmetic
                .iter()
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
        let mut made = Self {
            parsed,
            build,
            s: state,
            w: std::cell::RefCell::new(Some(EmitWalk::fresh())),
            failed: std::cell::RefCell::new(None),
            world: EmitWorld {
                words: &parsed.interner,
                aliases: &parsed.aliases,
                items: &parsed.program.items,
                library,
            },
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
    fn name(&self, sym: Symbol) -> std::borrow::Cow<'_, str> {
        escaped(self.text(sym))
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

    /// **The field this unrolled turn stands at**, where `base.member` is a
    /// read of the loop's own binding ([ADR-304](../../docs/specification/adr/adr-304.md)
    /// D2).
    ///
    /// `None` everywhere else, which is every program that does not walk a
    /// shape: the binding's name has to match the one the `for` wrote and the
    /// member has to be the one asked for, so an ordinary `x.name` on a struct
    /// called `field` is untouched.
    fn reflected(&self, base: &Expr, member: Symbol, wanted: &str) -> Option<String> {
        self.walk(|w| w.reflected(&self.world, base, member, wanted))
    }

    /// The type parameter a shape walk stands on — the `T` of
    /// `fn describe[T: Struct](value: T)`.
    ///
    /// The **first** one, which is what the checker bound: a second parameter
    /// beside it is an ordinary generic and keeps its place in the signature.
    fn walked_parameter(&self, item: &Item) -> String {
        self.s.walked_parameter(&self.world, item)
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
                    let written = self.item(&mut out, &item.node, &item.span);
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
                    let written = self.item(&mut out, &item.node, &item.span);
                    self.walking(|w| w.specialising = None);
                    written?;
                    out.record(&item.span, st);
                    out.push("\n");
                }
                continue;
            }
            {
                let st = out.mark();
                self.item(&mut out, &item.node, &item.span)?;
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
                    let written = self.item(&mut out, &item.node, &item.span);
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
                    let written = self.item(&mut out, &item.node, &item.span);
                    self.walking(|w| w.specialising = None);
                    written?;
                    out.record(&item.span, st);
                    out.push("\n");
                }
                continue;
            }
            {
                let st = out.mark();
                self.item(&mut out, &item.node, &item.span)?;
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
        out.push(&self.s.entry_point(&self.world, concurrent, &channel));
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

    fn item(&self, out: &mut Out, item: &Item, span: &Span) -> Result<()> {
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
            } => {
                out.push(&self.derives(*name));
                // **C's layout** (ADR-284 D14), and every field `pub` (D15).
                if *is_extern {
                    out.push("#[repr(C)]\n");
                }
                let vis = if *is_public { "pub " } else { "" };
                // The input lifetime first and the type parameters after it,
                // which is the order Rust wants them in (ADR-295 D3).
                let mut parts: Vec<String> = Vec::new();
                if self.s.borrowing.contains(self.text(*name)) {
                    parts.push(INPUT_LIFETIME.to_string());
                }
                parts.extend(generics.iter().map(|g| self.bounded(g)));
                let params = angled(&parts);
                out.push(&format!("{vis}struct {}{params} {{\n", self.name(*name)));
                for field in fields {
                    // Public, because the actions that build this struct are
                    // generated into the grammar's own module.
                    let slot = format!("{}.{}", self.text(*name), self.text(field.name));
                    let lowered = self.ty_counted(
                        &field.ty,
                        Lifetimes::Named,
                        self.count_at(SHARED_FIELDS, &slot),
                    );
                    out.push(&format!(
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
                out.push("}\n");
                Ok(())
            }
            Item::Fn { name, .. } => {
                if name.is_some_and(|name| self.s.built_at_build_time.contains(self.text(name))) {
                    out.push("#[allow(dead_code)]\n");
                }
                self.function(
                    out,
                    item,
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
                    target_parts.push(INPUT_LIFETIME.to_string());
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
                                &method.node,
                                1,
                                lifetimes,
                                carries,
                                Some(MethodOf {
                                    target: &target_name,
                                    declared_by: None,
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
                let head = match trait_name {
                    Some(t) => format!(
                        "impl{declares} {} for {target_name}{applied}",
                        self.text(*t)
                    ),
                    None => format!("impl{declares} {target_name}{applied}"),
                };
                out.push(&format!("{head} {{\n"));
                for method in methods {
                    let carries = self.s.carries_input.get(&(method.span.at() as i64));
                    {
                        let st = out.mark();
                        out.push("    ");
                        self.function(
                            out,
                            &method.node,
                            1,
                            lifetimes,
                            carries,
                            Some(MethodOf {
                                target: &target_name,
                                declared_by: trait_name.map(|t| self.text(t)),
                            }),
                        )?;
                        out.record(&method.span, st);
                    }
                }
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

    /// **The hull a call's result needs**, where the callee is a foreign
    /// declaration handing back a handle
    /// ([ADR-302](../../docs/specification/adr/adr-302.md) D11, D12): the
    /// handle's name, and whether the declaration said it **may be absent**.
    fn handle_from(&self, func: &Expr) -> Option<(String, bool)> {
        let handle = self.s.handle_from(&self.world, func)?;
        Some((handle, self.s.handle_from_may_be_null(&self.world, func)))
    }

    /// The callee as the source wrote it, for a message that names it.
    fn called_name(&self, func: &Expr) -> String {
        self.s.called_name(&self.world, func)
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
            params.push(format!(
                "{}: {reference}{}",
                self.name(arg.name),
                match foreign {
                    true => self.foreign_ty(&arg.ty),
                    false => self.ty(&arg.ty, Lifetimes::Elided),
                }
            ));
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
                    &method.node,
                    1,
                    lifetimes,
                    carries,
                    Some(MethodOf {
                        target,
                        declared_by: None,
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
        item: &Item,
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
        } = item
        else {
            return Err(anyhow!("not a function"));
        };

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
        let how = |name: Symbol| {
            if let Some(kept) = kept.filter(|_| self.tethered_position(&key, self.text(name))) {
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
        // **A parameter the body does not keep is a view**
        // ([ADR-094](../../docs/specification/adr/adr-094.md) D2). The `&` is
        // written here, in the declaration, and at the call — one answer read
        // twice (`contracts::keeps::lends`), because the two disagreeing is a
        // `&&T` or a moved value in the language below.
        let lent = self.s.own_contracts.functions.get(&key);
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
        // **Two or more of them and a view handed back** (D24): they and the
        // result share one named lifetime, `'p`.
        let shared = lifetimes == Lifetimes::Elided
            && receiver.is_none()
            && kept.is_none()
            && inner_views.is_none()
            && carries_input.is_none_or(|set| set.is_empty())
            && ret_type
                .as_ref()
                .is_some_and(|ty| self.text(ty.name) != SELF_DSL && self.carries_a_view(ty))
            && args
                .iter()
                .enumerate()
                .filter(|(at, a)| lowered_as_a_view(*at, a))
                .count()
                > 1;
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
                match shared {
                    true => "&'p mut ",
                    false => "&mut ",
                }
            } else if lent.is_some_and(|c| {
                crate::contracts::keeps::lends_in_own(
                    c,
                    at,
                    &[self.world.library, &self.s.described],
                )
            }) && !written_as_a_view
            {
                match shared {
                    true => "&'p ",
                    false => "&",
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
                    let position = match shared {
                        true => Lifetimes::Shared,
                        false => how(a.name),
                    };
                    let written = self.ty_counted(&a.ty, position, self.count_at(&key, name));
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
            let written = match (reference, plain_text, plain_list) {
                ("&" | "&'p ", true, _) => "str".to_string(),
                ("&" | "&'p ", _, true) if written.starts_with("Vec<") => {
                    format!("[{}]", &written[4..written.len() - 1])
                }
                ("", _, true) if written.starts_with("&Vec<") => {
                    format!("&[{}]", &written[5..written.len() - 1])
                }
                _ => written,
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
                let name = escaped(name);
                either_first.push(format!("let {name} = {name}.{into}();"));
                return format!(
                    "{name}: impl nikaia_std::either_text::{by}<Either = \
                     nikaia_std::either_text::EitherText<'e>>"
                );
            }
            format!("{}: {reference}{written}", escaped(name))
        }));
        // Kap 5.1: the language below has neither named arguments nor defaults,
        // so an option becomes an ordinary parameter here - in declaration
        // order, which is the order every call site fills in. The names stay
        // the source's, so a `rustc` diagnostic about one still lands on the
        // parameter the programmer wrote (ADR-300).
        params.extend(config.iter().map(|c| {
            let name = self.text(c.name);
            format!(
                "{}: {}",
                escaped(name),
                self.ty_counted(&c.ty, how(c.name), self.count_at(&key, name))
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
            && (carries_input.is_some_and(|set| !set.is_empty()) || inner_views.is_some())
        {
            true => std::iter::once("'a".to_string()).chain(declared).collect(),
            false => declared,
        };
        let declared: Vec<String> = match shared {
            true => std::iter::once("'p".to_string()).chain(declared).collect(),
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
                    && self.carries_a_view(ty)
                    && !borrows_from_something;
                let result = match (widens, kept) {
                    _ if shared => Lifetimes::Shared,
                    (_, Some(kept))
                        if self.tethered_position(&key, crate::contracts::tether::RESULT) =>
                    {
                        kept
                    }
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
                self.ty_counted(ty, result, self.count_at(&key, SHARED_RESULT))
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
        let ret = if *throws {
            format!(" -> Result<{returned}, {channel}>")
        } else if ret_type.is_some() {
            format!(" -> {returned}")
        } else {
            String::new()
        };

        let vis = if *is_public { "pub " } else { "" };
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
        if self.s.fold_callees.contains(&name) {
            out.push("#[inline]\n");
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
                Some(format!(
                    "{pad}#[allow(dead_code)]\n{pad}{}{{ {}{}{UNCHECKED}({}){} }}\n",
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
        let locals: std::collections::BTreeSet<String> =
            nikaia_std::tools::emit_locals::nonnegative_locals(self.world.words, body)
                .into_iter()
                .map(|s| self.text(s).to_string())
                .collect();
        self.walking(|w| w.nonnegative.insert(key.to_string(), locals));
        let flow = Flow {
            changed: &[],
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
        if !throws {
            // The function's own settle point stands around the body, so the
            // body is not settled a second time as a block (ADR-297 D2).
            let settled = self.s.facts.settle_fns.contains_key(key);
            let body_block = |out: &mut Out| match settled {
                true => self.block_plain(out, body, depth, flow, tail, None),
                false => self.block(out, body, depth, flow, tail),
            };
            if first.is_empty() {
                return body_block(out);
            }
            // Inside the body's own brace, so there is one block and not two.
            let mut written = Out::empty();
            body_block(&mut written)?;
            written.insert_after_first_brace(&format!(" {}", first.join(" ")));
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
        // **Not after a statement that never comes back** (0.0.240): a body
        // that ends in `panic(…)`, `throw` or `return` has left before the
        // `Ok(())`, and `rustc` said *unreachable expression* about a file
        // nobody wrote ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
        let leaves = body.stmts.last().is_some_and(|stmt| match &stmt.node {
            Stmt::Return(_) => true,
            Stmt::Expr(expr) => jumps(expr) || self.never_returns(expr),
            _ => false,
        });
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

    fn grammar(&self, out: &mut Out, def: &GrammarDef) -> Result<()> {
        out.push("grammar! {\n");
        out.push(&format!("    grammar {} {{\n", self.text(def.name)));

        let types = def
            .rules
            .iter()
            .filter_map(|rule| {
                let ty = rule.ret_type.as_ref()?;
                Some((
                    self.text(rule.name).to_string(),
                    self.ty(ty, Lifetimes::Named),
                ))
            })
            .collect();
        self.walking(|w| w.grammar_rule_types = types);
        for rule in &def.rules {
            out.push("\n");
            {
                let st = out.mark();
                self.grammar_rule(out, rule)?;
                out.record(&rule.span, st);
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
            Some(ty) => format!(" -> {}", self.ty(ty, Lifetimes::Named)),
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
                    // A grammar action is the rule's own body: what it ends
                    // in is what the rule hands back, so a `return` there is
                    // that value (ADR-296).
                    self.block(out, action, 3, Flow::PLAIN, Tail::Return)?;
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
            } => {
                out.push(self.text(*name));
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
    fn fold_step(&self, out: &mut Out, step: &Expr) -> Result<()> {
        if let Expr::Closure { params, body, .. } = step
            && let [accumulator, item] = params.as_slice()
            && let Some(Stmt::Expr(Expr::MethodCall {
                receiver, method, ..
            })) = body.stmts.last().map(|s| &s.node)
        {
            let on_accumulator =
                matches!(&**receiver, Expr::Variable(name, _) if name == accumulator);
            let mutates = self
                .s
                .by_name
                .get(self.text(*method))
                .is_some_and(Method::mutates_in_place);

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

        self.expr(out, step, 0, Flow::PLAIN)
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
            let owner = self.ty(
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
                },
                Lifetimes::Named,
            );
            out.push(&format!(
                "|mut a: {owner}, b| {{ a.{}({by_view}b); a }}",
                self.text(*name)
            ));
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
        self.s
            .path(&self.world, self.walk(|w| w.trusting), &segments)
    }

    /// **One side of a text `+`**, where a side read out of a list is handed
    /// over as a view (0.0.234).
    ///
    /// A read at a number is `*index::get(&w, 0)`, a place, which is right
    /// wherever the value is only looked at; handed to `concat::plus` by value
    /// it is a **move** out of the list, and `w[0] + w[1]` over a
    /// `Vec[String]` was `rustc`'s *cannot move out of a shared reference*
    /// about a file nobody wrote. A concatenation makes new text and needs
    /// only to read its sides, so the side is lent as `&str` - which is
    /// `AsRef` whether the list holds owned text or views of it.
    fn concatenated(&self, out: &mut Out, side: &Expr, depth: usize, flow: Flow<'_>) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_concatenated(
                &self.world,
                walk,
                out,
                self,
                side,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// Whether `owner::variant` is a variant of an enum this file declares.
    fn is_variant(&self, owner: &str, variant: &str) -> bool {
        self.s.is_variant(&self.world, owner, variant)
    }

    /// The count the analysis gave the `Shared` written at one position, with the
    /// atomic floor where it has no answer (ADR-037 D6).
    fn count_at(&self, function: &str, value: &str) -> crate::contracts::sharing::Count {
        self.s
            .count_at(self.build.crossings_are_possible(), function, value)
    }

    /// A type, with the atomic floor for any `Shared` inside it.
    ///
    /// Every position whose count is actually known names it with
    /// [`Emitter::ty_counted`] instead. The floor is what a position nothing
    /// decided gets, which is ADR-037 D6's answer and never a wrong one - only a
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
        self.nikaia(|walk| {
            self.s.emit_core_ty_counted(
                &self.world,
                walk,
                self.build.crossings_are_possible(),
                self.build.overlaps_user_code(),
                ty,
                lifetimes,
                count,
            )
        })
    }

    /// The queues a settle point drains: at `user_parallelism = yes` only the
    /// one whose work may cross threads, so the settle point may too
    /// (ADR-297 D2).
    fn settle_kind(&self) -> String {
        nikaia_std::tools::emit_core_types::emit_settle_kind(self.build.overlaps_user_code())
    }

    /// The wrapper a value with a cleanup is held in: the one whose parked work
    /// may move with its task where tasks run on threads (ADR-297 D1).
    fn cleaned(&self) -> String {
        nikaia_std::tools::emit_core_types::emit_cleaned(self.build.overlaps_user_code())
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

    /// `i < xs.len()` for a counter `i` that is never negative, written as
    /// `(i as usize) < xs.len()`; `None` where the comparison is not that.
    fn against_a_length(
        &self,
        out: &mut Out,
        op: &BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<Option<()>> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_against_a_length(
                &self.world,
                walk,
                out,
                self,
                *op,
                lhs,
                rhs,
                depth as i64,
                flow,
                &scope,
            )
        });
        Ok(self.crossed(crossed)?.then_some(()))
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

    /// A statement, and the map a `let` of [`Emitter::trusted_lets`] makes
    /// under its fast hash (#431).
    fn stmt(
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
        let written = self.a_stmt(out, stmt, span, depth, tail, flow);
        self.walking(|w| w.trusting = outer);
        written
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
                mutable,
                ty,
                value,
            } => {
                let mutable = if *mutable { "mut " } else { "" };
                // **A tuple of names is Rust's own tuple pattern**
                // ([ADR-291](../../docs/specification/adr/adr-291.md)): the
                // language below takes a tuple apart by position exactly as
                // this one does, so there is nothing to translate. No count is
                // looked up and no annotation is written, because a hull's
                // count is decided per **value** and a destructure names none -
                // which is the same answer `contracts::sharing` gives, and the
                // two have to agree.
                if let [_, _, ..] = names.as_slice() {
                    let bound: Vec<String> = names
                        .iter()
                        .map(|n| escaped(self.text(*n)).into_owned())
                        .collect();
                    out.push(&format!("let {mutable}({}) = ", bound.join(", ")));
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
                            let name = escaped(self.text(*name));
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
                        escaped(bound)
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
                        escaped(bound),
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
                out.push(&format!("let {mutable}{}{annotation} = ", escaped(bound)));
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
            // (ADR-306 D10).
            Stmt::Assign {
                target: target @ Expr::Variable(_, _),
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
                let names = bindings
                    .iter()
                    .map(|b| self.name(*b))
                    .collect::<Vec<_>>()
                    .join(", ");
                let bound = match bindings.len() > 1 {
                    true => format!("({names})"),
                    false => names.clone(),
                };
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
                if bindings.len() > 1 {
                    out.push(&format!("for ({names}) in "));
                } else {
                    out.push(&format!("for {names} in "));
                }
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
                        let name = self.name(*b);
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
                                let name = self.name(*b);
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
                let either = tail == Tail::Return
                    && self.returns_either(flow.function)
                    && !matches!(expr, Expr::LitNull(_));
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

    fn around(how: Option<&crate::check::Wrap>) -> (String, String) {
        nikaia_std::tools::emit_exprs::emit_around(how)
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

    /// Whether a struct's field is text both kinds of which flow into
    /// ([ADR-282](../../docs/specification/adr/adr-282.md) D14).
    fn either_field(&self, owner: &str, field: Symbol) -> bool {
        self.s.either_field(&self.world, owner, field)
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

    /// Whether `place` is a `mut` parameter of the function being emitted, or
    /// a field of one: a value that stays the caller's (ADR-094 D3).
    fn stays_the_callers(&self, place: &Expr, flow: Flow<'_>) -> bool {
        self.s.stays_the_callers(&self.world, flow.function, place)
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

    /// [`Emitter::expr`], without the wrap an arm beside a `null` takes.
    fn expr_as_written(
        &self,
        out: &mut Out,
        expr: &Expr,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
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
            Expr::LitChar(c, _) => out.push(&format!("'{c}'")),
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
                out.push(&format!("(*{})", self.name(*name)))
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
            Expr::Variable(name, _)
                if !self.walk(|w| w.in_a_kept_call)
                    && self
                        .s
                        .facts
                        .kept_functions
                        .contains_key(&((flow.statement as i64), self.text(*name).to_string())) =>
            {
                // **A named function where a function value is kept** is the
                // closure that calls it, written as the checker typed it.
                let (pauses, fails, arity) = self.s.facts.kept_functions
                    [&(flow.statement as i64, self.text(*name).to_string())];
                let (names, call) =
                    crate::check::kept_function_call(self.parsed, *name, arity as usize);
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
                    out.push(&format!(") as {}<_>", self.boxed_future()));
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
                out.push(&format!("*{}", self.name(*name)));
            }
            Expr::Variable(name, _) => {
                out.push(&self.name(*name));
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
            } => self.call(out, func, args, config, depth, flow)?,
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
                // **`s.find(needle; from:)` on text is `std`'s search**
                // (ADR-320 D6): the language below's `str::find` takes one
                // argument and answers a `usize`.
                if self.text(*method) == "find"
                    && self
                        .s
                        .facts
                        .text_finds
                        .contains(&crate::check::value_node(receiver))
                {
                    out.push("nikaia_std::search::find(&");
                    self.nested(out, receiver, u8::MAX, depth, flow)?;
                    out.push(", &");
                    match args.first() {
                        Some(needle) => self.nested(out, needle, u8::MAX, depth, flow)?,
                        None => out.push("\"\""),
                    }
                    out.push(", ");
                    match config.iter().find(|c| self.text(c.name) == "from") {
                        Some(from) => self.expr(out, &from.value, depth, flow)?,
                        None => out.push("0"),
                    }
                    out.push(")");
                    return Ok(());
                }
                // **`field.of(value)` is the field read a program would have
                // written by hand** ([ADR-304](../../docs/specification/adr/adr-304.md)
                // D2, D5's *at run time: nothing*): `value.name`, with no
                // descriptor and no dispatch left.
                // **A field that holds a function is called through the
                // field**: `(button.on_click)(4)`, awaited where it may pause.
                if self.a_field_call(out, receiver, false, *method, args, depth, flow)? {
                    return Ok(());
                }
                // **`variant.is(value)` is the pattern test a program would
                // have written by hand**: `matches!(value, Op::Add { .. })`,
                // which fits a variant of every shape.
                if let Some(variant) = self.reflected(receiver, *method, "is")
                    && let (Some(value), Some((_, on))) = (
                        args.first(),
                        self.walk(|w| w.specialising.clone().map(|pair| ((), pair.second))),
                    )
                {
                    out.push("matches!(");
                    self.expr(out, value, depth, flow)?;
                    out.push(&format!(", {on}::{} {{ .. }})", escaped(&variant)));
                    return Ok(());
                }
                // **`field.attribute(X)` is the value the attribute came to**
                // (ADR-331 D6): a literal per turn, `Some(…)` or `None`;
                // `.attributes(X)` the list of them in written order.
                for wanted in ["attribute", "attributes"] {
                    if let Some(field) = self.reflected(receiver, *method, wanted)
                        && let Some((_, on)) =
                            self.walk(|w| w.specialising.clone().map(|pair| ((), pair.second)))
                    {
                        // A field's, or a variant's where the walk is over
                        // `T::variants`.
                        let texts = match self.field_contract(&on, &field) {
                            Some(f) => f.attributes,
                            None => self
                                .type_contract(&on)
                                .and_then(|c| c.variants.into_iter().find(|v| v.name == field))
                                .map(|v| v.attributes)
                                .unwrap_or_default(),
                        };
                        out.push(&self.attributes_read(wanted, args, &texts));
                        return Ok(());
                    }
                }
                if let Some(field) = self.reflected(receiver, *method, "of")
                    && let Some(value) = args.first()
                {
                    self.postfix_base(out, value, depth, flow)?;
                    out.push(&format!(".{}", escaped(&field)));
                    return Ok(());
                }
                // **`error.full()` in a handler a named channel reached**
                // ([ADR-280](../../docs/specification/adr/adr-280.md) D10). Part
                // I 7.1 writes it as an ordinary call, and it is one — but the
                // envelope was opened at the binding, so the two halves of the
                // long form are `error` and the site beside it rather than one
                // value with a method on it.
                let long_form = flow.caught_named
                    && self.text(*method) == "full"
                    && args.is_empty()
                    && matches!(receiver.as_ref(), Expr::Variable(name, _) if self.text(*name) == CAUGHT);
                match long_form {
                    true => out.push(&format!("{SITE}.full_of(&{CAUGHT})")),
                    false => {
                        self.method_call(out, Some(receiver), *method, args, config, depth, flow)?;
                        // **Views of values that copy are the values**
                        // (ADR-231 D1).
                        if self
                            .s
                            .facts
                            .copied_walks
                            .contains(&crate::check::value_node(expr))
                        {
                            out.push(".copied()");
                        }
                    }
                }
            }
            // Part I 3.5 onto a **method**
            // ([ADR-278](../../../docs/specification/adr/adr-278.md)): the call
            // happens only where there is something to call it on.
            //
            // **A `match` and not the field's `map`**, and the difference is the
            // difference between a field and a method. A field access can
            // neither pause nor fail, so a closure is somewhere it can happen; a
            // method call may do both, and inside a closure an `.await` does not
            // compile and a `?` has nowhere to go. So the reach is written out,
            // which is the one shape that lets the call be whatever a call is.
            //
            // `None => None` and not `.map(…)`'s implicit one, for the flattened
            // case: where the method's own result is a `T?`, the `Some` arm
            // hands it back as it is and the reach does not nest.
            Expr::SafeMethod {
                receiver,
                method,
                args,
                config,
                ..
            } => {
                let flattens = self
                    .s
                    .facts
                    .flattened_reaches
                    .contains(&crate::check::value_node(receiver));
                // **The scrutinee is lent where the call changes nothing**
                // ([ADR-278](../../docs/specification/adr/adr-278.md) D18):
                // `find(1)?.greet("Hallo")` used to take `find(1)`'s value into
                // the `match`, so a receiver bound to a name was gone
                // afterwards and `rustc` said so about a file nobody wrote
                // (Part III C.1). What comes out is the **call's** result, so
                // nothing about the reach needs the state that is not built.
                let lent = match self
                    .s
                    .facts
                    .lent_reaches
                    .contains(&crate::check::value_node(receiver))
                {
                    true => ".as_ref()",
                    false => "",
                };
                out.push("match ");
                let held = self.walk(|w| {
                    w.held
                        .get(&(crate::check::value_node(receiver.as_ref())))
                        .cloned()
                });
                match held {
                    Some(name) => out.push(&name),
                    // A map's read is not lent (0.0.246), so nothing follows
                    // it and a `match` needs no parentheses around the `*`.
                    None if lent.is_empty() && matches!(receiver.as_ref(), Expr::Index { .. }) => {
                        self.expr(out, receiver, depth, flow)?
                    }
                    None => self.postfix_base(out, receiver, depth, flow)?,
                }
                out.push(lent);
                out.push(&format!(
                    " {{
{}",
                    "    ".repeat(depth + 1)
                ));
                out.push(&format!("Some({REACHED}) => "));
                if !flattens {
                    out.push("Some(");
                }
                // **A copy of the view reached** is `to_owned`, as the plain
                // call's is (ADR-293 D24): the receiver the rewrite reads is the
                // one this `match` bound.
                let copy = self
                    .s
                    .facts
                    .owned_copies
                    .contains(&crate::check::value_node(receiver));
                let held_copy = self.walking(|w| std::mem::replace(&mut w.reached_copy, copy));
                let written =
                    match self.a_field_call(out, receiver, true, *method, args, depth + 1, flow) {
                        Ok(true) => Ok(()),
                        Ok(false) => {
                            self.method_call(out, None, *method, args, config, depth + 1, flow)
                        }
                        Err(error) => Err(error),
                    };
                self.walking(|w| w.reached_copy = held_copy);
                written?;
                if !flattens {
                    out.push(")");
                }
                out.push(&format!(
                    ",
{}None => None,
{}}}",
                    "    ".repeat(depth + 1),
                    "    ".repeat(depth)
                ));
            }
            Expr::Match { value, arms, .. } => self.match_expr(out, value, arms, depth, flow)?,
            Expr::Tuple(parts, _) => {
                out.push("(");
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        out.push(", ");
                    }
                    self.expr(out, part, depth, flow)?;
                }
                out.push(")");
            }
            // **`vec![…]`, which is what a `Vec` already is below**
            // ([ADR-135](../../../docs/specification/adr/adr-135.md) D1). Part
            // I 2.2 offers one container and the literal writes that one, so
            // there is no second shape to choose between here.
            //
            // **Unless the use asked for an array**
            // ([ADR-152](../../../docs/specification/adr/adr-152.md) D4), which
            // is a fact about the literal's *type* and therefore the checker's
            // to hand over: the same three values are `vec![…]` in one position
            // and `[…]` in the other, and nothing in the source distinguishes
            // them.
            Expr::ListLit { items, .. } => {
                let array = self
                    .s
                    .facts
                    .array_literals
                    .contains(&crate::check::value_node(expr));
                out.push(match array {
                    true => "[",
                    false => "vec![",
                });
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(", ");
                    }
                    self.expr(out, item, depth, flow)?;
                }
                out.push("]");
            }
            Expr::Field { base, name, .. } | Expr::SafeField { base, name, .. }
                if matches!(expr, Expr::Field { .. })
                    || self
                        .s
                        .facts
                        .plain_reaches
                        .contains(&crate::check::value_node(expr)) =>
            {
                // **`field.name` is the field's own name as text**
                // ([ADR-304](../../docs/specification/adr/adr-304.md) D2): a
                // literal, because the turn this copy stands at is known.
                if let Some(field) = self.reflected(base, *name, "name") {
                    out.push(&format!("\"{}\"", field));
                    return Ok(());
                }
                // **`field.default` is its default as a `T?`** (ADR-331 D5).
                if let Some(field) = self.reflected(base, *name, "default")
                    && let Some((_, on)) =
                        self.walk(|w| w.specialising.clone().map(|pair| ((), pair.second)))
                {
                    let written = match self.field_contract(&on, &field) {
                        Some(f) if !f.default.is_empty() => held_literal(
                            &f.default,
                            &crate::contracts::ty::Ty::Nullable(Box::new(f.ty)),
                        ),
                        Some(f) => format!("None::<{}>", plain_rust_spelling(&f.ty)),
                        None => "None".to_string(),
                    };
                    out.push(&written);
                    return Ok(());
                }
                // **A boxed field is read through its box**
                // ([ADR-246](../../docs/specification/adr/adr-246.md) D2):
                // `(*base.field)` is a place that behaves as the unboxed field
                // would - read, lent, moved out of an owned value, assigned.
                let boxed = self
                    .s
                    .facts
                    .boxed_reads
                    .contains(&crate::check::value_node(expr));
                if boxed {
                    out.push("(*");
                }
                // **A field of an element is read through the reference the
                // read answers**, as a read of a read is (0.0.238): `xs[0].a`
                // was `(*get(&xs, 0)).a`, a deref the language below does on
                // its own and `clippy` refuses in a `std` written in Nikaia
                // (`tools/ty.nika`, ADR-294).
                match &**base {
                    Expr::Index {
                        base: inner,
                        index: at,
                        ..
                    } if !flow.in_a_place
                        && !self.slices(at)
                        && self.map_key(at, false).is_none()
                        && !self.reads_through_handle(flow, inner) =>
                    {
                        self.index_read(out, inner, at, depth, flow, false)?;
                    }
                    _ => self.postfix_base(out, base, depth, flow)?,
                }
                out.push(&format!(".{}", self.name(*name)));
                if boxed {
                    out.push(")");
                }
            }
            // Part I 3.5: `x?.name` reaches the field only where there is
            // something to reach it on.
            //
            // **`map` or `and_then`, and the checker says which.** Over a plain
            // field `map` is right; over a field that is *itself* a `T?` it
            // would make an `Option<Option<T>>`, and `and_then` is what
            // flattens - which is a question about the declared type, so it is
            // answered where the types are (ADR-288). `map` is the fallback,
            // because it is the one that cannot nest a plain field.
            Expr::SafeField { base, name, .. } => {
                let field = self.text(*name).to_string();
                let flattens = self
                    .s
                    .facts
                    .flattened_reaches
                    .contains(&crate::check::value_node(base));
                let how = if flattens { "and_then" } else { "map" };
                // **The receiver is lent where the member copies**
                // ([ADR-278](../../docs/specification/adr/adr-278.md) D18):
                // `user?.id` reads `user` where it lies and `user` is usable on
                // the next line, which is
                // [ADR-278](../../docs/specification/adr/adr-278.md) D16. Before
                // it, `Option::map` took its receiver and `rustc` said *use of
                // moved value* about a file nobody wrote (Part III C.1).
                //
                // **Only where the checker said the member copies**, which is
                // the safe direction: a member that would come out as a *view*
                // needs the state this compiler does not build, and a field
                // whose type this compiler could not work out is claimed
                // nothing about (Part III C.4). Both lower exactly as they did.
                let copies = self
                    .s
                    .facts
                    .copied_reaches
                    .contains(&crate::check::value_node(base));
                // **And where it comes out as a view** (0.0.244, issue #167
                // issue #167): a view of the member is a view *into* the receiver,
                // so the receiver has to be lent for it as well. `a.b?.c?.v`
                // took `a.b` out of a lent `a` - *cannot move out of `a.b`* -
                // and handed back a view of the value it had just moved.
                let viewed = self
                    .s
                    .facts
                    .viewed_reaches
                    .contains_key(&crate::check::value_node(base));
                let lent = match copies || viewed {
                    true => ".as_ref()",
                    false => "",
                };
                // **And how the member is taken out of that view**
                // ([ADR-278](../../docs/specification/adr/adr-278.md) D19). A
                // member that copies is read; one that does not comes out as a
                // view, and the two views are spelled differently below: a view
                // of `String` is `&str`
                // ([ADR-184](../../docs/specification/adr/adr-184.md) D2) and a
                // view of anything else is `&T`. The checker says which,
                // because this emitter has no types (ADR-296 D17).
                // **A boxed field is reached through its box**
                // ([ADR-246](../../docs/specification/adr/adr-246.md) D2).
                let place = match self
                    .s
                    .facts
                    .boxed_reads
                    .contains(&crate::check::value_node(expr))
                {
                    true => format!("(*__nikaia_it.{field})"),
                    false => format!("__nikaia_it.{field}"),
                };
                let reach = match (
                    self.s
                        .facts
                        .viewed_reaches
                        .get(&crate::check::value_node(base)),
                    flattens,
                ) {
                    (None, _) => place,
                    (Some(crate::check::Viewed::Text), false) => format!("{place}.as_str()"),
                    (Some(crate::check::Viewed::Text), true) => format!("{place}.as_deref()"),
                    (Some(crate::check::Viewed::Plain), false) => format!("&{place}"),
                    (Some(crate::check::Viewed::Plain), true) => format!("{place}.as_ref()"),
                };
                self.postfix_base(out, base, depth, flow)?;
                out.push(&format!("{lent}.{how}(|__nikaia_it| {reach})"));
            }
            Expr::Index { base, index, .. } => {
                // **A length is an `i64`, so an index is one too**
                // ([ADR-285](../../../docs/specification/adr/adr-285.md) D1), and
                // Rust indexes a sequence by `usize`. The conversion is emitted
                // and never written, in both directions - which is the whole of
                // what D1 buys.
                //
                // Around **every** index but one, because this emitter does not
                // know types (ADR-296 D17) and a map is indexed too:
                // `counts[path]` takes a `&str`. `nikaia_std::index::at` chooses
                // on the type of what is in the brackets and is the identity for
                // anything that is not a number, so a rule applied everywhere
                // cannot be applied to the wrong index.
                //
                // **The one exception is an index written only in literals.**
                // `xs[0]` and `&text[1..3]` need no conversion - Rust's own
                // inference gives a literal the `usize` the sequence wants - and
                // they cannot *take* one: `at(0)` has nothing to infer `I` from,
                // since every integer type answers with the same `usize`, and
                // `cannot infer type` about a generated file is exactly what
                // Part III C.1 forbids.
                //
                // **A read is a call and a place is brackets**
                // ([ADR-293](../../../docs/specification/adr/adr-293.md) D4). A
                // read through the brackets answers what the container can
                // promise — a `T` for a sequence, a `T?` for a map, because a
                // key that is not there is data about the world rather than a
                // bug — and `index::get` is the one trait that lets this
                // emitter write the same three tokens for both. The **left of
                // an assignment** is not a read: `self.bodies[0].vx = …` writes
                // through the index, so the brackets there stay the language
                // below's own.
                if !flow.in_a_place {
                    // **A slice is already a view, so it takes no `*`**
                    // ([ADR-182](../../docs/specification/adr/adr-182.md) D2). A read
                    // at a number answers a `&T` and the `*` is what makes it
                    // the `T` the program asked for; a read at a **range**
                    // answers a `&str` or a `&[T]`, and `*` over one of those
                    // is a `str` or a `[T]` — a value whose size the language
                    // below does not know, which is what it said: *the size
                    // for values of type `str` cannot be known at compilation
                    // time*, about a noun nobody wrote
                    // ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
                    //
                    // **Off the shape of what is in the brackets**, which is
                    // the one place this needs no type: a range is a run and a
                    // key is not, in this language and in the one below alike.
                    //
                    // **And no parentheses here at all**
                    // ([ADR-293](../../docs/specification/adr/adr-293.md) D33).
                    // A pair around `*get(…)` is what makes `(*get(…)).len()`
                    // the length of the deref rather than the deref of the
                    // length - but only where a postfix follows, and a postfix
                    // is written through `postfix_base`, which adds it. Written
                    // here, it stood around every read: `m[k] ?? 0` and
                    // `let x = m[k]` were `rustc` warnings about a file nobody
                    // wrote ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
                    //
                    // **A read that is lent is the reference it already was**
                    // (0.0.250): the `&` in front and the `*` cancel.
                    // **A read of a map of `T?` is one `T?`**
                    // ([ADR-293](../../docs/specification/adr/adr-293.md) D14):
                    // `index::flat` over the read, which is the `Option<&T>`
                    // a `ref T?` is below - so a lend in front of it is
                    // already there, and is taken.
                    if self
                        .s
                        .facts
                        .flat_reads
                        .contains(&crate::check::value_node(expr))
                    {
                        let _ = out.take_lend();
                        out.push("nikaia_std::index::flat(");
                        self.index_read(out, base, index, depth, flow, true)?;
                        out.push(")");
                        return Ok(());
                    }
                    // **A map's read lent is the option it already is**: `*get(…)`
                    // is the `Option<&V>` a `ref V?` is, and cancelling the `*`
                    // against the lend left `index::Found`, which no `ref V?`
                    // parameter takes (found moving `sharing`'s leaves into
                    // Nikaia, #125). A sequence's read keeps the cancelling.
                    if self.map_key(index, false).is_some() {
                        let _ = out.take_lend();
                        return self.index_read(out, base, index, depth, flow, true);
                    }
                    let lent = !self.slices(index) && out.take_lend();
                    return self.index_read(out, base, index, depth, flow, !lent);
                }
                self.postfix_base(out, base, depth, flow)?;
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
            }
            Expr::Cast { expr, ty, .. } => {
                let into = self.ty(ty, Lifetimes::Elided);
                // **A `for` lends, and `as` does not see through a view**
                // ([ADR-182](../../docs/specification/adr/adr-182.md) D1).
                // The checker says which operands those are, because which name is
                // a view is a question about the scope (ADR-288).
                //
                // **`num::value` and not a `*`**, which is
                // `nikaia_std::index::at`'s own reasoning one construct over:
                // for a number that is already a number this is the identity,
                // so the rule cannot be written onto the wrong operand — and a
                // `*` written onto one would be a `rustc` error about the
                // generated file, which is the very thing this closes.
                let viewed = match expr.as_ref() {
                    Expr::Variable(name, _) => self
                        .s
                        .facts
                        .viewed_numbers
                        .contains(&((flow.statement as i64), self.text(*name).to_string())),
                    _ => false,
                };
                match self.narrows(flow.statement, &into) {
                    // **A narrowing conversion is checked, and an unchecked one
                    // aborts** (ADR-285 D12). Rust's `as` truncates by definition
                    // and no setting changes that, so this is the one place in
                    // the arithmetic decision where the emitted code differs from
                    // what was written - and it differs into what a Rust
                    // programmer would have written here anyway.
                    Some(crate::check::Narrowing::Integer) => {
                        // `unwrap_or_else` and not `expect`: `expect` appends
                        // the error's `Debug`, which is `TryFromIntError(())` -
                        // Rust internals in a sentence a Nikaia user reads, and
                        // the same leak the diagnostics filter exists to stop.
                        out.push(&format!("{into}::try_from("));
                        self.operand(out, expr, viewed, depth, flow)?;
                        out.push(&format!(
                            ").unwrap_or_else(|_| panic!(\"the value does not fit in an `{into}`\"))"
                        ));
                    }
                    // `i32::try_from(f64)` does not exist, so the range and
                    // "not a number" are tested by a `std` helper instead.
                    Some(crate::check::Narrowing::FromFloat) => {
                        out.push(&format!("nikaia_std::num::to_{into}("));
                        self.operand(out, expr, viewed, depth, flow)?;
                        out.push(")");
                    }
                    None if viewed => {
                        self.operand(out, expr, viewed, depth, flow)?;
                        out.push(&format!(" as {into}"));
                    }
                    None => {
                        self.nested(out, expr, u8::MAX, depth, flow)?;
                        out.push(&format!(" as {into}"));
                    }
                }
            }
            Expr::StructLit { name, fields, .. } => {
                // `unaliased`, the same as a type: `h::Request(path: …)` builds
                // `http::Request` (ADR-286 D12).
                let owner = self.parsed.unaliased(self.text(*name));
                // A value with a cleanup is made in its `Cleaned` (ADR-297 D1).
                let cleaned = self.s.cleaned_types.contains(&owner);
                if cleaned {
                    out.push(&format!("{}::new(", self.cleaned()));
                }
                out.push(&format!("{owner} {{ "));
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(", ");
                    }
                    out.push(&self.name(field.name));
                    // Part I 2.3: a plain value in a field the struct declares
                    // nullable, by the value's id.
                    let how = self.s.facts.nullable_fields.get(&(
                        crate::check::value_node(expr),
                        self.text(field.name).to_string(),
                    ));
                    let (before, after) = Self::around(how);
                    // **A boxed field is built in its box**
                    // ([ADR-246](../../docs/specification/adr/adr-246.md) D3),
                    // around whatever else the value is wrapped in.
                    let boxed = self.is_boxed_field(&owner, self.text(field.name));
                    let (before, after) = match boxed {
                        true => (format!("Box::new({before}"), format!("{after})")),
                        false => (before.to_string(), after.to_string()),
                    };
                    let (before, after) = (before.as_str(), after.as_str());
                    // **Into a field both kinds of text flow into, each value
                    // goes as it is** (ADR-282 D14): a view borrowed, text of its
                    // own moved in.
                    let either = self.either_field(&owner, field.name)
                        && !matches!(field.value, Some(Expr::LitNull(_)));
                    // The nullable wrap goes around it: `Some(either(v))`
                    // (ADR-282 D5).
                    if either {
                        out.push(": ");
                        out.push(before);
                        out.push("nikaia_std::either_text::either(");
                        match &field.value {
                            Some(value) => self.expr(out, value, depth, flow)?,
                            None => out.push(&self.name(field.name)),
                        }
                        out.push(")");
                        out.push(after);
                    } else if let Some(value) = &field.value {
                        // **`name: name` is written `name`** (0.0.249), as the
                        // source may write it (Part I 4.1): the pair is what
                        // `clippy` refuses in `std`, and a lowering that only
                        // repeats the field's name adds nothing to say.
                        if before.is_empty()
                            && after.is_empty()
                            && matches!(value, Expr::Variable(v, _) if self.text(*v) == self.text(field.name))
                            && {
                                let mut inner_buf = Out::empty();
                                let inner = &mut inner_buf;
                                self.expr(inner, value, depth, flow)?;
                                inner_buf
                            }
                            .buf == self.name(field.name)
                        {
                            continue;
                        }
                        out.push(": ");
                        out.push(before);
                        self.expr(out, value, depth, flow)?;
                        out.push(after);
                    } else if how.is_some() || boxed {
                        // `Counter { db }` is the shorthand for `db: db`
                        // (Part I 4.1), and a wrapper has to be written around
                        // the name - which means writing the pair out.
                        let name = self.name(field.name);
                        let wrapped = match how.is_some() {
                            true => format!("Some({name})"),
                            false => name.to_string(),
                        };
                        match boxed {
                            true => out.push(&format!(": Box::new({wrapped})")),
                            false => out.push(&format!(": {wrapped}")),
                        }
                    }
                }
                // **A field left out takes its default** (ADR-331 D5): the
                // value the ledger recorded, written as an option's is.
                let written: Vec<&str> = fields.iter().map(|f| self.text(f.name)).collect();
                let mut after_one = !fields.is_empty();
                for (field, default, ty) in self.field_defaults(&owner) {
                    if written.contains(&field.as_str()) {
                        continue;
                    }
                    if after_one {
                        out.push(", ");
                    }
                    after_one = true;
                    let value = held_literal(&default, &ty);
                    let value = match self.is_boxed_field(&owner, &field) {
                        true => format!("Box::new({value})"),
                        false => value,
                    };
                    out.push(&format!("{}: {value}", escaped(&field)));
                }
                out.push(" }");
                if cleaned {
                    out.push(")");
                }
            }
            // **`p with { x: p.x + 1 }`** is Rust's functional update
            // ([ADR-118](../../docs/specification/adr/adr-118.md) D1, D3): the
            // named fields, then `..base`, from which every field the program
            // did not name comes **by move**. No copy is inserted that the
            // program did not write ([ADR-282](../../docs/specification/adr/adr-282.md)
            // D3), which is why this is `..base` and not `..base.clone()`.
            //
            // The type is the checker's answer, read back under the byte the
            // `with` stands at. There is always one, because a `with` this
            // compiler could not name a type for was refused with `NK1173`.
            Expr::With {
                base, fields, at, ..
            } => {
                let Some(owner) = self
                    .s
                    .facts
                    .with_types
                    .get(&crate::check::value_node(expr))
                    .cloned()
                else {
                    return Err(refused_at!(
                        *at as usize,
                        "The type of this `with` is unknown. This is a compiler bug: the \
                         check should have reported it first."
                    ));
                };
                out.push(&format!("{owner} {{ "));
                for field in fields {
                    out.push(&self.name(field.name));
                    // **A boxed field is built in its box** (ADR-246 D3).
                    if self.is_boxed_field(&owner, self.text(field.name)) {
                        out.push(": Box::new(");
                        match &field.value {
                            Some(value) => self.expr(out, value, depth, flow)?,
                            None => out.push(&self.name(field.name)),
                        }
                        out.push("), ");
                        continue;
                    }
                    let either = self.either_field(&owner, field.name)
                        && !matches!(field.value, Some(Expr::LitNull(_)));
                    match (&field.value, either) {
                        (Some(value), true) => {
                            out.push(": nikaia_std::either_text::either(");
                            self.expr(out, value, depth, flow)?;
                            out.push(")");
                        }
                        (Some(value), false) => {
                            out.push(": ");
                            self.expr(out, value, depth, flow)?;
                        }
                        (None, true) => out.push(&format!(
                            ": nikaia_std::either_text::either({})",
                            self.name(field.name)
                        )),
                        (None, false) => {}
                    }
                    out.push(", ");
                }
                out.push("..");
                self.expr(out, base, depth, flow)?;
                out.push(" }");
            }
            // A lambda's arguments are the ones it names (ADR-277): nothing is
            // read off the body, so `fn { … }` is `||`.
            Expr::Closure {
                params,
                mutable,
                body,
                ..
            } => {
                let names: Vec<String> =
                    params.iter().map(|p| self.name(*p).into_owned()).collect();
                // **A lambda where a function value is kept** is one shared
                // closure, and it takes what it captures with it.
                let kept = self
                    .s
                    .facts
                    .kept_lambdas
                    .get(&crate::check::value_node(expr))
                    .copied();
                if let Some((pauses, fails)) = kept {
                    // Each hull it captures is a handle of its own
                    // (ADR-230 D4): cloned in a block around the closure, so
                    // the name outside is not moved into it.
                    let hulls = self
                        .s
                        .facts
                        .kept_hulls
                        .get(&crate::check::value_node(expr))
                        .cloned()
                        .unwrap_or_default();
                    // **What a pausing one takes is owned once and lent to every
                    // call** (#95 step 6): moved behind an `Arc` here, and each
                    // call's future holds a count of it rather than a copy.
                    let owned = self
                        .s
                        .facts
                        .kept_owned
                        .get(&crate::check::value_node(expr))
                        .cloned()
                        .unwrap_or_default();
                    let block = !hulls.is_empty() || !owned.is_empty();
                    if block {
                        out.push("{ ");
                        for hull in &hulls {
                            let hull = escaped(hull);
                            out.push(&format!("let {hull} = {hull}.clone(); "));
                        }
                        for taken in &owned {
                            let taken = escaped(taken);
                            out.push(&format!("let {taken} = std::sync::Arc::new({taken}); "));
                        }
                    }
                    // **A `mut` parameter is the lambda's own value**: a kept
                    // function is handed its arguments, so `fn(mut conn)` is a
                    // binding it may change, written `mut conn`, and not the
                    // address a lock's door hands an `update` block.
                    let bound: Vec<String> = params
                        .iter()
                        .zip(&names)
                        .map(|(p, name)| match mutable.contains(p) {
                            true => format!("mut {name}"),
                            false => name.clone(),
                        })
                        .collect();
                    out.push(&format!(
                        "nikaia_std::func::Kept(std::sync::Arc::new(move |{}| ",
                        bound.join(", ")
                    ));
                    let changed: Vec<Symbol> = flow
                        .changed
                        .iter()
                        .copied()
                        .filter(|name| !params.contains(name))
                        .collect();
                    let inside = Flow {
                        in_lambda: !pauses,
                        throws: fails,
                        statement: flow.statement,
                        changed: &changed,
                        ..Flow::PLAIN
                    };
                    // **Each call takes a handle of its own of each hull**:
                    // the closure is called again (a supervisor's child, every
                    // connection of `net::serve`), and the future each call
                    // makes owns what it uses, so the closure's own handle is
                    // cloned per call rather than moved out of it.
                    if pauses {
                        if block {
                            out.push("{ ");
                            for hull in hulls.iter().chain(&owned) {
                                let hull = escaped(hull);
                                out.push(&format!("let {hull} = {hull}.clone(); "));
                            }
                        }
                        out.push("Box::pin(async move ");
                    }
                    // A type that may fail hands back a `Result`, as a
                    // `throws` function does: the value goes inside the `Ok`.
                    // Through a `let`: `Ok({ … })` around a body of one
                    // expression is `rustc`'s *unnecessary braces*, about a file
                    // nobody wrote.
                    match fails {
                        true => {
                            out.push("{ let __nikaia_done = ");
                            self.block(out, body, depth, inside, Tail::Value)?;
                            out.push("; Ok(__nikaia_done) }");
                        }
                        false => self.block(out, body, depth, inside, Tail::Return)?,
                    }
                    if pauses {
                        out.push(&format!(") as {}<_>", self.boxed_future()));
                        if block {
                            out.push(" }");
                        }
                    }
                    out.push("))");
                    if block {
                        out.push(" }");
                    }
                    return Ok(());
                }
                // **A lambda that pauses, where a `std` entry takes it**
                // ([ADR-233](../../docs/specification/adr/adr-233.md) D1): the
                // language below's own `async` closure, handed to the entry's
                // counterpart. Its body is an ordinary body that pauses.
                let pauses = self
                    .s
                    .facts
                    .pausing_lambdas
                    .contains_key(&crate::check::value_node(expr));
                if pauses {
                    out.push("async ");
                }
                // **`filter` is handed a reference to its item** (ADR-231 D2):
                // where the item copies, the pattern takes it out.
                let pattern = self
                    .s
                    .facts
                    .filter_patterns
                    .contains(&crate::check::value_node(expr));
                match (pattern, names.as_slice()) {
                    (true, [one]) => out.push(&format!("|&{one}| ")),
                    _ => out.push(&format!("|{}| ", names.join(", "))),
                }
                // **The `mut` ones reach inward** (ADR-281 D12), and the list is
                // the enclosing one plus this lambda's rather than this
                // lambda's alone: a lambda written inside an `update` block
                // still means the address when it names the outer `v`.
                let mut changed: Vec<Symbol> = flow.changed.to_vec();
                changed.extend(mutable.iter().copied());
                let flow = Flow {
                    changed: &changed,
                    ..flow
                };
                // A lambda's `return` leaves the lambda, not the function
                // around it, so it never carries the enclosing `Ok`. The
                // statement is kept, because the checker's answers about the
                // calls in here are keyed by it, and `in_lambda` is what makes
                // a pausing one refusable (ADR-055 §6).
                let inside = Flow {
                    in_lambda: !pauses,
                    statement: flow.statement,
                    // **And the `mut` parameters with it**: the body starts
                    // from `PLAIN` because a lambda's `return` leaves the
                    // lambda, and that must not throw away what D1 needs to
                    // write `(*v)`.
                    changed: &changed,
                    ..Flow::PLAIN
                };
                self.block(out, body, depth, inside, Tail::Return)?;
            }
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
            } => {
                // **A constant sum takes the first type that holds it**, the
                // way a constant does
                // ([ADR-285](../../../docs/specification/adr/adr-285.md) D22).
                // Asked only on the outermost expression that folds - `widen`
                // reaching inward is what makes the operands agree - and never
                // where the position decides the type.
                let flow = match flow.widen || flow.inferred {
                    true => flow,
                    false => match crate::fold::constant_of(expr, &crate::fold::nothing_is_known) {
                        Some(folded) if crate::fold::wants_widening(&folded) => flow.widened(),
                        _ => flow,
                    },
                };
                // **A `+` that joins text is a call**
                // ([ADR-081](../../docs/specification/adr/adr-081.md) D2).
                // Rust's operator takes `String + &str` and nothing else, so
                // three of the four shapes a program can write went below and
                // were refused there. `concat::plus` has an impl per shape and
                // the language below picks, which is `index::at`'s arrangement
                // a third time - and it keeps `String + &str` exactly as fast
                // as it is today, which a `format!` for every shape would not.
                //
                // **Only where the checker said so**, by the operator's own
                // span: a number's `+` may not move into `std`, where
                // `overflow-checks` is off and inlining would not carry the
                // abort across (ADR-285 D5).
                // **A file's name compares its bytes** (ADR-319 D3), against
                // a name or against text.
                if matches!(op, BinaryOp::Eq | BinaryOp::Ne)
                    && self.s.facts.path_comparisons.contains(&(at.at() as i64))
                {
                    if matches!(op, BinaryOp::Ne) {
                        out.push("!");
                    }
                    out.push("nikaia_std::fs::path::same(&(");
                    self.expr(out, lhs, depth, flow)?;
                    out.push("), &(");
                    self.expr(out, rhs, depth, flow)?;
                    out.push("))");
                    return Ok(());
                }
                // **A `u64` and a signed integer compare as numbers**
                // (ADR-285 D32): a sign test and a compare, the sides in the
                // order they were written.
                if let Some(unsigned_left) = self.s.facts.mixed_comparisons.get(&(at.at() as i64)) {
                    out.push(match unsigned_left {
                        true => "nikaia_std::num::unsigned_with_signed(",
                        false => "nikaia_std::num::signed_with_unsigned(",
                    });
                    self.expr(out, lhs, depth, flow)?;
                    out.push(", ");
                    self.expr(out, rhs, depth, flow)?;
                    out.push(match op {
                        BinaryOp::Lt => ").is_lt()",
                        BinaryOp::Le => ").is_le()",
                        BinaryOp::Gt => ").is_gt()",
                        BinaryOp::Ge => ").is_ge()",
                        BinaryOp::Eq => ").is_eq()",
                        _ => ").is_ne()",
                    });
                    return Ok(());
                }
                if self.s.facts.concatenations.contains(&(at.at() as i64)) {
                    out.push("nikaia_std::concat::plus(");
                    self.concatenated(out, lhs, depth, flow)?;
                    out.push(", ");
                    self.concatenated(out, rhs, depth, flow)?;
                    out.push(")");
                    return Ok(());
                }
                // **`x == null` asks whether there is one** (0.0.250):
                // `x.is_none()`, which needs nothing of what `x` holds, where
                // `x == None` needs it to compare - and is what `clippy`
                // refuses in `std`.
                if matches!(op, BinaryOp::Eq | BinaryOp::Ne)
                    && let Some(side) = match (lhs.as_ref(), rhs.as_ref()) {
                        (Expr::LitNull(_), other) | (other, Expr::LitNull(_))
                            if !matches!(other, Expr::LitNull(_)) =>
                        {
                            Some(other)
                        }
                        _ => None,
                    }
                {
                    self.postfix_base(out, side, depth, flow)?;
                    out.push(match op {
                        BinaryOp::Eq => ".is_none()",
                        _ => ".is_some()",
                    });
                    return Ok(());
                }
                // **A counter that is never negative is compared with a length
                // as a `usize`** ([ADR-270](../../docs/specification/adr/adr-270.md)
                // D8 step 1): `i < xs.len()` is `(i as usize) < xs.len()`, the
                // same comparison for `i >= 0`, and the one the optimiser can
                // match with the bounds check of `xs[i]` and drop it.
                if op.is_comparison()
                    && let Some(compared) = self.against_a_length(out, op, lhs, rhs, depth, flow)?
                {
                    return Ok(compared);
                }
                // **An operation proved to stay inside its type** is written
                // without its check, naming the type the proof was about
                // ([ADR-306](../../docs/specification/adr/adr-306.md) D10).
                if let Some(ty) = self.s.proven_arithmetic.get(&(at.at() as i64))
                    && !flow.widen
                    && !flow.inferred
                {
                    let name = match op {
                        BinaryOp::Add => "wrapping_add",
                        BinaryOp::Sub => "wrapping_sub",
                        _ => "wrapping_mul",
                    };
                    out.push(&format!("<{ty}>::{name}("));
                    self.expr(out, lhs, depth, flow)?;
                    out.push(", ");
                    self.expr(out, rhs, depth, flow)?;
                    out.push(")");
                    return Ok(());
                }
                // Parenthesised only where precedence needs it: the operators
                // mean the same in both languages, so `value * 10 + n` should
                // come out the way it went in.
                let here = precedence(*op);
                // **A view compared with a value is read** (0.0.238,
                // `check::Checked::compared_views`).
                let read = |side: &str| {
                    self.s
                        .facts
                        .compared_views
                        .contains(&((at.at() as i64), side.to_string()))
                };
                // **And a lent name of a type only the language below knows,
                // beside a number, is read as the number** (#531,
                // `check::Checked::compared_numbers`): `num::value` is the
                // identity on a number and reads one out of a view.
                let number = |side: &str| {
                    self.s
                        .facts
                        .compared_numbers
                        .contains(&((at.at() as i64), side.to_string()))
                };
                if number("lhs") {
                    out.push("nikaia_std::num::value(");
                    self.expr(out, lhs, depth, flow)?;
                    out.push(&format!(") {} ", binary_op(*op)));
                    self.nested(out, rhs, here + 1, depth, flow)?;
                    return Ok(());
                }
                if number("rhs") {
                    self.nested(out, lhs, here, depth, flow)?;
                    out.push(&format!(" {} nikaia_std::num::value(", binary_op(*op)));
                    self.expr(out, rhs, depth, flow)?;
                    out.push(")");
                    return Ok(());
                }
                if read("lhs") {
                    out.push("*");
                }
                // **A conversion left of `<<` keeps its parentheses** (#474),
                // as one left of `<` does: `x as i64 << 3` is `i64<…>` to Rust.
                // Only where `nested` would not write them itself.
                let cast_before_a_shift = matches!(op, BinaryOp::Shl)
                    && self.ends_in_a_cast(lhs)
                    && !matches!(lhs.as_ref(), Expr::Binary { op: inner, .. } if precedence(*inner) < here);
                if cast_before_a_shift {
                    out.push("(");
                }
                self.nested(out, lhs, here, depth, flow)?;
                if cast_before_a_shift {
                    out.push(")");
                }
                out.push(&format!(" {} ", binary_op(*op)));
                if read("rhs") {
                    out.push("*");
                }
                self.nested(out, rhs, here + 1, depth, flow)?;
            }
            // **A fallback that jumps is a `match` and not a closure**
            // ([ADR-276](../../../docs/specification/adr/adr-276.md) D20, and
            // [ADR-276](../../../docs/specification/adr/adr-276.md) D16's rule
            // about what a jump may not cross).
            //
            // `unwrap_or_else` takes a **closure**, so a `return` written in
            // the fallback would return from the closure and the program would
            // carry on — `let user = find(id) ?? throw NotFound(id)`, the shape
            // this record's own example is written in, would have thrown
            // nothing and bound a value nobody produced. The `match` puts the
            // jump in the function it was written in.
            //
            // **And so is one whose fallback is another `??` that jumps**:
            // `touches[key] ?? described(key) ?? continue` reads as
            // `touches[key] ?? (described(key) ?? continue)`, and the inner
            // `match` written inside the outer closure put the `continue` in a
            // closure all the same - `rustc`'s E0267 (found moving
            // `contracts::locks` into Nikaia).
            // **A `??` after a value that cannot be absent is its left side**
            // (ADR-279 D13): the fallback never runs and is not written.
            Expr::Coalesce { value, .. }
                if self
                    .s
                    .facts
                    .plain_reaches
                    .contains(&crate::check::value_node(expr)) =>
            {
                self.expr(out, value, depth, flow)?;
            }
            Expr::Coalesce {
                value, fallback, ..
            } if jumps_at_the_end(fallback) || self.never_returns(fallback) => {
                out.push("match ");
                self.expr(out, value, depth, flow)?;
                // **A loan on the left is opened, and the answer is a view**
                // ([ADR-279](../../docs/specification/adr/adr-279.md) D10).
                if let Some(opened) = self
                    .s
                    .facts
                    .lent_coalesces
                    .get(&crate::check::value_node(expr))
                {
                    out.push(opened);
                }
                // **A number read past a jump is read out of the view**
                // (#456, `Checked::copied_jump_reads`).
                let copied = self
                    .s
                    .facts
                    .copied_jump_reads
                    .contains(&crate::check::value_node(expr));
                out.push(if copied {
                    " { Some(__nikaia_value) => nikaia_std::num::value(__nikaia_value), None => "
                } else {
                    " { Some(__nikaia_value) => __nikaia_value, None => "
                });
                self.expr(out, fallback, depth, flow)?;
                out.push(" }");
            }
            // **A `??` whose answer is only read lends its left side**
            // ([ADR-279](../../docs/specification/adr/adr-279.md) D5): the
            // option is opened (`.as_deref()` for text, `.as_ref()` for the
            // rest) and the fallback is a view of what it names, so the answer
            // is a view and the name stays usable after the line.
            Expr::Coalesce {
                value, fallback, ..
            } if self
                .s
                .facts
                .lent_coalesces
                .contains_key(&crate::check::value_node(expr)) =>
            {
                let opened = self.s.facts.lent_coalesces[&crate::check::value_node(expr)].clone();
                out.push("nikaia_std::index::or(");
                self.expr(out, value, depth, flow)?;
                out.push(&opened);
                out.push(", || ");
                match (&**fallback, opened.as_str()) {
                    (Expr::LitStr { .. }, _) => self.expr(out, fallback, depth, flow)?,
                    (Expr::Variable(_, _), ".as_deref()") => {
                        out.push("std::convert::AsRef::<str>::as_ref(&");
                        self.expr(out, fallback, depth, flow)?;
                        out.push(")");
                    }
                    _ => {
                        out.push("&");
                        self.expr(out, fallback, depth, flow)?;
                    }
                }
                out.push(")");
            }
            Expr::Coalesce {
                value, fallback, ..
            } => {
                // Kap 3.5. `into()` because the fallback is written as the
                // value it stands for, not as the type the option holds -
                // `text ?? "none"` on a `String?` is the case it exists for.
                //
                // **Except for a number**, and that exception is Part I 2.4
                // rather than a special case: a number literal takes the type
                // its use asks for, so `0` already *is* whatever the option
                // holds and `0.into()` adds a conversion that has to be
                // resolved. Where the option's own type is still open - a
                // `channel::bounded` whose element type nothing has pinned yet
                // ([ADR-149](../../docs/specification/adr/adr-149.md)) - there
                // is nothing to resolve it from, and what a reader got was
                // *"type annotations needed"* about a file nobody wrote
                // (Part III, C.1). Without the `into` the literal is simply
                // one more use of the type, which is what decides it.
                //
                // **`index::or` and not `unwrap_or_else`**
                // ([ADR-293](../../docs/specification/adr/adr-293.md) D4): the
                // left of a `??` may now be a **view into a container**, because
                // a map read answers `Option<&V>` — the value reached is the
                // map's, and copying it is never the compiler's to do
                // ([ADR-283](../../docs/specification/adr/adr-283.md) D3). The
                // fallback is still written as the value it stands for, so the
                // two sides do not have the same type and the language below is
                // what joins them.
                // **And except for text where the left side is a view of text**
                // (ADR-283): the literal already is one.
                //
                // **And except for a variant of an enum this file declares**:
                // `Ty::Unknown` is already a `Ty`, and `.into()` on it is a
                // conversion to the same type, which the language below's
                // lint calls useless (found moving `Ty::parse` into Nikaia,
                // ADR-294).
                let a_declared_variant = match &**fallback {
                    Expr::Path(segments, _) => {
                        segments.len() == 2
                            && self.is_variant(self.text(segments[0]), self.text(segments[1]))
                    }
                    // ...and one built with its parts, `Ty::Tuple([])`.
                    Expr::Call { func, .. } => matches!(&**func, Expr::Path(segments, _)
                        if segments.len() == 2
                            && self.is_variant(self.text(segments[0]), self.text(segments[1]))),
                    _ => false,
                };
                // **And except for a constructor of a type this file
                // declares**, `FnContract::empty()`: its entry says it hands
                // back the type itself, so `.into()` is the same useless
                // conversion (found moving the ledger's reader, ADR-294).
                let a_declared_constructor = match &**fallback {
                    Expr::Call { func, .. } => match &**func {
                        Expr::Path(segments, _) if segments.len() == 2 => {
                            let owner = self.text(segments[0]);
                            let key = format!("{owner}::{}", self.text(segments[1]));
                            self.s.own_contracts
                                .functions
                                .get(&key)
                                .and_then(|contract| contract.signature.as_ref())
                                .and_then(|signature| signature.result.as_ref())
                                .is_some_and(|result| {
                                    matches!(result, crate::contracts::ty::Ty::Named { name, args, view: false }
                                        if name == owner && args.is_empty())
                                })
                        }
                        _ => false,
                    },
                    _ => false,
                };
                // **And a struct written out**, `Held { … }`, or a `bool`: it
                // is the type already, whatever the option holds.
                let a_struct_written =
                    matches!(&**fallback, Expr::StructLit { .. } | Expr::LitBool(_, _));
                let bare = a_number(fallback)
                    || a_declared_variant
                    || a_declared_constructor
                    || a_struct_written
                    || matches!(&**fallback, Expr::LitStr { .. } if self.s.facts.view_fallbacks.contains(&crate::check::value_node(fallback)))
                    || self
                        .s
                        .facts
                        .view_coalesces
                        .contains(&crate::check::value_node(expr));
                // **A fallback that may be `null` too** (Part I 3.5): the
                // first option that has a value, which is the language
                // below's `or_else`, and the answer is an option still.
                if self
                    .s
                    .facts
                    .optional_fallbacks
                    .contains(&crate::check::value_node(expr))
                {
                    out.push("nikaia_std::index::or_maybe(");
                    self.expr(out, value, depth, flow)?;
                    out.push(", || ");
                    self.expr(out, fallback, depth, flow)?;
                    out.push(")");
                    return Ok(());
                }
                out.push("nikaia_std::index::or(");
                // **A list's optional text is lent** (#342): the read is a
                // place, and `&` in front of it borrows the element.
                if self
                    .s
                    .facts
                    .lent_list_reads
                    .contains(&crate::check::value_node(expr))
                {
                    out.push("&");
                }
                self.expr(out, value, depth, flow)?;
                out.push(", || ");
                // **A name lent beside what a map holds**
                // (`check::Checked::lent_map_fallbacks`): the answer is a
                // view, and so is the fallback.
                if self
                    .s
                    .facts
                    .lent_map_fallbacks
                    .contains(&crate::check::value_node(expr))
                {
                    out.push("&");
                    self.expr(out, fallback, depth, flow)?;
                    out.push(")");
                    return Ok(());
                }
                // **A fallback read at an index is cloned out of the place**
                // (#561): the read is written `*get(…)`, a `*` that binds
                // looser than the `.into()` after it, so the conversion was
                // applied to the reference and its target was no longer
                // inferable (E0283) - and a `*` over text moves out of the
                // container. `get(…).clone().into()` is a value for a number
                // and for text alike.
                if !bare && matches!(&**fallback, Expr::Index { .. }) {
                    out.push("&");
                    self.expr(out, fallback, depth, flow)?;
                    out.push(".clone().into())");
                    return Ok(());
                }
                self.expr(out, fallback, depth, flow)?;
                out.push(match bare {
                    true => ")",
                    false => ".into())",
                });
            }
            Expr::TryCatch { expr, handler, .. } => {
                // Kap 7.1: the handler sees the error as `error`.
                out.push("match ");
                // `catch` needs the `Result`, not the value: a `dsl … from …`
                // propagates on its own everywhere else, and here the handler
                // is what handles it.
                // A written call inside the guarded half must not take the
                // `?`: the `match` below is what handles the failure. A grammar
                // entry is one of those calls now
                // ([ADR-296](../../docs/specification/adr/adr-296.md) D24), so it
                // needs no arm of its own — which is what that record meant by
                // *an ordinary call*.
                // **A joining block is told the handler is right here**
                // ([ADR-292](../../docs/specification/adr/adr-292.md) D18), and
                // only when it *is* the guarded half: one nested in an argument
                // hands back a tuple like any other value.
                let guarded = match &**expr {
                    Expr::Overlap(_, _) | Expr::Select(_, _) => flow.joined_here(),
                    _ => flow.guarded(),
                };
                self.expr(out, expr, depth, guarded)?;
                let pad = "    ".repeat(depth + 1);
                let close = "    ".repeat(depth);
                // **A handler that does not read the error binds `_error`**
                // ([ADR-308](../../docs/specification/adr/adr-308.md)). Kap 7.1
                // gives the failure that name whether or not the handler wants
                // it, so `catch { 1000 }` - the shape Part I 7.1 teaches first -
                // used to get `warning: unused variable: error` about a binding
                // that exists nowhere in the program, which is
                // [Part III C.1](../../docs/specification/30-nikaia-tooling.md)
                // one severity down.
                //
                // The question is `contracts::order`'s, already over-approximate
                // and already asked of this very block for the ordering: a false
                // *yes* keeps today's binding, and a false *no* would not
                // compile.
                let bound =
                    match crate::contracts::order::block_mentions(self.parsed, handler, CAUGHT) {
                        true => CAUGHT,
                        false => "_error",
                    };
                // **What the handler was handed**
                // ([ADR-280](../../docs/specification/adr/adr-280.md) D10). A
                // named channel hands it an **envelope** — the author's error
                // plus the site it was raised at — and what Part I 7.1 gives
                // the handler is the error: `match error { ConfigError::… }`
                // names variants, and `{error}` is the message the author
                // wrote. So the envelope is opened **once, at the binding**,
                // and the site is kept in a local beside it for the one
                // statement that needs it back (`throw error`, D3).
                let named = bound == CAUGHT && self.caught_channel_is_named(expr);
                let sum = match bound == CAUGHT {
                    true => self.caught_sum(expr),
                    false => None,
                };
                let flow = flow.handling(named).catching(sum.as_deref());
                // **The envelope is opened as the handler's first statement**
                // ([ADR-292](../../docs/specification/adr/adr-292.md) D20), and
                // not as a block wrapped around it. A block holding one
                // expression, inside another block, is `unused_braces` — a
                // warning about a file nobody wrote, which is
                // [Part III C.1](../../docs/specification/30-nikaia-tooling.md)
                // one severity down, and `catch { println(f"{error}") }` over a
                // named channel is all it took.
                let opening = named.then(|| format!("let ({CAUGHT}, {SITE}) = {CAUGHT}.split();"));
                out.push(&format!(
                    " {{\n{pad}Ok(value) => value,\n{pad}Err({bound}) => "
                ));
                // Kap 7.1 and ADR-292: the handler's last statement is the
                // value of the `catch`, so a `return` in it is the *function's*
                // return and has to stay one. `contracts::order`'s `diverts`
                // counts exactly that `return` when it refuses to overlap the
                // guarded read, so dropping it here made the ordering analysis
                // reason about a control flow the emitted program did not have.
                self.block_opening_with(
                    out,
                    handler,
                    depth + 1,
                    flow,
                    Tail::Value,
                    opening.as_deref(),
                )?;
                out.push(&format!(",\n{close}}}"));
            }
            Expr::Try(inner, _) => {
                self.postfix_base(out, inner, depth, flow)?;
                out.push("?");
            }
            // Kap 7.1: `throw e` leaves the function with `e` in the failure
            // channel. `Box::new` is what puts it there, and it is the emitter's
            // to write rather than the author's - ADR-023 D2 gives the language
            // one way to raise an error, not one way plus a conversion.
            // Kap 7.1: `throw e` leaves the function with `e` in the failure
            // channel. `raise` is what puts it there, and it carries the site
            // the compiler knew and the author did not have to write (D6). The
            // trace it may attach is off unless the program asked - measured at
            // 28 300 instructions an error, which is not a default.
            // **The four jumps, written as the language below writes them**
            // ([ADR-276](../../../docs/specification/adr/adr-276.md) D20): each
            // is `!` there too, so an arm that returns beside an arm that hands
            // back a value is a `match` of that value's type by the backend's
            // own rule and not by a coercion this had to write.
            // **What it hands back is wrapped by its own address**
            // (`check::Checked::wrapped`, read in [`Emitter::expr`]): the
            // statement it stands in is a `let`'s or a call's, and that
            // statement's wrap is theirs.
            Expr::Return(value, _) => match (&**value, flow.throws) {
                (Some(value), true) => {
                    out.push("return Ok(");
                    self.expr(out, value, depth, flow)?;
                    out.push(")");
                }
                (Some(value), false) => {
                    out.push("return ");
                    self.expr(out, value, depth, flow)?;
                }
                (None, true) => out.push("return Ok(())"),
                (None, false) => out.push("return"),
            },
            Expr::Break(_) | Expr::Continue(_) => {
                let word = match expr {
                    Expr::Break(_) => "break",
                    _ => "continue",
                };
                // **The same refusal the statement form has**
                // ([ADR-276](../../../docs/specification/adr/adr-276.md) D18):
                // the lowering states it too, so a path that reached here
                // without the checker's answer still writes no file `rustc`
                // would refuse.
                if !flow.in_loop {
                    return Err(refused_at!(
                        flow.statement,
                        "`{word}` only works inside a loop, and there's none here."
                    ));
                }
                out.push(word);
            }
            Expr::Throw(inner, _) => {
                // **`throw error` inside a handler passes the error on**
                // ([ADR-280](../../docs/specification/adr/adr-280.md) D11), with
                // the site it was **raised** at and not the one it was caught
                // at, which is what
                // [ADR-023](../../docs/specification/adr/adr-023.md) D6 asks
                // for. The handler's binding opened the envelope; this is where
                // it is closed again.
                let is_the_binding =
                    matches!(inner.as_ref(), Expr::Variable(name, _) if self.text(*name) == CAUGHT);
                let passing_on = flow.caught_named && is_the_binding && flow.caught_sum.is_none();
                if passing_on || (flow.caught_member.is_some() && is_the_binding) {
                    // **Back into the variant it came out of**, where the
                    // handler is inside a sum's member arm
                    // ([ADR-280](../../docs/specification/adr/adr-280.md) D17).
                    let put_back = |inner: String| match (flow.caught_sum, flow.caught_member) {
                        (Some(sum), Some(member)) => format!(
                            "{}::{}({inner})",
                            Self::sum_path(sum),
                            Self::sum_variant(member)
                        ),
                        _ => inner,
                    };
                    let held = match flow.caught_named {
                        true => format!("{SITE}.refill({CAUGHT})"),
                        false => CAUGHT.to_string(),
                    };
                    out.push(&format!("return Err({})", put_back(held)));
                    return Ok(());
                }
                // Three channels and three ways in. The **envelope** takes the
                // value and the site; the **box** takes it boxed, which is what
                // `raise` does and what needs `'static`; and a **library's
                // type** takes the value alone, because a channel that carries
                // no envelope has nowhere to put a site
                // ([ADR-280](../../docs/specification/adr/adr-280.md) D13).
                // **A sum takes the member it is**
                // ([ADR-280](../../docs/specification/adr/adr-280.md) D16), and
                // `into()` is what picks the variant: the `From` beside the
                // generated `enum` is written per member, so the thrown value
                // says which one it is by its own type.
                if self.sum_of(flow.function).is_some() {
                    let thrown = crate::contracts::throws::error_type(self.parsed, inner);
                    let own = thrown
                        .as_deref()
                        .and_then(|t| self.name_of_error(t))
                        .is_some();
                    match own {
                        true => {
                            out.push("return Err(nikaia_std::error::throwing(");
                            self.expr(out, inner, depth, flow)?;
                            out.push(&format!(", &{:?}).into())", flow.origin));
                        }
                        false => {
                            out.push("return Err(");
                            self.expr(out, inner, depth, flow)?;
                            out.push(".into())");
                        }
                    }
                    return Ok(());
                }
                // **A `throw` has a site whoever declared the error**
                // ([ADR-280](../../docs/specification/adr/adr-280.md) D14): a
                // library's error thrown by this program was raised here.
                match self.named_error(flow.function) {
                    Some(Named::Own(_) | Named::Library(_)) => {
                        out.push("return Err(nikaia_std::error::throwing(")
                    }
                    None => out.push("return Err(nikaia_std::error::raise("),
                }
                self.expr(out, inner, depth, flow)?;
                out.push(&format!(", &{:?}))", flow.origin));
            }
            // **Part I 8.2 and ADR-055 D5: a task.**
            //
            // `spawn fn { … }` is `TaskHandle::start(async move { … })`. Three
            // things are in that one line and each is a decision somewhere
            // else:
            //
            //   * an `async` **block** and not a closure, because a body is a
            //     block: wrapping it in a closure to call it once adds a call
            //     and takes nothing away - the same reason every vehicle in
            //     `task` takes futures (§6 step 3). Not because the language
            //     below has no `async` closure, which is what this line used to
            //     say and is false
            //     ([ADR-277](../../docs/specification/adr/adr-277.md) D13, D16);
            //   * `move`, which is Part I 8.3's implicit move: the captures go
            //     with the task because it may outlive the function that
            //     started it, and `NK2101` is what stands in front of that for
            //     data the parent still wanted;
            //   * a **handle**, always, whether or not the program keeps it. A
            //     task nobody joins still runs (D5), because the executor owns
            //     it - so an unused handle is a value dropped and not work
            //     cancelled.
            //
            // The lambda's `params` are not written: a task is handed nothing,
            // so a `spawn fn (x) { … }` has no argument to bind. `check`
            // refuses that rather than dropping it silently.
            Expr::Spawn { body, .. } => {
                let Expr::Closure { body, .. } = body.as_ref() else {
                    return Err(refused_at!(
                        flow.statement,
                        "`spawn` takes a lambda. Write `spawn fn {{ … }}`."
                    ));
                };
                // **And which of the two starters**, which is the one place
                // `user_parallelism` reaches a `spawn` (ADR-037 D2). At `yes` a
                // task may be polled on a thread that did not start it, so its
                // future has to be `Send` (§2 D6); at `no` it may not, so it
                // does not - and asking for `Send` there would refuse a task
                // holding the plain count ADR-312 D9 gives a `Shared` at one
                // user thread. One Nikaia line, two lowerings, and the switch
                // is what chooses.
                // **A handle the body names is duplicated, not moved**
                // ([ADR-312](../../../docs/specification/adr/adr-312.md) D1).
                // The step is written *outside* the future, in a block of its
                // own, so the name the body moves is the new handle and the
                // caller's own survives the `spawn` - which is the whole of D1
                // for the task half. Unconditional (D2): a line further down may
                // not decide what a line further up does to a cleanup point.
                let handles: Vec<&str> = self
                    .s
                    .facts
                    .task_handles
                    .iter()
                    .filter(|(at, _)| *at == (flow.statement as i64))
                    .map(|(_, name)| name.as_str())
                    .collect();
                if !handles.is_empty() {
                    out.push("{ ");
                    for name in &handles {
                        out.push(&format!("let {name} = {name}.clone(); "));
                    }
                }
                match self.build.overlaps_user_code() {
                    true => out.push("nikaia_std::task::TaskHandle::start_on_pool(async move "),
                    false => out.push("nikaia_std::task::TaskHandle::start(async move "),
                }
                // `Flow::PLAIN`, like any other lambda body: a `return` inside
                // the task leaves the task, and a failure in it is the task's.
                // `in_lambda` is deliberately **not** set - the body is a
                // future, so a call that pauses is exactly what it may hold.
                let inside = Flow {
                    statement: flow.statement,
                    ..Flow::PLAIN
                };
                self.block(out, body, depth, inside, Tail::Return)?;
                out.push(")");
                if !handles.is_empty() {
                    out.push(" }");
                }
            }
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
        // **`T::attribute(X)` on the type this copy is of** (ADR-331 D6): its
        // own attributes, as a field's are read.
        if let Expr::Path(segments, _) = func
            && let [parameter, member] = segments.as_slice()
            && matches!(self.text(*member), "attribute" | "attributes")
            && let Some((walked, on)) =
                self.walk(|w| w.specialising.clone().map(|pair| (pair.first, pair.second)))
            && walked == self.text(*parameter)
        {
            let texts = self
                .type_contract(&on)
                .map(|c| c.attributes)
                .unwrap_or_default();
            out.push(&self.attributes_read(self.text(*member), args, &texts));
            return Ok(());
        }
        // **A call to a function with a precondition** reaches its checked
        // entry unless the prover saw this call prove it, or check it here
        // with the call's arguments in place of the parameters, so that a
        // failure names the caller
        // ([ADR-269](../../docs/specification/adr/adr-269.md) D20). The
        // arguments of a checked call are ones the prover reads, so reading
        // them once more changes nothing.
        // A function of this file with a checked entry, or one of another
        // package whose ledger states a precondition: that package was
        // lowered with the two entries (ADR-269 D18, D20).
        let callee = match func {
            Expr::Variable(name, _) if self.s.facts.entries.contains_key(self.text(*name)) => {
                Some(self.text(*name).to_string())
            }
            Expr::Path(segments, _) => {
                let qualified = self.parsed.unaliased(
                    &segments
                        .iter()
                        .map(|s| self.text(*s))
                        .collect::<Vec<_>>()
                        .join("::"),
                );
                self.s
                    .own_contracts
                    .functions
                    .get(&qualified)
                    .is_some_and(|c| !c.requires.is_empty())
                    .then_some(qualified)
            }
            _ => None,
        };
        if let Some(callee) = callee
            && let Some(reach) =
                self.s
                    .facts
                    .reaches
                    .get(&crate::prove::call_key(self.parsed, &callee, args))
        {
            match reach {
                crate::prove::Reach::Proved => {
                    return self.call_inner(out, func, args, config, depth, flow, true);
                }
                crate::prove::Reach::Checked(checks) => {
                    // `(check, call).1`: an expression in every position the
                    // call could stand in, which a block at the start of a
                    // statement is not.
                    out.push(if checks.len() == 1 { "(" } else { "({ " });
                    for check in checks {
                        out.push(&failed_check(check));
                    }
                    out.push(if checks.len() == 1 { ", " } else { "}, " });
                    self.call_inner(out, func, args, config, depth, flow, true)?;
                    out.push(").1");
                    return Ok(());
                }
                crate::prove::Reach::Through => {}
            }
        }
        self.call_inner(out, func, args, config, depth, flow, false)
    }

    /// A call, past [`Self::call`]'s precondition check: to the unchecked
    /// entry where `unchecked` says so (ADR-269 D20).
    #[allow(clippy::too_many_arguments)]
    fn call_inner(
        &self,
        out: &mut Out,
        func: &Expr,
        args: &[Expr],
        config: &[crate::ast::ConfigArg],
        depth: usize,
        flow: Flow<'_>,
        unchecked: bool,
    ) -> Result<()> {
        // **An `assert` is a claim checked here** (ADR-269 D2, D4) unless it
        // was proved, and the failure is
        // written with what only this compiler knew - the claim as written
        // and its operands by name. The condition changes nothing (D3), which
        // is what makes reading an operand a second time, for the message,
        // the same value it compared.
        if let ([condition], Expr::Variable(name, _)) = (args, func)
            && self.text(*name) == crate::check::ASSERT
            && let Some(claim) = self
                .s
                .facts
                .claims
                .get(&((flow.statement as i64), crate::check::value_node(condition)))
        {
            // **A claim held before the program runs costs nothing when it
            // does** ([ADR-269](../../docs/specification/adr/adr-269.md) D4,
            // D5): a precondition is checked by the calls that do not prove
            // it, and every other claim here.
            if matches!(
                claim.held,
                Some(crate::prove::Held::Proved | crate::prove::Held::Precondition(..))
            ) {
                out.push("()");
                return Ok(());
            }
            out.push("if !(");
            self.expr(out, condition, depth, flow)?;
            out.push(&format!(
                ") {{ nikaia_std::abort::assertion_failed({:?}, ",
                claim.written
            ));
            match config.iter().find(|c| self.text(c.name) == "message") {
                Some(message) => {
                    out.push("Some(&(");
                    self.expr(out, &message.value, depth, flow)?;
                    out.push(")), ");
                }
                None => out.push("None, "),
            }
            out.push("&[");
            if let Expr::Binary { lhs, rhs, .. } = condition {
                let shown = [(&claim.left, lhs), (&claim.right, rhs)];
                for (label, operand) in shown {
                    let Some(label) = label else { continue };
                    out.push(&format!("({label:?}, (&&nikaia_std::abort::Operand(&("));
                    self.expr(out, operand, depth, flow)?;
                    out.push("))).shown()), ");
                }
            }
            out.push("]) }");
            return Ok(());
        }
        // **A kept function value called by name** is the closure it holds,
        // awaited where its type may pause and propagated where it may fail.
        if let Expr::Variable(name, _) = func {
            let key = (flow.statement as i64, self.text(*name).to_string());
            if let Some((pauses, fails)) = self.s.facts.kept_calls.get(&key).copied() {
                out.push(&format!("{}(", self.name(*name)));
                for (at, arg) in args.iter().enumerate() {
                    if at > 0 {
                        out.push(", ");
                    }
                    self.expr(out, arg, depth, flow)?;
                }
                out.push(")");
                if pauses {
                    out.push(".await");
                }
                if fails && flow.throws && !flow.caught {
                    out.push("?");
                }
                return Ok(());
            }
        }
        // ADR-055 D6: a recursive `async fn` is an infinitely sized future, and
        // Rust says so rather than guessing - so a call that closes a cycle
        // through pausing functions puts the future behind a pointer.
        // `pausing_reach` is computed once per unit (see that function for why
        // it over-approximates), and the emitter only looks names up in it: it
        // still resolves nothing (ADR-296 D17).
        // **A grammar is entered by an ordinary call**
        // ([ADR-296](../../docs/specification/adr/adr-296.md) D24), through a
        // **path** since [ADR-140](../../docs/specification/adr/adr-140.md) D3:
        // `Json::value(input)`, where `Json` names a grammar in this file and
        // `value` one of its `entry` rules. First, because it is not a call to
        // anything the recursion graph or the ledger knows.
        if let Expr::Path(segments, _) = func
            && let [grammar, rule] = segments.as_slice()
            && self.s.grammars.contains_key(self.text(*grammar))
            && args.len() == 1
        {
            return self.grammar_entry(out, *grammar, *rule, &args[0], depth, flow);
        }
        // **A package's `pub grammar`, entered by its path** (ADR-296 D25,
        // #520): `lib::Nums::number(input)` is the entry the package's ledger
        // records - a `ParseError` its failure and the text its one input -
        // and its crate's parser is driven as a grammar of this file's is.
        if let Expr::Path(segments, _) = func
            && let [package, grammar, rule] = segments.as_slice()
            && args.len() == 1
            && self.a_package_grammar_entry(&format!(
                "{}::{}::{}",
                self.text(*package),
                self.text(*grammar),
                self.text(*rule)
            ))
        {
            let question = match flow.caught {
                false => "?",
                true => "",
            };
            let (package, grammar, rule) =
                (self.text(*package), self.text(*grammar), self.text(*rule));
            // **A `par_fold` entry is driven in pieces** (#521), as its own
            // file drives it, with this build's parallelism.
            let pieces = self
                .s
                .own_contracts
                .functions
                .get(&format!("{package}::{grammar}::{rule}"))
                .is_some_and(|contract| contract.pieces);
            if pieces {
                out.push("nikaia_std::grammar::parse_pieces(&*");
                self.expr(out, &args[0], depth, flow)?;
                // Spelled out: this file has no `par_fold` of its own, so the
                // two names are not imported here.
                out.push(&format!(
                    ", |source| {package}::{grammar}::parse_{rule}_pieces(source, \
                     &winnow_grammar::ParseContext::<()>::default(), winnow_grammar::rt::{})){question}",
                    self.build.parallelism()
                ));
                return Ok(());
            }
            out.push("nikaia_std::grammar::parse(&*");
            self.expr(out, &args[0], depth, flow)?;
            out.push(&format!(
                ", {package}::{grammar}::parse_{rule}()){question}"
            ));
            return Ok(());
        }
        // **A variant with a boxed part is built with the part in its box**
        // ([ADR-246](../../docs/specification/adr/adr-246.md) D3). The mask
        // is `args`' to read, so every other thing an argument is written
        // with still happens inside the box.
        if let Expr::Path(segments, _) = func
            && let [.., owner, variant] = segments.as_slice()
        {
            let (owner, variant) = (self.text(*owner), self.text(*variant));
            let mask: Vec<bool> = (0..args.len())
                .map(|at| {
                    self.is_boxed(owner, &crate::check::boxed_member(variant, Some(at), None))
                })
                .collect();
            let holds_a_hull = self.is_variant(owner, variant)
                && args.iter().any(|arg| {
                    matches!(arg, Expr::Call { func, .. }
                        if matches!(func.as_ref(), Expr::Variable(name, _)
                            if matches!(self.text(*name), SHARED | SHARED_MUT | LOCKED)))
                });
            if mask.iter().any(|b| *b) || holds_a_hull {
                self.walking(|w| w.boxed_args = Some(mask));
                let counts = (0..args.len())
                    .map(|at| self.count_at(SHARED_FIELDS, &variant_slot(owner, variant, at)))
                    .collect();
                self.walking(|w| w.variant_counts = Some(counts));
                out.push(&self.path(&segments.iter().map(|s| self.text(*s)).collect::<Vec<_>>()));
                out.push("(");
                let written = self.args(out, variant, args, &[], depth, flow);
                self.walking(|w| w.boxed_args = None);
                self.walking(|w| w.variant_counts = None);
                written?;
                out.push(")");
                return Ok(());
            }
        }
        let pausing = self.pausing_key(func);
        if let Some(key) = pausing.as_deref().filter(|_| flow.in_lambda) {
            return Err(pausing_in_a_lambda(flow.statement, key));
        }
        // **A call to a code parameter whose type may pause**
        // ([ADR-277](../../docs/specification/adr/adr-277.md) D11). It hands back
        // a boxed future, so it is awaited — and it is never *boxed* here: a
        // parameter is not a name the recursion graph knows, and what it holds
        // is already behind a pointer.
        let awaits_a_parameter =
            matches!(func, Expr::Variable(name, _) if flow.awaited.contains(name));
        // **A call of a `sync(f)` function** (ADR-288 D31): awaited where the
        // caller can pause, and driven once where it cannot - the checker has
        // then proved the lambda handed to it does not pause, so neither does
        // the call.
        let decided = pausing.is_none() && self.decided_by_its_lambda(func);
        let settled = decided && (flow.in_lambda || !self.lowered_pausing(flow.function));
        if settled {
            out.push("nikaia_std::rt::exec::settled(");
        }
        let boxed = pausing
            .as_deref()
            .is_some_and(|key| self.closes_a_pausing_cycle(flow.function, key));
        if boxed {
            out.push("Box::pin(");
        }

        // **A handle is made at the call and nowhere else**
        // ([ADR-302](../../docs/specification/adr/adr-302.md) D11, D12). The
        // declaration hands back the address C returned; the hull goes on here,
        // where the claim the declaration made can be checked — or, where it
        // said `?`, where `None` is C's own `NULL`.
        let hull = self.handle_from(func);
        if let Some((handle, absent)) = &hull {
            out.push(&match absent {
                true => format!("{handle}::maybe("),
                false => format!("{handle}::from_c(\"{}\", ", self.called_name(func)),
            });
        }

        self.called(out, func, args, config, depth, flow, unchecked)?;

        if hull.is_some() {
            out.push(")");
        }

        if boxed {
            out.push(")");
        }

        // **ADR-055 D2: the `.await` goes before the `?`**, and the order is not
        // a choice: the future is what can fail, so it has to be driven before
        // there is a `Result` to propagate. `f().await?` and never `f()?.await`.
        if pausing.is_some() || awaits_a_parameter || (decided && !settled) {
            out.push(".await");
        }
        if settled {
            out.push(")");
        }

        // ADR-023 D8: `throws` propagates on its own, so a call to something
        // that can fail is where the failure leaves - and in the language below
        // that is spelled `?`. This is the written-call twin of the `?` a
        // fallible loop's step gets above: the same rule, one line earlier
        // (ADR-025 D1).
        //
        // Three conditions, and each removes a way of being wrong. The
        // enclosing function must be `throws`, or there is nowhere for the `?`
        // to go - and `NK2605` has already refused the program where it is
        // not, so this is not a silent choice. The call must not be the
        // guarded half of a `catch`, which wants the `Result` itself. And the
        // callee's contract must **say** it can fail, so nothing is added on a
        // guess: a call no ledger describes is left exactly as it was.
        //
        // A lambda's body is emitted with `Flow::PLAIN`, so a fallible call
        // inside one never takes a `?` from the function around it - which is
        // right, because its `return` leaves the lambda and not the function.
        if flow.throws && !flow.caught && self.can_fail(func) {
            out.push("?");
        }
        Ok(())
    }

    /// The call itself, without ADR-023 D8's propagation.
    #[allow(clippy::too_many_arguments)]
    fn called(
        &self,
        out: &mut Out,
        func: &Expr,
        args: &[Expr],
        config: &[crate::ast::ConfigArg],
        depth: usize,
        flow: Flow<'_>,
        unchecked: bool,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_called(
                &self.world,
                walk,
                out,
                self,
                func,
                args,
                config,
                unchecked,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// ADR-296 D5: the shadow struct a call to a DSL driver builds.
    ///
    /// A `;` at a call site means options everywhere else, and the callee's
    /// declaration is what tells the two apart: a driver said
    /// `...args: Self::dsl`, so what stands after its `;` is a DSL's deferred
    /// parameters and becomes one value rather than one argument each.
    ///
    /// A struct literal names its fields, so a call may write them in any
    /// order - the body's order decides the type's fields and nothing else.
    fn dsl_parameters(
        &self,
        out: &mut Out,
        callee: &str,
        args: usize,
        config: &[crate::ast::ConfigArg],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_dsl_parameters(
                &self.world,
                walk,
                out,
                self,
                callee,
                args as i64,
                config,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// Kap 5.1 at a **method** call: the options become positional, in the order
    /// the declaration gives, and one that was left out becomes its default.
    ///
    /// The same thing `call` does from `options_of`, off the checker's answer
    /// instead ([ADR-288](../../../docs/specification/adr/adr-288.md)) — and a
    /// DSL driver is not this: what stands after *its* `;` is the typed spread
    /// of [ADR-296](../../../docs/specification/adr/adr-296.md) D5, which
    /// `dsl_parameters` has already written as one value.
    fn method_options(
        &self,
        out: &mut Out,
        method: Symbol,
        args: usize,
        config: &[crate::ast::ConfigArg],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_method_options(
                &self.world,
                walk,
                out,
                self,
                method,
                args as i64,
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

    /// Whether a call from `caller` to `callee` closes a cycle of pausing
    /// functions, and so needs its future behind a pointer (ADR-055 D6).
    ///
    /// **The question is about the call and not about the callee.** A recursive
    /// `countdown` boxes the call it makes to itself; the call `main` makes to it
    /// holds that future exactly once and is finite, so it pays no allocation.
    /// Asking it the other way - boxing every call to anything recursive - was
    /// the first version, and it charged every caller for its callee's shape.
    ///
    /// `A -> A` beside `B -> B` where `B` also calls `A` is what makes "both are
    /// in a cycle" the wrong test: nothing about `B -> A` is recursive. So the
    /// test is that the callee can reach the caller again, which is what closing
    /// a cycle *is*.
    fn closes_a_pausing_cycle(&self, caller: &str, callee: &str) -> bool {
        self.s.closes_a_pausing_cycle(caller, callee)
    }

    /// The same, for a call whose callee this emitter cannot name
    /// ([ADR-055](../../docs/specification/adr/adr-055.md) D6, the method
    /// half).
    ///
    /// `stats.add(5)` names `add` and says nothing about what `stats` is
    /// ([ADR-288](../../docs/specification/adr/adr-288.md)), so the candidates
    /// are **every** pausing key ending in that name — the same widening
    /// `pausing_reach` draws its edges with, asked here at the call. Where the
    /// two readings differ this takes the wider, for that function's own
    /// reason: a box nobody needed costs one allocation, and a box that was
    /// needed and is missing is a program that does not compile.
    fn method_closes_a_pausing_cycle(&self, caller: &str, method: &str) -> bool {
        self.s.method_closes_a_pausing_cycle(caller, method)
    }

    /// Whether a type named here holds a view, and so carries the input lifetime
    /// wherever it is written (Part II, 10.6).
    ///
    /// Two sources and one answer: the file's own declarations, and the
    /// package's ledger for the types another file declares
    /// ([`Emitter::tethered`]).
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

    /// The **one error type** a function's failures can be, where the ledger
    /// names exactly one ([ADR-280](../../docs/specification/adr/adr-280.md)
    /// D1).
    ///
    /// `throws` in the source says *that* a function fails; the ledger's set
    /// says *with what*, derived over the call graph
    /// ([ADR-023](../../docs/specification/adr/adr-023.md) D1). Where that set
    /// has exactly one member and the member is a type **this unit declares**,
    /// the failure channel is that type, and a `catch` can match on its
    /// variants — which is Part I 7.1's own example and what a box makes
    /// impossible.
    ///
    /// `None` is the box, and it is the answer for everything else: a `"?"` in
    /// the set is *something this compiler cannot name*, a set with two members
    /// needs the generated sum that is not built, and a type another package
    /// declares is not this unit's to name in a signature.
    fn named_error(&self, key: &str) -> Option<Named<'_>> {
        self.named_error_of(&self.s.own_contracts.functions.get(key)?.fails_with)
    }

    /// [`Emitter::named_error`] asked of the **set** rather than of a function.
    ///
    /// A joining block has a set of its own — the union of what its branches
    /// throw — and no ledger key to look it up under
    /// ([ADR-292](../../docs/specification/adr/adr-292.md) D19).
    fn named_error_of<'t>(&'t self, throws: &'t [String]) -> Option<Named<'t>> {
        match self.s.named_error_of(&self.world, throws) {
            1 => Some(Named::Own(throws[0].as_str())),
            2 => Some(Named::Library(throws[0].as_str())),
            _ => None,
        }
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

    /// The sum an expression a `catch` guards fails in, where it is one.
    ///
    /// The same question [`Emitter::caught_channel_is_named`] asks, one channel
    /// over, and answered the same way: only the outermost call, and only a
    /// call by name.
    fn caught_sum(&self, expr: &Expr) -> Option<String> {
        self.s.caught_sum(&self.world, expr)
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

    fn sum_of(&self, key: &str) -> Option<String> {
        self.s.sum_of(key)
    }

    /// Whose one error type a **member** of a set is, asked of the name alone.
    fn name_of_error<'n>(&'n self, name: &'n str) -> Option<Named<'n>> {
        match self.s.name_of_error(&self.world, name) {
            1 => Some(Named::Own(name)),
            2 => Some(Named::Library(name)),
            _ => None,
        }
    }

    /// The variant a member is written as: its name with the module flattened,
    /// because a variant is one identifier.
    fn sum_variant(name: &str) -> String {
        sum_variant(name)
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

    /// How a sum is **named** wherever it is used.
    ///
    /// `crate::…`, always: the type is defined once at the crate root
    /// ([`Emitter::write_error_sums`]), and a module is a file of its own below,
    /// so a bare name would be a different type in each of them — and a failure
    /// that crosses a module boundary would have nowhere to go.
    fn sum_path(name: &str) -> String {
        format!("crate::{name}")
    }

    fn caught_channel_is_named(&self, expr: &Expr) -> bool {
        self.s.caught_channel_is_named(&self.world, expr)
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

    /// Whether the function under `key` is lowered as an `async fn`.
    fn lowered_pausing(&self, key: &str) -> bool {
        self.s.lowered_pausing(key)
    }

    /// Whether a call names an own function written `sync(f)`.
    fn decided_by_its_lambda(&self, func: &Expr) -> bool {
        self.s.decided_by_its_lambda(&self.world, func)
    }

    fn pausing_key(&self, func: &Expr) -> Option<String> {
        nikaia_std::tools::emit_calls::call_pausing_key(
            &self.parsed.interner,
            &self.parsed.aliases,
            &self.s.own_contracts,
            self.world.library,
            func,
        )
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

    /// Whether the contracts say a call by name can fail (Kap 7.1).
    ///
    /// The same resolution `options_of` uses, and for the same reason: the
    /// callee's contract is the fact, and this unit's own comes before `std`'s
    /// because a local name shadows nothing in a library. `false` where the
    /// callee cannot be resolved - which is the emitter *not guessing* rather
    /// than an answer, and is safe here because the checker has already
    /// reported every resolvable fallible call the function did not declare
    /// (`NK2605`); a call nothing describes reaches `rustc` as before.
    ///
    /// A **method** call is not asked here, because the answer is not the
    /// emitter's to work out: `stats.add(5)` names `add` and only the type
    /// checker knows what it goes to (ADR-288). `method_can_fail` reads the
    /// answer the checker handed over instead.
    fn can_fail(&self, func: &Expr) -> bool {
        nikaia_std::tools::emit_calls::call_can_fail(
            &self.parsed.interner,
            &self.s.own_contracts,
            self.world.library,
            &self.s.described,
            func,
        )
    }

    /// Whether the checker said this conversion narrows, and which kind it is.
    ///
    /// A lookup, not an analysis: `as i32` narrows or widens by what it is given,
    /// and nothing here knows that (ADR-288). `None` where the checker had no
    /// answer - a type it does not recognise, or two conversions to the same type
    /// in one statement where one of them widens - which leaves the conversion
    /// exactly as it was rather than guessing at it.
    fn narrows(&self, statement: usize, into: &str) -> Option<crate::check::Narrowing> {
        self.s.narrows(statement as i64, into)
    }

    /// Whether the checker said this method call can fail (Kap 7.1).
    ///
    /// A lookup, not an analysis: `fallible_methods` was computed by the one
    /// type checker this compiler has, against the same ledgers the program is
    /// built with, and nothing here resolves a receiver type. `false` where the
    /// checker had no answer - an unresolved receiver, a method no ledger
    /// describes, or two calls in one statement writing the same name where one
    /// of them cannot fail - which is the emitter leaving the call exactly as it
    /// was rather than guessing at it.
    /// **A call of a field that holds a function**, written through the
    /// field: `(button.on_click)(4)`, awaited where it may pause - `false`,
    /// and nothing written, where the checker recorded no such call. `reached`
    /// is the safe form's (#533): the field is read off [`REACHED`], the name
    /// the enclosing `match` bound, and `receiver` only keys the record.
    #[allow(clippy::too_many_arguments)]
    fn a_field_call(
        &self,
        out: &mut Out,
        receiver: &Expr,
        reached: bool,
        method: Symbol,
        args: &[Expr],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<bool> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_a_field_call(
                &self.world,
                walk,
                out,
                self,
                receiver,
                reached,
                method,
                args,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    fn method_can_fail(&self, flow: Flow<'_>, method: Symbol) -> bool {
        self.s
            .method_can_fail(flow.statement as i64, self.text(method))
    }

    /// Whether the checker said this method call pauses (ADR-055 D2).
    ///
    /// [`Emitter::method_can_fail`]'s twin, and a lookup for the same reason:
    /// the answer was computed once by the one type checker this compiler has,
    /// and `false` here is the emitter leaving the call as it was rather than
    /// guessing at it.
    fn method_pauses(&self, flow: Flow<'_>, method: Symbol) -> bool {
        self.s
            .method_pauses(flow.statement as i64, self.text(method))
    }

    /// The options a call by name has, from the contract that declares them.
    ///
    /// This unit's own first, then `std`'s - the order every other name
    /// resolution here uses. `None` where the callee has none, and where the
    /// callee cannot be resolved at all: an unresolvable call has no options to
    /// fill in, and the type checker is what says so in Nikaia's words.
    /// **What `.attribute(X)` or `.attributes(X)` comes to** over the values
    /// `texts` (ADR-331 D6): `Some(…)` or `None`, or the list. Typed where it
    /// is empty: nothing else on the line says what it would hold.
    fn attributes_read(&self, wanted: &str, args: &[Expr], texts: &[String]) -> String {
        self.s.attributes_read(&self.world, wanted, args, texts)
    }

    /// A type's contract, from this package's ledger or a dependency's.
    fn type_contract(&self, owner: &str) -> Option<crate::contracts::TypeContract> {
        self.s.type_contract(&self.world, owner)
    }

    /// **A reflected field's contract**, from this package's ledger or a
    /// dependency's: what `.attribute(X)` and `.default` read (ADR-331 D5-D6).
    fn field_contract(&self, owner: &str, field: &str) -> Option<crate::contracts::FieldContract> {
        self.s.field_contract(&self.world, owner, field)
    }

    /// **The fields of `owner` that have a default** (ADR-331 D5): name,
    /// value and type, from this package's ledger or a dependency's.
    fn field_defaults(&self, owner: &str) -> Vec<(String, String, crate::contracts::ty::Ty)> {
        self.s.field_defaults(&self.world, owner)
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
    fn boxed_future(&self) -> String {
        nikaia_std::tools::emit_core_types::emit_boxed_future(self.build.overlaps_user_code())
    }

    /// One arm's pattern. Every shape is one the language below spells the
    /// same way, so this is a transcription rather than a translation - a bare
    /// name binds here because it binds there, and the rule is drawn once.
    /// **Whether a member is boxed below**
    /// ([ADR-246](../../docs/specification/adr/adr-246.md) D1), by the
    /// owner's last segment: a qualified name reaches the same type.
    fn is_boxed(&self, owner: &str, member: &str) -> bool {
        self.s.is_boxed(owner, member)
    }

    /// **A field of a struct literal**, which may be a variant's written as a
    /// struct: `Expr::Neg { inner: … }` is the variant `Neg` of `Expr`, and its
    /// member is `Neg.inner`.
    fn is_boxed_field(&self, owner: &str, field: &str) -> bool {
        self.s.is_boxed_field(owner, field)
    }

    /// A member's lowered type, in its box where it has one.
    fn boxed_ty(&self, owner: &str, member: &str, lowered: String) -> String {
        self.s.boxed_ty(owner, member, lowered)
    }

    /// **A method call, for both of the ways a program may write one.**
    ///
    /// `x.m(…)` and `x?.m(…)` differ in *whether* the call happens and in
    /// nothing else ([ADR-278](../../../../docs/specification/adr/adr-278.md)),
    /// so what a call lowers to is written once: the truncating conversion, the
    /// `len` that becomes an `i64`, `collect`'s turbofish, the arguments, Kap
    /// 5.1's deferred parameters, ADR-055 D2's `.await` and ADR-023 D8's `?`.
    ///
    /// `receiver` is `None` for the safe form, where the receiver is the name
    /// the enclosing `match` already bound - so the call is written on
    /// [`REACHED`] rather than on an expression, and the value is reached
    /// exactly once whatever the receiver cost to produce.
    ///
    /// Eight arguments, and each is a part of the call this has to write: the
    /// receiver, the name, the positional arguments, Kap 5.1's zone, and the
    /// three the emitter carries everywhere (`out`, `depth`, `flow`). Splitting
    /// them into a struct would name a shape the AST already has.
    #[allow(clippy::too_many_arguments)]
    fn method_call(
        &self,
        out: &mut Out,
        receiver: Option<&Expr>,
        method: Symbol,
        args: &[Expr],
        config: &[crate::ast::ConfigArg],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        // **`text.chars().collect()` is a list of characters asked for at its
        // size** (`nikaia_std::list::chars`): the same list, without the
        // growing a `collect` over `Chars` does. Only where nothing declares
        // another thing to build.
        if let Some(Expr::MethodCall {
            receiver: text,
            method: chars,
            args: none,
            ..
        }) = receiver
            && self.text(method) == "collect"
            && args.is_empty()
            && none.is_empty()
            && self.text(*chars) == "chars"
            && !self
                .s
                .facts
                .collected_into
                .contains(&(crate::check::value_node(receiver.unwrap())))
        {
            out.push("nikaia_std::list::chars(");
            self.postfix_base(out, text, depth, flow)?;
            out.push(".chars())");
            return Ok(());
        }
        // **`x.truncating_i32()` is Rust's `as`** (ADR-285 D13): the
        // operation the name says. It is a name and not the operator
        // because keeping the low digits is said rather than assumed,
        // exactly as wrapping is (D2) - and since D4 made `as i32`
        // checked, this is the only way left to ask for truncation.
        //
        // Bare, and parenthesised by whoever needs it: `as` sits between
        // the unary operators and the binary ones in Rust's precedence
        // while a method call sits above all of them, so
        // `-x.truncating_i32()` and `x.truncating_i32().abs()` both need
        // the conversion to happen first. [`Emitter::emits_as_cast`] is
        // where that is decided, for the same reason a written `as`
        // decides it there - **a parenthesis nobody needs is a warning
        // about the generated file** (`let n = (big as i32);` is
        // "unnecessary parentheses around assigned value"), and Part III
        // C.1 says a reader must not meet one.
        // **A grammar is entered by an ordinary call**
        // ([ADR-296](../../docs/specification/adr/adr-296.md) D24):
        // `Json.value(input)`, where `Json` names a grammar in this file and
        // `value` one of its `entry` rules. It is a method call in the grammar
        // of this language and nothing else could have been — a grammar name is
        // not a value, so there is no receiver to resolve and no ambiguity to
        // settle.
        if let Some(Expr::Variable(name, _)) = receiver
            && self.s.grammars.contains_key(self.text(*name))
            && args.len() == 1
        {
            // **The dot is gone** (ADR-140 D3). `NK1147` is what a program
            // meets; this is here so that a caller who lowers without
            // checking gets the same sentence rather than a method call on
            // a name that is not a value.
            return Err(refused_at!(
                flow.statement,
                "A grammar's rules are reached with `::`, not a dot. Write `{}::{}(…)`.",
                self.text(*name),
                self.text(method)
            ));
        }
        // **A method of a file's name is `std`'s function**
        // ([ADR-319](../../docs/specification/adr/adr-319.md) D3): Rust's
        // `Path` has methods by these names that compare whole components,
        // so the call is written to `nikaia_std::fs::path`, which compares
        // the bytes.
        if let Some(on) = receiver
            && let Some(known_text) = self.s.facts.path_methods.get(&crate::check::value_node(on))
        {
            let name = self.text(method).to_string();
            // **`to_text` on a name made from text cannot fail** (ADR-319
            // D5), and the checker said where.
            let known = name == "to_text" && *known_text;
            let written = match known {
                true => "text_of",
                false => name.as_str(),
            };
            out.push(&format!("nikaia_std::fs::path::{written}(&("));
            self.expr(out, on, depth, flow)?;
            out.push(")");
            if !args.is_empty() {
                out.push(", ");
                self.args(out, &name, args, &[], depth, flow)?;
            }
            out.push(")");
            if flow.throws && !flow.caught && self.method_can_fail(flow, method) {
                out.push("?");
            }
            return Ok(());
        }
        if let Some(into) = truncating(self.text(method)) {
            self.receiver(out, receiver, depth, flow, true)?;
            out.push(&format!(" as {into}"));
            return Ok(());
        }
        // **`x.checked_i32()` is Rust's `try_from`, kept or not**
        // ([ADR-315](../../docs/specification/adr/adr-315.md)): the number
        // where the destination holds it, `None` where it does not. A call
        // around the receiver, so nothing about precedence is left to decide.
        if let Some(into) = checked(self.text(method)) {
            out.push(&format!("{into}::try_from("));
            match receiver {
                // An argument: nothing binds looser than the call's own
                // parentheses, so it needs none of its own.
                Some(expr) => self.nested(out, expr, 0, depth, flow)?,
                None => self.receiver(out, receiver, depth, flow, false)?,
            }
            out.push(").ok()");
            return Ok(());
        }
        // **`len` hands back an `i64`** (ADR-285 D1), and Rust's hands
        // back a `usize`. Parenthesised where it has to be and nowhere
        // else, exactly as the conversion above is.
        //
        // A name and not a rule, because what it encodes is a fact about
        // *Rust's* library rather than about this language: four entries
        // in `std.contracts` return a length, `len` is what all four are
        // called, and `the_four_lengths_are_i64` in `tests/contracts.rs`
        // is what keeps the two from drifting apart.
        // **`xs.drain()` is the written form of taking the elements away**
        // ([ADR-094](../../docs/specification/adr/adr-094.md) D4). Since a
        // `for` lends, a body that hands an element to a callee which *keeps*
        // it has to say so — and what it says is this. Below, that is
        // `into_iter`: D4's words are *"removing a name from scope"*, which is
        // consuming the container rather than emptying one somebody still
        // holds, and Rust spells the first `into_iter` and the second `drain`.
        //
        // A name and not a rule, for `len`'s reason one paragraph down: it
        // encodes a fact about *Rust's* library rather than about this
        // language.
        // **D5's one door for a stamped value**
        // ([ADR-281](../../docs/specification/adr/adr-281.md)).
        // `kasse.set(neu; after: stand)` is written `set_after(neu, stand)`:
        // the compare and the store happen while the lock is open **once**,
        // which is the whole of what the door is for and what a `get` followed
        // by a `set` cannot do.
        //
        // Two questions and not one. The type checker's, that this receiver is
        // a lock (`witnessed_sets`, ADR-288); and this file's, that the `after:`
        // is written *here* — because the set is keyed by the statement, and a
        // statement may hold a second `set` that carries none.
        let witness = match self.text(method) == "set"
            && self
                .s
                .facts
                .witnessed_sets
                .contains(&(flow.statement as i64))
        {
            true => config
                .iter()
                .find(|a| self.text(a.name) == "after")
                .map(|a| &a.value),
            false => None,
        };
        // **The text form of text is the text** (ADR-282 D8): nothing is
        // called, and whether it is lent, built or handed over is decided
        // around it as for any other text.
        if let Some(text) = receiver.filter(|receiver| {
            self.text(method) == "to_string"
                && args.is_empty()
                && self
                    .s
                    .facts
                    .text_as_is
                    .contains(&crate::check::value_node(receiver))
        }) {
            return self.postfix_base(out, text, depth, flow);
        }
        // **A list of text asked for a view of text** (`check::Checked::
        // text_in_lists`): compared element by element, which a `String` and
        // a `str` can be.
        if let Some(list) = receiver.filter(|receiver| {
            self.text(method) == "contains"
                && args.len() == 1
                && self
                    .s
                    .facts
                    .text_in_lists
                    .contains(&crate::check::value_node(receiver))
        }) {
            // The value lent with `&` whichever text it is (#375): `&str`,
            // `String` and their references compare with each other.
            out.push("nikaia_std::list::contains(&");
            self.expr(out, list, depth, flow)?;
            out.push(", &");
            self.expr(out, &args[0], depth, flow)?;
            out.push(")");
            return Ok(());
        }
        // **Where the container stays the caller's, `drain` empties it**
        // (#564, [ADR-094](../../docs/specification/adr/adr-094.md) D3, D4): a
        // `mut` parameter is lowered as `&mut T`, and `into_iter` over it
        // neither takes the elements away nor hands out owned ones - what a
        // body that stores them needs. `drain(..)` does both, and leaves the
        // list empty in the caller's hands, which is what taking the elements
        // away from something somebody still holds means.
        if self.text(method) == "drain"
            && args.is_empty()
            && let Some(receiver) = receiver
            && self.stays_the_callers(receiver, flow)
        {
            self.postfix_base(out, receiver, depth, flow)?;
            out.push(".drain(..)");
            return Ok(());
        }
        let copies = match receiver {
            Some(receiver) => self
                .s
                .facts
                .owned_copies
                .contains(&crate::check::value_node(receiver)),
            None => self.walk(|w| w.reached_copy),
        };
        let written = match self.text(method) {
            "drain" if args.is_empty() => "into_iter",
            // **A copy is `to_owned` below** (ADR-293 D24): `.clone()` of a
            // `&str` or a `&[T]` is the reference, and a read-only `String`
            // parameter *is* a `&str` (ADR-282 D5).
            "clone" if args.is_empty() && copies => "to_owned",
            "set" if witness.is_some() => "set_after",
            other => other,
        };
        let length = is_length(self.text(method), args)
            || receiver.is_some_and(|receiver| self.is_counted(receiver, method, args));
        // **ADR-055 D6, the method half.** A recursive `async fn` is an
        // infinitely sized future, and a call that closes a cycle of pausing
        // functions puts it behind a pointer. `call` has asked this since D6's
        // first sharp edge; a *method* could not be asked, because the emitter
        // cannot name its callee — `stats.add(5)` names `add` and only the type
        // checker knows what it goes to (ADR-288). It does not have to name it:
        // `pausing_reach` already draws an edge to **every** pausing method of
        // that name, which is the over-approximation its own note describes, so
        // the same widening answers the question here. Boxing a call that did
        // not need it costs one allocation; missing one is `rustc`'s *recursion
        // in an async fn requires boxing*, about a file nobody wrote (C.1).
        let boxed = self.method_closes_a_pausing_cycle(flow.function, self.text(method));
        if boxed {
            out.push("Box::pin(");
        }
        // **`par_iter()` walks the list on every core at `yes`, and is the
        // list's `iter()` at `no`**
        // ([ADR-235](../../docs/specification/adr/adr-235.md) D1). What the
        // lambdas handed to it may do is the checker's, at both settings, so
        // the program means the same at each.
        if self.text(method) == "par_iter"
            && args.is_empty()
            && self.s.own_contracts.candidates("par_iter").is_empty()
        {
            match self.build.user_parallelism {
                UserParallelism::Yes => {
                    out.push("nikaia_std::par::iter(&");
                    self.receiver(out, receiver, depth, flow, false)?;
                    out.push(")");
                }
                UserParallelism::No => {
                    self.receiver(out, receiver, depth, flow, false)?;
                    out.push(".iter()");
                }
            }
            if boxed {
                out.push(")");
            }
            return Ok(());
        }
        // **A lambda that pauses, handed to a `std` entry**
        // ([ADR-233](../../docs/specification/adr/adr-233.md) D1, D2): the
        // entry's counterpart in `nikaia_std::seq`, which takes the `async`
        // closure the lambda is written as. The checker says which, because it
        // is the receiver's type that decides.
        let pausing = args
            .last()
            .filter(|arg| matches!(arg, Expr::Closure { .. }))
            .and_then(|arg| {
                self.s
                    .facts
                    .pausing_lambdas
                    .get(&crate::check::value_node(arg))
            })
            .cloned();
        match &pausing {
            Some(PausingEntry::Function(name)) => {
                out.push(&format!("nikaia_std::seq::{name}("));
                self.receiver(out, receiver, depth, flow, false)?;
                out.push(", ");
            }
            Some(PausingEntry::Lent(name)) => {
                out.push(&format!("nikaia_std::seq::{name}(&mut "));
                self.receiver(out, receiver, depth, flow, false)?;
                out.push(", ");
            }
            Some(PausingEntry::Method(name)) => {
                self.receiver(out, receiver, depth, flow, false)?;
                out.push(&format!(".{name}("));
            }
            None => {
                self.receiver(out, receiver, depth, flow, false)?;
                out.push(&format!(".{written}"));
            }
        }
        // Nikaia's `collect` builds a List; Rust's needs to be told
        // what to build, and with no types here that is `Vec<_>`.
        //
        // **Except over a sequence whose step pauses**
        // ([ADR-172](../../docs/specification/adr/adr-172.md) D5), where the
        // walk is `std`'s own `async fn` and has nothing to be told: an
        // `Iterator::collect` never pauses, so the site pausing is exactly the
        // difference. Guessed from the method's name alone this would have
        // been a turbofish on a method that takes no type at all.
        //
        // **And except where the place it goes declares what it builds**
        // ([ADR-293](../../docs/specification/adr/adr-293.md) D25): a map, a
        // set or text, which the language below's own `collect`
        // builds when nothing overrides the type it reads off that place.
        let declared = receiver.is_some_and(|receiver| {
            self.s
                .facts
                .collected_into
                .contains(&crate::check::value_node(receiver))
        });
        if self.text(method) == "collect"
            && args.is_empty()
            && !declared
            && !self.method_pauses(flow, method)
        {
            out.push("::<Vec<_>>");
        }
        if pausing.is_none() {
            out.push("(");
        }
        let takes = self.takes_a_handle(self.text(method));
        // **Views put into a keeper that drops entries are held** (ADR-283 D12),
        // each value with a handle on every buffer it points into - as many as
        // the keeper's widest value needs, so that every element is one type
        // (ADR-283 D15).
        let held = receiver
            .and_then(|receiver| crate::contracts::keep::root_of(self.parsed, receiver))
            .and_then(|root| {
                let plan = self.keep_plan(flow.function)?;
                let by_arg = plan.holds.get(&(flow.statement as i64, root.clone()))?;
                let width = plan.widths.get(&root).copied().unwrap_or(1);
                Some(
                    by_arg
                        .iter()
                        .map(|(arg, buffers)| {
                            let mut keeps: Vec<i64> = buffers.iter().copied().collect();
                            while (keeps.len() as i64) < width {
                                keeps.push(keeps[keeps.len() - 1]);
                            }
                            (*arg, keeps)
                        })
                        .collect(),
                )
            });
        self.walking(|w| w.hold_args = held);
        let written_args = self.args(out, self.text(method), args, &takes, depth, flow);
        self.walking(|w| w.hold_args = None);
        written_args?;
        // **And a method that takes a keep is given one** (D2).
        let keeping = crate::contracts::keep::keeping_method_key(
            &self.s.own_contracts,
            flow.function.rsplit_once("::").map(|(t, _)| t),
            matches!(receiver, Some(Expr::Variable(n, _)) if self.text(*n) == "self"),
            self.text(method),
        );
        if let Some(keep) = keeping.and_then(|key| self.keep_argument(flow, &key)) {
            if !args.is_empty() {
                out.push(", ");
            }
            out.push(&keep);
        }
        match witness {
            // The witness is an **argument** of the door and not an option of
            // it, so it is written where the signature puts it — after the
            // value, and never through `dsl_parameters`.
            //
            // **The `&` is written here rather than looked up**, and that is
            // the one place this file decides a reference for itself. `lends`
            // withholds its claim on every *method* argument, because the
            // emitter cannot resolve a receiver
            // ([ADR-288](../../docs/specification/adr/adr-288.md)) — and this
            // position needs no resolving: the checker has already said this
            // call is the door, and the door's witness is `seen: &$T` in the
            // ledger, always. A written `&` is `NK1137` before it gets here.
            Some(seen) => {
                out.push(", &");
                // Parenthesised exactly where a postfix would be: `&` binds
                // tighter than every binary operator, so `&a + b` is `(&a) + b`
                // and a witness that is an expression would mean something
                // else. A name gets no parentheses, which is where every
                // witness anybody writes lands.
                self.postfix_base(out, seen, depth, flow)?;
            }
            None => {
                self.dsl_parameters(out, self.text(method), args.len(), config, depth, flow)?;
                self.method_options(out, method, args.len(), config, depth, flow)?;
            }
        }
        out.push(")");
        // **What hands back an element of a held list reads it through its
        // handle** (ADR-283 D17), as the brackets and a `for` do.
        if matches!(self.text(method), "iter" | "first" | "last" | "get")
            && receiver.is_some_and(|r| self.reads_through_handle(flow, r))
        {
            out.push(HELD_READ);
        }

        // ADR-023 D8, the method half. The same three conditions the
        // call by name is given in `call` below, and the same rule -
        // only the last question is asked of a different source,
        // because the emitter cannot ask it itself. The enclosing
        // function must be `throws`, or there is nowhere for the `?` to
        // go, and `NK2605` has already refused the program where it is
        // not. The call must not be the guarded half of a `catch`,
        // which wants the `Result`. And the callee must be one the
        // checker established can fail.
        // ADR-055 D2, the method half, and **before the `?`** for the
        // reason `call` gives: the future is what can fail, so it has
        // to be driven before there is a `Result` to propagate.
        // **Before the `.await`**, because it is the future that is boxed and
        // not what awaiting it comes to.
        if boxed {
            out.push(")");
        }

        // **A walk of a pausing sequence that has no pausing form**
        // ([ADR-172](../../docs/specification/adr/adr-172.md) D5,
        // [ADR-234](../../docs/specification/adr/adr-234.md)). Every walk `Seq`
        // has today has one; an entry added without one is refused here with a
        // line, rather than left to `rustc` about a method the generated file's
        // receiver does not have (Part III, C.1).
        if self
            .s
            .facts
            .pausing_walks
            .contains(&((flow.statement as i64), self.text(method).to_string()))
        {
            let name = self.text(method);
            return Err(refused_at!(
                flow.statement,
                "`{name}` can't be used on a sequence whose steps pause. Write a \
                 `for x in … {{ … }}` loop instead, and do the work inside it."
            ));
        }
        if self.method_pauses(flow, method) {
            if flow.in_lambda {
                return Err(pausing_in_a_lambda(flow.statement, self.text(method)));
            }
            out.push(".await");
        }

        let fails = self.method_can_fail(flow, method);
        if flow.throws && !flow.caught && fails {
            out.push("?");
        }
        // **A length that can fail is already an `i64`** (`fs::File::len`, #546):
        // it is the system's answer, read by `std` and converted there, and a
        // cast here would land on the `Result` a `catch` matches on.
        if length && !fails {
            out.push(" as i64");
        }
        Ok(())
    }

    /// The receiver of a [`Emitter::method_call`]: the expression, or the name
    /// a `?.` already bound.
    ///
    /// `tight` is for the one caller that needs the conversion to bind tighter
    /// than everything around it - `x.truncating_i32()` is Rust's `as`, which
    /// sits below the unary operators in its precedence.
    fn receiver(
        &self,
        out: &mut Out,
        receiver: Option<&Expr>,
        depth: usize,
        flow: Flow<'_>,
        tight: bool,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_receiver(
                &self.world,
                walk,
                out,
                self,
                receiver,
                depth as i64,
                flow,
                &scope,
                tight,
            )
        });
        self.crossed(crossed)
    }

    /// **A read through the brackets** (ADR-293 D4): `*get(&base, at(i))`,
    /// or without the `*` where `deref` is off - which is how a read that is
    /// itself the base of a read is written (0.0.238).
    fn index_read(
        &self,
        out: &mut Out,
        base: &Expr,
        index: &Expr,
        depth: usize,
        flow: Flow<'_>,
        deref: bool,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_index_read(
                &self.world,
                walk,
                out,
                self,
                base,
                index,
                deref,
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

    /// **Whether this comes out as a Rust `as` conversion**, whatever it was
    /// written as.
    ///
    /// Three things do: a written `as` (ADR-285 D12), `x.truncating_i32()`, which
    /// *is* Rust's `as` (D7), and `xs.len()`, whose `usize` becomes the `i64` the
    /// ledger promises ([ADR-285](../../../../docs/specification/adr/adr-285.md)
    /// D1). What they have in common is where they sit in Rust's precedence -
    /// below the unary operators, above the binary ones - which is not where a
    /// method call sits, so the two places that decide parentheses have to ask
    /// about the emitted shape rather than the written one.
    ///
    /// Asked in one place because the alternative is parenthesising always, and a
    /// parenthesis nobody needs is a Rust warning about a file nobody wrote
    /// (Part III, C.1).
    /// Whether `expr` is written ending in `as T`: a conversion, or an
    /// operation whose right operand is written bare and ends in one.
    fn ends_in_a_cast(&self, expr: &Expr) -> bool {
        self.s.ends_in_a_cast(&self.world, expr)
    }

    /// **`std`'s count of a sequence**, which counts in `usize` below and is
    /// an `i64` here, as a length is (Part I 2.2): the checker says which
    /// `count()` resolved to it, so a program's own `count` is left alone.
    fn is_counted(&self, receiver: &Expr, method: winnow_grammar::Symbol, args: &[Expr]) -> bool {
        self.s.is_counted(receiver, self.text(method), args)
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

    fn args(
        &self,
        out: &mut Out,
        // The callee as the source wrote it. Part I 2.3's wrap at an argument
        // is keyed by it, because a statement may hold several calls and one
        // call several arguments (`check::Checked::nullable_args`).
        callee: &str,
        args: &[Expr],
        // Which positional parameters of the callee take a **handle** on a shared
        // value by value, where a ledger describes the callee. Empty where nothing
        // does or nothing is known, which is almost every call.
        takes_a_handle: &[bool],
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_args(
                &self.world,
                walk,
                out,
                self,
                callee,
                args,
                takes_a_handle,
                depth as i64,
                flow,
                &scope,
            )
        });
        self.crossed(crossed)
    }

    /// Which of a callee's positional parameters take a handle on a shared value
    /// **by value**, where a ledger describes the callee.
    ///
    /// Resolved the way `contracts::sharing::Analysis::parameters` resolves it -
    /// this program's own ledger first, then `std`'s, by name and then by suffix
    /// (ADR-296 D17) - so that the duplication written here and the duplication
    /// `--sharing` names are the same call's.
    ///
    /// A `&Shared[T]` parameter answers **no**, which is ADR-312 D1's correction:
    /// lending the inner value out hands no handle on.
    fn takes_a_handle(&self, callee: &str) -> Vec<bool> {
        self.s.takes_a_handle(&self.world, callee)
    }

    /// Whether a ledger key is a grammar entry another package offers: what
    /// `contracts` records for every `entry rule` - one input, `input`, and a
    /// `ParseError` its one failure (#520).
    fn a_package_grammar_entry(&self, key: &str) -> bool {
        self.s.a_package_grammar_entry(key)
    }

    /// `Measurements.file(data)` - the whole of what a user writes to run a
    /// grammar. Everything the parallel form needs is already in the grammar
    /// (ADR-296): the frame says where the input may be cut, the `par_fold`
    /// says how the pieces combine. What is left is choosing the executor, and
    /// that is the build's decision, not the program's.
    fn grammar_entry(
        &self,
        out: &mut Out,
        grammar: Symbol,
        entry: Symbol,
        input: &Expr,
        depth: usize,
        flow: Flow<'_>,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| {
            self.s.emit_core_grammar_entry(
                &self.world,
                walk,
                out,
                self,
                grammar,
                entry,
                input,
                self.build.parallelism(),
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

/// Whether this expression is a **number written down**, sign and all.
///
/// The negation is part of the answer rather than a wrapper around it: the
/// unary arm above folds `-1` into one literal, and `-1.into()` in the language
/// below is `-(1.into())`, which is a second reason not to write the `into` at
/// all.
fn a_number(expr: &Expr) -> bool {
    nikaia_std::tools::emit_ops::a_number(expr)
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

/// Whether a `??`'s fallback ends in a jump: a jump itself, another `??`
/// whose own fallback does - the chain `a ?? b ?? continue` is one fallback
/// that may leave - or a block whose last statement does (#457).
fn jumps_at_the_end(expr: &Expr) -> bool {
    nikaia_std::tools::emit_ops::jumps_at_the_end(expr)
}

/// `const` or `mut`, the word Rust puts between the `*` and the type.
fn pointing(mutable: bool) -> String {
    nikaia_std::tools::emit_ops::pointing(mutable)
}

fn only_literals(index: &Expr) -> bool {
    nikaia_std::tools::emit_ops::only_literals(index)
}

/// `xs.len()` - the four `std` entries that hand back a length, by the name all
/// four of them have ([ADR-285](../../../docs/specification/adr/adr-285.md) D1).
fn is_length(method: &str, args: &[Expr]) -> bool {
    nikaia_std::tools::emit_ops::is_length(method, args)
}

fn precedence(op: BinaryOp) -> u8 {
    nikaia_std::tools::emit_ops::precedence(op) as u8
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

/// **A lambda whose body pauses, which this lowering cannot write.**
///
/// **What is left of it is a callee nothing describes.** A lambda that pauses
/// is written as an `async` closure where the parameter is declared in this
/// language ([ADR-277](../../../docs/specification/adr/adr-277.md) D11), and
/// handed to the counterpart in `nikaia_std::seq` where it is one of `std`'s
/// entries ([ADR-233](../../../docs/specification/adr/adr-233.md)) - every one
/// of them: `map`, `filter`, a list's `map`, `sort_by_key`, `or_insert_with`,
/// `and_modify`, and the four doors, where a pause is `NK2202` by the
/// language's own rule, and `par_iter()`'s walks, where it is `NK2209`
/// ([ADR-235](../../../docs/specification/adr/adr-235.md)). What reaches this
/// is a lambda handed to a method whose receiver the checker could not
/// resolve, so there is no signature to say what shape the lambda takes, and a
/// plain closure holding an `.await` is a `rustc` error about a file nobody
/// wrote (Part III, C.1).
fn pausing_in_a_lambda(at: usize, callee: &str) -> anyhow::Error {
    refused_at!(
        at,
        "This lambda calls `{callee}`, which can pause, but the compiler knows nothing \
         about what the lambda is passed to, so it can't let it pause. Call `{callee}` \
         outside the lambda and pass in the value."
    )
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
        return (*code.result)
            .as_ref()
            .is_some_and(|result| names_borrowing(result, borrowing));
    }
    borrowing.contains(&ty.name) || ty.generics.iter().any(|g| names_borrowing(g, borrowing))
}

/// The type a `truncating_` method converts to, if the name is one
/// ([ADR-285](../../../docs/specification/adr/adr-285.md) D13).
///
/// The target is **in the name** rather than inferred, for the reason the
/// `wrapping_` names have their own entry per type: there is one method per
/// destination, and the ledger has to be able to write its signature down.
///
/// The list is closed and matches the ledger's entries. A name this does not
/// recognise is an ordinary method call, so a program that writes
/// `truncating_u8` is refused by the type checker for the method not existing
/// rather than quietly emitting a conversion nobody described.
fn truncating(method: &str) -> Option<String> {
    nikaia_std::tools::emit_text::truncating(method)
}

/// The type a `checked_` conversion converts to, if the name is one
/// ([ADR-315](../../../docs/specification/adr/adr-315.md)): the four
/// `truncating_` has. `checked_add` and the arithmetic are Rust's own methods
/// and lower as any method does.
fn checked(method: &str) -> Option<String> {
    nikaia_std::tools::emit_text::checked(method)
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

    /// The keep a call to `callee` is given in this statement, where the
    /// callee takes one.
    fn keep_argument(&self, flow: Flow<'_>, callee: &str) -> Option<String> {
        self.walk(|w| {
            self.s
                .keep_handed(w, flow.function, flow.statement as i64, callee)
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
pub(crate) fn nonnegative_names(parsed: &Parsed, body: &Block) -> HashSet<Symbol> {
    nonnegative_locals(parsed, body)
}

fn nonnegative_locals(parsed: &Parsed, body: &Block) -> HashSet<Symbol> {
    nikaia_std::tools::emit_locals::nonnegative_locals(&parsed.interner, body)
        .into_iter()
        .collect()
}

/// What a `dsl` block names: `pkg::Grammar` for a package's, the bare name
/// for one the program declares and for `html`
/// ([ADR-299](../../docs/specification/adr/adr-299.md) D20).
pub(crate) fn dsl_name(parsed: &Parsed, package: Option<Symbol>, target: Symbol) -> String {
    nikaia_std::tools::emit_items::dsl_name(&parsed.interner, package, target)
}

/// **A literal as a field of `ty` holds it** (ADR-331 D5-D6, ADR-318 D4):
/// text for an owned `String` is one, and a value for a `T?` is wrapped.
fn held_literal(literal: &str, ty: &crate::contracts::ty::Ty) -> String {
    nikaia_std::tools::emit_text::held_literal(literal, ty)
}

/// A field's type as the language below writes it, for a `None` nothing else
/// types: the plain shapes, and `_` for the rest, which leaves it to `rustc`.
fn plain_rust_spelling(ty: &crate::contracts::ty::Ty) -> String {
    nikaia_std::tools::emit_text::plain_rust_spelling(ty)
}

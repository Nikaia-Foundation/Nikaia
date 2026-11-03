// crates/nikaia/src/contracts/mod.rs
//
// The Borrow Contract Ledger (Part III, 13.5; ADR-005 D3; ADR-020).
//
// What a caller has to know about a function it cannot see the body of: does it
// pause, can it fail, and does what it hands back point into what it was given.
// Part III 13.5 specifies a file that records exactly that, derived rather than
// written, committed like a lockfile, and **shipped with a published package**
// so that a consumer builds against contracts instead of guesses.
//
// This is that file, for what Stage 0 knows, and the three answers come from
// three different places.
//
// `throws` is *declared* in the source and recorded exactly. The borrow
// contract is *inferred from the signature* - the widest one a signature can
// support - rather than by the whole-program analysis ADR-005 D3 describes.
// `sync` is *inferred from the body* (ADR-288): a function that provably cannot
// pause gets the promise whether or not anyone wrote the word, and where the
// word is written it stays an assertion for `NK2202` to check. `sharing` is
// inferred from the bodies too (ADR-037 D7), and it is the one column that can
// only ever get *better*: its floor is the safe answer, so a ledger that says
// nothing about a `Shared` still describes a correct program.
//
// That is why the header names the inference that produced the ledger. A later
// compiler that infers more will write a different name there, and `--locked`
// will say so rather than quietly accepting the weaker answer.

pub mod keep;
pub mod keeps;
pub mod locks;
pub mod maps;
pub mod order;
pub mod send;
pub mod sharing;
pub mod sync;
pub mod tether;
pub mod throws;
pub mod touch;
pub mod trust;
pub mod ty;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow};

use crate::ast::Expr;
use crate::emit::{borrowing_structs_beside, package_borrowing};
use crate::parser::Parsed;

/// **The ledger's small records are declared in Nikaia**
/// ([ADR-294](../../../docs/specification/adr/adr-294.md) step (b)):
/// `nikaia-std/src/tools/ty.nika`, beside the type language their fields
/// hold. Re-exported here so that every reader keeps its path.
pub use nikaia_std::tools::ty::{
    ConfigContract, Crosses, FieldContract, FnContract, Ledger, Lock, Notes, Provenance, Signature,
    Sync, Threads, TypeContract, VariantContract,
};

/// What stays Rust of [`Sync`]: the name a `sync` was taken from.
pub trait SyncOps {
    fn from(&self) -> Option<&str>;
}

impl SyncOps for Sync {
    fn from(&self) -> Option<&str> {
        // In Nikaia (`tools/contracts_core.nika`, #125).
        nikaia_std::tools::contracts_core::contracts_sync_from(self)
    }
}

/// The contracts `std` ships, as the library ships them.
///
/// Part III 13.5 has a consumer read a package's ledger from the package.
/// Stage 0's compiler and `std` ship together, so the file is embedded here
/// rather than looked up - the same file, read at build time instead of at run
/// time, and the same one a reviewer reads.
pub const STD: &str = include_str!("../../../nikaia-std/std.contracts");

/// The inference this ledger was produced by.
///
/// Recorded in the header so that a ledger can say what it knows, and a ledger
/// produced by reading signatures must not be mistaken for one produced by
/// reading bodies. Stage 0 reads signatures for the borrow contract and for
/// `throws`; since ADR-288 it reads **bodies** for `sync`, since ADR-023 D1 for
/// the *errors* a `throws` names, and since ADR-037 D7 for `sharing` - the name
/// says which half is which.
/// The whole-program analysis of ADR-005 D3 will read bodies for the borrow
/// contract too and will write a different name again.
pub const INFERENCE: &str = "stage0-signatures+sync-bodies+throws-bodies+sharing-bodies";

/// The format version of the file itself.
///
/// 2 since [ADR-023](../../../../docs/specification/adr/adr-023.md) D1: `throws`
/// was a boolean and is a list of the errors that can leave the function. A
/// version 1 file still reads - `true` is taken as `["?"]`, which is what it
/// always meant - and the version is what tells a *reader* that the file it has
/// may say more than it knows how to use.
///
/// 4 since [ADR-views-3](../../../../docs/specification/adr/adr-views-3.md) D10: a
/// type's `tethered` lists its places in the order the fields are written, and
/// a reading package takes one lifetime per place from it.
pub const VERSION: u32 = 4;

/// What stays Rust of a variant's entry until its text moves (ADR-294 step
/// (c)): reading one back. The declaration and `text` are `tools/ty.nika`'s.
pub trait VariantOps {
    fn parse(text: &str) -> VariantContract;
}

impl VariantOps for VariantContract {
    fn parse(text: &str) -> VariantContract {
        nikaia_std::tools::ledger::variant_of(text)
    }
}

/// What stays Rust of a [`Signature`] (ADR-294 step (b)): the parameters
/// after the receiver, as a slice of the entry's own, and reading one back.
/// The declaration, `text` and the rest are `tools/ty.nika`'s.
pub trait SignatureOps: Sized {
    fn arguments(&self) -> &[(String, ty::Ty)];
    fn parse(text: &str) -> Result<Self>;
}

impl SignatureOps for Signature {
    /// The arguments a *call* passes, which is the parameters after a receiver.
    fn arguments(&self) -> &[(String, ty::Ty)] {
        nikaia_std::tools::contracts_core::contracts_arguments(self)
    }

    /// Read one back from the text above.
    fn parse(text: &str) -> Result<Signature> {
        nikaia_std::tools::ledger::signature_of(text).map_err(|refusal| anyhow!("{refusal}"))
    }
}

/// What the compiler does with a ledger: infer, absorb, render and read it
/// back. The record is `tools/ty.nika`'s (ADR-294 D12); these stay Rust until
/// step (c) and (d) move them.
pub trait LedgerOps: Sized {
    fn empty() -> Self;
    fn infer(parsed: &Parsed) -> Self;
    fn infer_package(units: &[&Parsed], library: &Ledger) -> Self;
    fn published(&self, foreign: &BTreeSet<String>) -> Ledger;
    fn stale_against(&self, sources: &BTreeMap<String, String>) -> Vec<String>;
    fn absorb(&mut self, module: Option<&str>, other: Ledger);
    fn absorb_renaming(
        &mut self,
        module: Option<&str>,
        renames: &std::collections::BTreeMap<String, String>,
        other: Ledger,
    );
    fn infer_checked(parsed: &Parsed) -> (Self, crate::check::Checked);
    fn infer_package_checked(
        units: &[&Parsed],
        library: &Ledger,
    ) -> (Self, Vec<crate::check::Checked>);
    /// [`LedgerOps::infer_package_checked`], and what the `sync` inference
    /// learned for the build to say ([`sync::Noted`], ADR-288 D29 and D32).
    fn infer_package_noted(
        units: &[&Parsed],
        library: &Ledger,
    ) -> (Self, Vec<crate::check::Checked>, sync::Noted);
    fn lookup(&self, name: &str) -> Option<(String, &FnContract)>;
    fn render(&self) -> String;
    fn render_derived(&self) -> String;
    fn read_beside(path: &std::path::Path) -> Option<Ledger>;
    fn render_description(&self, crate_name: &str, version: &str, notes: &Notes) -> String;
    fn render_from(&self, out: &mut String);
    fn render_from_with(&self, out: &mut String, notes: &Notes);
    fn parse(text: &str) -> Result<Self>;
}

impl LedgerOps for Ledger {
    /// The contracts of a parsed program.
    ///
    /// Two passes, because the second needs the first to have finished. The
    /// item loop records what each declaration *says*; then [`sync::infer`]
    /// reads the bodies and gives `sync` to what earns it, which it can only do
    /// once every function in the unit has an entry to be looked up in.
    ///
    /// The library it resolves calls against is `std`'s shipped ledger. It used
    /// to be closed for a reason that has since been answered rather than
    /// abandoned: 13.5 makes this file a pure function of (source, toolchain),
    /// so a ledger inferred against a *different* library would be a different
    /// file for the same source. [ADR-100](../../../docs/specification/adr/adr-100.md)
    /// D1 and D5 make a dependency's ledger a **build input** — one answer with
    /// one author, existing before its consumer is checked — so
    /// [`Self::infer_package`] takes the library and this entry point keeps
    /// `std` alone.
    /// A ledger with a header and nothing in it - what a program of several
    /// files starts from before it absorbs its modules.
    fn empty() -> Self {
        // In Nikaia (`tools/contracts_core.nika`, #125).
        nikaia_std::tools::contracts_core::contracts_headed(VERSION, &toolchain(), INFERENCE)
    }

    fn infer(parsed: &Parsed) -> Self {
        Self::infer_checked(parsed).0
    }

    /// The contracts of a **package**: every unit of it, inferred as one graph
    /// ([ADR-100](../../../docs/specification/adr/adr-100.md) D2).
    ///
    /// A package is one namespace ([ADR-286](../../../docs/specification/adr/adr-286.md)
    /// D1), so a call from one of its files to a function in another is a call
    /// this compiler can see - and the inference has to see it too. Inferring
    /// each file alone made a callee in the file next door indistinguishable
    /// from one in code nothing describes: `reach_of` found it in neither `own`
    /// nor `std` and set `blocked`, which `Sync::No` spells *"can pause **or**
    /// could not be vouched for"* and every reader takes as the first. `NK1129`
    /// is where that showed - a trait method refused as pausing for calling a
    /// plain function two lines away in another file.
    ///
    /// The units arrive in the order they were read, which Part III 13.5 needs:
    /// this file is a pure function of the source tree, and the fixpoints below
    /// walk a `BTreeMap` so that the answer does not depend on the order
    /// anyway.
    ///
    /// **`library` is `std`'s ledger and every dependency's**
    /// ([ADR-100](../../../docs/specification/adr/adr-100.md) D1), each under
    /// the name this package reaches it by. A call that leaves the package is
    /// answered from it; what is in neither it nor the package's own entries is
    /// code no ledger describes, and *that* is what fails closed
    /// ([ADR-288](../../../docs/specification/adr/adr-288.md)).
    fn infer_package(units: &[&Parsed], library: &Ledger) -> Self {
        Self::infer_package_checked(units, library).0
    }

    /// This package's **own** entries: what it publishes, with everything that
    /// belongs to a package *it* depends on left out
    /// ([ADR-286](../../../docs/specification/adr/adr-286.md) D21).
    ///
    /// A package's ledger file is the program's record of a whole build, so a
    /// library's carries its own dependencies' entries under their names. A
    /// consumer reading it (ADR-100 D1) must not take those: a transitive
    /// package is deliberately invisible, and absorbing `c::Id` under this
    /// package's name would make `lib::c::Id` — a type nothing declares and
    /// nobody can write. The derived answer has never had them, so this is what
    /// makes the believed one the *same* answer rather than a bigger one.
    ///
    /// `foreign` is the depending build's record of what words this package
    /// uses for packages of its own (`modules::Dependency::reachable`).
    fn published(&self, foreign: &BTreeSet<String>) -> Ledger {
        // In Nikaia (`tools/ledger_ops.nika`, ADR-294, #125).
        nikaia_std::tools::ledger_ops::published_entries(self, foreign)
    }

    /// Whether this ledger may be **believed** for the sources given, and which
    /// units say otherwise ([ADR-100](../../../docs/specification/adr/adr-100.md)
    /// D3).
    ///
    /// Empty means believe it. A non-empty answer names the units that moved,
    /// and the caller derives that package again rather than trusting an entry
    /// derived from a file that is no longer there.
    ///
    /// **A ledger with no `[sources]` at all is never believed against sources
    /// that exist.** It was written before this table or by hand, and the one
    /// polarity this must not get wrong is reading *nothing was recorded* as
    /// *nothing changed* — a stale `touches` is a data race with no message
    /// ([ADR-010](../../../docs/specification/adr/adr-010.md) D1). A package
    /// with a ledger and **no sources** is the third row of D3's table and is
    /// not this function's question: nothing calls it with an empty `sources`.
    fn stale_against(&self, sources: &BTreeMap<String, String>) -> Vec<String> {
        // In Nikaia (`tools/ledger_ops.nika`).
        nikaia_std::tools::ledger_ops::stale_units(self, sources)
    }

    /// Every entry of `other`, under `module::`.
    ///
    /// A program of several files has **one** ledger (Part III, 13.5 puts it at
    /// the project root), and its keys are qualified exactly the way a caller
    /// writes them - which is the shape `std.contracts` has had since ADR-020:
    /// `fs::map`, `io::lines`, `text::digit_value`.
    ///
    /// That is what makes multi-file compilation almost free for everything
    /// built on the ledger. The type checker, the `sync` check, the provenance
    /// analysis, Kap 5.1's options and ADR-025's fallible loops all resolve a
    /// call by asking a ledger for `a::b`. None of them learns a new trick to
    /// work across files; the file they ask simply has more in it.
    /// **The types inside an entry are qualified too, and not only the keys.**
    /// A module's own signature says `-> Conn`, because that is how the file
    /// declaring it writes the name, while its type is keyed `pool::Conn`. A
    /// caller who annotated `pool::Conn` and called `pool::make()` was then told
    /// the two were different types, and the help line asked for what was already
    /// written. They are one type with two spellings, and this is the only place
    /// that knows both the module and what it declares.
    fn absorb(&mut self, module: Option<&str>, other: Ledger) {
        self.absorb_renaming(module, &std::collections::BTreeMap::new(), other)
    }

    /// The same, for a package that writes **its own** word for a package this
    /// build has a word for too
    /// ([ADR-286](../../../docs/specification/adr/adr-286.md) D20).
    ///
    /// Qualifying is about the names a package declares; this is about the names
    /// it *reaches*, and they are the half that used to come out wrong. A library
    /// that depends on the same directory a program depends on writes its own
    /// manifest key in its signatures — `c::Id` where the program wrote
    /// `deep::Id` — and the checker, having only the two spellings, called them
    /// two types. They are one package, so they are one type, and `renames` is
    /// the translation `project::renames_in` computed from the directories.
    ///
    /// Applied **after** qualifying and not instead of it: a name the package
    /// declares itself is its own and is never renamed, because `qualify` has
    /// already put this package's word in front of it.
    fn absorb_renaming(
        &mut self,
        module: Option<&str>,
        renames: &std::collections::BTreeMap<String, String>,
        other: Ledger,
    ) {
        // In Nikaia (`tools/ledger_ops.nika`, ADR-294, #125).
        nikaia_std::tools::ledger_ops::absorb_into(self, module, renames, &other);
    }

    /// The contracts, and the type checker's pass that helped produce them.
    ///
    /// Three steps, and the order is forced:
    ///
    /// 1. read the declarations, so every function has an entry to be looked
    ///    up in;
    /// 2. run the **type checker** against that, which resolves each method
    ///    call to the function it goes to (ADR-288);
    /// 3. infer `sync` from the bodies, using both.
    ///
    /// **Step 2 does not depend on step 3, and that is what makes this sound
    /// rather than circular.** The checker reads `signature`, `fields` and
    /// `iterates` from a ledger and never `sync`, so resolving a method against
    /// the step-1 ledger gives the same answer as resolving it against the
    /// finished one. `the_checker_does_not_depend_on_the_sync_it_helps_infer`
    /// in `tests/contracts.rs` is that invariant, held to.
    ///
    /// The pass is handed back rather than thrown away because the compiler
    /// wants it too - it carries the findings and the fallible loops - and
    /// running the checker twice per build to get one of them would be waste,
    /// not caution.
    fn infer_checked(parsed: &Parsed) -> (Self, crate::check::Checked) {
        let (ledger, mut checked) = Self::infer_package_checked(&[parsed], std_ledger());
        (ledger, checked.remove(0))
    }

    /// The same, over every unit of a package — see [`Self::infer_package`].
    ///
    /// **One `Checked` per unit, in the units' order**, and that is not a
    /// convenience: almost everything that pass answers is keyed by the **byte**
    /// a statement starts at, which names a position in one file and nothing at
    /// all in a package. Only `methods` is keyed by a name, so only `methods` is
    /// merged - and merging it is sound for the reason the package is one graph
    /// in the first place: one namespace, so one function per name
    /// (`modules::collect` refuses the second).
    fn infer_package_checked(
        units: &[&Parsed],
        library: &Ledger,
    ) -> (Self, Vec<crate::check::Checked>) {
        let (ledger, checked, _) = Self::infer_package_noted(units, library);
        (ledger, checked)
    }

    fn infer_package_noted(
        units: &[&Parsed],
        library: &Ledger,
    ) -> (Self, Vec<crate::check::Checked>, sync::Noted) {
        let mut ledger = Self::empty();
        // **The order of the passes, in Nikaia** (`tools/ledger_items.nika`,
        // `ledger_infer_package`, ADR-294, #125): the item loop, the checker,
        // each derived column and the defaults. What holds a view is computed
        // per unit here, by name (#544): a `Symbol` is interned by the parse
        // of one file, so a set of them means nothing to another.
        let views: Vec<nikaia_std::tools::parsed_view::ParsedView<'_, '_, '_, '_, '_, '_>> = units
            .iter()
            .map(|parsed| crate::check::parsed_view_of(parsed))
            .collect();
        let package_borrowing = package_borrowing(units);
        let borrowing: Vec<BTreeSet<String>> = units
            .iter()
            .map(|parsed| {
                borrowing_structs_beside(parsed, &package_borrowing)
                    .iter()
                    .map(|s| parsed.text(*s).to_string())
                    .collect()
            })
            .collect();
        let host = InferHost {
            units,
            library,
            checked: RefCell::new(Vec::new()),
            resolved: RefCell::new(BTreeMap::new()),
            stored: RefCell::new(BTreeMap::new()),
            noted: RefCell::new(None),
            column: RefCell::new(None),
        };
        nikaia_std::tools::ledger_items::ledger_infer_package(
            &mut ledger,
            &views,
            &borrowing,
            library,
            &host,
        );
        let mut noted = host.noted.take().unwrap_or_default();
        if let Some(column) = host.column.take() {
            noted.column = column;
        }
        (ledger, host.checked.take(), noted)
    }

    /// A function by the name a caller wrote.
    ///
    /// **Exactly the name, since
    /// [ADR-313](../../../docs/specification/adr/adr-313.md)**: a `std` entry
    /// that lives in a module is reached through the module, `text::digit_value`
    /// and not `digit_value`, and what needs no prefix is the list on Part I's
    /// first page — whose entries are keyed **bare** here, so the exact lookup
    /// is the whole rule.
    ///
    /// It used to match on the last segment, which was name-for-name resolution
    /// (ADR-296 D17) rather than import tracking, and it is what a compiler
    /// without a module graph could honestly do before there was a written
    /// list. What it cost, beyond the prelude being undefined, was answering
    /// about the **wrong function**: a program with its own `fn read` found
    /// `io::read` in a unit that does not carry the ledger of the package
    /// beside it, and the `throws` column then spoke for a callee nobody had
    /// resolved.
    fn lookup(&self, name: &str) -> Option<(String, &FnContract)> {
        self.functions
            .get(name)
            .map(|contract| (name.to_string(), contract))
    }

    /// The file, as it is written out: `tools/ledger_text.nika` (ADR-294,
    /// #125), beside `ledger.nika`, which reads it back.
    fn render(&self) -> String {
        nikaia_std::tools::ledger_text::ledger_text(self)
    }

    /// **What the contract beside it was derived from**
    /// ([ADR-251](../../../docs/specification/adr/adr-251.md) D1), in Nikaia.
    fn render_derived(&self) -> String {
        nikaia_std::tools::ledger_text::derived_text(self)
    }

    /// The contract at `path` with the record beside it read in, where there
    /// is one: `nikaia.contracts` and `nikaia.derived`,
    /// `contracts/<crate>.contracts` and `contracts/<crate>.derived`.
    ///
    /// **A contract with no record beside it has no sources**, which every
    /// reader of `sources` already takes for *nothing recorded*, the
    /// fail-closed answer (ADR-100 D3).
    fn read_beside(path: &std::path::Path) -> Option<Ledger> {
        // The files are read here; what they say is `tools/contracts_core.nika`'s.
        let written = std::fs::read_to_string(path).ok()?;
        let derived = std::fs::read_to_string(derived_path(path)).ok();
        nikaia_std::tools::contracts_core::contracts_read_beside(&written, derived)
    }

    /// The same file under a **description's** header
    /// ([ADR-290](../../../docs/specification/adr/adr-290.md) D5), in Nikaia.
    fn render_description(&self, crate_name: &str, version: &str, notes: &Notes) -> String {
        nikaia_std::tools::ledger_text::description_text(self, crate_name, version, notes)
    }

    /// The header line and everything after it, shared by both renderings.
    fn render_from(&self, out: &mut String) {
        self.render_from_with(out, &Notes::empty());
    }

    /// The same, with the describer's notes spliced above the entries they are
    /// about ([ADR-290](../../../docs/specification/adr/adr-290.md) D3), in
    /// Nikaia.
    fn render_from_with(&self, out: &mut String, notes: &Notes) {
        out.push_str(&nikaia_std::tools::ledger_text::contract_text(self, notes));
    }

    /// Read a ledger back - a library's, or this project's own.
    ///
    /// Deliberately a small reader for the small format `render` writes rather
    /// than a TOML parser: the file is generated, so the shapes it can take are
    /// the shapes written above, and a dependency to read one's own output back
    /// is a dependency to keep in step.
    /// **Read in Nikaia** (`tools/ledger.nika`, ADR-294 step (c)): the file's
    /// shape, what each table and key means, and a signature back from the
    /// file's spelling.
    fn parse(text: &str) -> Result<Self> {
        nikaia_std::tools::ledger::read(text).map_err(|refusal| anyhow!("{refusal}"))
    }
}

/// `std`'s ledger, parsed once.
///
/// Embedded at build time and parsed on first use rather than on every
/// inference. **Every reader of `std`'s ledger reads this one** (0.0.296):
/// seven places parsed `STD` again for themselves, which was 83 M of the 93 M
/// instructions an empty program cost. A reader that only reads borrows it; one
/// that adds a package's own entries takes [`std_library`], a copy. It failing to parse is a broken compiler, not a broken program:
/// `crates/nikaia/tests/contracts.rs` reads the same bytes and would have said
/// so long before a user got here.
pub(crate) fn std_ledger() -> &'static Ledger {
    static PARSED: std::sync::OnceLock<Ledger> = std::sync::OnceLock::new();
    PARSED.get_or_init(|| Ledger::parse(STD).expect("std ships a ledger this compiler can read"))
}

/// `std`'s ledger as a library to **build on**: the floor every package's
/// inference starts from, before its own dependencies are absorbed into it
/// ([ADR-100](../../../docs/specification/adr/adr-100.md) D1).
///
/// A clone, because a consumer's library is `std` *plus* what its dependencies
/// published and the shipped one is shared and immutable. One `std` per package
/// inferred is a few hundred entries copied — measured against what it replaces,
/// which is deriving that dependency's whole source tree again.
pub fn std_library() -> Ledger {
    std_ledger().clone()
}

/// The receiver's type, as a caller sees it: the type the `impl` is for, with
/// the `&` the receiver was written with.
/// One method of a `trait`, as a contract a bound can be answered from.
///
/// A builder of its own rather than `Ledger::function` with the body ignored,
/// for the reason the emitter has a second writer: that one records `borrows`,
/// `sharing` and a `touches` set that later passes fill in **by reading the
/// body**, and a declaration has none. What is here is what a declaration can
/// say: whether it pauses, whether it can fail, and what its parameters and
/// result are.
///
/// **`sync` is the declaration's own word**
/// ([ADR-288](../../../docs/specification/adr/adr-288.md) D24): a trait method
/// reads like a function type, so without `sync` it **may pause**, exactly as a
/// function without the word may.
///
/// What produced the contracts: this compiler, not the one it emits Rust for.
///
/// `sync`, `throws` and the borrow contract are decided here and never by
/// `rustc`, so the version that matters to a ledger diff is Nikaia's.
/// Where the derivation record of the contract at `path` is: the same
/// directory and stem, `.derived` for `.contracts`.
pub fn derived_path(path: &std::path::Path) -> std::path::PathBuf {
    path.with_extension("derived")
}

fn toolchain() -> String {
    format!("nikaia {}", env!("CARGO_PKG_VERSION"))
}

/// A value that may itself hold a quote - which a signature does, the moment an
/// option's default is a string: `method: &str = "GET"`.
/// **And a `\n` becomes `\\n`**, which is
/// The name an error gets when the compiler cannot name it - ADR-024 D1's `?`,
/// which is the absence of a claim rather than a type.
pub const UNNAMED_ERROR: &str = "?";

/// What a parse fails with
/// ([ADR-296](../../../docs/specification/adr/adr-296.md) D38).
///
/// A `std` type with no module in front, which is `Overtaken`'s shape: a
/// program never writes a path to it, because it arrives in a `catch`.
pub const PARSE_ERROR: &str = "ParseError";

/// The one parameter every grammar entry takes: the text to parse
/// ([ADR-296](../../../docs/specification/adr/adr-296.md) D24).
///
/// A name rather than four spellings of it, because three analyses have to
/// agree on it: the loop below writes the signature, [`tether::infer`] writes
/// the state of the views a parse hands back, and [`keeps::infer`] says whether
/// the entry keeps it. A column that names a position no signature has is a
/// column nobody can read.
pub const INPUT: &str = "input";

/// The `throws` list exactly as the ledger writes it, in Nikaia
/// (`tools/ledger_text.nika`): the note is the contract (Part III C.4).
pub fn throws_text(throws: &[String]) -> String {
    nikaia_std::tools::ledger_text::throws_text(throws)
}

/// Every type name this file declares.
pub fn declared_types(parsed: &Parsed) -> BTreeSet<String> {
    nikaia_std::tools::ledger_items::types_this_file_declares(&parsed.program, &parsed.interner)
}

/// The type parameters an `impl` head declares, in the order it writes them.
///
/// `impl Stack[T]` is `impl<T> Stack<T>` and `impl Stack[i64]` is
/// `impl Stack<i64>`, and what tells them apart is whether the slot names a
/// type: a bare name that is neither one of Part I 2.2's types nor one this
/// file declares stands for a type rather than being one
/// ([ADR-295](../../../docs/specification/adr/adr-295.md) D4).
///
/// **The `impl` has no parameter list of its own**, and that is the decision
/// rather than a gap: Part I 4.6 writes `struct Box[T]` and nothing writes
/// `impl[T]`, so a second list would be a spelling the specification does not
/// have. The rule above reads the one list that is written.
///
/// One function, called by the ledger and by the emitter, so the names a
/// method's signature is recorded with are the names its `impl` head declares -
/// two rules here would be one silent disagreement.
pub fn impl_parameters(
    parsed: &Parsed,
    target: &crate::ast::Type,
    declared: &BTreeSet<String>,
) -> Vec<String> {
    nikaia_std::tools::ledger_items::impl_parameters(&parsed.interner, target, declared)
}

/// A literal, as the grammar once required a default to be: its text is the
/// value, in this language and the one below.
pub fn a_literal(expr: &Expr) -> bool {
    nikaia_std::tools::ledger_items::a_literal(expr)
}

/// **A build-time value as the literal the ledger records** (ADR-318 D1-D3):
/// an initialiser of literals is folded, and anything else is compiled and run
/// where the build has a workshop (ADR-321 D1), as the checker computes the
/// same value; a call is never interpreted (D19). `None` where it cannot be
/// computed or has no literal - the checker refuses it there.
fn computed_literal(
    units: &[&Parsed],
    parsed: &Parsed,
    name: &str,
    expr: &Expr,
    ty: &ty::Ty,
    library: &Ledger,
) -> Option<String> {
    let reads = crate::assets::Reads::none();
    let nothing = |_: &str| -> Option<crate::build_time::Value> { None };
    // **Literals alone are folded** (D1's last paragraph), with or without a
    // workshop; the folder runs no call, so anything else is compiled.
    if let Ok(folded) =
        crate::build_time::BuildTime::new(parsed, units, &reads, &nothing).evaluate(expr)
    {
        return literal_of(&folded);
    }
    let workshop = crate::comptime_run::defaults_workshop()?;
    let here = units.iter().position(|unit| std::ptr::eq(*unit, parsed))?;
    let ty = crate::comptime_run::as_built(ty.clone());
    match crate::comptime_run::compute(
        units,
        here,
        name,
        expr,
        &ty,
        &nothing,
        &|_: &str| None,
        library,
        &workshop,
        workshop.bounds(),
        None,
    ) {
        crate::comptime_run::Computed::Value(value) => literal_of(&value),
        _ => None,
    }
}

/// A value as the literal both languages spell it the same way, where it has
/// one.
pub fn literal_of(value: &crate::build_time::Value) -> Option<String> {
    nikaia_std::tools::ledger_items::literal_of(value, &|f: f64| format!("{f:?}"))
}

/// What `ledger_infer_package` asks of the compiler (`tools/ledger_items.nika`,
/// #125): the units, the checker and the columns still inferred in Rust, and
/// what they answer, kept for `infer_package_noted` to hand back.
struct InferHost<'a> {
    units: &'a [&'a Parsed],
    library: &'a Ledger,
    checked: RefCell<Vec<crate::check::Checked>>,
    resolved: RefCell<BTreeMap<String, crate::check::MethodCalls>>,
    stored: RefCell<BTreeMap<String, crate::check::StoredCode>>,
    noted: RefCell<Option<sync::Noted>>,
    column: RefCell<Option<keeps::Column>>,
}

impl nikaia_std::tools::ledger_items::LedgerInferHost for InferHost<'_> {
    fn check_units(&self, ledger: &Ledger) {
        // With the other files **beside** it, so a type another file
        // declares has its fields and variants here as there (#125).
        let units = self.units;
        let checked: Vec<crate::check::Checked> = units
            .iter()
            .enumerate()
            .map(|(at, parsed)| {
                let beside: Vec<&Parsed> = units
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != at)
                    .map(|(_, other)| *other)
                    .collect();
                crate::proofs::in_unit(at, || {
                    crate::check::check_for_the_ledger(
                        parsed,
                        &beside,
                        ledger,
                        self.library,
                        &crate::assets::Reads::none(),
                    )
                })
            })
            .collect();
        *self.resolved.borrow_mut() = checked
            .iter()
            .flat_map(|c| c.methods.iter().map(|(k, v)| (k.clone(), v.clone())))
            .collect();
        *self.stored.borrow_mut() = nikaia_std::tools::ledger_items::ledger_items_stored(
            &mut checked
                .iter()
                .map(|c| c.stored_code.clone())
                .collect::<Vec<_>>(),
        );
        *self.checked.borrow_mut() = checked;
    }

    fn infer_sync(&self, ledger: &mut Ledger) {
        let noted = sync::infer(ledger, self.units, self.library, &self.resolved.borrow());
        *self.noted.borrow_mut() = Some(noted);
    }

    fn reads_back(&self, condition: &str, readable: &BTreeSet<String>) -> bool {
        crate::prove::reads_back(condition, readable)
    }

    fn infer_throws(&self, ledger: &mut Ledger) {
        throws::infer(
            ledger,
            self.units,
            self.library,
            &self.resolved.borrow(),
            &self.stored.borrow(),
        );
    }

    fn infer_touch(&self, ledger: &mut Ledger) {
        touch::infer(ledger, self.units, self.library, &self.resolved.borrow());
    }

    fn result_sources(
        &self,
        unit: i64,
        own: &Ledger,
    ) -> Vec<nikaia_std::tools::views::ResultSource> {
        crate::views::result_sources(self.units[unit as usize], own, self.library)
    }

    fn places_stored(
        &self,
        unit: i64,
        own: &Ledger,
    ) -> Vec<nikaia_std::tools::views::PlacesStored> {
        crate::views::places_stored(self.units[unit as usize], own, self.library)
    }

    fn infer_keeps(&self, ledger: &mut Ledger) {
        let column = keeps::infer(ledger, self.units, self.library, &self.resolved.borrow());
        *self.column.borrow_mut() = Some(column);
    }

    fn infer_sharing(&self, ledger: &mut Ledger, unit: i64) {
        sharing::infer(ledger, self.units[unit as usize], self.library);
    }

    fn infer_maps(&self, ledger: &mut Ledger) {
        maps::infer(ledger, self.units);
    }

    fn infer_locks(&self, ledger: &mut Ledger) {
        locks::infer(
            ledger,
            self.units,
            self.library,
            &self.resolved.borrow(),
            &self.stored.borrow(),
        );
    }

    fn infer_tether(&self, ledger: &mut Ledger) {
        tether::infer(ledger, self.units, self.library);
    }

    fn known_default(&self, unit: i64, key: &str, at: i64) -> Option<String> {
        crate::comptime_run::known_default(
            self.units[unit as usize].package.as_deref(),
            key,
            at as usize,
        )
    }

    fn computed_literal(&self, unit: i64, name: &str, expr: &Expr, ty: &ty::Ty) -> Option<String> {
        computed_literal(
            self.units,
            self.units[unit as usize],
            name,
            expr,
            ty,
            self.library,
        )
    }
}

//! **Running a grammar while the program is built is not interpretation**
//! (issue #178, Part II 10.2 A).
//!
//! A grammar could be run here by walking the grammar tree this compiler
//! already holds. **It must not be**, and the reason is not the size of the
//! work. `winnow-grammar` is a code **generator**: its model crate parses the
//! grammar language, validates it, and hands the result to a macro that writes
//! a parser. There is no interpreter in it to borrow — so interpreting would be
//! a **second implementation of the same semantics**, and Part II 10.2's
//! promise that one grammar means the same thing at both stages would stop
//! being a property and become a hope. The disagreements would land in the
//! corners — implicit whitespace, repetition bounds, the commit point, frames
//! and resynchronisation, interning, spans — and would present as *this file
//! parsed while the program was built and fails while it runs*, for the same
//! file and the same grammar.
//!
//! So this compiles the **generated** parser and runs it. Then there is one
//! implementation and the agreement is a tautology rather than a claim.
//!
//! **What it costs is a second compilation**, which [ADR-310](../../../docs/specification/adr/adr-310.md)
//! Q4 named. The sub-project is keyed on what went into it, so the cost is paid
//! when the grammar changes rather than on every build. The compiler already
//! emits Rust and already drives Cargo, so the machinery is not new.
//!
//! **And it needs no new security model.** A grammar's action blocks are
//! Nikaia, and [ADR-287](../../../docs/specification/adr/adr-287.md) already
//! says what a build-time body may do; the bytes come from `asset("…")`, which
//! [ADR-310](../../../docs/specification/adr/adr-310.md) already bounds. What
//! is new here is neither the permission nor the input.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use nikaia_std::rt::exec::block_on;
use nikaia_std::tools::grammar_run::{GrammarRunHost, grammar_run_compile_and_run};

use crate::ast::Item;
use crate::contracts::ty::Ty;
use crate::parser::Parsed;

/// Why a grammar could not be run, where the reason is this compiler's rather
/// than the input's: declared in Nikaia (`tools/check_reports.nika`, ADR-294,
/// #125), the checker's report reads it as it is.
pub use nikaia_std::tools::check_reports::CheckWall as Wall;

/// One run: which grammar, which rule, and the bytes.
pub struct Ask<'a> {
    pub grammar: &'a str,
    pub rule: &'a str,
    /// The bytes the parser is pointed at.
    pub input: &'a str,
    /// What the rule declares it hands back, for the dump this generates.
    pub result: &'a Ty,
    /// The program's files, a dependency's among them: what an action may
    /// call of a dependency is compiled in as the module the program names
    /// it by ([ADR-321](../../../docs/specification/adr/adr-321.md) D13).
    pub units: &'a [&'a Parsed],
}

/// Where a build compiles the parsers it runs, and the guard against nesting.
///
/// **One per build**, carried with the rest of what a build may do while it
/// builds. A build with no place to put a sub-project is one no person runs:
/// every real build has a `target/`, and a bare `check` in a test has none.
#[derive(Debug, Default)]
pub struct Workshop {
    at: Option<PathBuf>,
    /// The grammars being run, innermost last — a ring rather than a depth,
    /// because the message names what it found.
    running: std::sync::Mutex<Vec<String>>,
    /// **The library build-time code links against**
    /// ([ADR-321](../../../docs/specification/adr/adr-321.md) D3), built the
    /// first time a `comptime` needs it and kept for the rest of the build.
    bundle: std::sync::OnceLock<Result<crate::comptime_run::Bundle, String>>,
    /// What a `comptime`'s run may spend (ADR-321 D7, D10).
    bounds: crate::comptime_run::Bounds,
}

impl Workshop {
    /// A build with nowhere to compile a parser.
    pub fn none() -> Workshop {
        Workshop::default()
    }

    /// Whether there is anywhere to compile a parser.
    pub fn somewhere(&self) -> bool {
        self.at.is_some()
    }

    /// A build that compiles its parsers under `at`.
    pub fn at(at: impl Into<PathBuf>) -> Workshop {
        // **Absolute**, because Cargo and `rustc` are started elsewhere
        // (ADR-325 D1).
        let at = at.into();
        Workshop {
            at: Some(std::path::absolute(&at).unwrap_or(at)),
            running: std::sync::Mutex::new(Vec::new()),
            bundle: std::sync::OnceLock::new(),
            bounds: crate::comptime_run::Bounds::default(),
        }
    }

    /// The same, with other bounds on what a `comptime`'s run may spend.
    pub fn bounded(self, bounds: crate::comptime_run::Bounds) -> Workshop {
        Workshop { bounds, ..self }
    }

    /// Where this workshop builds, where it has a place at all.
    pub fn place(&self) -> Option<&Path> {
        self.at.as_deref()
    }

    /// What a `comptime`'s run may spend here.
    pub fn bounds(&self) -> crate::comptime_run::Bounds {
        self.bounds
    }

    /// **Compile and run a `comptime`'s program**
    /// ([ADR-321](../../../docs/specification/adr/adr-321.md) D1): what it
    /// printed, or why it did not (`crate::comptime_run`).
    pub fn run_comptime(&self, program: &str, name: &str) -> Result<String, Wall> {
        let Some(at) = &self.at else {
            return Err(Wall::NowhereToBuild);
        };
        let bundle = self
            .bundle
            .get_or_init(|| crate::comptime_run::bundle(at))
            .as_ref()
            .map_err(|detail| Wall::DidNotBuild {
                detail: detail.clone(),
            })?;
        crate::comptime_run::run(at, bundle, program, name)
    }

    /// Run `ask`'s rule over `ask`'s file, in a parser compiled from `parsed`.
    ///
    /// `parsed` is the file the **grammar** was declared in, which need not be
    /// the file the `comptime` stands in: a program's files share one namespace
    /// (Part I 9.1) and each owns the interner its symbols resolve in.
    pub fn run(&self, parsed: &Parsed, ask: &Ask<'_>) -> Result<String, Wall> {
        let Some(at) = &self.at else {
            return Err(Wall::NowhereToBuild);
        };
        // **A grammar run inside a grammar run is refused**, because the inner
        // one would compile a compiler. The sub-project carries no item-level
        // `comptime`, so what reaches here is one inside a body.
        {
            let mut running = self.running.lock().map_err(|_| Wall::DidNotBuild {
                detail: "the workshop's lock was poisoned".to_string(),
            })?;
            if let Some(outer) = running.first() {
                return Err(Wall::InsideAnother {
                    outer: outer.clone(),
                });
            }
            running.push(ask.grammar.to_string());
        }
        let host = Host {
            parsed,
            ask,
            refused: RefCell::new(None),
        };
        let outcome = block_on(grammar_run_compile_and_run(&host, at, ask.input));
        if let Ok(mut running) = self.running.lock() {
            running.pop();
        }
        // **The driver's own refusal comes back as it was**: the lowered
        // function sees only that the driver threw, and the host kept why.
        match (host.refused.into_inner(), outcome) {
            (Some(wall), _) => Err(wall),
            (None, outcome) => outcome.map_err(|thrown| thrown.split().0),
        }
    }
}

/// The sub-project's manifest.
///
/// **Its own workspace**, because it lives under a `target/` that may be inside
/// one: a member Cargo did not expect is an error about a file nobody wrote.
/// The dependencies are the ones a generated program already gets
/// (`project::runtime_dependencies`), so the parser compiled here is the parser
/// the program links.
/// Where every grammar run's parser is compiled: the user cache, beside the
/// compiled `std` (ADR-002 D4).
///
/// One entry per compiler, so an old compiler's parsers are evicted as a whole
/// (#615).
fn shared_target() -> PathBuf {
    let compiler = orchestrator::cache::sha256_hex(
        format!(
            "{}\n{}",
            env!("CARGO_PKG_VERSION"),
            env!("NIKAIA_RUSTC_VERSION")
        )
        .as_bytes(),
    );
    let entry = orchestrator::cache::Layout::user_cache_dir()
        .join("build-time-parsers")
        .join(&compiler[..16]);
    orchestrator::cache::use_entry(&entry);
    entry.join("target")
}

/// What the lowered build and run (`tools/grammar_run.nika`, GR) asks of the
/// compiler: the driver, the manifest and the two directories.
struct Host<'a> {
    parsed: &'a Parsed,
    ask: &'a Ask<'a>,
    /// Why the driver refused, handed back after the run.
    refused: RefCell<Option<Wall>>,
}

impl GrammarRunHost for Host<'_> {
    fn driver(&self) -> Result<String, Box<dyn std::error::Error>> {
        driver(self.parsed, self.ask).map_err(|wall| {
            let message = wall.to_string();
            *self.refused.borrow_mut() = Some(wall);
            message.into()
        })
    }

    fn manifest(&self, key: &str, program: &str) -> String {
        manifest(key, program)
    }

    fn shared_target(&self) -> PathBuf {
        shared_target()
    }

    fn where_tools_start(&self) -> PathBuf {
        orchestrator::cache::Layout::where_tools_start()
    }
}

fn manifest(key: &str, program: &str) -> String {
    let dependencies = crate::project::runtime_dependencies_for(program);
    nikaia_std::tools::grammar_decode::gr_manifest(key, &dependencies)
}

/// The whole sub-program, written by `tools/grammar_run.nika`'s
/// `grammar_run_driver` (#125): what stays here is choosing the files it
/// lowers, which are the program's syntax, and lowering them.
fn driver(parsed: &Parsed, ask: &Ask<'_>) -> Result<String, Wall> {
    let without_main_and_constants = |unit: &Parsed| {
        unit.keeping(|item| match item {
            // **The program's `main` is not this program's** — the driver
            // below is.
            Item::Fn { name, .. } => name.map(|n| unit.text(n) != "main").unwrap_or(true),
            // **And its constants are not needed**, which is also what stops
            // this recursing: a grammar run happens in a `comptime`
            // initialiser, and the sub-project has none to evaluate.
            Item::Comptime { .. } => false,
            _ => true,
        })
        // **The sub-program owns what the program views**
        // ([ADR-179](../../../docs/specification/adr/adr-179.md) D3): a rule
        // action builds a `Vec`, so a struct field the program declares
        // `&[T]` is a `Vec[T]` here. The dump below is generated from the
        // **program's** declaration and reads a run either way, which is what
        // keeps the two sides one crossing rather than two opinions.
        .growing()
    };
    // **The package's other files** (ADR-321 D13): a package is one
    // namespace (Part I 9.1), so an action may call a function a file beside
    // the grammar's declares. Each without its `main` and its `comptime`s, as
    // the grammar's own file is.
    let package: Vec<&Parsed> = ask
        .units
        .iter()
        .copied()
        .filter(|u| u.package == parsed.package && !std::ptr::eq(*u, parsed))
        .collect();
    let mut all: Vec<Parsed> = std::iter::once(parsed)
        .chain(package.iter().copied())
        .map(without_main_and_constants)
        .collect();
    // **And without what reads a constant** (#528): a function or a test that
    // names one would name what this program does not declare.
    let constants = crate::comptime_run::constants_of(std::iter::once(parsed).chain(package));
    let _ = crate::comptime_run::without_readers(&mut all, constants);
    let beside = all.split_off(1);
    let items = all.pop().expect("the grammar's own file");
    let others: Vec<&Parsed> = ask
        .units
        .iter()
        .copied()
        .filter(|u| u.package.is_some() && u.package != parsed.package)
        .collect();
    let host = Driver {
        parsed,
        items,
        beside,
        others,
        ledger: std::cell::OnceCell::new(),
    };
    nikaia_std::tools::grammar_run::grammar_run_driver(
        &host,
        &parsed.interner,
        &parsed.aliases,
        &parsed.program,
        crate::contracts::std_ledger(),
        ask.result,
        ask.grammar,
        ask.rule,
    )
    // **Every refusal the driver throws is a `Wall`**, handed back as it was.
    .map_err(|thrown| match nikaia_std::error::holds::<Wall>(&*thrown) {
        true => nikaia_std::error::opened::<Wall>(thrown),
        false => Wall::DidNotBuild {
            detail: thrown.to_string(),
        },
    })
}

/// The files a grammar run's driver lowers: the grammar's own without its
/// `main` and constants, the package's files beside it, and the dependencies.
struct Driver<'a> {
    parsed: &'a Parsed,
    items: Parsed,
    beside: Vec<Parsed>,
    /// **A dependency's files, for the actions that call them** (D13). The
    /// grammar's own package is the program here, and the program's own
    /// files are not visible from a dependency's grammar.
    others: Vec<&'a Parsed>,
    /// The package's ledger over the grammar's file and those beside it,
    /// inferred the first time a file beside it is lowered.
    ledger: std::cell::OnceCell<crate::contracts::Ledger>,
}

impl nikaia_std::tools::grammar_run::GrammarDriverHost for Driver<'_> {
    fn lowered_own(&self) -> Result<String, Box<dyn std::error::Error>> {
        crate::emit::emit_program(&self.items, crate::emit::Build::default())
            .map(|emitted| emitted.rust)
            .map_err(|error| format!("{error:#}").into())
    }

    fn beside(&self) -> i64 {
        i64::try_from(self.beside.len()).unwrap_or(i64::MAX)
    }

    fn lowered_beside(&self, index: i64) -> Result<String, Box<dyn std::error::Error>> {
        use crate::contracts::LedgerOps;
        let index = usize::try_from(index)?;
        let mut all: Vec<&Parsed> = vec![&self.items];
        all.extend(self.beside.iter());
        let ledger = self.ledger.get_or_init(|| {
            crate::contracts::Ledger::infer_package(&all, crate::contracts::std_ledger())
        });
        let provenance =
            crate::contracts::trust::analyse(self.parsed, crate::contracts::std_ledger())
                .provenance;
        let unit = &crate::comptime_run::without_foreign(&self.beside[index], ledger);
        crate::emit::emit_module_body_at(
            unit,
            &all,
            crate::emit::Build::default(),
            provenance,
            ledger,
            &crate::contracts::Ledger::blank(),
            false,
            &crate::assets::Reads::none(),
            None,
        )
        .map(|body| body.rust)
        .map_err(|error| format!("{error:#}").into())
    }

    fn dependency_modules(&self) -> Option<String> {
        let packages = crate::comptime_run::by_package(&self.others);
        let provenance =
            crate::contracts::trust::analyse(self.parsed, crate::contracts::std_ledger())
                .provenance;
        crate::comptime_run::dependency_modules(
            &self.others,
            &packages,
            crate::emit::Build::default(),
            provenance,
        )
    }
}

/// **Reading back what the sub-program wrote.**
///
/// The encoder is generated above and this reads it — two halves of one format,
/// which is the shape [`crate::fixed`] already carries a warning about. What
/// holds them together is not care but a test that **runs** a program: a
/// decoder that agreed with a generator it never met would prove nothing.
///
/// The form is s-expressions, one per shape a build-time value has:
/// `(i 42)`, `(f 1.5)`, `(b true)`, `(s "…")`, `(l …)`, `(t Name (field …) …)`
/// and `(v Ty Variant …)`. Text carries Rust's escapes, which is the set the
/// rest of this compiler reads.
///
/// **A variant's tag is two words and not one**, which is the whole of what
/// the one defect here was: `(v Shade Odd)` read the type and then read the
/// variant from the *same* position, so the variant came back empty and the
/// payload loop met an `O`. A separator between two words has to be skipped by
/// whoever reads the second one.
pub fn decode(text: &str) -> Result<crate::build_time::Value, Wall> {
    match nikaia_std::tools::grammar_decode::gr_decode(text, &crate::build_time::char_of) {
        nikaia_std::tools::grammar_decode::GrDecoded::Value(value) => Ok(value),
        nikaia_std::tools::grammar_decode::GrDecoded::Unreadable(detail) => {
            Err(Wall::Unreadable { detail })
        }
    }
}

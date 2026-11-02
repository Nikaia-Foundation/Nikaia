// crates/nikaia/src/diagnostics/mod.rs
//
// Where an error is reported.
//
// Stage 0 emits Rust and hands it to `rustc`, which then reports everything -
// its own type errors and the parser backend's frame check alike - against a
// file the user never wrote. That is not a small annoyance: it is the whole
// difference between a compiler and a code generator with a compiler bolted on.
//
// The fix is not a second checker (ADR-296 D18). It is a map: the emitter
// records which `.nika` span produced which byte range of the emitted Rust
// (`emit::SourceMap`), rustc reports byte offsets into that same text with
// `--error-format=json`, and this module trades one for the other.
//
// What it deliberately does not do is rewrite the message. Because the lowering
// is name-for-name (ADR-296 D17), "`until(…)` in rule `NAME` can run through the
// boundary" is already a sentence about the Nikaia source - only its position
// was wrong. Correcting a position is a lookup; translating a text would be a
// second compiler.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::ast::Span;
use crate::emit::SourceMap;

/// A statement about the **user's program**, as opposed to a failure of this
/// compiler.
///
/// The two used to leave by the same door, and the door was Rust's: a type error
/// in somebody's `.nika` file ended as an `anyhow::Error` out of `main`, so after
/// the diagnostics had said their piece the terminal also got
///
/// ```text
/// Error: 1 type error
///
/// Stack backtrace:
///    0: anyhow::error::<impl anyhow::Error>::msg
///    1: nikaia::project::check
/// ```
///
/// Ten frames of this compiler's own functions, reached whenever
/// `RUST_BACKTRACE` is set, which is a normal thing for a developer to have set.
/// Part III C.1's rule is about messages from the backend, and this is the same
/// promise from the other side: nothing about how this compiler is built reaches
/// somebody who only wrote a program.
///
/// **An error that is not one of these keeps the backtrace**, deliberately. A
/// compiler that cannot read a file or cannot run `rustc` has a failure of its
/// own, and the frames are then the most useful thing on the screen.
#[derive(Debug)]
pub struct Refused {
    pub message: String,
    /// The byte the statement this is about starts at, where the refusal knew
    /// one ([ADR-171](../../../docs/specification/adr/adr-171.md) D1).
    ///
    /// **A refusal from the lowering had no position at all**, so a reader was
    /// told what was wrong and never where — which
    /// [Part III C.2](../../../docs/specification/30-nikaia-tooling.md) asks of
    /// every diagnostic and which every `NK…` code already does. The lowering
    /// has the byte: `Flow::statement` carries it for the type checker's sake,
    /// and it is the same number.
    ///
    /// `None` where the refusal is about no statement in particular — a
    /// manifest key, a switch, a whole file.
    pub at: Option<usize>,
    /// The whole message, where the refusal is one: a parse error is said the
    /// way a checker finding is, and rendered by whoever holds the file's path
    /// ([`refused_finding`]).
    pub finding: Option<Box<crate::check::Finding>>,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(&self.message)
    }
}

impl std::error::Error for Refused {}

/// Whether an error - or anything it was wrapped in - is a statement about the
/// program rather than about this compiler.
///
/// The whole chain, because the path a file was read from is added as context
/// around a refusal and the refusal is then no longer the outermost error.
pub fn is_a_refusal(error: &anyhow::Error) -> bool {
    error.chain().any(|link| link.is::<Refused>())
}

/// A refusal, written the way `anyhow!` is written - and [`refuse!`], written the
/// way `bail!` is.
///
/// Macros, and named to mirror that pair exactly, so that a call site changes by
/// one word: `anyhow!(…)` becomes `refused!(…)` and `bail!(…)` becomes
/// `refuse!(…)`, arguments and all. What makes that worth two macros is that
/// **the choice between them is the whole decision** - is this a statement about
/// the program, or a failure of this compiler? - and a reader should be able to
/// see which was made at a glance, rather than by unpicking a `format!`.
#[macro_export]
macro_rules! refused {
    ($($arg:tt)*) => {
        $crate::diagnostics::refuse(format!($($arg)*))
    };
}

/// `bail!`, for a statement about the program. See [`refused!`].
#[macro_export]
macro_rules! refuse {
    ($($arg:tt)*) => {
        return Err($crate::diagnostics::refuse(format!($($arg)*)))
    };
}

/// [`refused!`], about a **statement the program wrote**
/// ([ADR-171](../../../docs/specification/adr/adr-171.md) D1).
///
/// The first argument is the byte it starts at, which the lowering has in
/// `Flow::statement` wherever it is emitting one.
#[macro_export]
macro_rules! refused_at {
    ($at:expr_2021, $($arg:tt)*) => {
        $crate::diagnostics::refuse_at($at, format!($($arg)*))
    };
}

/// Make a refusal, as an `anyhow::Error` so it travels the paths every other
/// error already travels.
pub fn refuse(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(Refused {
        message: message.into(),
        at: None,
        finding: None,
    })
}

/// A refusal that is a whole finding: a parse error, which the parser can say
/// but not place, since it has the source and not the path. `message` is what
/// a caller that has no path prints.
pub fn refuse_finding(finding: crate::check::Finding, message: String) -> anyhow::Error {
    anyhow::Error::new(Refused {
        message,
        at: Some(finding.span.at()),
        finding: Some(Box::new(finding)),
    })
}

/// The finding a refusal carries, for the caller that can render it.
pub fn refused_finding(error: &anyhow::Error) -> Option<&crate::check::Finding> {
    error
        .chain()
        .find_map(|link| link.downcast_ref::<Refused>())
        .and_then(|refused| refused.finding.as_deref())
}

/// The same, about a **statement** the program wrote
/// ([ADR-171](../../../docs/specification/adr/adr-171.md) D1).
///
/// The byte is rendered into a line and a caret by whoever holds the source,
/// which is the unit being lowered — the same arrangement `render_finding` has,
/// because a refusal the lowering makes and one the checker makes should not
/// look different to a reader.
pub fn refuse_at(at: usize, message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(Refused {
        message: message.into(),
        at: Some(at),
        finding: None,
    })
}

/// A refusal's position, where it has one and the error is a refusal at all.
pub fn refusal_at(error: &anyhow::Error) -> Option<(usize, String)> {
    error
        .chain()
        .find_map(|link| link.downcast_ref::<Refused>())
        .and_then(|refused| refused.at.map(|at| (at, refused.message.clone())))
}

/// **A refusal from the lowering, on the line it is about**
/// ([ADR-171](../../../docs/specification/adr/adr-171.md) D2).
///
/// The same shape `NK2202` and every relayed `rustc` message use, because a
/// rule this compiler enforces in the lowering should not look different from
/// one it enforces in the checker. There is no code in front of it: these are
/// refusals the catalogue does not name, and inventing numbers for them is
/// [ADR-171](../../../docs/specification/adr/adr-171.md) §4's own open question.
pub fn render_refusal(message: &str, at: usize, path: &str, source: &str) -> String {
    let finding = crate::check::Finding {
        severity: crate::check::Severity::Error,
        span: crate::ast::Span::new(at, at),
        code: String::new(),
        message: message.to_string(),
        notes: Vec::new(),
        help: None,
        labels: Vec::new(),
    };
    render_finding(&finding, path, source)
}

/// Whether a backend diagnostic is about something somebody wrote.
///
/// **A warning that maps to no Nikaia line does not reach the user.** Part III
/// C.1's rule is that a message about the generated Rust is not a message the user
/// gets, and a warning with no `.nika` line behind it is exactly that: it is about
/// a decision the emitter made. The measured case was every multi-file build
/// printing `unused import: super::*` - a `use` item this compiler writes itself -
/// with a note saying *"no Nikaia source maps to this"* beside it. Recognising it
/// and printing it anyway is the worst of the three possible behaviours.
///
/// **An error is kept even with nowhere to put it**, and that is not an
/// inconsistency. A warning suppressed costs nothing: the build goes on and the
/// user was never able to act on it. An error suppressed leaves a build that failed
/// with no reason given anywhere, which is worse than a message pointing at a
/// generated line. Such an error is a defect in this compiler - the lowering
/// produced Rust that does not compile - and it is reported so that the defect is
/// visible rather than silent.
pub fn is_about_the_program(diagnostic: &Diagnostic) -> bool {
    diagnostic.location.is_some() || diagnostic.level != "warning"
}

/// One message, placed in the `.nika` file where that was possible.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: String,
    pub message: String,
    /// Where in the `.nika` source, if the map knew.
    pub location: Option<Location>,
    /// Where in the emitted Rust it was reported, kept whether or not the map
    /// knew: an unmapped error still has to be reportable, and a mapped one is
    /// worth being able to check against its origin.
    pub generated_line: Option<usize>,
    pub notes: Vec<String>,
    /// The backend's own words, kept **only** where translating them made the
    /// message say the same thing on both sides of an `expected … found …`.
    ///
    /// That is not a message about the program: it is this compiler having
    /// emitted two different Rust types for one Nikaia type, which is a defect
    /// of its own ([ADR-056](../../../../docs/specification/adr/adr-056.md) D2).
    /// So the message becomes an internal error and this is what it carries -
    /// shown to the reader and written to a log, because whoever fixes the
    /// compiler needs the words the compiler below actually said.
    pub internal: Option<String>,
    /// What the backend wrote under the caret, where it wrote anything: the
    /// half of *mismatched types* that says which types
    /// ([ADR-290](../../../../docs/specification/adr/adr-290.md) D21 reads it).
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Location {
    /// 1-based, as an editor counts.
    pub line: usize,
    pub column: usize,
    pub span: Span,
    /// Which file of the program, as an index into the units the map was built
    /// from. A single-file program has one, and it is 0.
    ///
    /// The map has carried this since a program could be several files
    /// (ADR-300); until the project build started reading these messages,
    /// nothing asked for it, and a message about the third module would have
    /// been printed against the first module's text.
    pub unit: usize,
}

/// Turn `rustc --error-format=json` output about the emitted Rust into
/// messages about the Nikaia source.
///
/// Lines that are not JSON, and diagnostics that carry no information about a
/// place (rustc's own "aborting due to N previous errors"), are dropped.
pub fn translate(rustc_json: &str, map: &SourceMap, source: &str) -> Vec<Diagnostic> {
    translate_units(rustc_json, map, &[source])
}

/// The same, for a program of several files - and for what **`cargo`** writes.
///
/// Two things make one function serve both. A program is several `.nika` files
/// joined into one Rust file (Part I 9.1), and the map already knows which file
/// each span came from, so the line a message is about has to be looked up in
/// that file's text rather than in the first one's. And `cargo
/// --message-format=json` wraps each `rustc` diagnostic in a line of its own
/// (`{"reason": "compiler-message", "message": {...}}`), which is one
/// `["message"]` away from what `rustc --error-format=json` writes directly.
///
/// The second is why the project build can report anything at all. Until it
/// existed, `nikaia build` handed Cargo's stderr to the terminal untouched, so
/// every backend diagnostic reached the user as Rust about a generated file -
/// which Part III C.1 calls a bug in this compiler, and which
/// [ADR-005](../../../../docs/specification/adr/adr-005.md) D7 now enumerates
/// `E0277` among.
pub fn translate_units(json: &str, map: &SourceMap, sources: &[&str]) -> Vec<Diagnostic> {
    // **Read here, placed in Nikaia** (`tools/diagnostics.nika`, ADR-294,
    // #125): this side reads the JSON and asks the emitter's map where the
    // primary byte came from; what the message becomes is decided there.
    use nikaia_std::tools::diagnostics::{DiagnosticsChild, DiagnosticsRead};
    let mut read = Vec::new();
    for line in json.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        // Cargo's own lines ("compiler-artifact", "build-finished") carry no
        // diagnostic; the one that does carries it under `message`.
        let value = match value["reason"].as_str() {
            Some("compiler-message") => value["message"].clone(),
            Some(_) => continue,
            None => value,
        };
        let primary = value["spans"].as_array().and_then(|spans| {
            spans
                .iter()
                .find(|s| s["is_primary"].as_bool() == Some(true))
        });
        let located = primary
            .and_then(|s| s["byte_start"].as_u64())
            .and_then(|offset| map.locate(offset as usize));
        read.push(DiagnosticsRead {
            level: value["level"].as_str().unwrap_or_default().to_string(),
            message: value["message"].as_str().unwrap_or_default().to_string(),
            generated_line: primary
                .and_then(|s| s["line_start"].as_u64())
                .map(|l| l as i64),
            label: primary
                .and_then(|s| s["label"].as_str())
                .map(str::to_string),
            located: located.is_some(),
            unit: located.map_or(0, |(unit, _)| unit as i64),
            span: located.map_or(Span::from(0..0), |(_, span)| span),
            children: value["children"]
                .as_array()
                .map(|children| {
                    children
                        .iter()
                        .map(|c| DiagnosticsChild {
                            level: c["level"].as_str().map(str::to_string),
                            message: c["message"].as_str().map(str::to_string),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    nikaia_std::tools::diagnostics::diagnostics_placed(&read, sources)
        .into_iter()
        .map(|placed| Diagnostic {
            level: placed.level,
            message: placed.message,
            location: placed.located.then_some(Location {
                line: placed.line as usize,
                column: placed.column as usize,
                span: placed.span,
                unit: placed.unit as usize,
            }),
            generated_line: placed.generated_line.map(|l| l as usize),
            notes: placed.notes,
            internal: placed.internal,
            label: placed.label,
        })
        .collect()
}

/// **A boundary whose ledger may be stale**
/// ([ADR-290](../../../../docs/specification/adr/adr-290.md) D21): a package
/// this program depends on, or a Rust crate it describes, by the word a call
/// writes in front of `::`.
pub use nikaia_std::tools::boundaries::Boundary;

/// **A boundary mismatch the backend reports is said as a stale ledger**
/// ([ADR-100](../../../../docs/specification/adr/adr-100.md) D6,
/// [ADR-290](../../../../docs/specification/adr/adr-290.md) D21). Which
/// boundary, and what the message becomes, is **`tools/boundaries.nika`**'s
/// answer (#125); where it has none, the message is left as the backend said
/// it - this compiler's defect, reported as such (Part III C.1).
pub fn a_stale_boundary(
    diagnostic: &mut Diagnostic,
    source: &str,
    boundaries: &[Boundary],
) -> bool {
    let reported = nikaia_std::tools::boundaries::Reported {
        level: diagnostic.level.clone(),
        message: diagnostic.message.clone(),
        label: diagnostic.label.clone(),
        notes: diagnostic.notes.clone(),
    };
    let on_the_line = diagnostic
        .location
        .as_ref()
        .and_then(|at| source.get(at.span.bytes()))
        .map(str::to_string);
    let Some(restated) = nikaia_std::tools::boundaries::restated(
        &reported,
        on_the_line.as_deref(),
        source,
        boundaries,
    ) else {
        return false;
    };
    diagnostic.message = restated.message;
    diagnostic.notes = restated.notes;
    true
}

/// Render a diagnostic the way a compiler does: the place, the message, the
/// line it is about, and what part of it.
/// Write the backend's own words for every internal error to a log beside the
/// generated Rust, and answer where it went
/// ([ADR-056](../../../../docs/specification/adr/adr-056.md) D2).
///
/// Appended rather than replaced, because a build reports what that build found
/// and a reader collecting a report wants the run before it too. A log that
/// cannot be written costs the report and never the build: this is already a
/// message about a defect, and failing to record it would replace a bad message
/// with no message.
pub fn log_internal(diagnostics: &[Diagnostic], gen_dir: &Path) -> Option<PathBuf> {
    use std::io::Write;

    let originals: Vec<&String> = diagnostics
        .iter()
        .filter_map(|d| d.internal.as_ref())
        .collect();
    if originals.is_empty() {
        return None;
    }

    let path = gen_dir.join("internal-errors.log");
    std::fs::create_dir_all(gen_dir).ok()?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    for original in originals {
        writeln!(file, "{original}").ok()?;
    }
    Some(path)
}

pub fn render(diagnostic: &Diagnostic, path: &str, source: &str, generated_path: &str) -> String {
    // **In Nikaia** (`tools/render.nika`, ADR-294, #125): the layout, the
    // internal error and the message nothing maps.
    let (located, span, word) = match &diagnostic.location {
        Some(location) => (
            true,
            location.span,
            source
                .get(location.span.bytes())
                .unwrap_or_default()
                .lines()
                .next()
                .unwrap_or_default()
                .to_string(),
        ),
        None => (false, Span::from(0..0), String::new()),
    };
    nikaia_std::tools::render::relayed(
        &nikaia_std::tools::render::Relayed {
            level: diagnostic.level.clone(),
            message: diagnostic.message.clone(),
            internal: diagnostic.internal.clone(),
            generated_line: diagnostic.generated_line.unwrap_or(0) as i64,
            located,
            span,
            word,
            notes: diagnostic.notes.clone(),
        },
        path,
        source,
        generated_path,
    )
}

/// A finding of the compiler's own, reported the way it reports rustc's:
/// `NK2202`, a `sync` promise something in the body breaks. What it says -
/// a construct that waits, a call `std` or this program says can pause, or a
/// call another package never promised - is **`tools/render.nika`**'s
/// (`paused`, #125).
pub fn render_sync_violation(
    violation: &crate::contracts::sync::Violation,
    path: &str,
    source: &str,
) -> String {
    let pause = nikaia_std::tools::render::Pause {
        span: violation.span,
        caller: violation.caller.clone(),
        promise: violation.promise.clone(),
        callee: violation.callee.clone(),
        from_library: violation.from_library,
        unpromised: violation.unpromised,
        construct: violation.construct,
    };
    nikaia_std::tools::render::rendered(&nikaia_std::tools::render::paused(&pause), path, source)
}

/// One of the type checker's findings, on the `.nika` line it is about.
///
/// The same shape `NK2202` and every relayed `rustc` message use, because a
/// rule the compiler checks itself should not look different from one it
/// relays (ADR-300). Part III C.2 asks for a headline, the reason, and one
/// concrete way out; a `Finding` carries all three and **`tools/render.nika`**
/// writes them down (#125): the headline, the place, each label underlined
/// whole (`^` where the error is, `-` where something explains it), the notes
/// and the help.
pub fn render_finding(finding: &crate::check::Finding, path: &str, source: &str) -> String {
    nikaia_std::tools::render::rendered(&shown(finding), path, source)
}

/// [`render_finding`] over a source already split into its lines, for a caller that renders every finding of one file (#727).
pub fn render_finding_in(finding: &crate::check::Finding, path: &str, lines: &[&str]) -> String {
    nikaia_std::tools::render::rendered_in(&shown(finding), path, lines)
}

fn shown(finding: &crate::check::Finding) -> nikaia_std::tools::render::Shown {
    use nikaia_std::tools::render::{Marked, Shown};
    Shown {
        warning: matches!(finding.severity, crate::check::Severity::Warning),
        code: finding.code.to_string(),
        message: finding.message.clone(),
        notes: finding.notes.clone(),
        help: finding.help.clone(),
        span: finding.span,
        labels: finding
            .labels
            .iter()
            .map(|label| Marked {
                span: label.span,
                word: label.word.clone(),
                text: label.text.clone(),
                main: label.main,
            })
            .collect(),
    }
}

/// **A line of a message as a sentence** (Part III C.2, rule 5): it starts
/// with a capital and ends with a full stop, a question mark or a colon. Said
/// here, once, so that a message built from pieces reads like one written
/// whole, whichever piece came first.
pub fn said(text: &str) -> String {
    nikaia_std::tools::rustc_words::said(text)
}

#[cfg(test)]
mod stale_boundaries {
    use super::*;

    fn an_error(message: &str, span: std::ops::Range<usize>) -> Diagnostic {
        let span = Span::from(span);
        Diagnostic {
            level: "error".to_string(),
            message: message.to_string(),
            location: Some(Location {
                line: 1,
                column: 1,
                span,
                unit: 0,
            }),
            generated_line: None,
            notes: Vec::new(),
            internal: None,
            label: None,
        }
    }

    fn boundary(word: &str, described: bool) -> Boundary {
        Boundary {
            word: word.to_string(),
            described,
            moved: Vec::new(),
        }
    }

    /// **The boundary the line calls is the one named**, as a package's ledger
    /// or a crate's description, and the backend's words are kept.
    #[test]
    fn the_line_names_its_boundary() {
        let source = "let n = util::count()\nlet m = fremd::drei()\n";
        let boundaries = [boundary("util", false), boundary("fremd", true)];

        let mut on_util = an_error("`i64` is not a future", 8..21);
        assert!(a_stale_boundary(&mut on_util, source, &boundaries));
        assert_eq!(
            on_util.message,
            "The ledger of `util` doesn't match its sources."
        );
        assert!(on_util.notes[0].contains("is not a future"), "{on_util:#?}");

        let mut on_fremd = an_error(
            "the `?` operator can only be applied to values that implement `Try`",
            30..43,
        );
        assert!(a_stale_boundary(&mut on_fremd, source, &boundaries));
        assert_eq!(
            on_fremd.message,
            "The description of `fremd` doesn't match the crate."
        );
    }

    /// **Off the line, a boundary whose sources moved is named first**, and
    /// what moved is said; else the only one the file calls. Written in
    /// Nikaia since 0.0.358 (`tools/boundaries.nika`).
    #[test]
    fn off_the_line_the_moved_or_the_only_boundary() {
        let source = "let n = util::count()\nlet m = fremd::drei()\nlet k = n + m\n";
        let mut moved = boundary("fremd", true);
        moved.moved = vec!["src/lib.rs".to_string()];
        let boundaries = [boundary("util", false), moved];
        let mut off = an_error("cannot add `{integer}` to `Result<i64, String>`", 44..49);
        assert!(a_stale_boundary(&mut off, source, &boundaries));
        assert_eq!(
            off.message,
            "The description of `fremd` doesn't match the crate."
        );
        assert!(
            off.notes
                .iter()
                .any(|note| note == "`fremd`'s src/lib.rs changed since it was described."),
            "{off:#?}"
        );

        let only = [boundary("util", false)];
        let mut alone = an_error("`i64` is not a future", 44..49);
        assert!(a_stale_boundary(
            &mut alone,
            "let n = util::count()\nlet k = n\n",
            &only
        ));
        assert_eq!(
            alone.message,
            "The ledger of `util` doesn't match its sources."
        );
    }

    /// **Silence where the shape is not a boundary's, or no boundary is
    /// called**: that is this compiler's own defect, said as the backend said
    /// it.
    #[test]
    fn nothing_else_is_renamed() {
        let boundaries = [boundary("fremd", true)];
        let mut other = an_error("mismatched types", 0..5);
        assert!(!a_stale_boundary(&mut other, "fremd::x()", &boundaries));
        let mut unnamed = an_error("`i64` is not a future", 0..5);
        assert!(!a_stale_boundary(&mut unnamed, "let n = 1", &boundaries));
        assert_eq!(unnamed.message, "`i64` is not a future");
    }
}

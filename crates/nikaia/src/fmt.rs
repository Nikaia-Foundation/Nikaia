//! `nikaia fmt` ([ADR-337](../../../docs/specification/adr/adr-337.md),
//! Part III 13.2). The printer is Nikaia (`tools/fmt.nika`); this is the
//! adapter: it parses the file for its comments (D2), hands the printer their
//! places in characters, and guards what comes back (D3).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use orchestrator::cache::Layout;

use crate::parser::parse_to_ast;
use crate::project::EXTENSION;
use crate::refuse;

/// The file in the one style, or why it cannot be formatted.
///
/// **The guard** (D2, D3): the result is parsed again, and it has to be the
/// same tree, with its places set aside, and the same comments, in the same
/// order. The same tree lowers to the same Rust, so a formatter error is
/// refused here rather than written.
pub fn format(source: &str) -> Result<String> {
    let parsed = parse_to_ast(source)?;
    let mut starts = Vec::with_capacity(parsed.comments.len());
    let mut ends = Vec::with_capacity(parsed.comments.len());
    for span in &parsed.comments {
        starts.push(characters_before(source, span.start as usize));
        ends.push(characters_before(source, span.end as usize));
    }
    let formatted = nikaia_std::tools::fmt::fmt_source(source, starts, ends)
        .map_err(|refusal| anyhow::anyhow!("{refusal}"))?;
    let again = match parse_to_ast(&formatted) {
        Ok(again) => again,
        Err(error) => bail!("the formatted file does not parse: {error}"),
    };
    if comments_of(source, &parsed.comments) != comments_of(&formatted, &again.comments) {
        bail!("formatting would move or lose a comment");
    }
    if placeless(&format!("{:#?}", parsed.program)) != placeless(&format!("{:#?}", again.program)) {
        bail!("formatting would change what the file means");
    }
    Ok(formatted)
}

/// How many characters stand before this byte.
fn characters_before(source: &str, byte: usize) -> i64 {
    source[..byte].chars().count() as i64
}

fn comments_of<'a>(source: &'a str, spans: &[crate::ast::Span]) -> Vec<&'a str> {
    spans
        .iter()
        .map(|span| &source[span.start as usize..span.end as usize])
        .collect()
}

/// A tree's printed form, one field a line (`{:#?}`), with every place in
/// the text taken out: a span's two ends, a node's offset, and a node's id,
/// which counts the parser's attempts rather than anything written.
fn placeless(tree: &str) -> String {
    let mut idless = String::with_capacity(tree.len());
    let mut rest = tree;
    while let Some(at) = rest.find("NodeId(") {
        idless.push_str(&rest[..at + "NodeId(".len()]);
        rest = rest[at + "NodeId(".len()..].trim_start_matches(|c: char| c.is_ascii_digit());
    }
    idless.push_str(rest);
    idless
        .lines()
        .filter(|line| {
            let field = line.trim_start();
            let place = ["start: ", "end: ", "at: "]
                .iter()
                .find_map(|name| field.strip_prefix(name));
            !place.is_some_and(|rest| {
                let digits = rest.trim_end_matches(',');
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `nikaia fmt [FILE...] [--check]` (D5): the exit code.
///
/// No file named means every `.nika` file of the package `start` stands in:
/// the one whose `nikaia.toml` is found walking up, its directories read
/// down to the next package's own `nikaia.toml`, without `target/` and the
/// hidden ones. A directory named is read the same way. Without `--check` a
/// file that changes is rewritten and named; with it nothing is written and
/// each file that would change is named. Either way a file the guard refuses
/// is named with its reason and left as it is, and the code is 1 when a file
/// was refused or, under `--check`, would change.
pub fn run(start: &Path, named: &[PathBuf], check: bool) -> Result<i32> {
    let mut files = Vec::new();
    if named.is_empty() {
        let layout = Layout::resolve(&start.join("nikaia.toml"));
        if !layout.in_project {
            refuse!(
                "There's no `nikaia.toml` in {} or any folder above it.\n\
                 `nikaia fmt` alone formats a package. To format single files, \
                 name them: `nikaia fmt main.nika`.",
                start.display()
            );
        }
        sources_under(&layout.root, &mut files)?;
    } else {
        for path in named {
            match path.is_dir() {
                true => sources_under(path, &mut files)?,
                false => files.push(path.clone()),
            }
        }
    }

    let mut code = 0;
    for file in &files {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        let formatted = match format(&source) {
            Ok(formatted) => formatted,
            Err(refusal) => {
                eprintln!("{}: not formatted: {refusal:#}", file.display());
                code = 1;
                continue;
            }
        };
        if formatted == source {
            continue;
        }
        if check {
            println!("{}", file.display());
            code = 1;
        } else {
            std::fs::write(file, &formatted)
                .with_context(|| format!("cannot write {}", file.display()))?;
            println!("formatted {}", file.display());
        }
    }
    Ok(code)
}

/// Every `.nika` file under `dir`, in name order: not in `target/`, a hidden
/// directory, or a directory with a `nikaia.toml` of its own (another package).
fn sources_under(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name == "target" || name.starts_with('.') || path.join("nikaia.toml").is_file() {
                continue;
            }
            sources_under(&path, files)?;
        } else if path.extension().is_some_and(|e| e == EXTENSION) {
            files.push(path);
        }
    }
    Ok(())
}

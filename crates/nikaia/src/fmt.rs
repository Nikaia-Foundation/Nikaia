//! `nikaia fmt` ([ADR-337](../../../docs/specification/adr/adr-337.md),
//! Part III 13.2). The printer is Nikaia (`tools/fmt.nika`); this is the
//! adapter: it parses the file for its comments (D2), hands the printer their
//! places in characters, and guards what comes back (D3).

use anyhow::{Result, bail};

use crate::parser::parse_to_ast;

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

//! The event stream of each file named on the command line, through
//! `saphyr-parser`, in the bench's event form (README, *The output*).
use saphyr_parser::{Event, Parser, ScalarStyle, Tag};
use std::fmt::Write as _;
use std::io::Write as _;

fn escaped(out: &mut String, t: &str) {
    for c in t.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
}

fn props(out: &mut String, anchor: usize, tag: Option<&Tag>) {
    if anchor > 0 {
        let _ = write!(out, " &{anchor}");
    }
    if let Some(t) = tag {
        let _ = write!(out, " <{}{}>", t.handle, t.suffix);
    }
}

/// The file's events appended to `out`, or `Err` where the parser refuses it.
fn events(input: &str, out: &mut String) -> Result<(), ()> {
    for next in Parser::new_from_str(input) {
        let (event, _) = next.map_err(|_| ())?;
        match event {
            Event::StreamStart => out.push_str("+STR"),
            Event::StreamEnd => out.push_str("-STR"),
            Event::DocumentStart(explicit) => {
                out.push_str(if explicit { "+DOC ---" } else { "+DOC" })
            }
            Event::DocumentEnd => out.push_str("-DOC"),
            Event::SequenceStart(a, t) => {
                out.push_str("+SEQ");
                props(out, a, t.as_deref());
            }
            Event::SequenceEnd => out.push_str("-SEQ"),
            Event::MappingStart(a, t) => {
                out.push_str("+MAP");
                props(out, a, t.as_deref());
            }
            Event::MappingEnd => out.push_str("-MAP"),
            Event::Alias(a) => {
                let _ = write!(out, "=ALI *{a}");
            }
            Event::Scalar(v, style, a, t) => {
                out.push_str("=VAL");
                props(out, a, t.as_deref());
                out.push_str(match style {
                    ScalarStyle::Plain => " :",
                    ScalarStyle::SingleQuoted => " '",
                    ScalarStyle::DoubleQuoted => " \"",
                    ScalarStyle::Literal => " |",
                    ScalarStyle::Folded => " >",
                });
                escaped(out, &v);
            }
            Event::Nothing => continue,
        }
        out.push('\n');
    }
    Ok(())
}

fn main() {
    let mut out = String::new();
    for path in std::env::args().skip(1) {
        let input = std::fs::read_to_string(&path).expect("a UTF-8 file");
        let mut one = String::new();
        match events(&input, &mut one) {
            Ok(()) => out.push_str(&one),
            Err(()) => out.push_str("error\n"),
        }
    }
    std::io::stdout().write_all(out.as_bytes()).unwrap();
}

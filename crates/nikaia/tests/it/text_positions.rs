//! **A position the text hands out cuts it without a view** (ADR-320 D5,
//! Part I 2.6, #453 step 6): `find`'s result, either end of the text and such a
//! position moved by the byte length of a text literal cut the text they came
//! from; a number the program made up, or a position of another text, is
//! `NK1214`, and a cut inside a scalar stops the program in the language's words.

use crate::common;

/// The status, the standard output and the standard error of `nikaia <verb>`.
fn nikaia(name: &str, verb: &str, source: &str) -> (bool, String, String) {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg(verb)
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    (
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

fn run(name: &str, source: &str) -> String {
    let (ok, out, err) = nikaia(name, "run", source);
    assert!(ok, "{out}{err}");
    out
}

fn refused(name: &str, source: &str) -> String {
    let (ok, out, err) = nikaia(name, "check", source);
    assert!(!ok, "{out}{err}");
    format!("{out}{err}")
}

/// Part I's example: the name before the `;` and the rest after it, the end
/// of the text, and a position moved by a literal's `.len()`.
#[test]
fn a_position_from_find_cuts_its_text() {
    let source = "fn split(line: ref String) {\n\
         \x20   let sep = line.find(\";\") ?? return\n\
         \x20   let name = line[..<sep]\n\
         \x20   let rest = line[sep + 1..]\n\
         \x20   let same = line[sep + \";\".len()..<line.len()]\n\
         \x20   let whole = line[0..<line.len()]\n\
         \x20   println(f\"{name}|{rest}|{same}|{whole}\")\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   split(\"Hamburg;12.0\")\n\
         \x20   split(\"M\u{fc}nchen;-3.5\")\n\
         \x20   split(\"none\")\n\
         }\n";
    assert_eq!(
        run("positions-find", source),
        "Hamburg|12.0|12.0|Hamburg;12.0\nM\u{fc}nchen|-3.5|-3.5|M\u{fc}nchen;-3.5\n"
    );
}

/// `find(";"; from: p)` hands out a position of the same text, and a fallback
/// of `0` (the start of every text) keeps it one; the `+ 1` a one-byte needle
/// allows is not carried past such a fallback, so the literal's length is
/// written.
#[test]
fn a_position_found_from_a_position_cuts() {
    let source = "fn main() {\n\
         \x20   let line = \"a;b\u{e4};c\"\n\
         \x20   let sep = line.find(\";\") ?? 0\n\
         \x20   let next = line.find(\";\"; from: sep + 1) ?? return\n\
         \x20   println(line[..<sep])\n\
         \x20   println(line[sep + \";\".len()..<next])\n\
         \x20   println(line[next + 1..])\n\
         }\n";
    assert_eq!(run("positions-from", source), "a\nb\u{e4}\nc\n");
}

/// A number the program made up needs a view, and the help names both.
#[test]
fn a_made_up_number_is_refused_with_the_view() {
    let err = refused(
        "positions-made-up",
        "fn main() {\n    let s = \"Hamburg;12.0\"\n    println(s[0..<10])\n}\n",
    );
    assert!(err.contains("NK1214"), "{err}");
    assert!(err.contains("s.bytes[0..<10].text()"), "{err}");
    assert!(err.contains("s.graphemes.take(10)"), "{err}");
}

/// A counter is made up too, and so is a `+ 1` after a needle of two bytes.
#[test]
fn a_counter_and_a_wrong_step_are_refused() {
    let counter = refused(
        "positions-counter",
        "fn main() {\n    let s = \"abc\"\n    let mut i = 0\n    while i < 2 {\n        println(s[i..<i + 1])\n        i = i + 1\n    }\n}\n",
    );
    assert!(counter.contains("NK1214"), "{counter}");
    let step = refused(
        "positions-step",
        "fn main() {\n    let s = \"a::b\"\n    let at = s.find(\"::\") ?? return\n    println(s[at + 1..])\n}\n",
    );
    assert!(step.contains("NK1214"), "{step}");
}

/// A position of one text does not cut another.
#[test]
fn a_position_of_another_text_is_refused() {
    let err = refused(
        "positions-other",
        "fn main() {\n    let a = \"ab;c\"\n    let b = \"abcdef\"\n    let sep = a.find(\";\") ?? return\n    println(b[..<sep])\n}\n",
    );
    assert!(err.contains("NK1214"), "{err}");
    assert!(err.contains("another text"), "{err}");
}

/// Cuts of `s.bytes` and of a `Bytes` take any number.
#[test]
fn a_cut_of_bytes_takes_any_number() {
    let source = "fn head(s: String) throws {\n\
         \x20   let part = s.bytes[0..<3]\n\
         \x20   println(f\"{part.len()} {s.bytes[1..<3].text()}\")\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   head(\"abcd\") catch {\n\
         \x20       println(\"failed\")\n\
         \x20   }\n\
         }\n";
    assert_eq!(run("positions-bytes", source), "3 bc\n");
}

/// A cut inside `ä` - one byte past where it starts, which a literal's length
/// can reach - stops the program in the language's words.
#[test]
fn a_cut_inside_a_scalar_stops_the_program() {
    let (ok, out, err) = nikaia(
        "positions-inside",
        "run",
        "fn main() {\n    let s = \"x\u{e4}y\"\n    let at = s.find(\"\u{e4}\") ?? return\n    println(s[at + \"a\".len()..])\n}\n",
    );
    assert!(!ok, "{out}{err}");
    assert!(err.contains("byte position 2 is inside a scalar"), "{err}");
    assert!(!err.contains("char boundary"), "{err}");
}

/// `rfind` hands out a position of its text, as `find` does.
#[test]
fn a_position_from_rfind_cuts_its_text() {
    let out = run(
        "positions-rfind",
        "fn last(name: String) -> String {\n    let colon = name.rfind(\"::\") ?? return name.clone()\n    return name[colon + \"::\".len()..].clone()\n}\n\nfn main() {\n    println(last(\"std::io::Write\"))\n    println(last(\"plain\"))\n}\n",
    );
    assert_eq!(out, "Write\nplain\n");
}

/// The start of a text moved by a literal's length, and its end moved back.
#[test]
fn the_ends_moved_by_a_literal_cut() {
    let out = run(
        "positions-literal-ends",
        "fn main() {\n    let t = \"ab-cdx\"\n    println(t[\"ab\".len()..])\n    println(t[..<t.len() - \"x\".len()])\n}\n",
    );
    assert_eq!(out, "-cdx\nab-cd\n");
}

/// An `rfind` position of one text does not cut another.
#[test]
fn an_rfind_position_of_another_text_is_refused() {
    let err = refused(
        "positions-rfind-other",
        "fn main() {\n    let a = \"ab;c\"\n    let b = \"abcdef\"\n    let sep = a.rfind(\";\") ?? return\n    println(b[..<sep])\n}\n",
    );
    assert!(err.contains("NK1214"), "{err}");
    assert!(err.contains("another text"), "{err}");
}

/// **A fallback that never comes back keeps `find`'s position** (#683): the
/// value of `find(..) ?? panic(..)`, `?? throw` or `?? return` is always the
/// left side's, so each cuts the text; a made-up number as the fallback does
/// not (`0`, the start of every text, would).
#[test]
fn a_fallback_that_never_returns_keeps_the_position() {
    let out = run(
        "positions-never-returns",
        "enum Bad { Missing }\n\
         impl Error for Bad { fn message(ref self) -> String { return \"bad\" } }\n\
         \n\
         fn by_panic(line: ref String) -> String {\n\
         \x20   let at = line.find(\";\") ?? panic(\"no separator\")\n\
         \x20   return line[..<at]\n\
         }\n\
         \n\
         fn by_throw(line: ref String) -> String throws {\n\
         \x20   let at = line.find(\";\") ?? throw Bad::Missing\n\
         \x20   return line[at + 1..]\n\
         }\n\
         \n\
         fn by_return(line: ref String) {\n\
         \x20   let at = line.find(\";\") ?? return\n\
         \x20   println(line[..<at])\n\
         }\n\
         \n\
         fn main() throws {\n\
         \x20   println(by_panic(\"ab;1.0\"))\n\
         \x20   println(by_throw(\"ab;1.0\"))\n\
         \x20   by_return(\"cd;2.0\")\n\
         }\n",
    );
    assert_eq!(out, "ab\n1.0\ncd\n");
    let err = refused(
        "positions-fallback-number",
        "fn main() {\n    let line = \"ab;1.0\"\n    let at = line.find(\";\") ?? 1\n    println(line[..<at])\n}\n",
    );
    assert!(err.contains("NK1214"), "{err}");
}

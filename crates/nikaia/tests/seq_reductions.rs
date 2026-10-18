//! **The reductions and searches over a sequence, and two cuts of text**
//! (`docs/selfhosting-gaps.md`, LANGUAGE): `any`, `all`, `find`, `position`,
//! `filter_map`, `last`, `take_while`, `skip_while`, `max`, `min`, `sum`,
//! `min_by_key`, `max_by_key` and `flat_map` on `Seq`, `rsplit`, `split_at`, `split_once` and `rsplit_once` on text. Each is run with `nikaia run`, with
//! its empty case, and the three that stop early show where they stopped.
mod common;

fn run(purpose: &str, body: &str) -> String {
    let dir = common::scratch_dir(purpose);
    let source = format!(
        "use std::cli\n\nfn half(x: i64) -> i64? {{\n    if x % 2 == 0 {{\n        return x / 2\n    }}\n    return null\n}}\n\n\
         fn main() {{\n    let xs: Vec[i64] = [3, 8, 1, 9, 4]\n    let none: Vec[i64] = []\n{body}\n}}\n"
    );
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).into_owned()
}

#[test]
fn any_stops_at_the_first_yes() {
    let out = run(
        "seq-any",
        "    let mut seen = 0\n    let big = xs.iter().any fn(x) {\n        seen = seen + 1\n        x > 5\n    }\n    println(f\"{big} {seen} {none.iter().any fn(x) { x > 5 }}\")",
    );
    assert_eq!(out, "true 2 false\n");
}

#[test]
fn all_stops_at_the_first_no() {
    let out = run(
        "seq-all",
        "    let mut seen = 0\n    let small = xs.iter().all fn(x) {\n        seen = seen + 1\n        x < 5\n    }\n    println(f\"{small} {seen} {none.iter().all fn(x) { x < 5 }}\")",
    );
    assert_eq!(out, "false 2 true\n");
}

#[test]
fn find_stops_at_the_first_match() {
    let out = run(
        "seq-find",
        "    let mut seen = 0\n    let found = xs.iter().find fn(x) {\n        seen = seen + 1\n        x > 5\n    } ?? -1\n    let lost = none.iter().find fn(x) { x > 5 } ?? -1\n    println(f\"{found} {seen} {lost}\")",
    );
    assert_eq!(out, "8 2 -1\n");
}

#[test]
fn position_is_an_i64() {
    let out = run(
        "seq-position",
        "    let at: i64 = xs.iter().position fn(x) { x == 9 } ?? -1\n    let not = none.iter().position fn(x) { x == 9 } ?? -1\n    println(f\"{at} {not}\")",
    );
    assert_eq!(out, "3 -1\n");
}

#[test]
fn filter_map_leaves_out_null() {
    let out = run(
        "seq-filter-map",
        "    let halves: Vec[i64] = xs.iter().filter_map(fn(x) { half(x) }).collect()\n    let empty: Vec[i64] = none.iter().filter_map(fn(x) { half(x) }).collect()\n    println(f\"{halves.len()} {halves[0]} {halves[1]} {empty.len()}\")",
    );
    assert_eq!(out, "2 4 2 0\n");
}

#[test]
fn find_map_stops_at_the_first_value() {
    let out = run(
        "seq-find-map",
        "    let first: i64? = xs.iter().find_map(fn(x) { half(x) })\n    let not: i64? = none.iter().find_map(fn(x) { half(x) })\n    println(f\"{first ?? -1} {not ?? -1}\")",
    );
    assert_eq!(out, "4 -1\n");
}

#[test]
fn args_has_a_length() {
    let out = run(
        "cli-args-len",
        "    let n: i64 = cli::args().len()\n    println(f\"{n > 0} {cli::args().is_empty()}\")",
    );
    assert_eq!(out, "true false\n");
}

#[test]
fn last_is_the_last_element() {
    let out = run(
        "seq-last",
        "    println(f\"{xs.iter().last() ?? -1} {none.iter().last() ?? -1}\")",
    );
    assert_eq!(out, "4 -1\n");
}

#[test]
fn take_while_and_skip_while_cut_at_the_first_no() {
    let out = run(
        "seq-while",
        "    let front: Vec[i64] = xs.iter().take_while(fn(x) { x != 1 }).collect()\n    let back: Vec[i64] = xs.iter().skip_while(fn(x) { x != 1 }).collect()\n    let nothing: Vec[i64] = none.iter().take_while(fn(x) { x != 1 }).collect()\n    println(f\"{front.len()} {back.len()} {back[0]} {nothing.len()}\")",
    );
    assert_eq!(out, "2 3 1 0\n");
}

#[test]
fn max_and_min_are_null_when_empty() {
    let out = run(
        "seq-max-min",
        "    println(f\"{xs.iter().max() ?? -1} {xs.iter().min() ?? -1} {none.iter().max() ?? -1} {none.iter().min() ?? -1}\")",
    );
    assert_eq!(out, "9 1 -1 -1\n");
}

#[test]
fn sum_of_nothing_is_zero() {
    let out = run(
        "seq-sum",
        "    let total: i64 = xs.iter().sum()\n    let zero: i64 = none.iter().sum()\n    println(f\"{total} {zero}\")",
    );
    assert_eq!(out, "25 0\n");
}

#[test]
fn rsplit_walks_the_pieces_from_the_back() {
    let out = run(
        "text-rsplit",
        "    let name = \"a::b::c\"\n    let tail = name.rsplit(\"::\").next() ?? name\n    let pieces = name.rsplit(\"::\").count()\n    let whole = \"\".rsplit(\"::\").count()\n    println(f\"{tail} {pieces} {whole}\")",
    );
    assert_eq!(out, "c 3 1\n");
}

#[test]
fn split_at_cuts_the_text_in_two() {
    let out = run(
        "text-split-at",
        "    let name = \"a::b::c\"\n    let (before, from) = name.split_at(3)\n    let (empty, all) = name.split_at(0)\n    println(f\"{before}|{from}|{empty}|{all}\")",
    );
    assert_eq!(out, "a::|b::c||a::b::c\n");
}

/// **`split_once` and `rsplit_once` cut at the first and the last
/// separator** (ADR-364, #648): two views of the text as a `(A, B)?`, `null`
/// where the separator does not occur, an empty half where it stands at an
/// end; taken by `?? return`, `??` with a tuple, a `match` and `?.`.
#[test]
fn split_once_and_rsplit_once_cut_at_one_separator() {
    let out = run(
        "text-split-once",
        "    let line = \"key=value=x\"\n\
         \x20   let (k, v) = line.split_once(\"=\") ?? (\"\", \"\")\n\
         \x20   let (a, b) = line.rsplit_once(\"=\") ?? (\"\", \"\")\n\
         \x20   println(f\"{k}|{v} {a}|{b}\")\n\
         \x20   match \"plain\".split_once(\"=\") {\n\
         \x20       null => println(\"none\"),\n\
         \x20       (x, y) => println(f\"{x}{y}\"),\n\
         \x20   }\n\
         \x20   println(f\"{\"plain\".rsplit_once(\"=\") == null}\")\n\
         \x20   let p = \"=end\".split_once(\"=\")\n\
         \x20   let q = \"end=\".rsplit_once(\"=\")\n\
         \x20   println(f\"[{p?.0 ?? \"-\"}][{p?.1 ?? \"-\"}] [{q?.0 ?? \"-\"}][{q?.1 ?? \"-\"}]\")\n\
         \x20   let (m, n) = \"std::io::Read\".rsplit_once(\"::\") ?? return\n\
         \x20   println(f\"{m} {n} {\"a::b\".split_once(\"::\")?.1?.len() ?? 0}\")",
    );
    assert_eq!(
        out,
        "key|value=x key=value|x\nnone\ntrue\n[][end] [end][]\nstd::io Read 1\n"
    );
}

#[test]
fn min_and_max_by_key_pick_by_the_key_and_are_null_when_empty() {
    let out = run(
        "seq-by-key",
        "    let near = xs.iter().min_by_key(fn(x) { (x - 5).abs() }) ?? -1\n    let high = xs.iter().max_by_key(fn(x) { x % 5 }) ?? -1\n    let first = xs.iter().min_by_key(fn(x) { x % 2 }) ?? -1\n    let lost = none.iter().min_by_key(fn(x) { x }) ?? -1\n    let gone = none.iter().max_by_key(fn(x) { x }) ?? -1\n    let words: Vec[String] = [\"bb\".clone(), \"a\".clone(), \"ccc\".clone()]\n    let short = words.iter().min_by_key(fn(w) { w.len() }) ?? \"none\"\n    let long = words.iter().max_by_key(fn(w) { w.len() }) ?? \"none\"\n    println(f\"{near} {high} {first} {lost} {gone} {short} {long}\")",
    );
    assert_eq!(out, "4 4 8 -1 -1 a ccc\n");
}

#[test]
fn flat_map_takes_a_list_or_a_sequence_from_its_lambda() {
    let out = run(
        "seq-flat-map",
        "    let pairs: Vec[i64] = xs.iter().flat_map(fn(x) { [x, x * 10] }).collect()\n    let runs: Vec[i64] = xs.iter().flat_map(fn(x) { 0..<x }).collect()\n    let rows: Vec[Vec[i64]] = [[1, 2], [3]]\n    let doubled: Vec[i64] = rows.iter().flat_map(fn(r) { r.iter().map(fn(v) { v * 2 }) }).collect()\n    let empty: Vec[i64] = none.iter().flat_map(fn(x) { [x] }).collect()\n    println(f\"{pairs.len()} {pairs[0]} {pairs[1]} {runs.len()} {doubled.len()} {doubled[2]} {empty.len()}\")",
    );
    assert_eq!(out, "10 3 30 25 3 6 0\n");
}

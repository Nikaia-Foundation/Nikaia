//! **`s.bytes`** (ADR-320 D11, Part I 2.6, #453 step 3): a text's own bytes are
//! a `Bytes`, borrowed, and every read of a `Bytes` (D7) works on them as on
//! the bytes `fs::read` hands out.

use crate::common;

fn run(name: &str, source: &str, files: &[(&str, &[u8])]) -> String {
    let dir = common::scratch_dir(name);
    for (file, bytes) in files {
        std::fs::write(dir.join(file), bytes).expect("a file");
    }
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
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).into_owned()
}

const READS: &str = "fn reads(s: String) throws {\n\
    \x20   let first = s.bytes[0]\n\
    \x20   let part = s.bytes[1..<3]\n\
    \x20   let n = s.bytes.len()\n\
    \x20   let semi = s.bytes.find(59; from: 3) ?? -1\n\
    \x20   let head = s.bytes[0..<4].text()\n\
    \x20   let mut sum = 0\n\
    \x20   for b in s.bytes {\n\
    \x20       sum = sum + b as i64\n\
    \x20   }\n\
    \x20   println(f\"{first} {part.len()} {n} {semi} {head} {sum}\")\n\
    }\n\
    \n\
    fn main() {\n\
    \x20   reads(\"ab;cd;ef\") catch {\n\
    \x20       println(\"failed\")\n\
    \x20   }\n\
    \x20   reads(\"äb;c;d\") catch {\n\
    \x20       println(\"failed\")\n\
    \x20   }\n\
    \x20   reads(\"abcä;\") catch {\n\
    \x20       println(\"not text\")\n\
    \x20   }\n\
    }\n";

/// Each read D7 lists, on ASCII and on text with `ä` (two bytes, 0xC3 0xA4):
/// a cut inside the `ä` is no text, and `text()` says so.
#[test]
fn every_read_of_a_bytes_works_on_s_bytes() {
    let ascii: i64 = "ab;cd;ef".bytes().map(i64::from).sum();
    let umlaut: i64 = "äb;c;d".bytes().map(i64::from).sum();
    assert_eq!(
        run("text-bytes-reads", READS, &[]),
        format!("97 2 8 5 ab;c {ascii}\n195 2 7 3 äb; {umlaut}\nnot text\n")
    );
}

/// `s.bytes.len()` counts bytes, not characters: `"ä"` is two.
#[test]
fn the_bytes_of_a_umlaut_are_two() {
    let source = "fn main() {\n\
         \x20   let s = \"ä\"\n\
         \x20   println(f\"{s.bytes.len()} {s.bytes[0]} {s.bytes[1]}\")\n\
         }\n";
    assert_eq!(run("text-bytes-len", source, &[]), "2 195 164\n");
}

/// One function over bytes takes both a buffer `fs::read` handed out and a
/// text's own bytes.
#[test]
fn a_function_over_bytes_takes_both() {
    let source = "use std::fs\n\
         \n\
         fn semis(b: Bytes) -> i64 {\n\
         \x20   let mut n = 0\n\
         \x20   for x in b {\n\
         \x20       if x == 59 {\n\
         \x20           n = n + 1\n\
         \x20       }\n\
         \x20   }\n\
         \x20   return n\n\
         }\n\
         \n\
         fn both(path: ref String) throws {\n\
         \x20   let read = fs::read(ref path, fs::Root::Anywhere)\n\
         \x20   let s = \"ä;b;c\"\n\
         \x20   println(f\"{semis(read)} {semis(s.bytes)} {read.len()} {s.bytes.len()}\")\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   both(\"x\") catch {\n\
         \x20       println(\"failed\")\n\
         \x20   }\n\
         }\n";
    assert_eq!(
        run("text-bytes-both", source, &[("x", "ä;;;".as_bytes())]),
        "3 2 5 6\n"
    );
}

/// Kept beyond the text, in a field or handed back, `s.bytes` is copied into a
/// `Bytes` of its own, as an owned `String` is.
#[test]
fn s_bytes_kept_beyond_the_text_is_a_copy() {
    let source = "struct Kept {\n\
         \x20   b: Bytes\n\
         }\n\
         \n\
         fn keep(s: String) -> Kept {\n\
         \x20   return Kept { b: s.bytes }\n\
         }\n\
         \n\
         fn back(s: String) -> Bytes {\n\
         \x20   return s.bytes\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let k = keep(\"ä;b\")\n\
         \x20   let b = back(\"abc\")\n\
         \x20   println(f\"{k.b.len()} {k.b[0]} {b.len()} {b[2]}\")\n\
         }\n";
    assert_eq!(run("text-bytes-kept", source, &[]), "4 195 3 99\n");
}

/// The copy follows the value: `s.bytes` bound by a `let` is the view, and the
/// name is copied where it is kept - handed back, pushed into a list, stored in
/// a field, written out or in the shorthand - while every other read of it
/// costs no copy.
#[test]
fn s_bytes_through_a_let_is_copied_where_it_is_kept() {
    let source = "struct Kept {\n\
         \x20   b: Bytes\n\
         }\n\
         \n\
         fn back(s: String) -> Bytes {\n\
         \x20   let k = s.bytes\n\
         \x20   return k\n\
         }\n\
         \n\
         fn listed(s: String) -> Vec[Bytes] {\n\
         \x20   let mut v: Vec[Bytes] = []\n\
         \x20   v.push(s.bytes)\n\
         \x20   let k = s.bytes\n\
         \x20   v.push(k)\n\
         \x20   v.push(k)\n\
         \x20   return v\n\
         }\n\
         \n\
         fn keep(s: String) -> Kept {\n\
         \x20   let k = s.bytes\n\
         \x20   return Kept { b: k }\n\
         }\n\
         \n\
         fn keep_short(s: String) -> Kept {\n\
         \x20   let b = s.bytes\n\
         \x20   return Kept { b }\n\
         }\n\
         \n\
         fn semis(b: Bytes) -> i64 {\n\
         \x20   let mut n = 0\n\
         \x20   for x in b {\n\
         \x20       if x == 59 {\n\
         \x20           n = n + 1\n\
         \x20       }\n\
         \x20   }\n\
         \x20   return n\n\
         }\n\
         \n\
         fn reads(s: String) -> i64 {\n\
         \x20   let k = s.bytes\n\
         \x20   let mut sum = 0\n\
         \x20   for x in k {\n\
         \x20       sum = sum + x as i64\n\
         \x20   }\n\
         \x20   return sum + k.len() + k[0] as i64 + semis(k)\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let b = back(\"abc\")\n\
         \x20   let v = listed(\"xy\")\n\
         \x20   let k = keep(\"ä;b\")\n\
         \x20   let ks = keep_short(\"q;;\")\n\
         \x20   println(f\"{b.len()} {b[2]} {v.len()} {v[0][0]} {v[1][1]} {v[2][0]} {k.b.len()} {k.b[0]} {ks.b.len()} {reads(\"a;b\")}\")\n\
         }\n";
    let sum: i64 = "a;b".bytes().map(i64::from).sum();
    assert_eq!(
        run("text-bytes-let-kept", source, &[]),
        format!("3 99 3 120 121 120 4 195 3 {}\n", sum + 3 + 97 + 1)
    );
}

/// `clone()` is a `Bytes` of its own: of the bytes `fs::read` hands out, of a
/// text's own bytes, of a name bound to them, and of a read-only parameter.
#[test]
fn clone_makes_a_bytes_of_its_own() {
    let source = "use std::fs\n\
         \n\
         fn copied(b: Bytes) -> Bytes {\n\
         \x20   return b.clone()\n\
         }\n\
         \n\
         fn copies(path: ref String) throws {\n\
         \x20   let read = fs::read(ref path, fs::Root::Anywhere)\n\
         \x20   let c = read.clone()\n\
         \x20   let s = \"äbc\"\n\
         \x20   let d = s.bytes.clone()\n\
         \x20   let t = s.bytes\n\
         \x20   let e = t.clone()\n\
         \x20   let f = copied(s.bytes)\n\
         \x20   let g = copied(read)\n\
         \x20   println(f\"{c.len()} {c[0]} {read.len()} {d.len()} {d[0]} {e[2]} {f[3]} {g.len()}\")\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   copies(\"x\") catch {\n\
         \x20       println(\"failed\")\n\
         \x20   }\n\
         }\n";
    assert_eq!(
        run("text-bytes-clone", source, &[("x", b"q;;")]),
        "3 113 3 4 195 98 99 3\n"
    );
}

/// **A `for` over a `Bytes` walks `u8`s** (ADR-320 D7), so a `'…'` compared
/// with one is a byte (D8) - over `s.bytes`, a cut of it, and a `Bytes` (the
/// lowering wrote a `char` there, which `rustc` refused against a `u8`).
#[test]
fn a_char_literal_compared_with_a_walked_byte_is_a_u8() {
    let source = "fn lines(s: ref String, b: Bytes) -> i64 {\n\
         \x20   let mut n = 0\n\
         \x20   for c in s.bytes {\n\
         \x20       if c == '\\n' { n = n + 1 }\n\
         \x20   }\n\
         \x20   for c in s.bytes[0..<2] {\n\
         \x20       if c == '\\n' { n = n + 1 }\n\
         \x20   }\n\
         \x20   for c in b {\n\
         \x20       if c == ';' { n = n + 1 }\n\
         \x20   }\n\
         \x20   return n\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let s = \"a\\nb\\nc\"\n\
         \x20   println(f\"{lines(s, \"a;b;\".bytes[0..<4])}\")\n\
         }\n";
    assert_eq!(run("bytes-walked-literal", source, &[]), "5\n");
}

/// **A word is read in the order its name says** (ADR-320 D12, #709): the
/// four reads on a buffer `fs::read` handed out and on a text's own bytes, a
/// read that ends exactly at the end; widened with `as`, and a signed number
/// out of the same bits by name (Part I 2.2).
#[test]
fn a_word_is_read_in_the_order_its_name_says() {
    let source = "use std::fs\n\
         \n\
         fn words(path: ref String) throws {\n\
         \x20   let b = fs::read(ref path, fs::Root::Anywhere)\n\
         \x20   println(f\"{b.u32_le(0)} {b.u32_be(0)} {b.u64_le(1)} {b.u64_be(1)}\")\n\
         \x20   println(f\"{b.u64_le(8).bits_as_i64()} {b.u32_be(12) as i64} {b.u64_le(8)} {b.u32_le(0) as i64 - 1}\")\n\
         \x20   let s = \"abcdefgh\"\n\
         \x20   println(f\"{s.bytes.u32_le(4)} {s.bytes.u64_be(0)}\")\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   words(\"w\") catch {\n\
         \x20       println(\"failed\")\n\
         \x20   }\n\
         }\n";
    let mut file: Vec<u8> = (1u8..=8).collect();
    file.extend([0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    let head = u32::from_le_bytes([1, 2, 3, 4]);
    let head_be = u32::from_be_bytes([1, 2, 3, 4]);
    let one = u64::from_le_bytes([2, 3, 4, 5, 6, 7, 8, 0xFE]);
    let one_be = u64::from_be_bytes([2, 3, 4, 5, 6, 7, 8, 0xFE]);
    let tail = u64::from_le_bytes(file[8..16].try_into().unwrap());
    let text_le = u32::from_le_bytes(*b"efgh");
    let text_be = u64::from_be_bytes(*b"abcdefgh");
    assert_eq!(
        run("text-bytes-words", source, &[("w", &file)]),
        format!(
            "{head} {head_be} {one} {one_be}\n-2 4294967295 {tail} {}\n{text_le} {text_be}\n",
            head - 1
        )
    );
}

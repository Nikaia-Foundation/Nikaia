//! **`s.bytes`** (ADR-320 D11, Part I 2.6, #453 step 3): a text's own bytes are
//! a `Bytes`, borrowed, and every read of a `Bytes` (D7) works on them as on
//! the bytes `fs::read` hands out.

mod common;

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

//! **`s.find(needle; from:)`** (ADR-374 D2, Part I 2.6, #453 step 4): the
//! first match at or after `from`, a `text::Match`, or `null`, through `std`'s
//! search rather than the language below's `str::find`.

use crate::common;
#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};

fn ran(purpose: &str, source: &str) -> String {
    let dir = common::scratch_dir(purpose);
    let file = dir.join("main.nika");
    std::fs::write(&file, source).expect("the source");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .arg("run")
        .arg(&file)
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).to_text()
}

/// From the start, from a position, nothing past the last match. A match's
/// position cuts the text it came from.
#[test]
fn find_searches_from_a_position() {
    let source = "use std::text\n\
         \n\
         fn station(line: ref String) -> String {\n\
         \x20   let sep = line.find(\";\")?.start ?? line.start()\n\
         \x20   return line.before(sep).clone()\n\
         }\n\
         \n\
         fn at(t: ref String, found: text::Match?) -> i64 {\n\
         \x20   let m = found ?? return -1\n\
         \x20   return t.before(m.start).bytes.len()\n\
         }\n\
         \n\
         fn main() throws {\n\
         \x20   let line = \"Hamburg;12.0;x\"\n\
         \x20   let sep = line.find(\";\") ?? return\n\
         \x20   let next = line.find(\";\"; from: sep.end) ?? return\n\
         \x20   let none = line.find(\";\"; from: next.end)\n\
         \x20   let owned: String = \"\u{e4}a;\u{e4};\"\n\
         \x20   let inside = owned.find(\";\"; from: (owned.find(\"a\") ?? return).start)\n\
         \x20   println(f\"{at(line, sep)} {at(line, next)} {at(line, none)} {at(owned, inside)} {station(line)}\")\n\
         }\n";
    assert_eq!(ran("text-find", source), "7 12 -1 3 Hamburg\n");
}

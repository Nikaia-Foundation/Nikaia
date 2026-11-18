// The json rows' Rust side (#1620): standard input read into a
// `serde_json::Value` and written back as JSON text without blanks, the way
// the json package's `parse` and `to_string` do. serde_json is built with
// `preserve_order` (members keep the order of the text, as the package keeps
// them), `arbitrary_precision` (a number is kept as the text it was written
// as, as the package keeps it) and `unbounded_depth` (serde_json stops at 128
// levels by default; the package reads 512, the deep input's depth).
use serde::Deserialize;
use std::io::{Read, Write};

fn main() {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .expect("cannot read standard input");
    let mut reader = serde_json::Deserializer::from_str(&text);
    reader.disable_recursion_limit();
    let value = serde_json::Value::deserialize(&mut reader).expect("standard input is not JSON");
    reader.end().expect("standard input is not JSON");
    let mut out = serde_json::to_string(&value).expect("cannot write JSON");
    out.push('\n');
    std::io::stdout()
        .write_all(out.as_bytes())
        .expect("cannot write standard output");
}

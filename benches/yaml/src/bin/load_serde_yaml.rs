//! The tree of each file named on the command line, through
//! `serde_yaml::Value` (one per document of the stream), in the bench's
//! canonical dump (README, *The output*).
use serde::Deserialize;
use serde_yaml::Value;
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

fn dump(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("~\n"),
        Value::Bool(b) => out.push_str(if *b { "true\n" } else { "false\n" }),
        Value::Number(n) if n.is_f64() => {
            out.push('%');
            out.push_str(&n.to_string());
            out.push('\n');
        }
        Value::Number(n) => {
            out.push('#');
            out.push_str(&n.to_string());
            out.push('\n');
        }
        Value::String(s) => {
            out.push('"');
            escaped(out, s);
            out.push('\n');
        }
        Value::Sequence(items) => {
            out.push_str("[\n");
            for x in items {
                dump(x, out);
            }
            out.push_str("]\n");
        }
        Value::Mapping(m) => {
            out.push_str("{\n");
            for (k, x) in m {
                dump(k, out);
                dump(x, out);
            }
            out.push_str("}\n");
        }
        Value::Tagged(t) => dump(&t.value, out),
    }
}

fn load(input: &str, out: &mut String) -> Result<(), serde_yaml::Error> {
    for doc in serde_yaml::Deserializer::from_str(input) {
        let v = Value::deserialize(doc)?;
        out.push_str("---\n");
        dump(&v, out);
    }
    Ok(())
}

fn main() {
    let mut out = String::new();
    for path in std::env::args().skip(1) {
        let input = std::fs::read_to_string(&path).expect("a UTF-8 file");
        let mut one = String::new();
        match load(&input, &mut one) {
            Ok(()) => out.push_str(&one),
            Err(_) => out.push_str("error\n"),
        }
    }
    std::io::stdout().write_all(out.as_bytes()).unwrap();
}

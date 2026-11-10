//! The tree of each file named on the command line, through
//! `saphyr::Yaml::load_from_str`, in the bench's canonical dump (README,
//! *The output*).
use saphyr::{LoadableYamlNode, Scalar, Yaml};
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

fn dump(v: &Yaml, out: &mut String) {
    match v {
        Yaml::Value(Scalar::Null) => out.push_str("~\n"),
        Yaml::Value(Scalar::Boolean(b)) => out.push_str(if *b { "true\n" } else { "false\n" }),
        Yaml::Value(Scalar::Integer(i)) => {
            out.push('#');
            out.push_str(&i.to_string());
            out.push('\n');
        }
        Yaml::Value(Scalar::FloatingPoint(f)) => {
            out.push('%');
            out.push_str(&f.to_string());
            out.push('\n');
        }
        Yaml::Value(Scalar::String(s)) | Yaml::Representation(s, _, _) => {
            out.push('"');
            escaped(out, s);
            out.push('\n');
        }
        Yaml::Sequence(items) => {
            out.push_str("[\n");
            for x in items {
                dump(x, out);
            }
            out.push_str("]\n");
        }
        Yaml::Mapping(m) => {
            out.push_str("{\n");
            for (k, x) in m {
                dump(k, out);
                dump(x, out);
            }
            out.push_str("}\n");
        }
        Yaml::Tagged(_, inner) => dump(inner, out),
        Yaml::Alias(a) => {
            out.push('*');
            out.push_str(&a.to_string());
            out.push('\n');
        }
        Yaml::BadValue => out.push_str("?\n"),
    }
}

fn main() {
    let mut out = String::new();
    for path in std::env::args().skip(1) {
        let input = std::fs::read_to_string(&path).expect("a UTF-8 file");
        match Yaml::load_from_str(&input) {
            Ok(docs) => {
                for d in &docs {
                    out.push_str("---\n");
                    dump(d, &mut out);
                }
            }
            Err(_) => out.push_str("error\n"),
        }
    }
    std::io::stdout().write_all(out.as_bytes()).unwrap();
}

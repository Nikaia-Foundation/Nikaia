//! **`s.normalized(text::Form::…)` and a `'…'` of several scalars**
//! (ADR-320 D8, D10, Part I 2.6, #453 steps 2 and 8): `==` compares bytes
//! until the program writes a normalisation form, and a character literal an
//! editor stored decomposed is the one scalar NFC composes it to.

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

/// A precomposed and a decomposed `ä` differ as bytes, and agree once both are
/// written in one form: composed (`Nfc`) or decomposed (`Nfd`).
#[test]
fn nfc_and_nfd_make_equal_what_bytes_do_not() {
    let out = run(
        "normalized_nfc_nfd",
        r#"fn main() {
    let composed = "ä"
    let decomposed = "a\u{308}"
    println(f"{composed == decomposed}")
    println(f"{composed.normalized(text::Form::Nfc) == decomposed.normalized(text::Form::Nfc)}")
    println(f"{composed.normalized(text::Form::Nfd) == decomposed}")
    println(f"{decomposed.normalized(text::Form::Nfc) == composed}")
    println(f"{composed.normalized(text::Form::Nfd).len()}")
}
"#,
    );
    assert_eq!(out, "false\ntrue\ntrue\ntrue\n3\n");
}

/// `Nfkc` makes the ligature `ﬁ` the two letters; `Nfc` leaves it.
#[test]
fn nfkc_folds_a_ligature_and_nfc_leaves_it() {
    let out = run(
        "normalized_nfkc",
        r#"fn main() {
    let ligature = "ﬁ"
    println(f"{ligature.normalized(text::Form::Nfkc) == "fi"}")
    println(f"{ligature.normalized(text::Form::Nfc) == "fi"}")
    println(f"{ligature.normalized(text::Form::Nfc) == ligature}")
    println(ligature.normalized(text::Form::Nfkd))
}
"#,
    );
    assert_eq!(out, "true\nfalse\ntrue\nfi\n");
}

/// A form is a value: held in a `let` and passed on to a function that
/// normalises with it, twice.
#[test]
fn a_form_is_held_and_passed_on() {
    let out = run(
        "normalized_form_value",
        r#"fn same(a: String, b: String, form: text::Form) -> bool {
    return a.normalized(form) == b.normalized(form)
}

fn main() {
    let form = text::Form::Nfc
    println(f"{same("ä", "a\u{308}", form)}")
    println(f"{same("ﬁ", "fi", form)}")
    let wide = text::Form::Nfkd
    println(f"{same("ﬁ", "fi", wide)}")
    println(f"{form == text::Form::Nfc}")
}
"#,
    );
    assert_eq!(out, "true\nfalse\ntrue\ntrue\n");
}

/// `'e\u{301}'` composes to `'é'`: it is that scalar, with the warning
/// `NK1213`, written with an escape and as the decomposed character itself.
#[test]
fn a_decomposed_char_literal_is_the_composed_scalar() {
    for (name, written) in [
        ("normalized_char_escape", "e\\u{301}".to_string()),
        ("normalized_char_source", "e\u{301}".to_string()),
    ] {
        let source = format!(
            "fn main() {{\n    let c = '{written}'\n    println(f\"{{c}}\")\n    println(f\"{{c == 'é'}}\")\n    println(f\"{{c as i64}}\")\n}}\n"
        );
        let (ok, out, err) = nikaia(name, "run", &source);
        assert!(ok, "{out}{err}");
        assert_eq!(out, "é\ntrue\n233\n", "{written}: {err}");
        assert!(err.contains("NK1213"), "{written}: {err}");
        assert!(err.contains("'é'"), "{written}: {err}");
    }
}

/// `'👍🏽'` is two scalars that compose to two: refused with `NK1212`, and the
/// help hands over the text literal.
#[test]
fn a_char_literal_of_a_grapheme_is_refused() {
    let (ok, out, err) = nikaia(
        "normalized_char_grapheme",
        "check",
        "fn main() {\n    let c = '👍🏽'\n    println(f\"{c}\")\n}\n",
    );
    assert!(!ok, "{out}{err}");
    assert!(err.contains("NK1212"), "{err}");
    assert!(err.contains("\"👍🏽\""), "{err}");
}

/// A surrogate names no scalar: refused as an escape that names nothing.
#[test]
fn a_surrogate_char_literal_is_refused() {
    let (ok, out, err) = nikaia(
        "normalized_char_surrogate",
        "check",
        "fn main() {\n    let c = '\\u{D800}'\n    println(f\"{c}\")\n}\n",
    );
    assert!(!ok, "{out}{err}");
    assert!(err.contains("NK1184"), "{err}");
    assert!(!err.contains("NK1212"), "{err}");
}

/// `s.quoted()` is Rust's `{:?}` of a string (#804).
#[test]
fn quoted_escapes_like_debug() {
    let out = run(
        "text_quoted",
        r#"fn main() {
    println("a\"b\\c\nd\te".quoted())
    println("ä".quoted())
    println("".quoted())
}
"#,
    );
    assert_eq!(out, "\"a\\\"b\\\\c\\nd\\te\"\n\"ä\"\n\"\"\n");
}

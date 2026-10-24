//! A `match` over a part of an element read off a `ref` list (#806): the
//! element is lent, so the part is matched by reference and nothing moves out
//! of the shared reference (no `E0507` reaches the user).

use std::process::Command;

fn run(purpose: &str, call: &str, extra: &str) -> String {
    let source = format!(
        "struct Alt {{\n    action: String?,\n}}\n\nstruct Rule {{\n    alts: Vec[Alt],\n}}\n\n{extra}\n\
         fn main() {{\n    let r = Rule {{ alts: [Alt {{ action: \"go\".clone() }}] }}\n    print({call})\n}}\n"
    );
    let dir = crate::common::scratch_dir(&format!("match-element-{purpose}"));
    std::fs::write(dir.join("program.nika"), source).expect("write the source");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "program.nika"])
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        out.status.success(),
        "{purpose}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn a_match_over_a_part_of_a_let_bound_element_borrows() {
    let f = "fn first_action(rule: ref Rule, i: i64) -> String {\n    let alt = rule.alts[i]\n    return match alt.action {\n        Some(a) => a.clone(),\n        else => \"none\",\n    }\n}\n";
    assert_eq!(run("let", "first_action(r, 0)", f), "go");
}

#[test]
fn a_match_over_a_part_of_an_indexed_element_borrows() {
    let f = "fn first_action(rule: ref Rule, i: i64) -> String {\n    return match rule.alts[i].action {\n        Some(a) => a.clone(),\n        else => \"none\",\n    }\n}\n";
    assert_eq!(run("direct", "first_action(r, 0)", f), "go");
}

#[test]
fn a_match_over_an_owned_local_still_moves() {
    let f = "fn take(text: String) -> String {\n    let own = Alt { action: text }\n    return match own.action {\n        Some(a) => a,\n        else => \"none\",\n    }\n}\n";
    assert_eq!(run("owned", "take(\"go\".clone())", f), "go");
}

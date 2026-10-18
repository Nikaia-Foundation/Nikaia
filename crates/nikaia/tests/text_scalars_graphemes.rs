//! **`s.scalars` and `s.graphemes`** (ADR-320 D3, D4, D9, Part I 2.6, #453
//! step 3): a text walked by scalar, or by what a reader sees as one character,
//! with `take`, `skip`, `count` and `for`.

mod common;

fn run(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
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

/// `"aä€"` is three scalars of one, two and three bytes; `for` finds the `ä`,
/// on a local and on a parameter.
#[test]
fn scalars_count_and_for() {
    let out = run(
        "views_scalars_count",
        r#"fn find(name: String) {
    for c in name.scalars {
        if c == 'ä' {
            println("found in a parameter")
        }
    }
}

fn main() {
    let name = "aä€"
    println(f"{name.scalars.count()}")
    for c in name.scalars {
        if c == 'ä' {
            println("found")
        }
    }
    find(name)
}
"#,
    );
    assert_eq!(out, "3\nfound\nfound in a parameter\n");
}

/// `take(2)` and `skip(1)` count scalars, not bytes.
#[test]
fn scalars_take_and_skip() {
    let out = run(
        "views_scalars_take",
        r#"fn main() {
    let name = "aä€"
    for c in name.scalars.take(2) {
        println(f"take {c}")
    }
    for c in name.scalars.skip(1) {
        println(f"skip {c}")
    }
    let two: Vec[scalar] = name.scalars.take(2).collect()
    println(f"{two.len()}")
}
"#,
    );
    assert_eq!(out, "take a\ntake ä\nskip ä\nskip €\n2\n");
}

/// A family emoji is seven scalars joined by zero-width joiners, and `a` with
/// a combining diaeresis is two: each is one grapheme.
#[test]
fn graphemes_count_clusters() {
    let out = run(
        "views_graphemes_count",
        r#"fn main() {
    let family = "👨‍👩‍👧‍👦"
    println(f"{family.graphemes.count()} {family.scalars.count()}")
    let decomposed = "a\u{308}"
    println(f"{decomposed.graphemes.count()} {decomposed.scalars.count()}")
}
"#,
    );
    assert_eq!(out, "1 7\n1 2\n");
}

/// `take(10)` and `skip` on a longer text, and a grapheme compared with a
/// text literal.
#[test]
fn graphemes_take_skip_and_compare() {
    let out = run(
        "views_graphemes_take",
        r#"fn main() {
    let title = "Mädchen und Jungen"
    let mut preview = ""
    for g in title.graphemes.take(10) {
        preview = f"{preview}{g}"
    }
    println(preview)
    let mut found = 0
    for g in title.graphemes {
        if g == "ä" {
            found = found + 1
        }
    }
    println(f"{found}")
    println(f"{title.graphemes.skip(12).count()}")
    let kept: Vec[ref String] = title.graphemes.take(3).collect()
    println(f"{kept.len()}")
}
"#,
    );
    assert_eq!(out, "Mädchen un\n1\n6\n3\n");
}

/// `for` over graphemes hands each one out whole, an emoji with its skin tone
/// included.
#[test]
fn graphemes_for_prints_each() {
    let out = run(
        "views_graphemes_for",
        r#"fn main() {
    let s = "aä👍🏽!"
    for g in s.graphemes {
        println(g)
    }
}
"#,
    );
    assert_eq!(out, "a\nä\n👍🏽\n!\n");
}

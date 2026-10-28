pub fn agree(test: &ProgramTest, outcomes: &[(String, Outcome)]) -> bool {
    let Some((_, first)) = outcomes.first() else {
        return false;
    };
    let same = outcomes.windows(2).all(|pair| pair[0].1 == pair[1].1);
    if first.passed {
        return true;
    }
    if !same {
        return false;
    }
    test.stdout.as_ref().is_none_or(|want| *want == *first.stdout)
}

pub fn explain(output: &OutputTest, outcome: &Outcome) {
    for ((relative, want), (_, got)) in output.files.iter().zip(&outcome.files) {
        match got {
            None => println!("{relative}: not written"),
            Some(got) => println!("{relative}: {}", got.len()),
        }
    }
}

pub fn write_all(test: &ProgramTest, first: &Outcome) -> Result<()> {
    let write = |extension: &str, bytes: &[u8]| {
        std::fs::write(test.dir.join(format!("{}.{extension}", test.name)), bytes)
    };
    write("stdout", &first.stdout)?;
    let path = test.dir.join("out");
    std::fs::write(path, Bytes::empty())?;
    let said = String::from_utf8_lossy(&first.stdout);
    println!("{said}");
    Ok(())
}

pub fn sorted(xs: &[i64], name: String, size: i64) -> Item {
    let up = xs.windows(2).all(|p| p[0] < p[1]);
    let n = xs.first().map(|x| x + 1);
    let k = match size {
        0 => {
            n.unwrap_or(1)
        }
        _ => size,
    };
    Item { name, size: size + k }
}

fn plural(n: i64) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn named(test: &str) -> usize {
    test.len()
}

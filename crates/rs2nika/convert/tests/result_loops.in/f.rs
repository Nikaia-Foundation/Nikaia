fn program_tests(dir: &Path, mut names: BTreeSet<String>) -> Result<Vec<Test>> {
    // A name with a program beside it is not an output test.
    names.retain(|name| !name.ends_with(".nika"));
    names
        .into_iter()
        .map(|name| {
            let size = measure(&name)?;
            Ok(Test {
                name,
                size,
            })
        })
        .collect()
}

fn sizes(dir: &Path, names: Vec<String>) -> Result<Vec<i64>> {
    names.into_iter().map(|name| {
        let size = measure(&name)?;
        Ok(size + 1)
    }).collect()
}

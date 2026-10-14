fn joining(package: &[String], copies: &BTreeSet<String>, parts: &BTreeMap<String, Vec<Ty>>) -> Vec<String> {
    let joining: Vec<String> = package
        .iter()
        .filter(|name| !copies.contains(*name))
        .filter(|name| {
            let Some(held) = parts.get(*name) else { return false };
            held.iter().all(|ty| ty.copies())
        })
        .cloned()
        .collect();
    joining
}

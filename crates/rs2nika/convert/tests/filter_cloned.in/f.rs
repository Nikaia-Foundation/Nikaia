fn kept(thrown: &[Thrown], items: &[Item]) -> Vec<Thrown> {
    let fatal: Vec<Thrown> = thrown.iter().filter(|t| t.fatal).cloned().collect();
    let first = items.iter().find_map(|item| item.name.clone());
    fatal
}

fn gather(units: &[Unit], library: &mut Library, contract: &Contract, name: &String) {
    let mut packages: BTreeMap<String, Vec<Parsed>> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for unit in units {
        packages.entry(unit.name.clone()).or_default().push(unit.parsed);
        *counts.entry(unit.name.clone()).or_insert(0) += 1;
    }
    library.functions.entry(name.clone()).or_insert_with(|| contract.clone());
    library.seen.entry(name.clone()).or_insert_with(BTreeSet::new).insert(contract.id);
    let held = counts.entry(name.clone()).or_default();
    if project.entry() == name {
        return;
    }
}

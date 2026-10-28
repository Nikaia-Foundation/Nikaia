fn names(units: &[Unit]) -> BTreeSet<String> {
    for unit in units {
        let Some(package) = unit.package.clone() else {
            continue;
        };
        println!("{package}");
    }
    units.iter().filter_map(|u| u.package.clone()).collect()
}

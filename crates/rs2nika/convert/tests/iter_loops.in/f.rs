fn walk(units: &[Unit], files: &[(String, String)], extra: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (at, unit) in units.iter().enumerate() {
        if unit.skipped {
            continue;
        }
        out.push(format!("{at}"));
    }
    for (k, _) in files.iter().enumerate() {
        out.push(format!("{k}"));
    }
    for (name, body) in files.iter().skip(1) {
        out.push(name.clone());
    }
    out.extend(extra.iter().cloned());
    let needs = units.iter().fold(Needs::empty(), |acc, unit| acc.join(Needs::of(unit)));
    let more = self.units.iter().fold(Needs::default(), |acc, u| {
        acc.join(Needs::of(u))
    });
    let theirs: Vec<&Unit> = units.iter().copied().filter(|u| u.package == needs.name).collect();
    for (at, unit) in units.iter_mut().enumerate() {
        unit.skipped = true;
    }
    out
}

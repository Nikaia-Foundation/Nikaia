fn groups(units: &[Unit]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    notes.push(format!("{} units", units.len()));
    let mut plain: BTreeMap<String, Ledger> =
        BTreeMap::new();
    out.push(plain.len());
    out
}

/// The keys that were inferred.
fn lost(keys: &[String], committed: &BTreeMap<String, Contract>, pauses: &BTreeMap<String, usize>) -> String {
    let mut out = String::new();
    for key in keys {
        let was = committed
            .get(key)
            .is_some_and(|before| before.sync_claim == crate::contracts::Sync::Inferred);
        if key.is_empty() || !was {
            continue;
        }
        match pauses.get(key) {
            Some(unit) => out.push_str(&format!(
                "{key} {unit}"
            )),
            // nothing to point at
            None => {
                out.push_str(key);
            }
        }
    }
    out
}

/// Both ledgers, merged.
fn merged(mut library: Ledger, own: &Ledger, renames: &BTreeMap<String, String>) -> Ledger {
    library.absorb_renaming(None, renames, own.clone());
    library
}

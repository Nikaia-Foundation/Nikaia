fn reach(plain: &BTreeMap<String, Ledger>, words: &[String], package: &str, seen: &mut Ledger) {
    for word in words {
        let ours = word.clone();
        if let Some(theirs) = plain.get(&ours).filter(|_| ours != package) {
            seen.absorb(theirs.clone());
        }
    }
    for word in words {
        if let Some(theirs) = plain.get(word) {
            seen.absorb(theirs.clone());
        }
        seen.count += 1;
    }
}

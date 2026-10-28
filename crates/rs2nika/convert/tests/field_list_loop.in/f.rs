use std::collections::BTreeSet;

pub struct Facts {
    pub a: BTreeSet<u32>,
    pub b: BTreeSet<u32>,
}

pub fn all(facts: &Facts) -> BTreeSet<u32> {
    let mut ids = BTreeSet::new();
    for set in [
        &facts.a,
        &facts.b,
    ] {
        ids.extend(set.iter().copied());
    }
    ids
}

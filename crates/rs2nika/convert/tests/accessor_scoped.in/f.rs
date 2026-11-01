/// The counters of each rule, each walked in its function.
pub fn counted(parsed: &Parsed, names: &[String]) -> BTreeSet<i64> {
    let mut out = BTreeSet::new();
    for (at, name) in names.iter().enumerate() {
        let walked = crate::proofs::in_function(name, at, || walk(parsed, at as i64));
        crate::proofs::record_bound_invariants(&walked.loops);
        out.extend(walked.counters);
    }
    out
}

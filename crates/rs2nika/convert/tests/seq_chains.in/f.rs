fn chains(units: &[&Parsed], errors: &[String], members: &[Member], parsed: &Parsed, held: &[ClaimHeld], at: usize) -> u64 {
    for parsed in units.iter().copied() {
        visit(parsed);
    }
    for (unit, parsed) in units.iter().copied().enumerate() {
        visit_at(unit, parsed);
    }
    for name in errors.iter().cloned() {
        visit_name(name);
    }
    let fatal: Vec<String> = errors.iter().filter(|e| e.len() > 3).cloned().collect();
    let mut out: Vec<ClaimHeld> = held.iter().filter(|h| h.position == "self").cloned().collect();
    let context = Context { lengths: lengths.iter().copied().collect(), sized: sized.iter().copied().collect() };
    let at_of: BTreeMap<&Path, usize> = members.iter().enumerate().map(|(at, m)| (m.root.as_path(), at)).collect();
    let beside: Vec<&Parsed> = units.iter().enumerate().filter(|(other, _)| *other != at).map(|(_, other)| other).collect();
    let names: Vec<usize> = parsed.items.iter().enumerate().filter_map(|(i, item)| {
        if item.public { Some(i) } else { None }
    }).collect();
    let hash = name.bytes().fold(0u64, |h, b| (h ^ u64::from(b)).wrapping_mul(3));
    let provenance = held.iter().map(|r| r.provenance).fold(Provenance::Trusted, Provenance::join);
    let total = errors.iter().fold(0, |acc, e| {
        let n = e.len();
        acc + n
    });
    for calls in held.iter().flat_map(|c| c.methods.values()) {
        visit_calls(calls);
    }
    let declared: BTreeSet<String> = units.iter().flat_map(|u| declares(u)).collect();
    let rest: Vec<String> = errors.iter().skip(1).cloned().collect();
    let one = held.get(at).cloned();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    return held.iter().map(|h| h.weight).fold(0, worst_of);
}

fn join(answers: Vec<Crossing>) -> Crossing {
    answers
        .into_iter()
        .fold(Crossing::May, threads::worst_of)
}

fn casts(head: Expr, casts: Vec<Ty>) -> Expr {
    casts.into_iter().fold(head, |expr, ty| Expr::Cast { expr: Box::new(expr), ty })
}

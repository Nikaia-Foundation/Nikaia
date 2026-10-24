fn payload(parsed_here: &Parsed, beside: &[&Parsed], ty: &str) -> Option<Vec<Ty>> {
    std::iter::once(parsed_here)
        .chain(beside.iter().copied())
        .find_map(|parsed| {
            let t = parsed_view_of(parsed);
            check_payload_in(&t, ty)
        })
}

fn walks(generics: &[G], parsed: &Parsed) {
    let walked = generics.iter().find_map(|g| {
        let parameter = parsed.text(g.name).to_string();
        let asked = g.bounds.iter().any(|b| shape(b));
        asked.then_some(parameter)
    });
    let hit = items.iter().find(|item| item.name == name);
    let low = lowered.iter().find(|l| l.name == name && l.ty.is_a_view());
}

/// Every name a file's functions call.
fn called_in(files: &[&crate::parser::Parsed]) -> std::collections::BTreeSet<String> {
    fn note(parsed: &crate::parser::Parsed, e: &Expr, names: &mut std::collections::BTreeSet<String>) {
        if let Expr::Variable(n, _) = e {
            names.insert(parsed.text(*n).to_string());
        }
    }

    let mut names = std::collections::BTreeSet::new();
    for parsed in files {
        for item in &parsed.program.items {
            if let Item::Fn { body, .. } = &item.node {
                crate::emit::visit_block(body, &mut |e| note(parsed, e, &mut names));
            }
        }
    }
    names
}

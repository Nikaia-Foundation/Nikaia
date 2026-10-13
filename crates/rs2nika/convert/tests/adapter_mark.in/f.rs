fn payload_of(
    parsed_here: &Parsed,
    beside: &[&Parsed],
    ty: &str,
    variant: &str,
) -> Option<Vec<Ty>> {
    std::iter::once(parsed_here)
        .chain(beside.iter().copied())
        .find_map(|parsed| {
            parsed
                .program
                .items
                .iter()
                .find_map(|item| match &item.node {
                    Item::Enum { name, variants, .. } if parsed.text(*name) == ty => variants
                        .iter()
                        .find(|held| parsed.text(held.name) == variant)
                        .map(|held| match &held.fields {
                            ast::VariantFields::Unit => Vec::new(),
                            ast::VariantFields::Tuple(types) => {
                                types.iter().map(|t| Ty::from_ast(parsed, t)).collect()
                            }
                            ast::VariantFields::Named(fields) => {
                                fields.iter().map(|f| Ty::from_ast(parsed, &f.ty)).collect()
                            }
                        }),
                    _ => None,
                })
        })
}

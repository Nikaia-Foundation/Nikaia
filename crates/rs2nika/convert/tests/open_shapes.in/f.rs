fn shapes(attribute: &Attribute, names: &Names, state: &mut State) -> Option<String> {
    let Some(fields) = state.fields_of(names) else {
        state.missing += 1;
        return None;
    };
    let word = |arg: &Expr| match arg {
        Expr::Variable(word, _) => Some(names.text(*word).to_string()),
        _ => None,
    };
    let required: Vec<&Field> =
        fields.iter().filter(|f| f.default.is_empty()).collect();
    match (&attribute.args[..], required.is_empty()) {
        ([], true) => None,
        ([arg], _) => word(arg),
        _ => None,
    }
}

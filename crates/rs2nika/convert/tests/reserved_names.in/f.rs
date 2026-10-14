fn entered(grammar: &str, throws: bool) -> String {
    let with = grammar.len();
    if throws {
        return format!("{grammar} {}", with);
    }
    grammar.to_string()
}

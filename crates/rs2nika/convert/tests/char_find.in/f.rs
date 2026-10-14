fn tick(text: &str) -> usize {
    let at = text.find('`').unwrap_or(0);
    let quote = text.rfind('\'').unwrap_or(0);
    at + quote
}

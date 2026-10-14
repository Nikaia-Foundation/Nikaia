fn first_word(found: Option<(String, i64)>, text: &str) -> i64 {
    match found {
        Some((head, n))
            if text.chars().next().is_some_and(char::is_alphanumeric) =>
        {
            n + head.len() as i64
        }
        Some((_, n)) => n,
        None => 0,
    }
}

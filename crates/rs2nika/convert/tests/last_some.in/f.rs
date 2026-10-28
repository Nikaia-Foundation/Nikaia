fn last_name(segments: &[String], names: &mut Vec<String>) {
    if let Some(last) = segments.last() {
        names.push(last.clone());
    }
}

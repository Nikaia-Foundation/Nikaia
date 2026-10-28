fn comment(bytes: &[u8], at: usize) -> usize {
    match bytes[at] {
        47 if bytes.get(at + 1) == Some(&47) => 1,
        47 if bytes.get(at + 1) == Some(&42) => 2,
        _ => 0,
    }
}

fn quoted(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&34) {
        return None;
    }
    Some(i + 1)
}

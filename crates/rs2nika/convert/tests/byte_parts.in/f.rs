/// The packed bytes.
fn packed(name: &str, body: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// The key, trimmed.
fn key(text: &str) -> String {
    text.trim().to_string()
}

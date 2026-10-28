fn is_refusal(text: &[u8]) -> bool {
    (text.starts_with(b"error") || text.windows(6).any(|w| w == b"\nerror")) && !failed(text)
}

fn failed(text: &[u8]) -> bool {
    text.windows(6).any(|w| w == b"failed")
}

fn packed(body: &[u8]) -> usize {
    body.len() + 1
}

fn difference(expected: &[u8], produced: &[u8]) -> String {
    let (Ok(want), Ok(got)) = (std::str::from_utf8(expected), std::str::from_utf8(produced))
    else {
        return "  not text\n".to_string();
    };
    lines(want, got)
}

fn a(s: &str) -> Option<usize> {
    let at = 1 + s.find(':')?;
    let n = s.find(',')? + 1;
    let m = s.get(1..)?.len();
    Some(at + n + m)
}

fn b(s: &str) -> String {
    let w = s.parse::<u64>().unwrap_or(0);
    let r = s.find(char::is_whitespace);
    let q = s.replace(['-', '.'], "_");
    let k = 7u32 + 0usize as u32;
    let first = s.as_bytes()[0];
    let quote = s.rfind('\'');
    let bytes = b"ab";
    format!("{w:>4}{:x}{{}}", k)
}

fn c(one: char) -> String {
    let s = format!("'\\u{{{:x}}}'", one as u32);
    format!("{{{}}}", s)
}

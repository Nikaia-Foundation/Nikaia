fn shown(byte: u8) -> String {
    match byte {
        b'\\' | b'\'' => format!("b'\\{}'", byte as char),
        0x20..=0x7e => format!("b'{}'", byte as char),
        b'\x7f' | b'\n' => "odd".to_string(),
        _ => format!("b'\\x{byte:02x}'"),
    }
}

fn peeled(mut value: &Expr) -> &Expr {
    while let Expr::Block(block, _) = value && let Some(tail) = tail_of(block) {
        value = tail;
    }
    value
}

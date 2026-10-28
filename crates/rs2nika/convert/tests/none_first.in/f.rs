fn key(cpu: Option<String>) -> String {
    let k = match cpu {
        Some(_) => name(),
        None => String::new(),
    };
    k
}

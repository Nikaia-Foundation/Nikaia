fn host_target() -> Option<Target> {
    Target::read_triple(env!("HOST")).ok()
}

fn flushed(file: &mut File, line: &str) {
    std::io::stdout().flush().ok();
    writeln!(file, "{line}").ok();
}

fn store(entry: &Path) {
    if std::fs::create_dir_all(entry).is_err() {
        return;
    }
    if !std::fs::remove_file(entry).is_ok() {
        eprintln!("kept");
        return;
    }
    done();
}

fn mode(other: &str) -> bool {
    if other.parse::<i64>().is_ok() {
        return true;
    }
    !other.parse::<u64>().is_ok()
}

fn kept(path: &Path) -> Option<String> {
    let current = std::fs::read_to_string(path).ok();
    current
}

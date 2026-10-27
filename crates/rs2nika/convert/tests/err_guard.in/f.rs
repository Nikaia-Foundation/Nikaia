fn run_all() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if is_a_refusal(&error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn size(path: &Path) -> u64 {
    match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
        Err(_) => 1,
    }
}

fn last(path: &Path) -> u64 {
    match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
    }
}

fn twice(text: &str) -> Result<Tree> {
    let again = match read_tree(text) {
        Ok(again) => again,
        Err(error) => bail!("does not parse: {error}"),
    };
    Ok(again)
}

fn format_all(files: Vec<String>) -> i64 {
    let mut code = 0;
    for file in files {
        let formatted = match format(&file) {
            Ok(formatted) => formatted,
            Err(refusal) => {
                eprintln!("{file}: {refusal}");
                code = 1;
                continue;
            }
        };
        println!("{formatted}");
    }
    code
}

fn stage(text: &str) -> Stage {
    match lower(text) {
        Ok(_) => Stage::Lowered,
        Err(_) => Stage::Refused,
    }
}

fn hole(text: &str) -> Hole {
    match read_tree(text) {
        Ok(expr) => Hole::Parsed(expr),
        Err(e) => Hole::Refused(e.to_string()),
    }
}

fn dump(ty: &str) -> Computed {
    let dump = match dumper(ty) {
        Ok(dump) => dump,
        Err(Wall::NoForm { because }) => {
            return Computed::NotHere(because);
        }
        Err(_) => {
            return Computed::NotHere("no".to_string());
        }
    };
    Computed::Value(dump)
}

fn run_it() {
    match run() {
        Ok(mut held) => {
            held.push(1);
            show(held);
        }
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    }
    done();
}

fn open_cache() -> Option<Cache> {
    let cache = match Cache::open() {
        Ok(cache) => Some(cache),
        Err(error) => {
            eprintln!("warning: {error:#}");
            None
        }
    };
    cache
}

fn position(starts: &[usize], at: usize) -> usize {
    let line = match starts.binary_search(&at) {
        Ok(i) => i,
        Err(i) => i - 1,
    };
    line
}

fn report(said: Result<Out>) -> i64 {
    match said {
        Ok(out) => {
            show(out);
        }
        Err(error) => {
            eprintln!("{error}");
        }
    }
    0
}

fn first_line(path: &str) -> String {
    if let Ok(text) = read_file(path) {
        show(&text);
    } else {
        return String::new();
    }
    path.to_string()
}

fn note(path: &str) {
    if let Ok(text) = read_file(path) {
        show(&text);
    }
    done();
}

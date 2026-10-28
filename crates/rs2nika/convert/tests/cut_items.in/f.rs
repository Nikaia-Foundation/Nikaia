use std::path::Path;

struct Kept {
    root: String,
}

impl Kept {
    fn first(&self) -> usize {
        1
    }

    fn second(&self) -> usize {
        2
    }

    fn third(&self) -> usize {
        3
    }
}

fn left_out() {}

fn whole() -> bool {
    true
}

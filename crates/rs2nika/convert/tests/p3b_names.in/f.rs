use crate::escapes::NotAnEscape as Refused;

enum Blessed {
    Unchanged,
    Refused(&'static str),
}

fn bless(n: i64) -> Blessed {
    if n == 0 {
        return Blessed::Refused("none");
    }
    Blessed::Unchanged
}

pub fn run(n: i64) -> bool {
    matches!(bless(n), Blessed::Unchanged)
}

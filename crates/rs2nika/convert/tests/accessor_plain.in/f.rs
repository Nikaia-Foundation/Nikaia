/// Whether the goal is proved under no facts.
pub fn answer(arena: &TermArena, goal: i64) -> bool {
    crate::proofs::ask(arena, &[], goal) == crate::proofs::Asked::Proved
}

/// Whether both goals are proved: `answer` asks, so this one passes it on.
pub fn both(arena: &TermArena, a: i64, b: i64) -> bool {
    answer(arena, a) && answer(arena, b)
}

/// Asks nothing: no parameter.
pub fn width(a: i64) -> i64 {
    a + 1
}

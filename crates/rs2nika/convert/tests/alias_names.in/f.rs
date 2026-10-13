fn f(h: &Held, b: &Block) -> bool {
    use crate::prove::Held;
    tail_of(b).is_some() && matches!(h, Held::Proved)
}

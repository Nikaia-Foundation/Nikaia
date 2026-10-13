fn counted(rows: &[Held]) -> usize {
    let count = |want: fn(&Held) -> bool| {
        rows.iter()
            .filter(|c| want(c))
            .count()
    };
    count(|h| matches!(h, Held::Proved))
}

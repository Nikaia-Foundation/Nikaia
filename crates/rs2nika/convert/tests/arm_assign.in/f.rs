fn tally(arms: &[bool]) -> usize {
    let mut numbers = 0;
    let mut known = 0;
    for arm in arms {
        match arm {
            true => numbers += 1,
            false => known = 2,
        }
    }
    numbers + known
}

fn lets() -> usize {
    let (mut at, mut depth, mut start) = (0usize, 0i64, true);
    let (mut a, b) = (1, 2);
    let (c, d) = (3, 4);
    at += 1;
    a += b + c + d;
    at
}

fn r73_shown(literal: bool, p: Point, xs: &[Item]) -> Option<Point> {
    let moved = Point { x: 1, y, ..p };
    for (i, ((a, b), c)) in pairs(xs) {
        use_it(i, a, b, c);
    }
    apply(xs, &|ty: &str, n: usize| width(ty, n));
    (!literal).then(|| moved)
}

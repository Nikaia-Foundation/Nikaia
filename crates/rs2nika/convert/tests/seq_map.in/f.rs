fn range_map_probe(n: usize, xs: &[i64]) -> Vec<i64> {
    (0..n).map(|i| xs[i] + 1).collect()
}

fn lines_map_probe(text: &str) -> Vec<usize> {
    text.lines().filter(|l| !l.is_empty()).map(|l| l.len()).collect()
}

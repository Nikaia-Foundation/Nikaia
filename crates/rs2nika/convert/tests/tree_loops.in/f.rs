fn halves(xs: &[i64], seen: &mut Vec<i64>) -> Vec<i64> {
    xs.iter()
        .map(|x| {
            seen.push(*x);
            let h = x / 2;
            h + 1
        })
        .collect()
}

fn half(n: i64) -> Option<i64> {
    if n % 2 == 0 { Some(n / 2) } else { None }
}

fn evens(rows: &[Vec<i64>], count: &mut i64) -> Vec<Vec<i64>> {
    rows.iter()
        .filter_map(|row| {
            *count += 1;
            if row.is_empty() {
                return None;
            }
            let first = half(row[0])?;
            let rest = row.iter().map(|n| half(*n)).collect::<Option<Vec<_>>>()?;
            Some(vec![first, rest.len() as i64])
        })
        .collect()
}

fn plain(xs: &[i64]) -> Vec<i64> {
    xs.iter()
        .map(|x| {
            let y = x * 2;
            y + 1
        })
        .collect()
}

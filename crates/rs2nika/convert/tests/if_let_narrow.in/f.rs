fn big(limit: Option<i64>, other: Option<i64>) -> i64 {
    if let Some(limit) = limit && limit > 3 {
        return limit + 1;
    }
    if let Some(n) = other {
        return n * 2;
    }
    0
}

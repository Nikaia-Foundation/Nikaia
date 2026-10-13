fn how(claim: &Claim) -> String {
    match &claim.held {
        Some(Held::Proved) => "proved".to_string(),
        Some(Held::AtRunTime(why)) => format!("checked at run time: {why}"),
        Some(Held::Refused) | None => {
            "refused".to_string()
        }
    }
}

fn first(rows: &[Row]) -> i64 {
    for row in rows {
        let n = match row.kind {
            Some(Kind::A) => 1,
            Some(Kind::B(x)) => x,
            None => continue,
        };
        return n;
    }
    0
}

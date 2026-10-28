fn summary(all: &[Finding], extra: usize) -> String {
    let findings: Vec<&Finding> = all.iter().filter(|f| f.error).collect();
    let mut refused = Vec::new();
    let count = |family: &str| {
        findings
            .iter()
            .filter(|f| f.code.starts_with(family))
            .count()
    };
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    let more = |n: usize| n + extra;
    let kept = |f: &Finding| f.code != "NK1";
    let types = count("NK1");
    if types > 0 {
        refused.push(format!("{types} type error{}", plural(types)));
    }
    let locks = ["NK2203", "NK2204"].iter().map(|code| count(code)).sum::<usize>();
    refused.push(format!("{} more", more(locks)));
    let left: Vec<&&Finding> = findings.iter().filter(kept).collect();
    refused.join(", ")
}

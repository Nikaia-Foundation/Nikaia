fn soften(findings: &mut [Finding], on: &str) {
    if on != "warn" {
        return;
    }
    for finding in findings.iter_mut().filter(|f| f.code == "NK1207") {
        finding.severity = Severity::Warning;
        finding.notes.push("softened".to_string());
    }
    for item in self.items.iter_mut().filter(|it| it.on && it.code != "x") {
        item.count += 1;
    }
    for (a, b) in pairs.iter_mut().filter(|p| p.0 == 1) {
        a.push(b);
    }
}

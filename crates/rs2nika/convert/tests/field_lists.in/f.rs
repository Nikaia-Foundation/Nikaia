pub struct Unit {
    pub package: Option<String>,
    pub source: String,
}

pub struct Program {
    pub units: Vec<Unit>,
}

impl Program {
    /// What each unit says.
    pub fn report(&self, at: usize) -> String {
        let mut out = String::new();
        let finding = Finding {
            code: 1,
        };
        out.push_str(&render_finding(
            &finding,
            &self.units[at].source,
        ));
        if self.units.len() == 1 {
            out.push_str(&self.units[0].source);
        }
        out
    }

    /// The packages.
    pub fn names(&self) -> Vec<String> {
        self.units
            .iter()
            .filter_map(|u| u.package.clone())
            .collect()
    }
}

pub struct Program {
    pub units: Vec<Unit>,
    pub entry_shown_as: Option<String>,
    pub contracts: crate::contracts::Ledger,
}

impl Program {
    /// Each unit's path.
    pub fn first_path(&self, at: usize) -> String {
        if self.is_single_file() {
            return self.units[0].path.clone();
        }
        let shown = self.entry_shown_as.clone().unwrap_or_default();
        format!("{shown} {}", self.units[at].path)
    }

    pub fn is_single_file(&self) -> bool {
        self.units.len() == 1
    }
}

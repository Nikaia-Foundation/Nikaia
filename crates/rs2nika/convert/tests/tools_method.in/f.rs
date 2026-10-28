impl Ledger {
    pub fn size(&self) -> usize {
        self.functions.len()
    }

    pub fn merged(&self, other: Ledger) -> Ledger {
        let mut out = self.clone();
        out.functions.extend(other.functions);
        out
    }
}

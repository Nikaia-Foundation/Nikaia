impl Checker {
    fn count(&mut self, name: &str) -> usize {
        self.s.seen.push(name.to_string());
        self.s.seen.len()
    }

    fn twice(&mut self, name: &str) {
        for item in &self.parsed.program.items {
            self.count(name);
        }
    }
}

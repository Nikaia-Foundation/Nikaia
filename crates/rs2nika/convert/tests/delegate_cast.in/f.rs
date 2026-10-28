struct Walk<'a> {
    names: &'a Vec<String>,
    limit: &'a i64,
    found: Vec<String>,
}

impl Walk<'_> {
    fn visit(&mut self, at: usize) {
        if self.wanted(at) {
            self.found.push(self.names[at].clone());
            let n = self.size();
        }
    }

    fn wanted(&self, at: usize) -> bool {
        self.names[at].len() as i64 > *self.limit
    }

    fn size(&self) -> usize {
        self.names.len() as usize
    }
}

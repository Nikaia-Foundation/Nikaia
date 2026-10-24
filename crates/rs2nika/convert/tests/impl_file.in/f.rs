pub fn top(x: &str) -> bool {
    x.is_empty()
}

impl<'a> Checker<'a> {
    fn walk(&mut self, a: &str) -> bool {
        self.note(a);
        self.errors += 1; self.host().x();
        a.is_empty()
    }

    fn note(&self, a: &str) {
        let _ = a;
    }
}

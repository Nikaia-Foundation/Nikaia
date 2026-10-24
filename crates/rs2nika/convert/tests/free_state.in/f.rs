impl Checker {
    fn bump(&mut self, by: usize) {
        self.count += by;
    }
    fn both(&mut self) {
        self.bump(1);
        self.bump(2);
        self.s.other();
    }
    fn peek(&self) -> usize {
        self.count
    }
}

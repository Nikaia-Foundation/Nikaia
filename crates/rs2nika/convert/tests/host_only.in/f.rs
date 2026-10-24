impl<'a> Checker<'a> {
    fn outer(&mut self, a: &str) -> bool {
        self.inner(a);
        self.leaf(a)
    }

    fn inner(&self, a: &str) {
        let _ = self.nikaia(|state, world, host| run(state, world, host, a));
    }

    fn leaf(&self, a: &str) -> bool {
        a.is_empty()
    }
}

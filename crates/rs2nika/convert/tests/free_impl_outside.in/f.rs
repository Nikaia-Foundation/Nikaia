impl Codegen {
    fn get(&self) -> u32 { self.n }
    fn twice(&self) -> u32 { self.get() * 2 }
    fn bump(&mut self) { self.n += 1; }
    fn make() -> u32 { Self::helper() + 1 }
    fn helper() -> u32 { 3 }
}
impl std::fmt::Display for Codegen {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "c") }
}

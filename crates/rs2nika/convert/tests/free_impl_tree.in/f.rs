pub struct Manifest { notes: Vec<String>, name: Option<String> }
impl Manifest {
    pub fn new() -> Self { Manifest { notes: Vec::new(), name: None } }
    pub fn notes(&self) -> &[String] { &self.notes }
    pub fn package_name(&self) -> Option<&str> { self.name.as_deref() }
    pub fn has_notes(&self) -> bool { !self.notes().is_empty() }
    pub fn add(&mut self, n: &str) { self.notes.push(n.to_string()); }
    fn helper() -> u32 { 3 }
    pub fn count(&self) -> u32 { Self::helper() + self.notes.len() as u32 }
}

pub enum Mode { Yes, No }

impl Mode {
    pub fn is_on(self) -> bool { matches!(self, Mode::Yes) }
}

pub fn fresh() -> Manifest { Manifest::new() }
fn load_checked(path: &Path) -> Result<String> {
    let other = load(path).context("cannot load")?;
    if source.is_empty() {
        return Err(crate::diagnostics::refuse(format!("{} is empty", path.display())));
    }
    Ok(source)
}

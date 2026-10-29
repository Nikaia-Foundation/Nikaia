#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    Fast,
    #[default]
    Kept,
}

pub struct Counts {
    pub mode: Mode,
    pub names: Vec<String>,
    pub help: Option<String>,
    pub size: i64,
}

fn label(n: i64) -> Option<String> {
    if n > 0 { Some(format!("n{n}")) } else { None }
}

pub(crate) fn fresh() -> Counts {
    Counts {
        mode: Mode::default(),
        names: Default::default(),
        help: None,
        size: Default::default(),
    }
}

pub(crate) fn help_of(c: &Counts) -> String {
    c.help.as_deref().unwrap_or_default().to_string()
}

pub(crate) fn label_or_empty(n: i64) -> String {
    label(n).unwrap_or_default()
}

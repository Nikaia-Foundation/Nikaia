/// Whether to stay quiet: inside the wrapper, or warned already.
fn quiet(marker: bool) -> bool {
    marker || WARNED.with(|warned| warned.get())
}

/// The name the entry is shown as.
fn shown(program: &mut Program) {
    program.entry_shown_as = SHOWN_AS.with(|cell| cell.borrow().clone());
}

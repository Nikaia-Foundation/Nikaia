//! Calls the engine's `parse_items_reader` of the `@stream` entry
//! `Words::items`, which returns the fold's count and is private to the
//! grammar's module (ADR-grammar-2 D7, #1464): only `Words::items_reader` is
//! the crate's. The build fails with E0603.

fn main() {
    let n = nikaia_lines::Words::parse_items_reader(std::io::Cursor::new("a;b;"));
    let _ = n;
}

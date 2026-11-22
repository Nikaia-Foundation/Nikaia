//! Reads the items of the `@stream` entry `Words::items` of `nikaia_lines`,
//! whose item type views the input in Nikaia, from a `std::io::Cursor`: the
//! reader hands out `ItemOwned`, which owns its text, so the items are kept
//! after the reader and its frames are gone.

use std::io::Cursor;

use nikaia_lines::{ItemOwned, KindOwned, Words};

fn main() {
    let mut kept: Vec<ItemOwned> = Vec::new();
    for read in Words::items_reader(Cursor::new(b"a;qbc;de;".to_vec())) {
        kept.push(read.expect("an item"));
    }
    for item in &kept {
        let kind = match &item.kind {
            KindOwned::Plain => String::from("plain"),
            KindOwned::Quoted(text) => format!("quoted {text}"),
        };
        let note: Option<String> = item.note.clone();
        println!("{} {:?} {} {kind} {}", item.word, note, item.inner.text, item.size);
    }
    match Words::items_reader(Cursor::new(b"p;q".to_vec())).last() {
        Some(Err(error)) => println!("p;q fails: {}", error.to_string().lines().next().unwrap_or("")),
        other => println!("p;q: {}", other.is_some()),
    }
}

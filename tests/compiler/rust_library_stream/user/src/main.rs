//! Reads the items of the `@stream` entry `Words::items` of `nikaia_lines`
//! from a `std::io::Cursor`, one at a time: each item owned, a `break` that
//! stops before an error the parse never reaches, and an error that is the
//! sequence's last element.

use std::io::Cursor;

use nikaia_lines::{Item, Words};

fn main() {
    for read in Words::items_reader(Cursor::new(b"a;b;c;".to_vec())) {
        let Item { word } = read.expect("an item");
        println!("read {word}");
    }
    let mut kept: Vec<Item> = Vec::new();
    for read in Words::items_reader(Cursor::new(b"x;stop;y".to_vec())) {
        let item = read.expect("an item before the stop");
        if item.word == "stop" {
            break;
        }
        kept.push(item);
    }
    println!("kept {} before the stop", kept.len());
    match Words::items_reader(Cursor::new(b"p;q".to_vec())).last() {
        Some(Err(error)) => println!("p;q fails: {}", error.to_string().lines().next().unwrap_or("")),
        other => println!("p;q: {}", other.is_some()),
    }
}

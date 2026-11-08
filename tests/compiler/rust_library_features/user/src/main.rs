//! Uses the core and the feature `stream` of `nikaia_demo`, and nothing of
//! `tree`, which it did not ask for.

use nikaia_demo::{Event, core_add, event};

fn main() {
    let Event { at } = event(4);
    println!("core_add: {}, event at {at}", core_add(2, 3));
}

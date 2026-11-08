//! Calls the hand-written root of `nikaia_demo` and the lowered items under
//! `nikaia_demo::native`.

use nikaia_demo::native::{Point, checked_half};
use nikaia_demo::{area, cube, described};

fn main() {
    let Point { x, y } = Point { x: 1, y: 2 };
    println!("area {}, cube {}, {}, point ({x}, {y})", area(3), cube(2), described(4));
    // A function with two errors fails with a sum generated below `native`
    // (#1447).
    for n in [8, 7, -2] {
        match checked_half(n) {
            Ok(half) => println!("half of {n}: {half}"),
            Err(error) => println!("half of {n}: {error}"),
        }
    }
}

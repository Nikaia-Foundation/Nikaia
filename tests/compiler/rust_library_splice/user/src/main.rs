//! Calls the hand-written root of `nikaia_demo` and the lowered items under
//! `nikaia_demo::native`.

use nikaia_demo::native::Point;
use nikaia_demo::{area, cube, described};

fn main() {
    let Point { x, y } = Point { x: 1, y: 2 };
    println!("area {}, cube {}, {}, point ({x}, {y})", area(3), cube(2), described(4));
}

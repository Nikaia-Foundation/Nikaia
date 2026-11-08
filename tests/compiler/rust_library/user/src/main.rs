//! Calls the Nikaia package `demo` as the Rust crate `nikaia_demo`.

use nikaia_demo::{Point, Shape, area, centre};

fn main() {
    for shape in [Shape::Circle(2), Shape::Square(4)] {
        let name = match shape {
            Shape::Circle(_) => "circle",
            Shape::Square(_) => "square",
        };
        let measure = area(&shape);
        let Point { x, y } = centre(shape);
        println!("{name}: area {measure}, centre ({x}, {y})");
    }
}

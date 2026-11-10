//! The records file named on the command line deserialised into
//! `Vec<Record>` with `serde_yaml::from_str`, and a summary of it printed.
//! `facade/src/main.rs` is the same program over the yaml package's facade.
use serde::Deserialize;

#[derive(Deserialize)]
struct Point {
    x: i64,
    y: i64,
}

#[derive(Deserialize)]
struct Record {
    id: i64,
    name: String,
    score: f64,
    active: bool,
    tags: Vec<String>,
    point: Point,
    note: String,
}

fn main() {
    let path = std::env::args().nth(1).expect("a records file");
    let input = std::fs::read_to_string(path).expect("a UTF-8 file");
    let records: Vec<Record> = serde_yaml::from_str(&input).expect("records");
    let (mut ids, mut text, mut score, mut active, mut tags, mut xy) =
        (0i64, 0usize, 0f64, 0, 0, 0i64);
    for r in &records {
        ids += r.id;
        text += r.name.len() + r.note.len();
        score += r.score;
        active += usize::from(r.active);
        tags += r.tags.iter().map(String::len).sum::<usize>();
        xy += r.point.x * r.point.y;
    }
    println!(
        "{} records, ids {ids}, text {text}, score {score}, active {active}, tags {tags}, xy {xy}",
        records.len()
    );
}

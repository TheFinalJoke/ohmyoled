use chrono::Local;
use oledlib::matrix::TimeMatrix;
use ohmyoled_matrix::Color;
use std::path::Path;

fn main() {
    let tm = TimeMatrix::new(
        Color::WHITE,
        Some(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/4x6.bdf"))),
    )
    .unwrap();
    let img = tm.frame(Local::now());
    let lit: usize = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
    println!("lit pixels: {lit}");
    println!("ASCII preview ('#' = lit):");
    for y in 0..img.height() {
        let mut row = String::new();
        for x in 0..img.width() {
            let p = img.get_pixel(x, y);
            row.push(if p.0 == [0, 0, 0] { '.' } else { '#' });
        }
        println!("{row}");
    }
}

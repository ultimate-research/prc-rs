use prc::open;
use prc::xml::to_xml;

use std::fs::File;
use std::io::BufWriter;
use std::time::Instant;

fn main() {
    let mut now = Instant::now();
    let filename = std::env::args().nth(1).unwrap();
    let param = open(&filename).unwrap();
    println!("Opened in {}", now.elapsed().as_secs_f32());

    now = Instant::now();
    let mut writer = BufWriter::new(File::create("output.xml").unwrap());
    to_xml(&param, &mut writer).unwrap();
    println!("Converted to XML in {}", now.elapsed().as_secs_f32());
}
use prc;
use serde_json::to_string_pretty;
use std::time::Instant;
use std::fs::File;
use std::io::Write;

fn main() {
    let now = Instant::now();
    println!("Initializing file...");
    match prc::open("Path\\To\\Param\\File.prc") {
        Ok(x) => {
            println!("Serializing to json...");
            let mut file = File::create("out.json").unwrap();
            file.write_all(to_string_pretty(&x).unwrap().as_bytes()).unwrap();
            println!("Done!")
        }
        Err(x) => println!("{}", x),
    }
    println!("elapsed milliseconds: {}", now.elapsed().as_millis());
}

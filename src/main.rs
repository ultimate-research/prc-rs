use prc;
use serde_json::to_string_pretty;
use std::time::Instant;

fn main() {
    let now = Instant::now();
    match prc::open("Path\\To\\Param\\File.prc") {
        Ok(x) => {
            println!("OK! Serializing to json...");
            println!("{}", to_string_pretty(&x).unwrap())
        }
        Err(x) => println!("{}", x),
    }
    println!("elapsed time: {}", now.elapsed().as_millis());
}

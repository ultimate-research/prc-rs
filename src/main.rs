use prc;
use serde_yaml::to_string;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

fn main() {
    let now = Instant::now();
    println!("Initializing file...");
    match prc::open(
        r"C:\Users\Breakfast\Documents\_Ultimate\root\param\spirits\campaign\0xF9FDA894.prc",
    ) {
        Ok(x) => {
            println!("Serializing...");
            let mut file = File::create("out.yml").unwrap();
            file.write_all(to_string(&x).unwrap().as_bytes())
                .unwrap();
            println!("Done!")
        }
        Err(x) => println!("{}", x),
    }
    println!("elapsed milliseconds: {}", now.elapsed().as_millis());
}

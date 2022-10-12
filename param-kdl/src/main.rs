use prc::kdl::kdl::{KdlDocument, KdlEntry, KdlNode};

const VL: &str = include_str!("../vl.kdl");

fn main() {
    let mut doc: KdlDocument = VL.parse().unwrap();
    doc.fmt();
    println!("{}", doc);
}

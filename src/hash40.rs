use std::collections::HashMap;

pub fn crc(hash: u64) -> u32 {
    hash as u32
}

pub fn len(hash: u64) -> u8 {
    (hash >> 32) as u8
}

pub fn label(hash: u64, labels: HashMap<u64, &str>) -> String {
    match labels.get(&hash) {
        Some(x) => String::from(*x),
        None => format!("{:#012x}", hash),
    }
}

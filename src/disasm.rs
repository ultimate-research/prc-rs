use crate::param;
use byteorder::{LittleEndian,ReadBytesExt};

struct Disassembler {
    HashStart: u32,
    RefStart: u32,
    ParamStart: u32,
    HashTable: [u64],
    //ref table map, to reduce excessive reads from ref section
}

impl Disassembler {
    pub fn disassemble() {
        
    }
}
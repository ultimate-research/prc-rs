mod asm;
mod disasm;
pub mod param;

use std::fs::read;
use std::io::{Cursor, Error};

pub fn open(filepath: &str) -> Result<param::ParamKind, Error> {
    let buf = read(filepath).unwrap();
    disasm::disassemble(&mut Cursor::new(buf))
}

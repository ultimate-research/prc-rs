mod asm;
mod disasm;
pub mod hash40;
pub mod param;

use std::fs::read;
use std::io::Cursor;

pub fn open(filepath: &str) -> Result<param::ParamKind, String> {
    let buf = read(filepath).unwrap();
    disasm::disassemble(&mut Cursor::new(buf))
}

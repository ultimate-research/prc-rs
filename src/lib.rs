mod asm;
mod disasm;
pub mod param;

use std::fs::{read, write};
use std::io::{Cursor, Error, Read, Seek, Write};
use std::path::Path;

pub use hash40;
pub use strum;

pub(crate) type RefTable = Vec<(u32, u32)>;

pub fn read_stream<R>(reader: &mut R) -> Result<param::ParamStruct, Error>
where
    R: Read + Seek,
{
    disasm::disassemble(reader)
}

pub fn write_stream<W>(writer: &mut W, param_struct: &param::ParamStruct) -> Result<(), Error>
where
    W: Write + Seek,
{
    asm::assemble(writer, param_struct)
}

pub fn open<P: AsRef<Path>>(filepath: P) -> Result<param::ParamStruct, Error> {
    let buf = read(filepath)?;
    disasm::disassemble(&mut Cursor::new(buf))
}

pub fn save<P: AsRef<Path>>(filepath: P, param: &param::ParamStruct) -> Result<(), Error> {
    let mut writer = Cursor::new(Vec::<u8>::new());
    asm::assemble(&mut writer, param)?;
    write(filepath, &writer.into_inner())
}

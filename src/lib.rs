mod asm;
mod disasm;
mod param;
mod traits;
#[cfg(feature = "xml-feat")]
pub mod xml;

#[cfg(test)]
mod tests;

use std::fs::{read, write};
use std::io::{Cursor, Error, Read, Seek, Write};
use std::path::Path;

pub use hash40;
pub use param::*;
pub use prc_rs_derive::Prc;
pub use traits::*;

pub(crate) type RefTable = Vec<(u32, u32)>;

/// Attempts to read a param file from the given reader (requires [Seek]).
/// The reader should be positioned at the header of the filetype.
/// Returns a [ParamStruct] if successful, otherwise an [Error].
pub fn read_stream<R>(reader: &mut R) -> std::result::Result<param::ParamStruct, Error>
where
    R: Read + Seek,
{
    disasm::disassemble(reader)
}

/// Attempts to write a param file into the given writer (requires [Seek]).
/// Returns nothing if successful, otherwise an [Error].
pub fn write_stream<W>(
    writer: &mut W,
    param_struct: &param::ParamStruct,
) -> std::result::Result<(), Error>
where
    W: Write + Seek,
{
    asm::assemble(writer, param_struct)
}

/// Attempts to read a param file from the given filepath.
/// Returns a [ParamStruct] if successful, otherwise an [Error].
pub fn open<P: AsRef<Path>>(filepath: P) -> std::result::Result<param::ParamStruct, Error> {
    let buf = read(filepath)?;
    disasm::disassemble(&mut Cursor::new(buf))
}

/// Attempts to write a param file into the given filepath.
/// Returns nothing if successful, otherwise an [Error].
pub fn save<P: AsRef<Path>>(
    filepath: P,
    param: &param::ParamStruct,
) -> std::result::Result<(), Error> {
    let mut writer = Cursor::new(Vec::<u8>::new());
    asm::assemble(&mut writer, param)?;
    write(filepath, &writer.into_inner())
}

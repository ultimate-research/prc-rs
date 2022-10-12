use std::io::{ErrorKind, Read, Result, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use hash40::{Hash40, ReadHash40};

#[derive(Debug, Copy, Clone)]
pub struct Offsets {
    pub hashes: u64,
    pub ref_table: u64,
}

#[derive(Debug, Copy, Clone)]
pub struct StructData {
    pub position: u64,
    pub len: u32,
    pub ref_offset: u32,
}

fn check_type<R: Read + Seek>(reader: &mut R, value: u8) -> Result<()> {
    reader
        .read_u8()?
        .eq(&value)
        .then_some(())
        .ok_or_else(|| ErrorKind::InvalidData.into())
}

impl StructData {
    pub fn from_stream<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let position = reader.seek(SeekFrom::Current(0))?;
        check_type(reader, 12)?;
        let len = reader.read_u32::<LittleEndian>()?;
        let ref_offset = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Start(position))?;

        Ok(Self {
            position,
            len,
            ref_offset,
        })
    }

    // moves the reader to the child param with the provided hash
    pub fn search_child<R: Read + Seek>(
        &self,
        reader: &mut R,
        hash: Hash40,
        offsets: Offsets,
    ) -> Result<()> {
        // TODO: use a binary search instead of a linear one
        for i in 0..self.len {
            reader.seek(SeekFrom::Start(
                offsets.ref_table + self.ref_offset as u64 + (i as u64 * 8),
            ))?;

            let hash_index = reader.read_u32::<LittleEndian>()?;
            let param_offset = reader.read_u32::<LittleEndian>()?;

            reader.seek(SeekFrom::Start(offsets.hashes + (hash_index as u64 * 8)))?;
            if hash == reader.read_hash40::<LittleEndian>()? {
                reader.seek(SeekFrom::Start(self.position + (param_offset as u64)))?;
                return Ok(());
            }
        }
        Err(ErrorKind::NotFound.into())
    }
}

pub trait FromStream: Sized {
    /// Creates Self by reading the from the data. The reader should be
    /// positioned at the start of the param marker before calling this
    fn read_param<R: Read + Seek>(reader: &mut R, offsets: Offsets) -> Result<Self>;
}

/// Reads the header data and moves the reader to the start of the params
pub fn prepare<R: Read + Seek>(reader: &mut R) -> Result<Offsets> {
    reader.seek(SeekFrom::Current(8))?;
    let hashes_size = reader.read_u32::<LittleEndian>()?;
    let ref_table_size = reader.read_u32::<LittleEndian>()?;

    let hashes = reader.seek(SeekFrom::Current(0))?;

    reader.seek(SeekFrom::Current(hashes_size as i64))?;
    let ref_table = reader.seek(SeekFrom::Current(0))?;

    reader.seek(SeekFrom::Current(ref_table_size as i64))?;
    Ok(Offsets {
        hashes,
        ref_table,
    })
}

// basic implementations for all types except struct here

impl FromStream for bool {
    fn read_param<R: Read + Seek>(reader: &mut R, _offsets: Offsets) -> Result<Self> {
        reader
            .read_u8()?
            .eq(&1)
            .then_some(())
            .ok_or(ErrorKind::InvalidData)?;
        reader.read_u8().map(|byte| byte > 0)
    }
}

macro_rules! impl_read_byte {
    ($(($param_type:ty, $num:literal, $read_func:ident)),*) => {
        $(
            impl FromStream for $param_type {
                fn read_param<R: Read + Seek>(reader: &mut R, _offsets: Offsets) -> Result<Self> {
                    check_type(reader, $num)?;
                    ReadBytesExt::$read_func(reader)
                }
            }
        )*
    };
}

macro_rules! impl_read_value {
    ($(($param_type:ty, $num:literal, $read_func:ident)),*) => {
        $(
            impl FromStream for $param_type {
                fn read_param<R: Read + Seek>(reader: &mut R, _offsets: Offsets) -> Result<Self> {
                    check_type(reader, $num)?;
                    ReadBytesExt::$read_func::<LittleEndian>(reader)
                }
            }
        )*
    };
}

impl_read_byte!((i8, 2, read_i8), (u8, 3, read_u8));

impl_read_value!(
    (i16, 4, read_i16),
    (u16, 5, read_u16),
    (i32, 6, read_i32),
    (u32, 7, read_u32),
    (f32, 8, read_f32)
);

impl FromStream for Hash40 {
    fn read_param<R: Read + Seek>(reader: &mut R, offsets: Offsets) -> Result<Self> {
        check_type(reader, 9)?;
        let hash_index = reader.read_u32::<LittleEndian>()?;
        let end_position = reader.seek(SeekFrom::Current(0))?;

        reader.seek(SeekFrom::Start(offsets.hashes + (hash_index as u64 * 8)))?;
        let hash = reader.read_hash40::<LittleEndian>()?;

        reader.seek(SeekFrom::Start(end_position))?;
        Ok(hash)
    }
}

impl FromStream for String {
    fn read_param<R: Read + Seek>(reader: &mut R, offsets: Offsets) -> Result<Self> {
        let str_offset = reader.read_u32::<LittleEndian>()?;
        let end_position = reader.seek(SeekFrom::Current(0))?;

        reader.seek(SeekFrom::Start(offsets.ref_table + str_offset as u64))?;
        let mut string = String::new();

        loop {
            let byte = reader.read_u8()?;
            if byte == 0 {
                break;
            }
            string.push(byte as char);
        }

        reader.seek(SeekFrom::Start(end_position))?;
        Ok(string)
    }
}

impl<T: FromStream> FromStream for Vec<T> {
    fn read_param<R: Read + Seek>(reader: &mut R, offsets: Offsets) -> Result<Self> {
        let start = reader.seek(SeekFrom::Current(0))?;
        check_type(reader, 11)?;
        let len = reader.read_u32::<LittleEndian>()?;

        let mut list = vec![];

        for i in 0..len {
            reader.seek(SeekFrom::Start(start + 5 + (i as u64 * 4)))?;
            let offset = reader.read_u32::<LittleEndian>()?;
            reader.seek(SeekFrom::Start(start + offset as u64))?;
            list.push(T::read_param(reader, offsets)?);
        }

        Ok(list)
    }
}

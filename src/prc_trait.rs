use std::cmp::Ordering;
use std::convert::TryFrom;
use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use hash40::{Hash40, ReadHash40};

/// A trait allowing a type to be converted from the param container format
pub trait Prc: Sized {
    /// Creates Self by reading the from the data. The reader should be
    /// positioned at the start of the param marker before calling this
    fn read_param<R: Read + Seek>(reader: &mut R, offsets: FileOffsets) -> Result<Self>;

    /// A blanket implementation which reads the entire file to create
    /// Self. The reader should be at the beginning of the file before
    /// calling this.
    fn read_file<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let offsets = prepare(reader)?;
        Self::read_param(reader, offsets)
    }
}

// make custom reader struct to return errors in the correct type?

pub type Result<T> = std::result::Result<T, Error>;

/// The error type returned from [Prc] trait operations, including
/// the Hash40 path and reader position
#[derive(Debug)]
pub struct Error {
    pub path: Vec<ErrorPathPart>,
    pub position: std::io::Result<u64>,
    pub kind: ErrorKind,
}

/// The original error thrown
#[derive(Debug)]
pub enum ErrorKind {
    WrongParamNumber { expected: ParamNumber, received: u8 },
    ParamNotFound(Hash40),
    Io(std::io::Error),
}

/// Used for the path of an error. Could be a hash (for structs) or
/// an index (for a list)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorPathPart {
    Index(u32),
    Hash(Hash40),
}

/// Offsets to tables derived from the file header, necessary when reading
/// certain params
#[derive(Debug, Copy, Clone)]
pub struct FileOffsets {
    pub hashes: u64,
    pub ref_table: u64,
}

/// Information read from a struct to facilitate reading child params
#[derive(Debug, Copy, Clone)]
pub struct StructData {
    pub position: u64,
    pub len: u32,
    pub ref_offset: u32,
}

/// The number associated with each type of param in a file
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamNumber {
    Bool = 1,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    Float,
    Hash,
    String,
    List,
    Struct,
}

pub fn check_type<R: Read + Seek>(reader: &mut R, value: ParamNumber) -> Result<()> {
    let pre_pos = reader.stream_position();
    let read = reader.read_u8().map_err(|e| Error::new(e, reader))?;

    if read != value.into() {
        Err(Error::new_with_pos(
            ErrorKind::WrongParamNumber {
                expected: value,
                received: read,
            },
            pre_pos,
        ))
    } else {
        Ok(())
    }
}

impl StructData {
    pub fn from_stream<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let position = reader
            .seek(SeekFrom::Current(0))
            .map_err(|e| Error::new(e, reader))?;

        check_type(reader, ParamNumber::Struct)?;

        let len = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| Error::new(e, reader))?;
        let ref_offset = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| Error::new(e, reader))?;

        reader
            .seek(SeekFrom::Start(position))
            .map_err(|e| Error::new(e, reader))?;

        Ok(Self {
            position,
            len,
            ref_offset,
        })
    }

    /// Moves the reader to the child param with the provided hash
    fn search_child<R: Read + Seek>(
        &self,
        reader: &mut R,
        hash: Hash40,
        offsets: FileOffsets,
    ) -> Result<()> {
        let mut low = 0;
        // on a zero length vec, high = -1, which ends the loop
        let mut high = self.len as i64 - 1;
        while low <= high {
            let i = (low + high) / 2;
            reader
                .seek(SeekFrom::Start(
                    offsets.ref_table + self.ref_offset as u64 + (i as u64 * 8),
                ))
                .map_err(|e| Error::new(e, reader))?;

            let hash_index = reader
                .read_u32::<LittleEndian>()
                .map_err(|e| Error::new(e, reader))?;
            let param_offset = reader
                .read_u32::<LittleEndian>()
                .map_err(|e| Error::new(e, reader))?;

            reader
                .seek(SeekFrom::Start(offsets.hashes + (hash_index as u64 * 8)))
                .map_err(|e| Error::new(e, reader))?;
            let read_hash = reader
                .read_hash40::<LittleEndian>()
                .map_err(|e| Error::new(e, reader))?;

            match read_hash.cmp(&hash) {
                Ordering::Less => low = i + 1,
                Ordering::Greater => high = i - 1,
                Ordering::Equal => {
                    reader
                        .seek(SeekFrom::Start(self.position + (param_offset as u64)))
                        .map_err(|e| Error::new(e, reader))?;
                    return Ok(());
                }
            }
        }
        Err(Error::new_with_pos(
            ErrorKind::ParamNotFound(hash),
            Ok(self.position),
        ))
    }

    /// Moves the reader to the child param with the provided hash and reads
    /// the param fulfilling the [Prc] trait.
    pub fn read_child<R: Read + Seek, T: Prc>(
        &self,
        reader: &mut R,
        hash: Hash40,
        offsets: FileOffsets,
    ) -> Result<T> {
        // If the child param isn't found, we don't push that hash into the error path
        self.search_child(reader, hash, offsets)?;

        // Errors caused while doing anything else will add the hash to the path
        T::read_param(reader, offsets).map_err(|mut e| {
            e.path.insert(0, ErrorPathPart::Hash(hash));
            Error {
                path: e.path,
                position: e.position,
                kind: e.kind,
            }
        })
    }
}

/// Reads the header data and moves the reader to the start of the params
pub fn prepare<R: Read + Seek>(reader: &mut R) -> Result<FileOffsets> {
    prepare_internal(reader).map_err(|e| Error::new(e, reader))
}

fn prepare_internal<R: Read + Seek>(reader: &mut R) -> std::io::Result<FileOffsets> {
    reader.seek(SeekFrom::Current(8))?;
    let hashes_size = reader.read_u32::<LittleEndian>()?;
    let ref_table_size = reader.read_u32::<LittleEndian>()?;

    let hashes = reader.seek(SeekFrom::Current(0))?;

    reader.seek(SeekFrom::Current(hashes_size as i64))?;
    let ref_table = reader.seek(SeekFrom::Current(0))?;

    reader.seek(SeekFrom::Current(ref_table_size as i64))?;
    Ok(FileOffsets { hashes, ref_table })
}

// basic implementations for all types except struct here

impl Prc for bool {
    fn read_param<R: Read + Seek>(reader: &mut R, _offsets: FileOffsets) -> Result<Self> {
        check_type(reader, ParamNumber::Bool)?;
        reader
            .read_u8()
            .map_err(|e| Error::new(e, reader))
            .map(|byte| byte > 0)
    }
}

macro_rules! impl_read_byte {
    ($(($param_type:ty, $num:path, $read_func:ident)),*) => {
        $(
            impl Prc for $param_type {
                fn read_param<R: Read + Seek>(reader: &mut R, _offsets: FileOffsets) -> Result<Self> {
                    check_type(reader, $num)?;
                    ReadBytesExt::$read_func(reader).map_err(|e| Error::new(e, reader))
                }
            }
        )*
    };
}

macro_rules! impl_read_value {
    ($(($param_type:ty, $num:path, $read_func:ident)),*) => {
        $(
            impl Prc for $param_type {
                fn read_param<R: Read + Seek>(reader: &mut R, _offsets: FileOffsets) -> Result<Self> {
                    check_type(reader, $num)?;
                    ReadBytesExt::$read_func::<LittleEndian>(reader).map_err(|e| Error::new(e, reader))
                }
            }
        )*
    };
}

impl_read_byte!(
    (i8, ParamNumber::I8, read_i8),
    (u8, ParamNumber::U8, read_u8)
);

impl_read_value!(
    (i16, ParamNumber::I16, read_i16),
    (u16, ParamNumber::U16, read_u16),
    (i32, ParamNumber::I32, read_i32),
    (u32, ParamNumber::U32, read_u32),
    (f32, ParamNumber::Float, read_f32)
);

impl Prc for Hash40 {
    fn read_param<R: Read + Seek>(reader: &mut R, offsets: FileOffsets) -> Result<Self> {
        check_type(reader, ParamNumber::Hash)?;
        let hash_index = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| Error::new(e, reader))?;
        let end_position = reader
            .seek(SeekFrom::Current(0))
            .map_err(|e| Error::new(e, reader))?;

        reader
            .seek(SeekFrom::Start(offsets.hashes + (hash_index as u64 * 8)))
            .map_err(|e| Error::new(e, reader))?;
        let hash = reader
            .read_hash40::<LittleEndian>()
            .map_err(|e| Error::new(e, reader))?;

        reader
            .seek(SeekFrom::Start(end_position))
            .map_err(|e| Error::new(e, reader))?;
        Ok(hash)
    }
}

impl Prc for String {
    fn read_param<R: Read + Seek>(reader: &mut R, offsets: FileOffsets) -> Result<Self> {
        check_type(reader, ParamNumber::String)?;
        let str_offset = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| Error::new(e, reader))?;
        let end_position = reader
            .seek(SeekFrom::Current(0))
            .map_err(|e| Error::new(e, reader))?;

        reader
            .seek(SeekFrom::Start(offsets.ref_table + str_offset as u64))
            .map_err(|e| Error::new(e, reader))?;
        let mut string = String::new();

        loop {
            let byte = reader.read_u8().map_err(|e| Error::new(e, reader))?;
            if byte == 0 {
                break;
            }
            string.push(byte as char);
        }

        reader
            .seek(SeekFrom::Start(end_position))
            .map_err(|e| Error::new(e, reader))?;
        Ok(string)
    }
}

impl<T: Prc> Prc for Vec<T> {
    fn read_param<R: Read + Seek>(reader: &mut R, offsets: FileOffsets) -> Result<Self> {
        let start = reader
            .seek(SeekFrom::Current(0))
            .map_err(|e| Error::new(e, reader))?;
        check_type(reader, ParamNumber::List)?;
        let len = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| Error::new(e, reader))?;

        let mut list = Vec::with_capacity(len as usize);

        for i in 0..len {
            reader
                .seek(SeekFrom::Start(start + 5 + (i as u64 * 4)))
                .map_err(|e| Error::new(e, reader))?;
            let offset = reader
                .read_u32::<LittleEndian>()
                .map_err(|e| Error::new(e, reader))?;
            reader
                .seek(SeekFrom::Start(start + offset as u64))
                .map_err(|e| Error::new(e, reader))?;

            // read the type, and potentially add index to the error path
            let child = T::read_param(reader, offsets).map_err(|mut e| {
                e.path.insert(0, ErrorPathPart::Index(i));
                Error {
                    path: e.path,
                    position: e.position,
                    kind: e.kind,
                }
            })?;
            list.push(child);
        }

        Ok(list)
    }
}

impl TryFrom<u8> for ParamNumber {
    type Error = u8;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            1 => Ok(ParamNumber::Bool),
            2 => Ok(ParamNumber::I8),
            3 => Ok(ParamNumber::U8),
            4 => Ok(ParamNumber::I16),
            5 => Ok(ParamNumber::U16),
            6 => Ok(ParamNumber::I32),
            7 => Ok(ParamNumber::U32),
            8 => Ok(ParamNumber::Float),
            9 => Ok(ParamNumber::Hash),
            10 => Ok(ParamNumber::String),
            11 => Ok(ParamNumber::List),
            12 => Ok(ParamNumber::Struct),
            _ => Err(value),
        }
    }
}

impl From<ParamNumber> for u8 {
    fn from(param_num: ParamNumber) -> Self {
        param_num as u8
    }
}

impl Error {
    fn new<E: Into<ErrorKind>, S: Seek>(kind: E, seek: &mut S) -> Self {
        Error {
            path: vec![],
            position: seek.stream_position(),
            kind: kind.into(),
        }
    }

    fn new_with_pos<E: Into<ErrorKind>>(kind: E, pos: std::io::Result<u64>) -> Self {
        Error {
            path: vec![],
            position: pos,
            kind: kind.into(),
        }
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(e: std::io::Error) -> Self {
        ErrorKind::Io(e)
    }
}

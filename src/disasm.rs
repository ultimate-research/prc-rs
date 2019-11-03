use crate::param;
use byteorder::{LittleEndian, ReadBytesExt};
use hash40::{Hash40, ReadHash40};
use std::collections::HashMap;
use std::io::{Cursor, Error, ErrorKind, Read};

#[derive(Debug)]
struct FileData {
    ref_start: u32,
    param_start: u32,
    hash_table: Vec<Hash40>,
    //maps an offset to an index in a list of ref-tables
    ref_indeces: HashMap<u32, usize>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
// (hash index, param offset)
struct RefTable(Vec<(u32, u32)>);

pub fn disassemble(cursor: &mut Cursor<Vec<u8>>) -> Result<param::ParamKind, Error> {
    let mut magic_bytes = [0; 8];
    cursor.read(&mut magic_bytes)?;
    if &magic_bytes != param::MAGIC {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid file magic"));
    }

    let hashsize = cursor.read_u32::<LittleEndian>()?;
    let hashnum = (hashsize / 8) as usize;
    let refsize = cursor.read_u32::<LittleEndian>()?;

    let mut fd = FileData {
        ref_start: 0x10 + hashsize,
        param_start: 0x10 + hashsize + refsize,
        hash_table: Vec::with_capacity(hashnum),
        ref_indeces: HashMap::new(),
    };

    for _ in 0..hashnum {
        fd.hash_table.push(cursor.read_hash40::<LittleEndian>()?)
    }

    cursor.set_position(fd.param_start as u64);
    let first_byte = cursor.read_u8()?;
    if first_byte != 12 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "param file does not contain a root",
        ));
    }
    cursor.set_position(cursor.position() - 1);

    let mut refs = Vec::<RefTable>::new();

    read_param(cursor, &mut fd, &mut refs)
}

fn read_param(
    cursor: &mut Cursor<Vec<u8>>,
    fd: &mut FileData,
    refs: &mut Vec<RefTable>,
) -> Result<param::ParamKind, Error> {
    match cursor.read_u8()? {
        1 => {
            let val = cursor.read_u8()?;
            Ok(param::ParamKind::Bool(val != 0))
        }
        2 => {
            let val = cursor.read_i8()?;
            Ok(param::ParamKind::I8(val))
        }
        3 => {
            let val = cursor.read_u8()?;
            Ok(param::ParamKind::U8(val))
        }
        4 => {
            let val = cursor.read_i16::<LittleEndian>()?;
            Ok(param::ParamKind::I16(val))
        }
        5 => {
            let val = cursor.read_u16::<LittleEndian>()?;
            Ok(param::ParamKind::U16(val))
        }
        6 => {
            let val = cursor.read_i32::<LittleEndian>()?;
            Ok(param::ParamKind::I32(val))
        }
        7 => {
            let val = cursor.read_u32::<LittleEndian>()?;
            Ok(param::ParamKind::U32(val))
        }
        8 => {
            let val = cursor.read_f32::<LittleEndian>()?;
            Ok(param::ParamKind::Float(val))
        }
        9 => {
            let val = fd.hash_table[cursor.read_i32::<LittleEndian>()? as usize];
            Ok(param::ParamKind::Hash(val))
        }
        10 => {
            let strpos = cursor.read_u32::<LittleEndian>()?;
            //remembering where we were is actually unnecessary
            //let curpos = cursor.position();
            cursor.set_position((fd.ref_start + strpos) as u64);
            let mut val = String::new();
            let mut next: u8;
            loop {
                next = cursor.read_u8()?;
                if next != 0 {
                    val.push(next as char);
                } else {
                    break;
                }
            }
            //cursor.set_position(curpos);
            Ok(param::ParamKind::Str(val))
        }
        11 => {
            let pos = cursor.position() - 1;
            let size = cursor.read_u32::<LittleEndian>()?;

            let mut offsets = Vec::<u32>::with_capacity(size as usize);
            for _ in 0..size {
                offsets.push(cursor.read_u32::<LittleEndian>()?);
            }

            let mut params = Vec::<param::ParamKind>::with_capacity(size as usize);
            for offset in offsets {
                cursor.set_position(pos + offset as u64);
                params.push(read_param(cursor, fd, refs)?);
            }
            Ok(param::ParamKind::List(params))
        }
        12 => {
            let pos = cursor.position() - 1;
            let size = cursor.read_u32::<LittleEndian>()? as usize;
            let refpos = cursor.read_u32::<LittleEndian>()?;

            let index = match fd.ref_indeces.get(&refpos) {
                Some(i) => *i,
                None => {
                    let mut new_table: Vec<(u32, u32)> = Vec::with_capacity(size);
                    cursor.set_position((fd.ref_start + refpos) as u64);
                    for _ in 0..size {
                        new_table.push((
                            cursor.read_u32::<LittleEndian>()?,
                            cursor.read_u32::<LittleEndian>()?,
                        ));
                    }
                    new_table.sort_by(|a, b| a.0.cmp(&b.0));

                    let len = refs.len();
                    fd.ref_indeces.insert(refpos, len);
                    refs.push(RefTable(new_table));
                    len
                }
            };
            let len = refs[index].0.len();
            let mut cur = 0usize;
            let mut params = Vec::<(Hash40, param::ParamKind)>::with_capacity(size);

            while cur < len {
                let pair = &refs[index].0[cur];

                let hash = fd.hash_table[pair.0 as usize];
                cursor.set_position(pos + pair.1 as u64);
                params.push((hash, read_param(cursor, fd, refs)?));
                cur += 1;
            }
            Ok(param::ParamKind::Struct(params))
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "encountered invalid param number at position: {}",
                cursor.position() - 1
            ),
        )),
    }
}

use crate::param;
use crate::RefTable;
use byteorder::{LittleEndian, ReadBytesExt};
use hash40::{Hash40, ReadHash40};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom};

#[derive(Debug)]
struct FileData {
    ref_start: u32,
    param_start: u32,
    hash_table: Vec<Hash40>,
    //maps an offset to an index in a list of ref-tables
    ref_tables: HashMap<u32, RefTable>,
}

pub fn disassemble<C>(cursor: &mut C) -> Result<param::ParamStruct, Error>
where
    C: Read + Seek,
{
    let mut magic_bytes = [0; 8];
    cursor.read_exact(&mut magic_bytes)?;
    if &magic_bytes != param::MAGIC {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid file magic"));
    }

    let hashsize = cursor.read_u32::<LittleEndian>()?;
    let hashnum = (hashsize / 8) as usize;
    let refsize = cursor.read_u32::<LittleEndian>()?;
    // using relative addresses allows in-memory read of param files
    let hash_start = cursor.seek(SeekFrom::Current(0))? as u32;

    let mut fd = FileData {
        ref_start: hash_start + hashsize,
        param_start: hash_start + hashsize + refsize,
        hash_table: Vec::with_capacity(hashnum),
        ref_tables: HashMap::new(),
    };

    for _ in 0..hashnum {
        fd.hash_table.push(cursor.read_hash40::<LittleEndian>()?)
    }

    cursor.seek(SeekFrom::Start(fd.param_start as u64))?;
    let first_byte = cursor.read_u8()?;
    if first_byte != 12 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "param file does not contain a root",
        ));
    }
    cursor.seek(SeekFrom::Current(-1))?;

    // TODO: possible cleanup here
    if let param::ParamKind::Struct(s) = read_param(cursor, &mut fd)? {
        Ok(s)
    } else {
        //the earlier block guarantees we get a struct
        unreachable!()
    }
}

fn read_param<C>(cursor: &mut C, fd: &mut FileData) -> Result<param::ParamKind, Error>
where
    C: Read + Seek,
{
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
            cursor.seek(SeekFrom::Start((fd.ref_start + strpos) as u64))?;
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
            let pos = cursor.seek(SeekFrom::Current(0))? - 1;
            let size = cursor.read_u32::<LittleEndian>()?;

            let params = (0..size)
                .map(|_| cursor.read_u32::<LittleEndian>())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|offset| {
                    cursor.seek(SeekFrom::Start(pos + offset as u64))?;
                    read_param(cursor, fd)
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(param::ParamKind::List(params))
        }
        12 => {
            let pos = cursor.seek(SeekFrom::Current(0))? - 1;
            let size = cursor.read_u32::<LittleEndian>().unwrap() as usize;
            let refpos = cursor.read_u32::<LittleEndian>().unwrap();

            if !fd.ref_tables.contains_key(&refpos) {
                cursor.seek(SeekFrom::Start((fd.ref_start + refpos) as u64))?;
                let mut new_table = (0..size)
                    .map(|_| {
                        Ok((
                            cursor.read_u32::<LittleEndian>()?,
                            cursor.read_u32::<LittleEndian>()?,
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                new_table.sort_by_key(|a| a.0);
                fd.ref_tables.insert(refpos, new_table);
            }

            let table = fd.ref_tables.get(&refpos).unwrap();

            let params = table
                .iter()
                .map(|&(hash_index, offset)| (hash_index as usize, offset as u64))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|(hash_index, offset)| {
                    let hash = fd.hash_table[hash_index];
                    cursor.seek(SeekFrom::Start(pos + offset))?;
                    Ok((hash, read_param(cursor, fd)?))
                })
                .collect::<Result<Vec<_>, Error>>()?;

            Ok(param::ParamKind::Struct(params))
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "encountered invalid param number at position: {}",
                cursor.seek(SeekFrom::Current(0))? - 1
            ),
        )),
    }
}

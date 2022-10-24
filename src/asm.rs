use crate::param::*;
use crate::RefTable;
use byteorder::{LittleEndian, WriteBytesExt};
use hash40::{Hash40, WriteHash40};
use indexmap::IndexSet;
use std::hash::Hash;
use std::io::{Cursor, Error, Seek, SeekFrom, Write};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
enum RefEntry {
    RString(String),
    RTable(RefTable),
}

// TODO: this is just annoying
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct RefEntryWork {
    pub ref_entry: RefEntry,
    pub param_offset: u32,
    pub is_duplicate: bool,
    pub ref_offset: u32,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum HashKind {
    Key,
    Value,
}

struct FileData {
    hashes: IndexSet<(Hash40, HashKind)>,
    // map of ref-entries to their relative offset
    ref_entries: Vec<RefEntryWork>,
}

pub fn assemble<C>(cursor: &mut C, param: &ParamStruct) -> Result<(), Error>
where
    C: Write + Seek,
{
    // Duplicates are counted separately for names and values.
    // For example, trans and "trans" are counted separately in a flip.prc file.
    // <hash40 hash="name">trans</hash40>
    // <hash40 hash="trans">kyz</hash40>
    // TODO: Some value hashes can appear twice?
    let mut hashes = IndexSet::new();
    // hash table always starts with 0
    hashes.insert((Hash40(0), HashKind::Value));

    // iterate through all params twice, first time only for hashes.
    // this is required in order to assemble the tables 1 - 1.
    // we'll also use this to get the max number of ref entries we need.
    let mut ref_count = 0;
    iter_struct_hashes(&mut hashes, param, &mut ref_count);

    let mut fd = FileData {
        hashes,
        ref_entries: Vec::with_capacity(ref_count as usize),
    };

    // TODO: use with_capacity with some reasonable choice
    let mut param_cursor = Cursor::new(Vec::<u8>::new());
    write_param_struct(&mut param_cursor, &mut fd, param)?;

    let file_start = cursor.seek(SeekFrom::Current(0))?;
    cursor.write_all(MAGIC)?;

    let hash_size = 8 * fd.hashes.len() as u32;
    cursor.write_u32::<LittleEndian>(hash_size)?;
    cursor.seek(SeekFrom::Current(4))?;
    for (hash, _) in &fd.hashes {
        cursor.write_hash40::<LittleEndian>(*hash)?;
    }

    handle_ref_entries(&mut fd);
    write_ref_entries(cursor, &mut param_cursor, &fd)?;

    let param_pos = cursor.seek(SeekFrom::Current(0))?;
    let ref_size = (param_pos - (file_start + 0x10 + hash_size as u64)) as u32;
    // finish writing header
    cursor.seek(SeekFrom::Start(file_start + 0xc))?;
    cursor.write_u32::<LittleEndian>(ref_size)?;
    // write and consume the contents of the param writer
    cursor.seek(SeekFrom::Start(param_pos))?;
    param_cursor.set_position(0);
    cursor.write_all(&param_cursor.into_inner())?;

    Ok(())
}

fn iter_hashes(list: &mut IndexSet<(Hash40, HashKind)>, param: &ParamKind, count: &mut usize) {
    match param {
        ParamKind::Str(_) => {
            *count += 1;
        }
        ParamKind::Hash(val) => {
            list.insert((*val, HashKind::Value));
        }
        ParamKind::List(val) => {
            for p in &val.0 {
                iter_hashes(list, p, count);
            }
        }
        ParamKind::Struct(val) => {
            *count += 1;
            iter_struct_hashes(list, val, count);
        }
        _ => {}
    }
}

fn iter_struct_hashes(
    list: &mut IndexSet<(Hash40, HashKind)>,
    param_struct: &ParamStruct,
    count: &mut usize,
) {
    for (hash, p) in &param_struct.0 {
        list.insert((*hash, HashKind::Key));
        iter_hashes(list, p, count);
    }
}

fn write_param<C>(param_cursor: &mut C, fd: &mut FileData, param: &ParamKind) -> Result<(), Error>
where
    C: Write + Seek,
{
    match param {
        ParamKind::Bool(val) => {
            param_cursor.write_u8(1)?;
            param_cursor.write_u8(*val as u8)?;
            Ok(())
        }
        ParamKind::I8(val) => {
            param_cursor.write_u8(2)?;
            param_cursor.write_i8(*val)?;
            Ok(())
        }
        ParamKind::U8(val) => {
            param_cursor.write_u8(3)?;
            param_cursor.write_u8(*val)?;
            Ok(())
        }
        ParamKind::I16(val) => {
            param_cursor.write_u8(4)?;
            param_cursor.write_i16::<LittleEndian>(*val)?;
            Ok(())
        }
        ParamKind::U16(val) => {
            param_cursor.write_u8(5)?;
            param_cursor.write_u16::<LittleEndian>(*val)?;
            Ok(())
        }
        ParamKind::I32(val) => {
            param_cursor.write_u8(6)?;
            param_cursor.write_i32::<LittleEndian>(*val)?;
            Ok(())
        }
        ParamKind::U32(val) => {
            param_cursor.write_u8(7)?;
            param_cursor.write_u32::<LittleEndian>(*val)?;
            Ok(())
        }
        ParamKind::Float(val) => {
            param_cursor.write_u8(8)?;
            param_cursor.write_f32::<LittleEndian>(*val)?;
            Ok(())
        }
        ParamKind::Hash(val) => {
            param_cursor.write_u8(9)?;
            param_cursor.write_u32::<LittleEndian>(fd.hashes.get_index_of(&(*val, HashKind::Value)).unwrap() as u32)?;
            Ok(())
        }
        ParamKind::Str(val) => {
            param_cursor.write_u8(10)?;
            fd.ref_entries.push(RefEntryWork {
                ref_entry: RefEntry::RString(String::from(val)),
                param_offset: param_cursor.seek(SeekFrom::Current(0))? as u32,
                is_duplicate: false,
                ref_offset: 0,
            });
            param_cursor.write_u32::<LittleEndian>(0)?; // placeholder number
            Ok(())
        }
        ParamKind::List(val) => {
            let start_pos = param_cursor.seek(SeekFrom::Current(0))? as u32;

            param_cursor.write_u8(11)?;
            param_cursor.write_u32::<LittleEndian>(val.0.len() as u32)?;

            let mut table_pos = start_pos + 5;
            let mut param_pos = table_pos + (4 * val.0.len() as u32);
            for p in &val.0 {
                param_cursor.seek(SeekFrom::Start(table_pos as u64))?;
                param_cursor.write_u32::<LittleEndian>(param_pos - start_pos)?;
                table_pos += 4;

                param_cursor.seek(SeekFrom::Start(param_pos as u64))?;
                write_param(param_cursor, fd, p)?;
                param_pos = param_cursor.seek(SeekFrom::Current(0))? as u32;
            }
            Ok(())
        }
        ParamKind::Struct(val) => write_param_struct(param_cursor, fd, val),
    }
}

fn write_param_struct<C>(
    param_cursor: &mut C,
    fd: &mut FileData,
    param_struct: &ParamStruct,
) -> Result<(), Error>
where
    C: Write + Seek,
{
    let start_pos = param_cursor.seek(SeekFrom::Current(0))? as u32;

    param_cursor.write_u8(12)?;
    param_cursor.write_u32::<LittleEndian>(param_struct.0.len() as u32)?;
    param_cursor.write_u32::<LittleEndian>(0)?; // placeholder number

    // do I keep the separate pass for hashes or combine two loops into this func?
    let mut sorted = param_struct.0.iter().collect::<Vec<&_>>();
    sorted.sort_by_key(|p| p.0);

    // we don't know what our data will look like yet
    // but we reserve the space to keep it ordered
    let ref_index = fd.ref_entries.len();
    fd.ref_entries.push(RefEntryWork {
        ref_entry: RefEntry::RTable(Vec::with_capacity(param_struct.0.len())),
        param_offset: start_pos + 5,
        is_duplicate: false,
        ref_offset: 0,
    });

    for (hash, param) in sorted {
        // TODO: can we preserve the reference to t and iterate at the same time?
        if let RefEntry::RTable(ref mut t) = &mut fd.ref_entries[ref_index].ref_entry {
            t.push((
                fd.hashes.get_index_of(&(*hash, HashKind::Key)).unwrap() as u32,
                param_cursor.seek(SeekFrom::Current(0))? as u32 - start_pos,
            ));
        } else {
            unreachable!()
        }

        write_param(param_cursor, fd, param)?
    }
    Ok(())
}

fn handle_ref_entries(fd: &mut FileData) {
    let entries = &mut fd.ref_entries;
    let mut offset = 0u32;

    for i in 0..entries.len() {
        // test if the entry at i equals some previous entry at j
        let mut found_duplicate = false;
        for j in (0..i).rev() {
            if entries[j].ref_entry == entries[i].ref_entry {
                entries[i].is_duplicate = true;
                entries[i].ref_offset = entries[j].ref_offset;

                found_duplicate = true;
                break;
            }
        }
        if !found_duplicate {
            entries[i].ref_offset = offset;
            offset += match &entries[i].ref_entry {
                RefEntry::RString(s) => 1 + s.len() as u32, // 0-terminated
                RefEntry::RTable(t) => 8 * t.len() as u32,
            };
        }
    }
}

// TODO: comments and cleanup
fn write_ref_entries<C>(
    cursor: &mut C,
    param_cursor: &mut Cursor<Vec<u8>>,
    fd: &FileData,
) -> Result<(), Error>
where
    C: Write + Seek,
{
    let entries = &fd.ref_entries;

    for entry in entries {
        param_cursor.set_position(entry.param_offset as u64);
        param_cursor.write_u32::<LittleEndian>(entry.ref_offset)?;
        if !entry.is_duplicate {
            match &entry.ref_entry {
                RefEntry::RString(s) => {
                    cursor.write_all(s.as_bytes())?;
                    cursor.write_u8(0)?;
                }
                RefEntry::RTable(t) => {
                    for &(hash_ind, offset) in t {
                        cursor.write_u32::<LittleEndian>(hash_ind)?;
                        cursor.write_u32::<LittleEndian>(offset)?;
                    }
                }
            }
        }
    }
    Ok(())
}

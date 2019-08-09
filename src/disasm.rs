use crate::param;
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::BTreeMap;
use std::io;

struct Disassembler {
    ref_start: u32,
    param_start: u32,
    hash_table: Vec<u64>,
    //ref tables in the format <offset, Vec<(hash_index, param_offset)>>
    ref_table: BTreeMap<u32, Vec<(u32, u32)>>,
}

pub fn disassemble(cursor: &mut io::Cursor<Vec<u8>>) -> Result<param::ParamKind, String> {
    cursor.set_position(0);
    assert_eq!(param::MAGIC, cursor.read_u64::<LittleEndian>().unwrap());
    let hashsize = cursor.read_u32::<LittleEndian>().unwrap();
    let hashnum = (hashsize / 8) as usize;
    let refsize = cursor.read_u32::<LittleEndian>().unwrap();
    let mut d = Disassembler {
        ref_start: 0x10 + hashsize,
        param_start: 0x10 + hashsize + refsize,
        hash_table: Vec::with_capacity(hashnum),
        ref_table: BTreeMap::new()
    };
    for _ in 0..hashnum {
        d.hash_table
            .push(cursor.read_u64::<LittleEndian>().unwrap())
    }
    cursor.set_position(d.param_start as u64);
    let first_byte = cursor.read_u8().unwrap();
    if first_byte != 12 {
        return Err(String::from("param file does not contain a root"));
    }
    cursor.set_position(cursor.position() - 1);
    d.read_param(cursor)
}

impl Disassembler {
    fn read_param(&mut self, cursor: &mut io::Cursor<Vec<u8>>) -> Result<param::ParamKind, String> {
        match cursor.read_u8().unwrap() {
            1 => {
                let val = cursor.read_u8().unwrap();
                Ok(param::ParamKind::Bool(val != 0))
            }
            2 => {
                let val = cursor.read_i8().unwrap();
                Ok(param::ParamKind::I8(val))
            }
            3 => {
                let val = cursor.read_u8().unwrap();
                Ok(param::ParamKind::U8(val))
            }
            4 => {
                let val = cursor.read_i16::<LittleEndian>().unwrap();
                Ok(param::ParamKind::I16(val))
            }
            5 => {
                let val = cursor.read_u16::<LittleEndian>().unwrap();
                Ok(param::ParamKind::U16(val))
            }
            6 => {
                let val = cursor.read_i32::<LittleEndian>().unwrap();
                Ok(param::ParamKind::I32(val))
            }
            7 => {
                let val = cursor.read_u32::<LittleEndian>().unwrap();
                Ok(param::ParamKind::U32(val))
            }
            8 => {
                let val = cursor.read_f32::<LittleEndian>().unwrap();
                Ok(param::ParamKind::Float(val))
            }
            9 => {
                let val = self.hash_table[cursor.read_i32::<LittleEndian>().unwrap() as usize];
                Ok(param::ParamKind::Hash(val))
            }
            10 => {
                let strpos = cursor.read_u32::<LittleEndian>().unwrap();
                //remembering where we were is actually unnecessary
                //let curpos = cursor.position();
                cursor.set_position((self.ref_start + strpos) as u64);
                let mut val = String::new();
                let mut next: u8;
                loop {
                    next = cursor.read_u8().unwrap();
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
                let size = cursor.read_u32::<LittleEndian>().unwrap();

                let mut offsets: Vec<u32> = Vec::new();
                for _ in 0..size {
                    offsets.push(cursor.read_u32::<LittleEndian>().unwrap());
                }

                let mut params: Vec<param::ParamKind> = Vec::new();
                for offset in offsets {
                    cursor.set_position(pos + offset as u64);
                    params.push(self.read_param(cursor).unwrap());
                }
                Ok(param::ParamKind::List(params))
            }
            12 => {
                let pos = cursor.position() - 1;
                let size = cursor.read_u32::<LittleEndian>().unwrap() as usize;
                let refpos = cursor.read_u32::<LittleEndian>().unwrap();

                let t: &Vec<(u32, u32)>;
                match self.ref_table.get(&refpos) {
                    Some(x) => t = x,
                    None => {
                        let mut new_table: Vec<(u32, u32)> = Vec::with_capacity(size);
                        cursor.set_position((self.ref_start + refpos) as u64);
                        for _ in 0..size {
                            new_table.push((
                                cursor.read_u32::<LittleEndian>().unwrap(),
                                cursor.read_u32::<LittleEndian>().unwrap(),
                            ));
                        }
                        new_table.sort_by(|a, b| a.0.cmp(&b.0));
                        t = &new_table;
                        self.ref_table.insert(refpos, new_table);
                    }
                }

                let mut params: Vec<(u64, param::ParamKind)> = Vec::with_capacity(size);
                for &pair in t {
                    let hash = self.hash_table[pair.0 as usize];
                    cursor.set_position(pos + pair.1 as u64);
                    params.push((hash, self.read_param(cursor).unwrap()))
                }
                Ok(param::ParamKind::Struct(params))
            }
            _ => Err(format!(
                "encountered invalid param number at position: {}",
                cursor.position() - 1
            )),
        }
    }
}

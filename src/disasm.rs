use crate::param;
use std::io;
use byteorder::{LittleEndian,ReadBytesExt};

struct Disassembler {
    HashStart: u32,
    RefStart: u32,
    ParamStart: u32,
    HashTable: [u64],
    //ref table map, to reduce excessive reads from ref section
}

impl Disassembler {
    fn read_param(&self, mut cursor: io::Cursor<&[u8]>) -> Result<param::ParamKind, String> {
        match cursor.read_u8().unwrap() {
            1 => {
                let val = cursor.read_u8().unwrap();
                Ok(param::ParamKind::Bool(val != 0))
            },
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
                let val = self.HashTable[cursor.read_i32::<LittleEndian>().unwrap() as usize];
                Ok(param::ParamKind::Hash(val))
            }
            10 => {
                let strpos = cursor.read_u32::<LittleEndian>().unwrap();
                let curpos = cursor.position();
                cursor.set_position((self.RefStart + strpos) as u64);
                let mut val = String::new(); let mut next: u8;
                loop { next = cursor.read_u8().unwrap();
                    if next != 0 {
                        val.push(next as char);
                    } else { break; }
                }
                cursor.set_position(curpos);
                Ok(param::ParamKind::Str(val))
            }
            11 => Err(String::from("unimplemented param type: list")),
            12 => Err(String::from("unimplemented param type: struct")),
            _ => Err(format!("encountered invalid param number at position: {}", cursor.position() - 1))
        }
    }
}

pub fn disassemble(cursor: io::Cursor<&[u8]>) -> Result<param::ParamKind, String> {
    Err(String::from("unimplemented function"))
}
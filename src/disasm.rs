use crate::param;
use std::io;
use byteorder::{LittleEndian,ReadBytesExt};

struct Disassembler {
    HashStart: u32,
    RefStart: u32,
    ParamStart: u32,
    HashTable: Vec<u64>,
    //ref table map, to reduce excessive reads from ref section
}

pub fn disassemble(cursor: &mut io::Cursor<&[u8]>) -> Result<param::ParamKind, String> {
    cursor.set_position(0);
    assert_eq!(param::MAGIC, cursor.read_u64::<LittleEndian>().unwrap());
    let hashsize = cursor.read_u32::<LittleEndian>().unwrap();
    let hashnum = (hashsize / 8) as usize;
    let refsize = cursor.read_u32::<LittleEndian>().unwrap();
    let mut d = Disassembler {
        HashStart: 10,
        RefStart: 10 + hashsize,
        ParamStart: 10 + hashsize + refsize,
        HashTable: Vec::with_capacity(hashnum)
    };
    for _ in 1..hashnum {
        d.HashTable.push(cursor.read_u64::<LittleEndian>().unwrap())
    }
    cursor.set_position(d.ParamStart as u64);
    let first_byte = cursor.read_u8().unwrap();
    if first_byte != 12 {
        return Err(String::from("param file does not contain a root"));
    }
    cursor.set_position(cursor.position() - 1);
    d.read_param(cursor)
}

impl Disassembler {
    fn read_param(&self, cursor: &mut io::Cursor<&[u8]>) -> Result<param::ParamKind, String> {
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
                //remembering where we were is actually unnecessary
                //let curpos = cursor.position();
                cursor.set_position((self.RefStart + strpos) as u64);
                let mut val = String::new(); let mut next: u8;
                loop { next = cursor.read_u8().unwrap();
                    if next != 0 {
                        val.push(next as char);
                    } else { break; }
                }
                //cursor.set_position(curpos);
                Ok(param::ParamKind::Str(val))
            }
            11 => {
                let relpos = cursor.position() - 1;
                let size = cursor.read_u32::<LittleEndian>().unwrap();

                let mut offsets: Vec<u32> = Vec::new();
                for _ in 1..size { offsets.push(cursor.read_u32::<LittleEndian>().unwrap()); }
                
                let mut params: Vec<param::ParamKind> = Vec::new();
                for offset in offsets {
                    cursor.set_position(relpos + offset as u64);
                    params.push(self.read_param(cursor).unwrap());
                }
                Ok(param::ParamKind::List(params))
            }
            12 => Err(String::from("unimplemented param type: struct")),
            _ => Err(format!("encountered invalid param number at position: {}", cursor.position() - 1))
        }
    }
}
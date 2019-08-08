pub const MAGIC: &[u8; 8] = b"paracobn";

#[derive(Debug)]
pub enum ParamKind { //index starts at 1
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    Float(f32),
    Hash(u64),
    Str(String),
    List(Vec<ParamKind>),
    Struct(Vec<(u64, ParamKind)>)
}
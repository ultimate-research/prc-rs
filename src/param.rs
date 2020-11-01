use hash40::Hash40;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

pub const MAGIC: &[u8; 8] = b"paracobn"; //paracobn

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ParamKind {
    //index starts at 1
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    Float(f32),
    Hash(Hash40),
    Str(String),
    List(ParamList),
    Struct(ParamStruct),
}

pub type ParamList = Vec<ParamKind>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct ParamStruct(pub Vec<(Hash40, ParamKind)>);

impl ParamKind {
    pub fn unwrap_as_hashmap(&self) -> HashMap<Hash40, &ParamKind> {
        TryInto::<&ParamStruct>::try_into(self)
            .unwrap()
            .0
            .iter()
            .map(|(h, p)| (*h, p))
            .collect::<HashMap<_, _>>()
    }
}

use ParamKind::*;

macro_rules! impl_from_param {
    ($($param:ident ($t:ty)),*$(,)?) => {
        $(
            impl TryFrom<ParamKind> for $t {
                type Error = &'static str;

                fn try_from(param: ParamKind) -> Result<Self, Self::Error> {
                    if let $param(val) = param {
                        Ok(val)
                    } else {
                        Err("Tried to unwrap param into inconsistent type")
                    }
                }
            }

            impl<'a> TryFrom<&'a ParamKind> for &'a $t {
                type Error = &'static str;

                fn try_from(param: &'a ParamKind) -> Result<Self, Self::Error> {
                    if let $param(val) = param {
                        Ok(val)
                    } else {
                        Err("Tried to unwrap param into inconsistent type")
                    }
                }
            }

            impl<'a> TryFrom<&'a mut ParamKind> for &'a mut $t {
                type Error = &'static str;

                fn try_from(param: &'a mut ParamKind) -> Result<Self, Self::Error> {
                    if let $param(val) = param {
                        Ok(val)
                    } else {
                        Err("Tried to unwrap param into inconsistent type")
                    }
                }
            }
        )*
    }
}

impl_from_param! {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    Float(f32),
    Hash(Hash40),
    Str(String),
    List(ParamList),
    Struct(ParamStruct),
}

use hash40::Hash40;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

pub type ParamStruct = Vec<(Hash40, ParamKind)>;

pub trait FromParam: Sized {
    fn from_param(param: &ParamKind) -> Option<&Self>;

    fn from_param_owned(param: ParamKind) -> Option<Self>;
}

impl ParamKind {
    pub fn unwrap<T: FromParam>(&self) -> &T {
        <T>::from_param(self).unwrap()
    }

    pub fn unwrap_owned<T: FromParam>(self) -> T {
        <T>::from_param_owned(self).unwrap()
    }

    pub fn get<T: FromParam>(&self) -> Option<&T> {
        <T>::from_param(self)
    }

    pub fn get_owned<T: FromParam>(self) -> Option<T> {
        <T>::from_param_owned(self)
    }

    pub fn unwrap_as_hashmap(&self) -> Option<HashMap<Hash40, ParamKind>> {
        Some(
            self.get::<Vec<(Hash40, ParamKind)>>()?
                .clone()
                .into_iter()
                .collect(),
        )
    }
}

use ParamKind::*;

macro_rules! impl_from_param {
    ($($param:ident ($t:ty)),*$(,)?) => {
        $(
            impl FromParam for $t {
                fn from_param(param: &ParamKind) -> Option<&Self> {
                    if let $param(val) = param {
                        Some(val)
                    } else {
                        None
                    }
                }

                fn from_param_owned(param: ParamKind) -> Option<Self> {
                    if let $param(val) = param {
                        Some(val)
                    } else {
                        None
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

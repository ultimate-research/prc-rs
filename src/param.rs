use hash40::Hash40;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

pub const MAGIC: &[u8; 8] = b"paracobn"; //paracobn
const UNWRAP_ERR: &str = "Tried to unwrap param into inconsistent type";

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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct ParamList(pub Vec<ParamKind>);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct ParamStruct(pub Vec<(Hash40, ParamKind)>);

impl ParamKind {
    pub fn try_into_owned<T>(self) -> Result<T, T::Error>
    where
        T: TryFrom<ParamKind>,
    {
        self.try_into()
    }

    pub fn try_into_ref<'a, T>(
        &'a self,
    ) -> Result<&'a T, <&'a T as std::convert::TryFrom<&'a ParamKind>>::Error>
    where
        &'a T: TryFrom<&'a ParamKind>,
    {
        self.try_into()
    }

    pub fn try_into_mut<'a, T>(
        &'a mut self,
    ) -> Result<&'a mut T, <&'a mut T as TryFrom<&'a mut ParamKind>>::Error>
    where
        &'a mut T: TryFrom<&'a mut ParamKind>,
    {
        self.try_into()
    }

    pub fn unwrap_as_hashmap(self) -> HashMap<Hash40, ParamKind> {
        TryInto::<ParamStruct>::try_into(self)
            .unwrap()
            .0
            .drain(..)
            .collect::<HashMap<_, _>>()
    }

    pub fn unwrap_as_hashmap_ref(&self) -> HashMap<Hash40, &ParamKind> {
        TryInto::<&ParamStruct>::try_into(self)
            .unwrap()
            .0
            .iter()
            .map(|(h, p)| (*h, p))
            .collect::<HashMap<_, _>>()
    }

    pub fn unwrap_as_hashmap_mut(&mut self) -> HashMap<Hash40, &mut ParamKind> {
        TryInto::<&mut ParamStruct>::try_into(self)
            .unwrap()
            .0
            .iter_mut()
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
                        Err(UNWRAP_ERR)
                    }
                }
            }

            impl<'a> TryFrom<&'a ParamKind> for &'a $t {
                type Error = &'static str;

                fn try_from(param: &'a ParamKind) -> Result<Self, Self::Error> {
                    if let $param(val) = param {
                        Ok(val)
                    } else {
                        Err(UNWRAP_ERR)
                    }
                }
            }

            impl<'a> TryFrom<&'a mut ParamKind> for &'a mut $t {
                type Error = &'static str;

                fn try_from(param: &'a mut ParamKind) -> Result<Self, Self::Error> {
                    if let $param(val) = param {
                        Ok(val)
                    } else {
                        Err(UNWRAP_ERR)
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

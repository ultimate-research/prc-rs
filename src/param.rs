use hash40::Hash40;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

#[doc(hidden)]
pub const MAGIC: &[u8; 8] = b"paracobn";
const UNWRAP_ERR: &str = "Tried to unwrap param into inconsistent type";

/// The central data structure to param files and params.
/// Similar to tree-like recursive data formats such as JSON.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ParamKind {
    // index starts at 1
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

/// A list of params.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct ParamList(pub Vec<ParamKind>);

/// A list of key-value pairs of params.
/// Acts essentially like a hash-map, but is presented in list form to preserve key order, as well as to handle rare cases where a key may be duplicated.
/// Keys are hashed strings, represented by the [Hash40] type.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct ParamStruct(pub Vec<(Hash40, ParamKind)>);

impl ParamKind {
    /// Attempts to convert an owned param into the contained value.
    /// Returns an error if the contained value is not the expected type.
    pub fn try_into_owned<T>(self) -> Result<T, T::Error>
    where
        T: TryFrom<ParamKind>,
    {
        self.try_into()
    }

    /// Attempts to convert a param by reference into a reference of the contained value.
    /// Returns an error if the contained value is not the expected type.
    pub fn try_into_ref<'a, T>(
        &'a self,
    ) -> Result<&'a T, <&'a T as std::convert::TryFrom<&'a ParamKind>>::Error>
    where
        &'a T: TryFrom<&'a ParamKind>,
    {
        self.try_into()
    }

    /// Attempts to convert a param by mutable reference into a mutable reference of the contained value.
    /// Returns an error if the contained value is not the expected type.
    pub fn try_into_mut<'a, T>(
        &'a mut self,
    ) -> Result<&'a mut T, <&'a mut T as TryFrom<&'a mut ParamKind>>::Error>
    where
        &'a mut T: TryFrom<&'a mut ParamKind>,
    {
        self.try_into()
    }

    /// Converts an owned param into a [HashMap], indexing into the contained params.
    /// Panics if the param was not a [ParamKind::Struct].
    pub fn unwrap_as_hashmap(self) -> HashMap<Hash40, ParamKind> {
        TryInto::<ParamStruct>::try_into(self)
            .unwrap()
            .0
            .drain(..)
            .collect::<HashMap<_, _>>()
    }

    /// Converts a reference to a param into a [HashMap], indexing into references to the contained params.
    /// Panics if the param was not a [ParamKind::Struct].
    pub fn unwrap_as_hashmap_ref(&self) -> HashMap<Hash40, &ParamKind> {
        TryInto::<&ParamStruct>::try_into(self)
            .unwrap()
            .0
            .iter()
            .map(|(h, p)| (*h, p))
            .collect::<HashMap<_, _>>()
    }

    /// Converts a mutable reference to a param into a [HashMap], indexing into mutable references to the contained params.
    /// Panics if the param was not a [ParamKind::Struct].
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

            impl From<$t> for ParamKind {
                fn from(v: $t) -> ParamKind {
                    ParamKind::$param(v)
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

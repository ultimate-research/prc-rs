use diff::{Diff, VecDiff, VecDiffType};
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

// TODO: support custom diff implementations for lists using sub-params as keys
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Diff)]
#[serde(transparent)]
#[diff(attr(
    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
))]
pub struct ParamList(pub Vec<ParamKind>);

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
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

// DIFF IMPLEMENTATION
macro_rules! generate_param_diff {
    ($($param:ident ($t:ty)),*$(,)?) => {
        #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
        pub enum ParamDiff {
            $($param(<$t as Diff>::Repr)),*
        }

        impl Diff for ParamKind {
            type Repr = ParamDiff;
        
            fn diff(&self, other: &Self) -> Self::Repr {
                match (self, other) {
                    $(
                        (ParamKind::$param(a), ParamKind::$param(b)) => {
                            ParamDiff::$param(a.diff(b))
                        }
                        (_, ParamKind::$param(b)) => {
                            ParamDiff::$param(<$t as Diff>::identity().diff(b))
                        }
                    )*
                }
            }
        
            fn apply(&mut self, _diff: &Self::Repr) {
                todo!()
            }
        
            fn identity() -> Self {
                todo!()
            }
        }
    }
}

generate_param_diff!(
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
);

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ParamStructDiff(pub Vec<(Hash40, VecDiff<ParamKind>)>);

// in retrospect, accounting for duplicate keys causes a large overhead
// despite their presence only in specific game files

/// Returns the number of identical keys, and the key itself, in the sorted vec starting at `index`.
/// Returns `None` if the index is out of bounds
fn get_num_keys(v: &Vec<(Hash40, &ParamKind)>, ind: usize) -> Option<(usize, Hash40)> {
    if ind >= v.len() {
        None
    } else {
        let key = v[ind].0;
        let len = v[ind..].iter()
            .position(|&(h, _)| h != key)
            .unwrap_or(v.len() - ind);
        Some((len, key))
    }
}

impl Diff for ParamStruct {
    type Repr = ParamStructDiff;

    fn diff(&self, other: &Self) -> Self::Repr {
        let mut diff = ParamStructDiff::default();
        let mut a = self.0.iter().map(|(h, p)| (*h, p)).collect::<Vec<_>>();
        let mut b = other.0.iter().map(|(h, p)| (*h, p)).collect::<Vec<_>>();
        a.sort_by_key(|e| e.0);
        b.sort_by_key(|e| e.0);
        // iterate through both lists independently to match up elements
        let mut i = 0;
        let mut j = 0;

        loop {
            let mut keys_a = get_num_keys(&a, i);
            let mut keys_b = get_num_keys(&b, j);

            // if the two keys don't match, only one should advance
            // because the lower key must have been inserted or removed
            // depending on which side you are talking about
            if keys_a.is_some() && keys_b.is_some() {
                let a = keys_a.unwrap();
                let b = keys_b.unwrap();
                if a.1 < b.1 {
                    keys_b = None;
                } else if b.1 < a.1 {
                    keys_a = None;
                }
            }
            
            // advance the appropriate cursors and insert into the diff
            match (keys_a, keys_b) {
                // reached the end of both lists
                (None, None) => break,
                (Some(keys_a), None) => {
                    diff.0.push((
                        keys_a.1,
                        VecDiff(vec![VecDiffType::Removed {
                            len: keys_a.0,
                            index: i,
                        }]),
                    ));
                    i += keys_a.0;
                }
                (None, Some(keys_b)) => {
                    diff.0.push((
                        keys_b.1,
                        VecDiff(vec![VecDiffType::Inserted {
                            changes: b[j..j + keys_b.0]
                                .iter()
                                .map(|&(_, p)| ParamKind::identity().diff(&p))
                                .collect(),
                            index: i,
                        }]),
                    ));
                    j += keys_b.0;
                }
                (Some(keys_a), Some(keys_b)) => {
                    // copy both slices into Vecs and diff them
                    // TODO: use slices instead
                    let vec_a: Vec<ParamKind> = a[i..i + keys_a.0].iter().map(|&(_, p)| p.clone()).collect();
                    let vec_b: Vec<ParamKind> = b[j..j + keys_b.0].iter().map(|&(_, p)| p.clone()).collect();
                    let key_diff = vec_a.diff(&vec_b).0;
                    if !key_diff.is_empty() {
                        diff.0.push((
                            // keys must match at this point so doesn't matter which
                            keys_a.1,
                            VecDiff(key_diff),
                        ));
                    }
                    i += keys_a.0;
                    j += keys_b.0;
                }
            }
        }
        diff
    }

    fn apply(&mut self, _diff: &Self::Repr) {
        todo!()
    }

    fn identity() -> Self {
        ParamStruct(vec![])
    }
}

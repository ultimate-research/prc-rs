use std::io::{Cursor, Read, Result, Seek};

use hash40::{hash40, Hash40};

use crate::from_stream::{prepare, FromStream, Offsets, StructData};

static FIGHTER_PIKACHU_VL: &[u8] = include_bytes!("vl.prc");

#[derive(Debug)]
struct FighterPikachuVl {
    map_coll_data: Vec<MapColl>,
    jostle_map_coll_data: Vec<MapColl>,
    hit_data: Vec<HitData>,
}

#[derive(Debug)]
struct MapColl {
    node: Hash40,
    offset_x: f32,
    offset_y: f32,
    offset_z: f32,
}

#[derive(Debug)]
struct HitData {
    offset1_x: f32,
    offset1_y: f32,
    offset1_z: f32,
    offset2_x: f32,
    offset2_y: f32,
    offset2_z: f32,
    size: f32,
    node_id: Hash40,
    part: Hash40,
    height: Hash40,
    status: Hash40,
    check_type: Hash40,
}

macro_rules! derive_struct_read {
    ($struct_name:ident, $(($name:ident, $hash:expr)),*) => {
        impl FromStream for $struct_name {
            fn read_param<R: Read + Seek>(reader: &mut R, offsets: Offsets) -> Result<Self> {
                let data = StructData::from_stream(reader)?;
                Ok(Self {
                    $(
                        $name: data
                            .search_child(reader, $hash, offsets)
                            .and_then(|_| FromStream::read_param(reader, offsets))?,
                    )*
                })
            }
        }
    };
}

derive_struct_read!(
    FighterPikachuVl,
    (map_coll_data, hash40("map_coll_data")),
    (jostle_map_coll_data, hash40("jostle_map_coll_data")),
    (hit_data, hash40("hit_data"))
);

derive_struct_read!(
    MapColl,
    (node, hash40("node")),
    (offset_x, hash40("offset_x")),
    (offset_y, hash40("offset_y")),
    (offset_z, hash40("offset_z"))
);

derive_struct_read!(
    HitData,
    (offset1_x, hash40("offset1_x")),
    (offset1_y, hash40("offset1_y")),
    (offset1_z, hash40("offset1_z")),
    (offset2_x, hash40("offset2_x")),
    (offset2_y, hash40("offset2_y")),
    (offset2_z, hash40("offset2_z")),
    (size, hash40("size")),
    (node_id, hash40("node_id")),
    (part, hash40("part")),
    (height, hash40("height")),
    (status, hash40("status")),
    (check_type, hash40("check_type"))
);

#[test]
fn test_param_read_from_struct_def() {
    let mut reader = Cursor::new(FIGHTER_PIKACHU_VL);
    let offsets = prepare(&mut reader).unwrap();

    FighterPikachuVl::read_param(&mut reader, offsets).unwrap();
}

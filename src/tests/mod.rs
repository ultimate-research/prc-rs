use crate::prc_trait::{ErrorKind, ErrorPathPart, ParamNumber};
use crate::{write_stream, ParamKind, ParamStruct, Prc};

use std::io::Cursor;

use hash40::{hash40, Hash40};

static FIGHTER_PIKACHU_VL: &[u8] = include_bytes!("vl.prc");

#[derive(Debug, Default, PartialEq, Prc)]
#[prc(path = crate)]
struct FighterPikachuVl {
    map_coll_data: Vec<MapColl>,
    hit_target: Vec<i32>,
    // you can use a name you prefer, if you target the real name
    #[prc(name = "cliff_hang_data")]
    ledge_grab_data: Vec<LedgeGrabBox>,
}

#[derive(Debug, PartialEq, Prc)]
#[prc(path = crate)]
struct MapColl {
    // you can also target the hash of a param without knowing the name
    #[prc(hash = 0x04857fe845)]
    unk: Hash40,
    offset_x: f32,
    offset_y: f32,
    offset_z: f32,
}

#[derive(Debug, PartialEq, Prc)]
#[prc(path = crate)]
struct LedgeGrabBox {
    p1_x: f32,
    p1_y: f32,
    p2_x: f32,
    p2_y: f32,
}

#[test]
fn test_param_read_from_struct_def() {
    let mut reader = Cursor::new(FIGHTER_PIKACHU_VL);
    let pikachu_vl = FighterPikachuVl::read_file(&mut reader).unwrap();
    assert_eq!(
        pikachu_vl,
        FighterPikachuVl {
            map_coll_data: vec![
                MapColl {
                    unk: hash40("head"),
                    offset_x: 1.5,
                    offset_y: 0.0,
                    offset_z: 0.0
                },
                MapColl {
                    unk: hash40("shoulderr"),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    offset_z: 0.0
                },
                MapColl {
                    unk: hash40("shoulderl"),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    offset_z: 0.0
                },
                MapColl {
                    unk: hash40("footr"),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    offset_z: 0.0
                },
                MapColl {
                    unk: hash40("footl"),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    offset_z: 0.0
                },
                MapColl {
                    unk: hash40("hip"),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    offset_z: 0.0
                }
            ],
            hit_target: vec![1, 0, 6],
            ledge_grab_data: vec![LedgeGrabBox {
                p1_x: 16.0,
                p1_y: 21.0,
                p2_x: -11.0,
                p2_y: 5.0
            }]
        }
    );
}

#[derive(Debug, Default, PartialEq, Prc)]
#[prc(path = crate)]
struct FighterPikachuVlTestError1 {
    hit_target: Vec<i16>,
}

#[test]
fn test_derive_wrong_param_type_error() {
    let mut reader = Cursor::new(FIGHTER_PIKACHU_VL);
    let vl = FighterPikachuVlTestError1::read_file(&mut reader).unwrap_err();
    // I would use assert_eq! on the vl.kind, but std::io::Error doesn't implement
    // PartialEq, which limits me as well.
    match &vl.kind {
        ErrorKind::WrongParamNumber { expected, received } => {
            assert_eq!(*expected, ParamNumber::I16);
            assert_eq!(*received, ParamNumber::I32 as u8);
        }
        _ => panic!("Wrong error encountered"),
    }
    assert_eq!(vl.position.unwrap(), 0xd0b);
    let expected = vec![
        ErrorPathPart::Hash(hash40("hit_target")),
        ErrorPathPart::Index(0),
    ];
    // comparing vec's
    assert_eq!(vl.path.len(), expected.len());
    assert!(vl
        .path
        .iter()
        .enumerate()
        .all(|(i, path)| path == &expected[i]));
}

#[derive(Debug, Default, PartialEq, Prc)]
#[prc(path = crate)]
struct FighterPikachuVlTestError2 {
    cliff_hang_data: Vec<LedgeGrabBoxtestError2>,
}

#[derive(Debug, PartialEq, Prc)]
#[prc(path = crate)]
struct LedgeGrabBoxtestError2 {
    p1_x: f32,
    p1_y: f32,
    p2_x: f32,
    fake_name: f32,
}

#[test]
fn test_derive_param_not_found_error() {
    let mut reader = Cursor::new(FIGHTER_PIKACHU_VL);
    let vl = FighterPikachuVlTestError2::read_file(&mut reader).unwrap_err();
    // I would use assert_eq! on the vl.kind, but std::io::Error doesn't implement
    // PartialEq, which removes the easy choice.
    match &vl.kind {
        ErrorKind::ParamNotFound(hash) => {
            assert_eq!(*hash, hash40("fake_name"));
        }
        _ => panic!("Wrong error encountered"),
    }
    assert_eq!(vl.position.unwrap(), 0x1071);
    let expected = vec![
        ErrorPathPart::Hash(hash40("cliff_hang_data")),
        ErrorPathPart::Index(0),
    ];
    // comparing vec's
    assert_eq!(vl.path.len(), expected.len());
    assert!(vl
        .path
        .iter()
        .enumerate()
        .all(|(i, path)| path == &expected[i]));
}

#[derive(Debug, Prc, PartialEq, Eq)]
#[prc(path = crate)]
struct OptionalTestStruct {
    required_field: u8,
    optional_field: Option<bool>,
}

#[test]
fn test_optional_param() {
    let param_present = ParamStruct(vec![
        (hash40("required_field"), ParamKind::U8(0)),
        (hash40("optional_field"), ParamKind::Bool(true)),
    ]);
    let param_missing = ParamStruct(vec![
        (hash40("required_field"), ParamKind::U8(1))
    ]);

    let mut file_present = Cursor::new(vec![]);
    write_stream(&mut file_present, &param_present).unwrap();
    file_present.set_position(0);

    let mut file_missing = Cursor::new(vec![]);
    write_stream(&mut file_missing, &param_missing).unwrap();
    file_missing.set_position(0);

    let data_present = OptionalTestStruct::read_file(&mut file_present).unwrap();
    let data_missing = OptionalTestStruct::read_file(&mut file_missing).unwrap();

    assert_eq!(data_present.required_field, 0);
    assert_eq!(data_present.optional_field, Some(true));

    assert_eq!(data_missing.required_field, 1);
    assert_eq!(data_missing.optional_field, None);
}

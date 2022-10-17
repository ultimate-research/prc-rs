use crate::Prc;

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

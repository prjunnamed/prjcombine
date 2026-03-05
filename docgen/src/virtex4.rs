use std::collections::HashSet;

use indexmap::IndexMap;
use prjcombine_virtex4::defs::devdata;

use crate::{
    DocgenContext,
    bsdata::{
        FrameDirection, TileOrientation, check_devdata, check_misc_data, gen_bstiles,
        gen_devdata_table,
    },
    interconnect::{gen_devdata, gen_intdb},
};

pub fn gen_virtex4(ctx: &mut DocgenContext) {
    let reg_orientation = TileOrientation {
        frame_direction: FrameDirection::Vertical,
        flip_frame: false,
        flip_bit: false,
    };
    let tile_orientation = TileOrientation {
        frame_direction: FrameDirection::Vertical,
        flip_frame: false,
        flip_bit: true,
    };
    let gtz_orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: true,
    };
    let orientation = |tname: &str| {
        if tname.starts_with("REG.") {
            reg_orientation
        } else if tname == "GTZ" {
            gtz_orientation
        } else {
            tile_orientation
        }
    };

    for kind in ["virtex4", "virtex5", "virtex6", "virtex7"] {
        let db = prjcombine_virtex4::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        let part_names = Vec::from_iter(db.devices.iter().map(|part| part.name.as_str()));
        gen_intdb(ctx, kind, &db.int);
        let mut devdata = IndexMap::new();
        for device in &db.devices {
            devdata.insert(device.name.as_str(), &device.data);
        }

        gen_bstiles(ctx, kind, &db.bsdata, orientation);
        let misc_used = HashSet::new();
        let mut devdata_used = HashSet::new();
        match kind {
            "virtex4" => {}
            "virtex5" => {
                gen_devdata(
                    ctx,
                    "virtex5",
                    &db.int,
                    "iodelay-default",
                    &devdata,
                    &[devdata::IODELAY_V5_IDELAY_DEFAULT],
                );
                gen_devdata(
                    ctx,
                    "virtex5",
                    &db.int,
                    "ppc-clock-delay",
                    &devdata,
                    &[devdata::PPC440_CLOCK_DELAY],
                );
                gen_devdata(
                    ctx,
                    "virtex5",
                    &db.int,
                    "pll-in-dly-set",
                    &devdata,
                    &[devdata::PLL_V5_IN_DLY_SET],
                );
            }
            "virtex6" => {
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "virtex6",
                    "iodelay-default",
                    &["IODELAY:DEFAULT_IDELAY_VALUE"],
                );
                gen_devdata(
                    ctx,
                    "virtex6",
                    &db.int,
                    "pll-in-dly-set",
                    &devdata,
                    &[devdata::PLL_V6_IN_DLY_SET],
                );
            }
            "virtex7" => {}
            _ => unreachable!(),
        }
        check_misc_data(&db.bsdata, kind, &misc_used);
        check_devdata(&db.bsdata, kind, &devdata_used);
    }
}

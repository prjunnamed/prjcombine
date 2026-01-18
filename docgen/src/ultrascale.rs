use std::collections::HashSet;

use crate::{
    DocgenContext,
    bsdata::{FrameDirection, TileOrientation, check_devdata, check_misc_data, gen_bstiles},
    interconnect::gen_intdb,
};

pub fn gen_ultrascale(ctx: &mut DocgenContext) {
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
    let orientation = |tname: &str| {
        if tname.starts_with("REG.") {
            reg_orientation
        } else {
            tile_orientation
        }
    };

    for kind in ["ultrascale", "ultrascaleplus"] {
        let db = prjcombine_ultrascale::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        gen_intdb(ctx, kind, &db.int);
        gen_bstiles(ctx, kind, &db.bsdata, orientation);
        let misc_used = HashSet::new();
        let devdata_used = HashSet::new();
        check_misc_data(&db.bsdata, kind, &misc_used);
        check_devdata(&db.bsdata, kind, &devdata_used);
    }
}

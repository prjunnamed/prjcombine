use std::collections::HashSet;

use crate::{
    DocgenContext,
    bsdata::{FrameDirection, TileOrientation, check_misc_data, gen_bstiles},
    interconnect::gen_intdb,
};

pub fn gen_ecp(ctx: &mut DocgenContext) {
    let tile_orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };
    for kind in [
        "ecp", "xp", "machxo", "ecp2", "ecp2m", "xp2", "ecp3", "machxo2", "ecp4", "ecp5",
    ] {
        let db = prjcombine_ecp::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        gen_intdb(ctx, kind, &db.int);
        gen_bstiles(ctx, kind, &db.bsdata, |_| tile_orientation);
        let misc_used = HashSet::new();
        check_misc_data(&db.bsdata, kind, &misc_used);
    }
}

use std::collections::HashSet;

use crate::DocgenContext;

use crate::tiledb::{FrameDirection, TileOrientation, check_devdata, check_misc_data, gen_tiles};

pub fn gen_xc2000(ctx: &mut DocgenContext) {
    let tile_orientation = TileOrientation {
        frame_direction: FrameDirection::Vertical,
        flip_frame: true,
        flip_bit: true,
    };
    for kind in [
        "xc2000",
        "xc3000",
        "xc3000a",
        "xc4000",
        "xc4000a",
        "xc4000h",
        "xc4000e",
        "xc4000ex",
        "xc4000xla",
        "xc4000xv",
        "spartanxl",
        "xc5200",
    ] {
        let db = prjcombine_xc2000::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        gen_tiles(ctx, kind, &db.bsdata, |_| tile_orientation);
        let misc_used = HashSet::new();
        let devdata_used = HashSet::new();
        check_misc_data(&db.bsdata, kind, &misc_used);
        check_devdata(&db.bsdata, kind, &devdata_used);
    }
}

use std::collections::HashSet;

use crate::{
    DocgenContext,
    tiledb::{FrameDirection, TileOrientation, check_misc_data, gen_misc_table, gen_tiles},
};

pub fn gen_siliconblue(ctx: &mut DocgenContext) {
    let tile_orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };
    for kind in [
        "ice65l04", "ice65p04", "ice65l08", "ice65l01", "ice40p01", "ice40p03", "ice40p08",
        "ice40r04", "ice40t04", "ice40t05", "ice40t01",
    ] {
        let db = prjcombine_siliconblue::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        gen_tiles(ctx, kind, &db.tiles, |_| tile_orientation);
        let mut misc_used = HashSet::new();
        if matches!(kind, "ice65l04" | "ice65p04" | "ice65l08") {
            gen_misc_table(
                ctx,
                &db.tiles,
                &mut misc_used,
                kind,
                "iostd-drive",
                &["IOSTD:DRIVE"],
            );
            gen_misc_table(
                ctx,
                &db.tiles,
                &mut misc_used,
                kind,
                "iostd-misc",
                &["IOSTD:IOSTD_MISC"],
            );
        }
        check_misc_data(&db.tiles, kind, &misc_used);
    }
}

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
    for (kind, dbname) in [
        ("ice65l04", "iCE65L04"),
        ("ice65p04", "iCE65P04"),
        ("ice65l08", "iCE65L08"),
        ("ice65l01", "iCE65L01"),
        ("ice40p01", "iCE40LP1K"),
        ("ice40p03", "iCE40LP384"),
        ("ice40p08", "iCE40LP8K"),
        ("ice40r04", "iCE40LM4K"),
        ("ice40t04", "iCE5LP4K"),
        ("ice40t05", "iCE40UP5K"),
        ("ice40t01", "iCE40UL1K"),
    ] {
        let db = prjcombine_siliconblue::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{dbname}.zstd")),
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

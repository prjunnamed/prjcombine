use crate::{
    tiledb::{gen_tiles, FrameDirection, TileOrientation}, DocgenContext
};

pub fn gen_siliconblue(ctx: &mut DocgenContext) {
    let tile_orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };
    for (kind, dbname) in [
        ("ice65l01", "iCE65L01"),
        ("ice40p01", "iCE40LP1K"),
        ("ice40p03", "iCE40LP384"),
        ("ice40p08", "iCE40LP8K"),
        ("ice40t04", "iCE5LP4K"),
        ("ice40t01", "iCE40UL1K"),
        ("ice40t05", "iCE40UP5K"),
    ] {
        let db = prjcombine_siliconblue::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{dbname}.zstd")),
        )
        .unwrap();
        gen_tiles(ctx, kind, &db.tiles, |_| tile_orientation);
    }
}

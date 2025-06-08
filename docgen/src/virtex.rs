use std::collections::HashSet;

use crate::{
    bsdata::{
        check_devdata, check_misc_data, gen_bstiles, gen_misc_table, FrameDirection, TileOrientation
    }, interconnect::gen_intdb, DocgenContext
};

pub fn gen_virtex(ctx: &mut DocgenContext) {
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
    {
        let db = prjcombine_virtex::db::Database::from_file(
            ctx.ctx.root.join("../databases/virtex.zstd"),
        )
        .unwrap();
        gen_intdb(ctx, "virtex", &db.int);
        gen_bstiles(ctx, "virtex", &db.bsdata, orientation);
        let mut misc_used = HashSet::new();
        let devdata_used = HashSet::new();
        gen_misc_table(
            ctx,
            &db.bsdata,
            &mut misc_used,
            "virtex",
            "iostd-misc",
            &["IOSTD:V:IOSTD_MISC", "IOSTD:V:OUTPUT_MISC"],
        );
        gen_misc_table(
            ctx,
            &db.bsdata,
            &mut misc_used,
            "virtex",
            "iostd-drive",
            &["IOSTD:V:PDRIVE", "IOSTD:V:NDRIVE"],
        );
        gen_misc_table(
            ctx,
            &db.bsdata,
            &mut misc_used,
            "virtex",
            "iostd-slew",
            &["IOSTD:V:SLEW"],
        );
        gen_misc_table(
            ctx,
            &db.bsdata,
            &mut misc_used,
            "virtexe",
            "iostd-misc",
            &["IOSTD:VE:IOSTD_MISC", "IOSTD:VE:OUTPUT_MISC"],
        );
        gen_misc_table(
            ctx,
            &db.bsdata,
            &mut misc_used,
            "virtexe",
            "iostd-drive",
            &["IOSTD:VE:PDRIVE", "IOSTD:VE:NDRIVE"],
        );
        gen_misc_table(
            ctx,
            &db.bsdata,
            &mut misc_used,
            "virtexe",
            "iostd-slew",
            &["IOSTD:VE:SLEW"],
        );
        check_misc_data(&db.bsdata, "virtex", &misc_used);
        check_devdata(&db.bsdata, "virtex", &devdata_used);
    }
}

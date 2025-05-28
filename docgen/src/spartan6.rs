use std::collections::HashSet;

use crate::{
    DocgenContext,
    tiledb::{
        FrameDirection, TileOrientation, check_devdata, check_misc_data, gen_devdata_table,
        gen_misc_table, gen_tiles,
    },
};

pub fn gen_spartan6(ctx: &mut DocgenContext) {
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
    let db = prjcombine_spartan6::db::Database::from_file(
        ctx.ctx.root.join("../databases/spartan6.zstd"),
    )
    .unwrap();
    let part_names = Vec::from_iter(db.parts.iter().map(|part| part.name.as_str()));
    gen_tiles(ctx, "spartan6", &db.bsdata, orientation);
    let mut misc_used = HashSet::new();
    let mut devdata_used = HashSet::new();
    gen_misc_table(
        ctx,
        &db.bsdata,
        &mut misc_used,
        "spartan6",
        "iostd-drive",
        &["IOSTD:PDRIVE", "IOSTD:NDRIVE"],
    );
    gen_misc_table(
        ctx,
        &db.bsdata,
        &mut misc_used,
        "spartan6",
        "iostd-term",
        &["IOSTD:PTERM", "IOSTD:NTERM"],
    );
    gen_misc_table(
        ctx,
        &db.bsdata,
        &mut misc_used,
        "spartan6",
        "iostd-slew",
        &["IOSTD:PSLEW", "IOSTD:NSLEW"],
    );
    gen_misc_table(
        ctx,
        &db.bsdata,
        &mut misc_used,
        "spartan6",
        "iostd-lvdsbias",
        &["IOSTD:LVDSBIAS"],
    );

    gen_misc_table(
        ctx,
        &db.bsdata,
        &mut misc_used,
        "spartan6",
        "pll-lock",
        &[
            "PLL:PLL_LOCK_REF_DLY",
            "PLL:PLL_LOCK_FB_DLY",
            "PLL:PLL_LOCK_CNT",
            "PLL:PLL_LOCK_SAT_HIGH",
            "PLL:PLL_UNLOCK_CNT",
        ],
    );
    gen_misc_table(
        ctx,
        &db.bsdata,
        &mut misc_used,
        "spartan6",
        "pll-filter",
        &[
            "PLL:PLL_CP",
            "PLL:PLL_CP_REPL",
            "PLL:PLL_RES",
            "PLL:PLL_LFHF",
        ],
    );
    gen_devdata_table(
        ctx,
        &db.bsdata,
        &part_names,
        &mut devdata_used,
        "spartan6",
        "pci-ce-delay",
        &["PCI_CE_DELAY"],
    );
    check_misc_data(&db.bsdata, "spartan6", &misc_used);
    check_devdata(&db.bsdata, "spartan6", &devdata_used);
}

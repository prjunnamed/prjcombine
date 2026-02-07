use std::collections::HashSet;

use indexmap::IndexMap;
use prjcombine_spartan6::defs::devdata;

use crate::{
    DocgenContext,
    bsdata::{
        FrameDirection, TileOrientation, check_devdata, check_misc_data, gen_bstiles,
        gen_misc_table,
    },
    interconnect::{gen_devdata, gen_intdb},
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
    gen_intdb(ctx, "spartan6", &db.int);
    let mut devdata = IndexMap::new();
    for device in &db.devices {
        devdata.insert(device.name.as_str(), &device.data);
    }

    gen_bstiles(ctx, "spartan6", &db.bsdata, orientation);
    let mut misc_used = HashSet::new();
    let devdata_used = HashSet::new();
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

    gen_devdata(
        ctx,
        "spartan6",
        &db.int,
        "pci-ce-delay",
        &devdata,
        &[devdata::PCILOGICSE_PCI_CE_DELAY],
    );
    check_misc_data(&db.bsdata, "spartan6", &misc_used);
    check_devdata(&db.bsdata, "spartan6", &devdata_used);
}

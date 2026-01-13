use std::collections::HashSet;

use crate::{
    DocgenContext,
    bsdata::{
        FrameDirection, TileOrientation, check_devdata, check_misc_data, gen_bstiles,
        gen_devdata_table, gen_misc_table,
    },
    interconnect::gen_intdb,
};

pub fn gen_virtex2(ctx: &mut DocgenContext) {
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
    for kind in ["virtex2", "spartan3"] {
        let db = prjcombine_virtex2::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        let part_names = Vec::from_iter(db.devices.iter().map(|part| part.name.as_str()));
        gen_intdb(ctx, kind, &db.int);
        gen_bstiles(ctx, kind, &db.bsdata, orientation);
        let mut misc_used = HashSet::new();
        let mut devdata_used = HashSet::new();
        match kind {
            "virtex2" => {
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2",
                    "iostd-drive",
                    &["IOSTD:V2:PDRIVE", "IOSTD:V2:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2",
                    "iostd-slew",
                    &["IOSTD:V2:SLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2",
                    "iostd-output-misc",
                    &["IOSTD:V2:OUTPUT_MISC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2",
                    "iostd-output-diff",
                    &["IOSTD:V2:OUTPUT_DIFF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2",
                    "iostd-lvdsbias",
                    &["IOSTD:V2:LVDSBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2",
                    "iostd-dci-term-split",
                    &["IOSTD:V2:PMASK_TERM_SPLIT", "IOSTD:V2:NMASK_TERM_SPLIT"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2",
                    "iostd-dci-term-vcc",
                    &["IOSTD:V2:PMASK_TERM_VCC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2p",
                    "iostd-drive",
                    &["IOSTD:V2P:PDRIVE", "IOSTD:V2P:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2p",
                    "iostd-slew",
                    &["IOSTD:V2P:SLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2p",
                    "iostd-output-misc",
                    &["IOSTD:V2P:OUTPUT_MISC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2p",
                    "iostd-output-diff",
                    &["IOSTD:V2P:OUTPUT_DIFF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2p",
                    "iostd-lvdsbias",
                    &["IOSTD:V2P:LVDSBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2p",
                    "iostd-dci-term-split",
                    &["IOSTD:V2P:PMASK_TERM_SPLIT", "IOSTD:V2P:NMASK_TERM_SPLIT"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2p",
                    "iostd-dci-term-vcc",
                    &["IOSTD:V2P:PMASK_TERM_VCC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex2",
                    "gt10-PMA_SPEED",
                    &["GT10:PMA_SPEED"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "virtex2",
                    "dcm-data",
                    &["DCM:DESKEW_ADJUST", "DCM:VBG_PD", "DCM:VBG_SEL"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "virtex2",
                    "bs-data",
                    &["DOUBLE_GRESTORE", "FREEZE_DCI_NOPS"],
                );
            }
            "spartan3" => {
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3",
                    "iostd-drive",
                    &["IOSTD:S3:PDRIVE", "IOSTD:S3:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3",
                    "iostd-slew",
                    &["IOSTD:S3:SLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3",
                    "iostd-output-misc",
                    &["IOSTD:S3:OUTPUT_MISC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3",
                    "iostd-output-diff",
                    &["IOSTD:S3:OUTPUT_DIFF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3",
                    "iostd-lvdsbias",
                    &["IOSTD:S3:LVDSBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3",
                    "iostd-dci-term-split",
                    &["IOSTD:S3:PMASK_TERM_SPLIT", "IOSTD:S3:NMASK_TERM_SPLIT"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3",
                    "iostd-dci-term-vcc",
                    &["IOSTD:S3:PMASK_TERM_VCC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3e",
                    "iostd-drive",
                    &["IOSTD:S3E:PDRIVE", "IOSTD:S3E:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3e",
                    "iostd-slew",
                    &["IOSTD:S3E:SLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3e",
                    "iostd-output-misc",
                    &["IOSTD:S3E:OUTPUT_MISC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3e",
                    "iostd-output-diff",
                    &["IOSTD:S3E:OUTPUT_DIFF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3e",
                    "iostd-lvdsbias",
                    &["IOSTD:S3E:LVDSBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3a",
                    "iostd-tb-drive",
                    &["IOSTD:S3A.TB:PDRIVE", "IOSTD:S3A.TB:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3a",
                    "iostd-tb-slew",
                    &["IOSTD:S3A.TB:PSLEW", "IOSTD:S3A.TB:NSLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3a",
                    "iostd-tb-output-diff",
                    &["IOSTD:S3A.TB:OUTPUT_DIFF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3a",
                    "iostd-lr-drive",
                    &["IOSTD:S3A.LR:PDRIVE", "IOSTD:S3A.LR:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3a",
                    "iostd-lr-slew",
                    &["IOSTD:S3A.LR:PSLEW", "IOSTD:S3A.LR:NSLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "spartan3a",
                    "iostd-tb-lvdsbias",
                    &["IOSTD:S3A.TB:LVDSBIAS"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "spartan3",
                    "bram-opts",
                    &[
                        "BRAM:DDEL_A_DEFAULT",
                        "BRAM:DDEL_B_DEFAULT",
                        "BRAM:WDEL_A_DEFAULT",
                        "BRAM:WDEL_B_DEFAULT",
                    ],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "spartan3",
                    "pcilogicse-opts",
                    &["PCILOGICSE:DELAY_DEFAULT"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "spartan3",
                    "dcm-data",
                    &["DCM:DESKEW_ADJUST", "DCM:VBG_PD", "DCM:VBG_SEL"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "spartan3",
                    "config-data",
                    &["MISC:SEND_VGG_DEFAULT", "MISC:VGG_SENDMAX_DEFAULT"],
                );
            }
            _ => unreachable!(),
        }
        check_misc_data(&db.bsdata, kind, &misc_used);
        check_devdata(&db.bsdata, kind, &devdata_used);
    }
}

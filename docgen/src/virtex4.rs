use std::collections::HashSet;

use crate::{
    bsdata::{
        check_devdata, check_misc_data, gen_bstiles, gen_devdata_table, gen_misc_table, FrameDirection, TileOrientation
    }, interconnect::gen_intdb, DocgenContext
};

pub fn gen_virtex4(ctx: &mut DocgenContext) {
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
    let gtz_orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: true,
    };
    let orientation = |tname: &str| {
        if tname.starts_with("REG.") {
            reg_orientation
        } else if tname == "GTZ" {
            gtz_orientation
        } else {
            tile_orientation
        }
    };

    for kind in ["virtex4", "virtex5", "virtex6", "virtex7"] {
        let db = prjcombine_virtex4::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        let part_names = Vec::from_iter(db.parts.iter().map(|part| part.name.as_str()));
        gen_intdb(ctx, kind, &db.int);
        gen_bstiles(ctx, kind, &db.bsdata, orientation);
        let mut misc_used = HashSet::new();
        let mut devdata_used = HashSet::new();
        match kind {
            "virtex4" => {
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex4",
                    "iostd-misc",
                    &["IOSTD:OUTPUT_MISC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex4",
                    "iostd-drive",
                    &["IOSTD:PDRIVE", "IOSTD:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex4",
                    "iostd-slew",
                    &["IOSTD:PSLEW", "IOSTD:NSLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex4",
                    "iostd-lvds",
                    &["IOSTD:LVDS_T", "IOSTD:LVDS_C"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex4",
                    "iostd-lvdsbias",
                    &["IOSTD:LVDSBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex4",
                    "iostd-dci-lvdiv2",
                    &["IOSTD:DCI:LVDIV2"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex4",
                    "iostd-dci-mask-term-vcc",
                    &["IOSTD:DCI:PMASK_TERM_VCC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex4",
                    "iostd-dci-mask-term-split",
                    &["IOSTD:DCI:PMASK_TERM_SPLIT", "IOSTD:DCI:NMASK_TERM_SPLIT"],
                );
            }
            "virtex5" => {
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "iostd-misc",
                    &["IOSTD:OUTPUT_MISC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "iostd-drive",
                    &["IOSTD:PDRIVE", "IOSTD:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "iostd-slew",
                    &["IOSTD:PSLEW", "IOSTD:NSLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "iostd-lvds",
                    &["IOSTD:LVDS_T", "IOSTD:LVDS_C"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "iostd-lvdsbias",
                    &["IOSTD:LVDSBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "iostd-dci-lvdiv2",
                    &["IOSTD:DCI:LVDIV2"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "iostd-dci-mask-term-vcc",
                    &["IOSTD:DCI:PMASK_TERM_VCC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "iostd-dci-mask-term-split",
                    &["IOSTD:DCI:PMASK_TERM_SPLIT", "IOSTD:DCI:NMASK_TERM_SPLIT"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex5",
                    "pll-filter",
                    &["PLL:PLL_CP", "PLL:PLL_RES", "PLL:PLL_LFHF"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "virtex5",
                    "iodelay-default",
                    &["IODELAY:DEFAULT_IDELAY_VALUE"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "virtex5",
                    "ppc-clock-delay",
                    &["PPC:CLOCK_DELAY"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "virtex5",
                    "pll-in-dly-set",
                    &["PLL:PLL_IN_DLY_SET"],
                );
            }
            "virtex6" => {
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-misc",
                    &["IOSTD:OUTPUT_MISC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-drive",
                    &["IOSTD:PDRIVE", "IOSTD:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-slew",
                    &["IOSTD:PSLEW", "IOSTD:NSLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-lvds",
                    &["IOSTD:LVDS_T", "IOSTD:LVDS_C"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-lvdsbias",
                    &["IOSTD:LVDSBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-dci-output",
                    &["IOSTD:DCI:PREF_OUTPUT", "IOSTD:DCI:NREF_OUTPUT"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-dci-output-half",
                    &["IOSTD:DCI:PREF_OUTPUT_HALF", "IOSTD:DCI:NREF_OUTPUT_HALF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-dci-term-vcc",
                    &["IOSTD:DCI:PREF_TERM_VCC", "IOSTD:DCI:PMASK_TERM_VCC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "iostd-dci-term-split",
                    &[
                        "IOSTD:DCI:PREF_TERM_SPLIT",
                        "IOSTD:DCI:NREF_TERM_SPLIT",
                        "IOSTD:DCI:PMASK_TERM_SPLIT",
                        "IOSTD:DCI:NMASK_TERM_SPLIT",
                    ],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "mmcm-filter",
                    &["MMCM:CP", "MMCM:RES", "MMCM:LFHF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex6",
                    "mmcm-lock",
                    &[
                        "MMCM:LOCK_REF_DLY",
                        "MMCM:LOCK_FB_DLY",
                        "MMCM:LOCK_CNT",
                        "MMCM:LOCK_SAT_HIGH",
                        "MMCM:UNLOCK_CNT",
                    ],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "virtex6",
                    "iodelay-default",
                    &["IODELAY:DEFAULT_IDELAY_VALUE"],
                );
                gen_devdata_table(
                    ctx,
                    &db.bsdata,
                    &part_names,
                    &mut devdata_used,
                    "virtex6",
                    "mmcm-in-dly-set",
                    &["MMCM:IN_DLY_SET"],
                );
            }
            "virtex7" => {
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hp-iostd-drive",
                    &["HP_IOSTD:PDRIVE", "HP_IOSTD:NDRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hp-iostd-slew",
                    &["HP_IOSTD:PSLEW", "HP_IOSTD:NSLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hp-iostd-lvds",
                    &["HP_IOSTD:LVDS_T", "HP_IOSTD:LVDS_C"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hp-iostd-lvdsbias",
                    &["HP_IOSTD:LVDSBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hp-iostd-dci-output",
                    &["HP_IOSTD:DCI:PREF_OUTPUT", "HP_IOSTD:DCI:NREF_OUTPUT"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hp-iostd-dci-output-half",
                    &[
                        "HP_IOSTD:DCI:PREF_OUTPUT_HALF",
                        "HP_IOSTD:DCI:NREF_OUTPUT_HALF",
                    ],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hp-iostd-dci-term-split",
                    &["HP_IOSTD:DCI:NREF_TERM_SPLIT"],
                );

                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hr-iostd-drive",
                    &["HR_IOSTD:DRIVE"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hr-iostd-slew",
                    &["HR_IOSTD:SLEW"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hr-iostd-misc",
                    &["HR_IOSTD:OUTPUT_MISC"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hr-iostd-lvds",
                    &["HR_IOSTD:LVDS_T", "HR_IOSTD:LVDS_C"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hr-iostd-driverbias",
                    &["HR_IOSTD:DRIVERBIAS"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "hr-iostd-lvdsbias",
                    &["HR_IOSTD:LVDSBIAS:COMMON", "HR_IOSTD:LVDSBIAS:GROUP"],
                );

                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "mmcm-filter",
                    &["MMCM:CP", "MMCM:RES", "MMCM:LFHF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "mmcm-lock",
                    &[
                        "MMCM:LOCK_REF_DLY",
                        "MMCM:LOCK_FB_DLY",
                        "MMCM:LOCK_CNT",
                        "MMCM:LOCK_SAT_HIGH",
                        "MMCM:UNLOCK_CNT",
                    ],
                );

                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "pll-filter",
                    &["PLL:CP", "PLL:RES", "PLL:LFHF"],
                );
                gen_misc_table(
                    ctx,
                    &db.bsdata,
                    &mut misc_used,
                    "virtex7",
                    "pll-lock",
                    &[
                        "PLL:LOCK_REF_DLY",
                        "PLL:LOCK_FB_DLY",
                        "PLL:LOCK_CNT",
                        "PLL:LOCK_SAT_HIGH",
                        "PLL:UNLOCK_CNT",
                    ],
                );
            }
            _ => unreachable!(),
        }
        check_misc_data(&db.bsdata, kind, &misc_used);
        check_devdata(&db.bsdata, kind, &devdata_used);
    }
}

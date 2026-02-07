use indexmap::IndexMap;
use prjcombine_virtex2::defs::devdata;

use crate::{
    DocgenContext,
    interconnect::{gen_devdata, gen_intdb},
};

pub fn gen_virtex2(ctx: &mut DocgenContext) {
    for kind in ["virtex2", "spartan3"] {
        let db = prjcombine_virtex2::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        gen_intdb(ctx, kind, &db.int);
        let mut devdata = IndexMap::new();
        for device in &db.devices {
            devdata.insert(device.name.as_str(), &device.data);
        }

        match kind {
            "virtex2" => {
                gen_devdata(
                    ctx,
                    kind,
                    &db.int,
                    "dcm-data",
                    &devdata,
                    &[
                        devdata::DCM_DESKEW_ADJUST,
                        devdata::DCM_V2_VBG_PD,
                        devdata::DCM_V2_VBG_SEL,
                    ],
                );
                gen_devdata(
                    ctx,
                    kind,
                    &db.int,
                    "bs-data",
                    &devdata,
                    &[devdata::DOUBLE_GRESTORE, devdata::FREEZE_DCI_NOPS],
                );
            }
            "spartan3" => {
                gen_devdata(
                    ctx,
                    kind,
                    &db.int,
                    "pcilogicse-opts",
                    &devdata,
                    &[devdata::PCILOGICSE_DELAY],
                );
                gen_devdata(
                    ctx,
                    kind,
                    &db.int,
                    "dcm-data",
                    &devdata,
                    &[
                        devdata::DCM_DESKEW_ADJUST,
                        devdata::DCM_V2_VBG_PD,
                        devdata::DCM_V2_VBG_SEL,
                    ],
                );
                gen_devdata(
                    ctx,
                    kind,
                    &db.int,
                    "bram-opts",
                    &devdata,
                    &[
                        devdata::BRAM_DDEL_A,
                        devdata::BRAM_DDEL_B,
                        devdata::BRAM_WDEL_A,
                        devdata::BRAM_WDEL_B,
                    ],
                );
            }
            _ => unreachable!(),
        }
    }
}

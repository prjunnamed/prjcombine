use prjcombine_rawdump::{Part, TkWire};
use std::collections::HashMap;
use std::error::Error;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "dump_noc", about = "Dump Versal NOC structure from rawdump.")]
struct Opt {
    file: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let rd = Part::from_file(opt.file)?;
    println!(
        "PART {} {} {:?} {}×{}",
        rd.part, rd.family, rd.source, rd.width, rd.height
    );
    let mut n2ow = HashMap::new();
    let mut n2iw = HashMap::new();
    for (tkn, wires_out, wires_in) in [
        // bot XPIO
        (
            "DDRMC_DMC_CORE",
            &[
                "DDRMC_MAIN_0_TO_NOC_0",
                "DDRMC_MAIN_0_TO_NOC_1",
                "DDRMC_MAIN_0_TO_NOC_2",
                "DDRMC_MAIN_0_TO_NOC_3",
            ][..],
            &[
                "DDRMC_MAIN_0_FROM_NOC_0",
                "DDRMC_MAIN_0_FROM_NOC_1",
                "DDRMC_MAIN_0_FROM_NOC_2",
                "DDRMC_MAIN_0_FROM_NOC_3",
            ][..],
        ),
        (
            "NOC_HNOC_S3_PL_CORE",
            &[
                "NOC_NPS5555_TOP_0_OUT_0",
                "NOC_NPS5555_TOP_0_OUT_1",
                "NOC_NPS5555_TOP_0_OUT_2",
                "NOC_NPS5555_TOP_1_OUT_0",
                "NOC_NPS5555_TOP_1_OUT_2",
                "NOC_NPS5555_TOP_2_OUT_0",
                "NOC_NPS5555_TOP_2_OUT_2",
                "NOC_NPS5555_TOP_2_OUT_3",
            ],
            &[
                "NOC_NPS5555_TOP_0_IN_0",
                "NOC_NPS5555_TOP_0_IN_1",
                "NOC_NPS5555_TOP_0_IN_2",
                "NOC_NPS5555_TOP_1_IN_0",
                "NOC_NPS5555_TOP_1_IN_2",
                "NOC_NPS5555_TOP_2_IN_0",
                "NOC_NPS5555_TOP_2_IN_2",
                "NOC_NPS5555_TOP_2_IN_3",
            ],
        ),
        (
            "NOC_HNOC_S3_PS_CORE",
            &[
                "NOC_NPS5555_TOP_0_OUT_0",
                "NOC_NPS5555_TOP_0_OUT_1",
                "NOC_NPS5555_TOP_0_OUT_2",
                "NOC_NPS5555_TOP_0_OUT_3",
                "NOC_NPS5555_TOP_1_OUT_0",
                "NOC_NPS5555_TOP_1_OUT_1",
                "NOC_NPS5555_TOP_1_OUT_2",
                "NOC_NPS5555_TOP_1_OUT_3",
            ],
            &[
                "NOC_NPS5555_TOP_0_IN_0",
                "NOC_NPS5555_TOP_0_IN_1",
                "NOC_NPS5555_TOP_0_IN_2",
                "NOC_NPS5555_TOP_0_IN_3",
                "NOC_NPS5555_TOP_1_IN_0",
                "NOC_NPS5555_TOP_1_IN_1",
                "NOC_NPS5555_TOP_1_IN_2",
                "NOC_NPS5555_TOP_1_IN_3",
            ],
        ),
        (
            "NOC_PNOC_MONO_CORE",
            &[
                "NOC_NSU128_TOP_0_TO_NOC",
                "NOC_NMU128_TOP_1_TO_NOC",
                "NOC_NMU128_TOP_2_TO_NOC",
                "NOC_NSU128_TOP_3_TO_NOC",
                "NOC_NMU128_TOP_4_TO_NOC",
                "NOC_NMU128_TOP_5_TO_NOC",
                "NOC_NMU128_TOP_7_TO_NOC",
                "NOC_NMU128_TOP_8_TO_NOC",
                "NOC_NSU128_TOP_9_TO_NOC",
                "NOC_NSU128_TOP_10_TO_NOC",
                "NOC_NSU128_TOP_11_TO_NOC",
                "NOC_NSU128_TOP_12_TO_NOC",
                "NOC_NMU128_TOP_13_TO_NOC",
                "NOC_NMU128_TOP_14_TO_NOC",
                "NOC_NMU128_TOP_15_TO_NOC",
                "NOC_NMU128_TOP_16_TO_NOC",
            ],
            &[
                "NOC_NSU128_TOP_0_FROM_NOC",
                "NOC_NMU128_TOP_1_FROM_NOC",
                "NOC_NMU128_TOP_2_FROM_NOC",
                "NOC_NSU128_TOP_3_FROM_NOC",
                "NOC_NMU128_TOP_4_FROM_NOC",
                "NOC_NMU128_TOP_5_FROM_NOC",
                "NOC_NMU128_TOP_7_FROM_NOC",
                "NOC_NMU128_TOP_8_FROM_NOC",
                "NOC_NSU128_TOP_9_FROM_NOC",
                "NOC_NSU128_TOP_10_FROM_NOC",
                "NOC_NSU128_TOP_11_FROM_NOC",
                "NOC_NSU128_TOP_12_FROM_NOC",
                "NOC_NMU128_TOP_13_FROM_NOC",
                "NOC_NMU128_TOP_14_FROM_NOC",
                "NOC_NMU128_TOP_15_FROM_NOC",
                "NOC_NMU128_TOP_16_FROM_NOC",
            ],
        ),
        // VNOC
        (
            "NOC_NPS_VNOC_TOP",
            &[
                "NOC_NPS_VNOC_ATOM_0_OUT_0",
                "NOC_NPS_VNOC_ATOM_0_OUT_1",
                "NOC_NPS_VNOC_ATOM_0_OUT_2",
                "NOC_NPS_VNOC_ATOM_0_OUT_3",
            ],
            &[
                "NOC_NPS_VNOC_ATOM_0_IN_0",
                "NOC_NPS_VNOC_ATOM_0_IN_1",
                "NOC_NPS_VNOC_ATOM_0_IN_2",
                "NOC_NPS_VNOC_ATOM_0_IN_3",
            ],
        ),
        (
            "NOC_NMU512_TOP",
            &["NOC_NMU512_ATOM_0_TO_NOC"],
            &["NOC_NMU512_ATOM_0_FROM_NOC"],
        ),
        (
            "NOC_NSU512_TOP",
            &["NOC_NSU512_ATOM_0_TO_NOC"],
            &["NOC_NSU512_ATOM_0_FROM_NOC"],
        ),
        // top ME
        (
            "NOC_TNOC_ME_CORE_MX",
            &["NOC_NMU128_TOP_0_TO_NOC", "NOC_NSU128_TOP_1_TO_NOC"],
            &["NOC_NMU128_TOP_0_FROM_NOC", "NOC_NSU128_TOP_1_FROM_NOC"],
        ),
        (
            "NOC_TNOC_ME_CORE_R180",
            &["NOC_NMU128_TOP_0_TO_NOC", "NOC_NSU128_TOP_1_TO_NOC"],
            &["NOC_NMU128_TOP_0_FROM_NOC", "NOC_NSU128_TOP_1_FROM_NOC"],
        ),
        (
            "NOC_TNOC_NCRB_CORE_MX",
            &[
                "NOC_NCRB_TOP_0_OUT_0",
                "NOC_NCRB_TOP_0_OUT_1",
                "NOC_NCRB_TOP_1_OUT_0",
                "NOC_NCRB_TOP_1_OUT_1",
            ],
            &[
                "NOC_NCRB_TOP_0_IN_0",
                "NOC_NCRB_TOP_0_IN_1",
                "NOC_NCRB_TOP_1_IN_0",
                "NOC_NCRB_TOP_1_IN_1",
            ],
        ),
        (
            "NOC_TNOC_S2_CORE_MX",
            &[
                "NOC_NPS5555_TOP_0_OUT_0",
                "NOC_NPS5555_TOP_0_OUT_1",
                "NOC_NPS5555_TOP_0_OUT_2",
                "NOC_NPS5555_TOP_0_OUT_3",
                "NOC_NPS5555_TOP_1_OUT_0",
                "NOC_NPS5555_TOP_1_OUT_1",
                "NOC_NPS5555_TOP_1_OUT_2",
                "NOC_NPS5555_TOP_1_OUT_3",
            ],
            &[
                "NOC_NPS5555_TOP_0_IN_0",
                "NOC_NPS5555_TOP_0_IN_1",
                "NOC_NPS5555_TOP_0_IN_2",
                "NOC_NPS5555_TOP_0_IN_3",
                "NOC_NPS5555_TOP_1_IN_0",
                "NOC_NPS5555_TOP_1_IN_1",
                "NOC_NPS5555_TOP_1_IN_2",
                "NOC_NPS5555_TOP_1_IN_3",
            ],
        ),
        // top XPIO
        (
            "DDRMC_DMC_CORE_MX",
            &[
                "DDRMC_MAIN_0_TO_NOC_0",
                "DDRMC_MAIN_0_TO_NOC_1",
                "DDRMC_MAIN_0_TO_NOC_2",
                "DDRMC_MAIN_0_TO_NOC_3",
            ][..],
            &[
                "DDRMC_MAIN_0_FROM_NOC_0",
                "DDRMC_MAIN_0_FROM_NOC_1",
                "DDRMC_MAIN_0_FROM_NOC_2",
                "DDRMC_MAIN_0_FROM_NOC_3",
            ][..],
        ),
        (
            "NOC_HNOC_S3_PL_CORE_MX",
            &[
                "NOC_NPS5555_TOP_0_OUT_0",
                "NOC_NPS5555_TOP_0_OUT_1",
                "NOC_NPS5555_TOP_0_OUT_2",
                "NOC_NPS5555_TOP_1_OUT_0",
                "NOC_NPS5555_TOP_1_OUT_2",
                "NOC_NPS5555_TOP_2_OUT_0",
                "NOC_NPS5555_TOP_2_OUT_2",
                "NOC_NPS5555_TOP_2_OUT_3",
            ],
            &[
                "NOC_NPS5555_TOP_0_IN_0",
                "NOC_NPS5555_TOP_0_IN_1",
                "NOC_NPS5555_TOP_0_IN_2",
                "NOC_NPS5555_TOP_1_IN_0",
                "NOC_NPS5555_TOP_1_IN_2",
                "NOC_NPS5555_TOP_2_IN_0",
                "NOC_NPS5555_TOP_2_IN_2",
                "NOC_NPS5555_TOP_2_IN_3",
            ],
        ),
        (
            "NOC_XPIO_NCRB_CORE_MX",
            &[
                "NOC_NCRB_TOP_0_OUT_0",
                "NOC_NCRB_TOP_0_OUT_1",
                "NOC_NCRB_TOP_1_OUT_0",
                "NOC_NCRB_TOP_1_OUT_1",
            ],
            &[
                "NOC_NCRB_TOP_0_IN_0",
                "NOC_NCRB_TOP_0_IN_1",
                "NOC_NCRB_TOP_1_IN_0",
                "NOC_NCRB_TOP_1_IN_1",
            ],
        ),
        // top SSIT
        (
            "NOC_TNOC_SSIT_NCRB_CORE_MY",
            &[
                "NOC_NCRB_SSIT_TOP_0_OUT_0",
                "NOC_NCRB_SSIT_TOP_0_OUT_1",
                "NOC_NCRB_SSIT_TOP_1_OUT_0",
                "NOC_NCRB_SSIT_TOP_1_OUT_1",
            ],
            &[
                "NOC_NCRB_SSIT_TOP_0_IN_0",
                "NOC_NCRB_SSIT_TOP_0_IN_1",
                "NOC_NCRB_SSIT_TOP_1_IN_0",
                "NOC_NCRB_SSIT_TOP_1_IN_1",
            ],
        ),
        (
            "NOC_TNOC_BRIDGE_TOP_CORE",
            &[
                "NOC_NPP_RPTR_FLOP_1_OUT_LEFT",
                "NOC_NPP_RPTR_FLOP_2_OUT_LEFT",
                "NOC_NPP_RPTR_FLOP_6_OUT_RIGHT",
                "NOC_NPP_RPTR_FLOP_7_OUT_RIGHT",
                "NOC_NIDB_TOP_0_TX",
                "NOC_NIDB_TOP_5_TX",
                "NOC_NPS7575_TOP_3_OUT_1",
                "NOC_NPS7575_TOP_4_OUT_3",
            ],
            &[
                "NOC_NPP_RPTR_FLOP_1_IN_LEFT",
                "NOC_NPP_RPTR_FLOP_2_IN_LEFT",
                "NOC_NPP_RPTR_FLOP_6_IN_RIGHT",
                "NOC_NPP_RPTR_FLOP_7_IN_RIGHT",
                "NOC_NIDB_TOP_0_RX",
                "NOC_NIDB_TOP_5_RX",
                "NOC_NPS7575_TOP_3_IN_1",
                "NOC_NPS7575_TOP_4_IN_3",
            ],
        ),
        // bot SSIT
        (
            "NOC_TNOC_SSIT_NCRB_CORE",
            &[
                "NOC_NCRB_SSIT_TOP_0_OUT_0",
                "NOC_NCRB_SSIT_TOP_0_OUT_1",
                "NOC_NCRB_SSIT_TOP_1_OUT_0",
                "NOC_NCRB_SSIT_TOP_1_OUT_1",
            ],
            &[
                "NOC_NCRB_SSIT_TOP_0_IN_0",
                "NOC_NCRB_SSIT_TOP_0_IN_1",
                "NOC_NCRB_SSIT_TOP_1_IN_0",
                "NOC_NCRB_SSIT_TOP_1_IN_1",
            ],
        ),
        (
            "NOC_TNOC_BRIDGE_BOT_CORE",
            &[
                "NOC_NPP_RPTR_FLOP_1_OUT_LEFT",
                "NOC_NPP_RPTR_FLOP_2_OUT_LEFT",
                "NOC_NPP_RPTR_FLOP_6_OUT_RIGHT",
                "NOC_NPP_RPTR_FLOP_7_OUT_RIGHT",
                "NOC_NIDB_TOP_0_TX",
                "NOC_NIDB_TOP_5_TX",
                "NOC_NPS7575_TOP_3_OUT_1",
                "NOC_NPS7575_TOP_4_OUT_3",
            ],
            &[
                "NOC_NPP_RPTR_FLOP_1_IN_LEFT",
                "NOC_NPP_RPTR_FLOP_2_IN_LEFT",
                "NOC_NPP_RPTR_FLOP_6_IN_RIGHT",
                "NOC_NPP_RPTR_FLOP_7_IN_RIGHT",
                "NOC_NIDB_TOP_0_RX",
                "NOC_NIDB_TOP_5_RX",
                "NOC_NPS7575_TOP_3_IN_1",
                "NOC_NPS7575_TOP_4_IN_3",
            ],
        ),
        (
            "NOC_PNOC_SSIT_CORE",
            &[
                "NOC_NSU128_TOP_0_TO_NOC",
                "NOC_NMU128_TOP_1_TO_NOC",
                "NOC_NMU128_TOP_2_TO_NOC",
                "NOC_NSU128_TOP_3_TO_NOC",
                "NOC_NMU128_TOP_4_TO_NOC",
                "NOC_NMU128_TOP_5_TO_NOC",
                "NOC_NMU128_TOP_7_TO_NOC",
                "NOC_NMU128_TOP_8_TO_NOC",
                "NOC_NSU128_TOP_9_TO_NOC",
                "NOC_NSU128_TOP_10_TO_NOC",
                "NOC_NSU128_TOP_11_TO_NOC",
                "NOC_NSU128_TOP_12_TO_NOC",
                "NOC_NMU128_TOP_13_TO_NOC",
                "NOC_NMU128_TOP_14_TO_NOC",
                "NOC_NMU128_TOP_15_TO_NOC",
                "NOC_NMU128_TOP_16_TO_NOC",
            ],
            &[
                "NOC_NSU128_TOP_0_FROM_NOC",
                "NOC_NMU128_TOP_1_FROM_NOC",
                "NOC_NMU128_TOP_2_FROM_NOC",
                "NOC_NSU128_TOP_3_FROM_NOC",
                "NOC_NMU128_TOP_4_FROM_NOC",
                "NOC_NMU128_TOP_5_FROM_NOC",
                "NOC_NMU128_TOP_7_FROM_NOC",
                "NOC_NMU128_TOP_8_FROM_NOC",
                "NOC_NSU128_TOP_9_FROM_NOC",
                "NOC_NSU128_TOP_10_FROM_NOC",
                "NOC_NSU128_TOP_11_FROM_NOC",
                "NOC_NSU128_TOP_12_FROM_NOC",
                "NOC_NMU128_TOP_13_FROM_NOC",
                "NOC_NMU128_TOP_14_FROM_NOC",
                "NOC_NMU128_TOP_15_FROM_NOC",
                "NOC_NMU128_TOP_16_FROM_NOC",
            ],
        ),
        (
            "NOC_TNOC_SSIT_S2_CORE",
            &[
                "NOC_NPS7575_TOP_0_OUT_0",
                "NOC_NPS7575_TOP_0_OUT_1",
                "NOC_NPS7575_TOP_0_OUT_2",
                "NOC_NPS7575_TOP_0_OUT_3",
                "NOC_NPS7575_TOP_1_OUT_0",
                "NOC_NPS7575_TOP_1_OUT_1",
                "NOC_NPS7575_TOP_1_OUT_2",
                "NOC_NPS7575_TOP_1_OUT_3",
            ],
            &[
                "NOC_NPS7575_TOP_0_IN_0",
                "NOC_NPS7575_TOP_0_IN_1",
                "NOC_NPS7575_TOP_0_IN_2",
                "NOC_NPS7575_TOP_0_IN_3",
                "NOC_NPS7575_TOP_1_IN_0",
                "NOC_NPS7575_TOP_1_IN_1",
                "NOC_NPS7575_TOP_1_IN_2",
                "NOC_NPS7575_TOP_1_IN_3",
            ],
        ),
        // top HBM
        (
            "HBMMC_CORE",
            &[
                "HBMMC_MAIN_0_TO_NOC_0",
                "HBMMC_MAIN_0_TO_NOC_1",
                "HBMMC_MAIN_0_TO_NOC_2",
                "HBMMC_MAIN_0_TO_NOC_3",
            ][..],
            &[
                "HBMMC_MAIN_0_FROM_NOC_0",
                "HBMMC_MAIN_0_FROM_NOC_1",
                "HBMMC_MAIN_0_FROM_NOC_2",
                "HBMMC_MAIN_0_FROM_NOC_3",
            ][..],
        ),
        (
            "NOC_HBM_NCRB_CORE_MX",
            &[
                "NOC_NCRB_TOP_0_OUT_0",
                "NOC_NCRB_TOP_0_OUT_1",
                "NOC_NCRB_TOP_1_OUT_0",
                "NOC_NCRB_TOP_1_OUT_1",
            ],
            &[
                "NOC_NCRB_TOP_0_IN_0",
                "NOC_NCRB_TOP_0_IN_1",
                "NOC_NCRB_TOP_1_IN_0",
                "NOC_NCRB_TOP_1_IN_1",
            ],
        ),
        (
            "NOC_HBM_NPS8_CORE",
            &[
                "NOC_NPS4_TOP_0_OUT_REQ_0",
                "NOC_NPS4_TOP_0_OUT_REQ_1",
                "NOC_NPS4_TOP_0_OUT_REQ_2",
                "NOC_NPS4_TOP_0_OUT_REQ_3",
                "NOC_NPS4_TOP_0_OUT_RESP_0",
                "NOC_NPS4_TOP_0_OUT_RESP_1",
                "NOC_NPS4_TOP_0_OUT_RESP_2",
                "NOC_NPS4_TOP_0_OUT_RESP_3",
                "NOC_NPS4_TOP_1_OUT_REQ_0",
                "NOC_NPS4_TOP_1_OUT_REQ_1",
                "NOC_NPS4_TOP_1_OUT_REQ_2",
                "NOC_NPS4_TOP_1_OUT_REQ_3",
                "NOC_NPS4_TOP_1_OUT_RESP_0",
                "NOC_NPS4_TOP_1_OUT_RESP_1",
                "NOC_NPS4_TOP_1_OUT_RESP_2",
                "NOC_NPS4_TOP_1_OUT_RESP_3",
            ],
            &[
                "NOC_NPS4_TOP_0_IN_REQ_0",
                "NOC_NPS4_TOP_0_IN_REQ_1",
                "NOC_NPS4_TOP_0_IN_REQ_2",
                "NOC_NPS4_TOP_0_IN_REQ_3",
                "NOC_NPS4_TOP_0_IN_RESP_0",
                "NOC_NPS4_TOP_0_IN_RESP_1",
                "NOC_NPS4_TOP_0_IN_RESP_2",
                "NOC_NPS4_TOP_0_IN_RESP_3",
                "NOC_NPS4_TOP_1_IN_REQ_0",
                "NOC_NPS4_TOP_1_IN_REQ_1",
                "NOC_NPS4_TOP_1_IN_REQ_2",
                "NOC_NPS4_TOP_1_IN_REQ_3",
                "NOC_NPS4_TOP_1_IN_RESP_0",
                "NOC_NPS4_TOP_1_IN_RESP_1",
                "NOC_NPS4_TOP_1_IN_RESP_2",
                "NOC_NPS4_TOP_1_IN_RESP_3",
            ],
        ),
        (
            "NOC_HBM_S4_CORE",
            &[
                "NOC_NPS5555_TOP_0_OUT_0",
                "NOC_NPS5555_TOP_0_OUT_1",
                "NOC_NPS5555_TOP_1_OUT_1",
                "NOC_NPS5555_TOP_1_OUT_2",
                "NOC_NPS5555_TOP_1_OUT_3",
                "NOC_NPS5555_TOP_2_OUT_0",
                "NOC_NPS5555_TOP_2_OUT_1",
                "NOC_NPS5555_TOP_3_OUT_1",
                "NOC_NPS5555_TOP_3_OUT_2",
                "NOC_NPS5555_TOP_3_OUT_3",
            ],
            &[
                "NOC_NPS5555_TOP_0_IN_0",
                "NOC_NPS5555_TOP_0_IN_1",
                "NOC_NPS5555_TOP_1_IN_1",
                "NOC_NPS5555_TOP_1_IN_2",
                "NOC_NPS5555_TOP_1_IN_3",
                "NOC_NPS5555_TOP_2_IN_0",
                "NOC_NPS5555_TOP_2_IN_1",
                "NOC_NPS5555_TOP_3_IN_1",
                "NOC_NPS5555_TOP_3_IN_2",
                "NOC_NPS5555_TOP_3_IN_3",
            ],
        ),
        (
            "NOC_HBM_S4_NPS6_CORE",
            &[
                "NOC_NPS6_TOP_0_OUT_0",
                "NOC_NPS6_TOP_0_OUT_1",
                "NOC_NPS6_TOP_0_OUT_2",
                "NOC_NPS6_TOP_0_OUT_3",
                "NOC_NPS6_TOP_0_OUT_4",
                "NOC_NPS6_TOP_0_OUT_5",
                "NOC_NPS6_TOP_1_OUT_0",
                "NOC_NPS6_TOP_1_OUT_1",
                "NOC_NPS6_TOP_1_OUT_2",
                "NOC_NPS6_TOP_1_OUT_3",
                "NOC_NPS6_TOP_1_OUT_4",
                "NOC_NPS6_TOP_1_OUT_5",
                "NOC_NPS6_TOP_2_OUT_0",
                "NOC_NPS6_TOP_2_OUT_1",
                "NOC_NPS6_TOP_2_OUT_2",
                "NOC_NPS6_TOP_2_OUT_3",
                "NOC_NPS6_TOP_2_OUT_4",
                "NOC_NPS6_TOP_2_OUT_5",
                "NOC_NPS6_TOP_3_OUT_0",
                "NOC_NPS6_TOP_3_OUT_1",
                "NOC_NPS6_TOP_3_OUT_2",
                "NOC_NPS6_TOP_3_OUT_3",
                "NOC_NPS6_TOP_3_OUT_4",
                "NOC_NPS6_TOP_3_OUT_5",
            ],
            &[
                "NOC_NPS6_TOP_0_IN_0",
                "NOC_NPS6_TOP_0_IN_1",
                "NOC_NPS6_TOP_0_IN_2",
                "NOC_NPS6_TOP_0_IN_3",
                "NOC_NPS6_TOP_0_IN_4",
                "NOC_NPS6_TOP_0_IN_5",
                "NOC_NPS6_TOP_1_IN_0",
                "NOC_NPS6_TOP_1_IN_1",
                "NOC_NPS6_TOP_1_IN_2",
                "NOC_NPS6_TOP_1_IN_3",
                "NOC_NPS6_TOP_1_IN_4",
                "NOC_NPS6_TOP_1_IN_5",
                "NOC_NPS6_TOP_2_IN_0",
                "NOC_NPS6_TOP_2_IN_1",
                "NOC_NPS6_TOP_2_IN_2",
                "NOC_NPS6_TOP_2_IN_3",
                "NOC_NPS6_TOP_2_IN_4",
                "NOC_NPS6_TOP_2_IN_5",
                "NOC_NPS6_TOP_3_IN_0",
                "NOC_NPS6_TOP_3_IN_1",
                "NOC_NPS6_TOP_3_IN_2",
                "NOC_NPS6_TOP_3_IN_3",
                "NOC_NPS6_TOP_3_IN_4",
                "NOC_NPS6_TOP_3_IN_5",
            ],
        ),
        (
            "NOC_NMU_HBM2E_TOP_MX",
            &["NOC_NMU_HBM2E_ATOM_0_TO_NOC"],
            &["NOC_NMU_HBM2E_ATOM_0_FROM_NOC"],
        ),
    ] {
        for crd in rd.tiles_by_kind_name(tkn) {
            let tile = &rd.tiles[crd];
            let tk = &rd.tile_kinds[tile.kind];
            for &wn in wires_in {
                if rd.wires.get(wn).is_none() {
                    println!("OOPS {tkn} {wn}");
                }
                let wni = rd.wires.get(wn).unwrap();
                if tk.wires.get(&wni).is_none() {
                    println!("OOPS {tkn} {wn}");
                }
                let w = tk.wires.get(&wni).unwrap().1;
                if let TkWire::Connected(cwi) = *w {
                    if let Some(ni) = tile.conn_wires.get(cwi) {
                        assert!(!n2iw.contains_key(&ni));
                        n2iw.insert(ni, (&tile.name, wn));
                    }
                }
            }
            for &wn in wires_out {
                if rd.wires.get(wn).is_none() {
                    println!("OOPS {tkn} {wn}");
                }
                let wni = rd.wires.get(wn).unwrap();
                if tk.wires.get(&wni).is_none() {
                    println!("OOPS {tkn} {wn}");
                }
                let w = tk.wires.get(&wni).unwrap().1;
                if let TkWire::Connected(cwi) = *w {
                    if let Some(ni) = tile.conn_wires.get(cwi) {
                        assert!(!n2ow.contains_key(&ni));
                        n2ow.insert(ni, (&tile.name, wn));
                    }
                }
            }
        }
    }
    for (&ni, &(ti, wi)) in &n2iw {
        match n2ow.get(&ni) {
            Some(&(to, wo)) => {
                println!("{ti:30} {wi:30} <= {to:30} {wo:30}");
            }
            None => {
                println!("{ti:30} {wi:30} <= [NONE]");
            }
        }
    }
    for (&ni, &(to, wo)) in &n2ow {
        if !n2iw.contains_key(&ni) {
            println!(
                "[NONE]                                                        <= {to:30} {wo:30}"
            );
        }
    }
    Ok(())
}

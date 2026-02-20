use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_collector::diff::OcdMode;
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::{
    chip::ChipKind,
    defs::{
        bcls::SLICE_V5 as SLICE, bslots, enums, tslots, virtex5::tcls as tcls_v5,
        virtex6::tcls as tcls_v6, virtex7::tcls as tcls_v7,
    },
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::FuzzCtx,
        props::{pip::PinFar, relation::TileRelation},
    },
};

#[derive(Copy, Clone, Debug)]
struct ClbCinDown;

impl TileRelation for ClbCinDown {
    fn resolve(&self, backend: &IseBackend, mut tcrd: TileCoord) -> Option<TileCoord> {
        if tcrd.row.to_idx() == 0 {
            return None;
        }
        tcrd.row -= 1;
        if backend.edev.has_bel(tcrd.bel(bslots::SLICE[0])) {
            Some(tcrd.tile(tslots::BEL))
        } else {
            None
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    let (tcid_clbll, tcid_clblm) = match edev.kind {
        ChipKind::Virtex5 => (tcls_v5::CLBLL, tcls_v5::CLBLM),
        ChipKind::Virtex6 => (tcls_v6::CLBLL, tcls_v6::CLBLM),
        ChipKind::Virtex7 => (tcls_v7::CLBLL, tcls_v7::CLBLM),
        _ => unreachable!(),
    };

    for tcid in [tcid_clbll, tcid_clblm] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..2 {
            let bel = bslots::SLICE[i];
            let mut bctx = ctx.bel(bel);
            let is_m = i == 0 && tcid == tcid_clblm;

            // LUTs
            for attr in [SLICE::A6LUT, SLICE::B6LUT, SLICE::C6LUT, SLICE::D6LUT] {
                bctx.mode("SLICEL")
                    .test_bel_attr_multi(attr, MultiValue::Lut);
            }

            if is_m {
                // LUT RAM
                bctx.mode("SLICEM")
                    .attr("A6LUT", "#RAM:0")
                    .attr("A6RAMMODE", "SPRAM64")
                    .pin("WE")
                    .pin("CE")
                    .test_bel_attr_rename("WEMUX", SLICE::MUX_WE);
                for attr in [SLICE::WA7USED, SLICE::WA8USED] {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#RAM:0")
                        .attr("A6RAMMODE", "SPRAM64")
                        .pin("AX")
                        .pin("BX")
                        .pin("CX")
                        .pin("DX")
                        .test_bel_attr_bits(attr)
                        .attr(backend.edev.db[SLICE].attributes.key(attr), "0")
                        .commit();
                }
                if matches!(edev.kind, ChipKind::Virtex5) {
                    for (val, vname) in [
                        (enums::SLICE_MUX_ADI1::AX, "AX"),
                        (enums::SLICE_MUX_ADI1::ALT, "BDI1"),
                        (enums::SLICE_MUX_ADI1::ALT, "BMC31"),
                    ] {
                        bctx.mode("SLICEM")
                            .attr("A6LUT", "#RAM:0")
                            .attr("A6RAMMODE", "SPRAM64")
                            .pin("AX")
                            .test_bel_attr_val(SLICE::MUX_ADI1, val)
                            .attr("ADI1MUX", vname)
                            .commit();
                    }
                    for (val, vname) in [
                        (enums::SLICE_MUX_BDI1::BX, "BX"),
                        (enums::SLICE_MUX_BDI1::ALT, "DX"),
                        (enums::SLICE_MUX_BDI1::ALT, "CMC31"),
                    ] {
                        bctx.mode("SLICEM")
                            .attr("B6LUT", "#RAM:0")
                            .attr("B6RAMMODE", "SPRAM64")
                            .pin("BX")
                            .pin("DX")
                            .test_bel_attr_val(SLICE::MUX_BDI1, val)
                            .attr("BDI1MUX", vname)
                            .commit();
                    }
                    for (val, vname) in [
                        (enums::SLICE_MUX_CDI1::CX, "CX"),
                        (enums::SLICE_MUX_CDI1::ALT, "DX"),
                        (enums::SLICE_MUX_CDI1::ALT, "DMC31"),
                    ] {
                        bctx.mode("SLICEM")
                            .attr("C6LUT", "#RAM:0")
                            .attr("C6RAMMODE", "SPRAM64")
                            .pin("CX")
                            .pin("DX")
                            .test_bel_attr_val(SLICE::MUX_CDI1, val)
                            .attr("CDI1MUX", vname)
                            .commit();
                    }
                } else {
                    for (val, vname) in [
                        (enums::SLICE_MUX_ADI1::AI, "AI"),
                        (enums::SLICE_MUX_ADI1::ALT, "BDI1"),
                        (enums::SLICE_MUX_ADI1::ALT, "BMC31"),
                    ] {
                        bctx.mode("SLICEM")
                            .attr("A6LUT", "#RAM:0")
                            .attr("A6RAMMODE", "SPRAM64")
                            .pin("AI")
                            .test_bel_attr_val(SLICE::MUX_ADI1, val)
                            .attr("ADI1MUX", vname)
                            .commit();
                    }
                    for (val, vname) in [
                        (enums::SLICE_MUX_BDI1::BI, "BI"),
                        (enums::SLICE_MUX_BDI1::ALT, "DI"),
                        (enums::SLICE_MUX_BDI1::ALT, "CMC31"),
                    ] {
                        bctx.mode("SLICEM")
                            .attr("B6LUT", "#RAM:0")
                            .attr("B6RAMMODE", "SPRAM64")
                            .pin("BI")
                            .pin("DI")
                            .test_bel_attr_val(SLICE::MUX_BDI1, val)
                            .attr("BDI1MUX", vname)
                            .commit();
                    }
                    for (val, vname) in [
                        (enums::SLICE_MUX_CDI1::CI, "CI"),
                        (enums::SLICE_MUX_CDI1::ALT, "DI"),
                        (enums::SLICE_MUX_CDI1::ALT, "DMC31"),
                    ] {
                        bctx.mode("SLICEM")
                            .attr("C6LUT", "#RAM:0")
                            .attr("C6RAMMODE", "SPRAM64")
                            .pin("CI")
                            .pin("DI")
                            .test_bel_attr_val(SLICE::MUX_CDI1, val)
                            .attr("CDI1MUX", vname)
                            .commit();
                    }
                }
                for (val, vname) in [
                    (enums::SLICE_RAMMODE::RAM32, "SPRAM32"),
                    (enums::SLICE_RAMMODE::RAM32, "DPRAM32"),
                    (enums::SLICE_RAMMODE::RAM64, "SPRAM64"),
                    (enums::SLICE_RAMMODE::RAM64, "DPRAM64"),
                    (enums::SLICE_RAMMODE::SRL16, "SRL16"),
                    (enums::SLICE_RAMMODE::SRL32, "SRL32"),
                ] {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#RAM:0")
                        .test_bel_attr_val(SLICE::ARAMMODE, val)
                        .attr("A6RAMMODE", vname)
                        .commit();
                    bctx.mode("SLICEM")
                        .attr("B6LUT", "#RAM:0")
                        .test_bel_attr_val(SLICE::BRAMMODE, val)
                        .attr("B6RAMMODE", vname)
                        .commit();
                    bctx.mode("SLICEM")
                        .attr("C6LUT", "#RAM:0")
                        .test_bel_attr_val(SLICE::CRAMMODE, val)
                        .attr("C6RAMMODE", vname)
                        .commit();
                    bctx.mode("SLICEM")
                        .attr("D6LUT", "#RAM:0")
                        .test_bel_attr_val(SLICE::DRAMMODE, val)
                        .attr("D6RAMMODE", vname)
                        .commit();
                }
            }

            // carry chain
            bctx.mode("SLICEL")
                .attr("A5LUT", "#LUT:0")
                .attr("A6LUT", "#LUT:0")
                .attr("COUTUSED", "0")
                .pin("AX")
                .pin("COUT")
                .test_bel_attr_rename("ACY0", SLICE::MUX_ACY0);
            bctx.mode("SLICEL")
                .attr("B5LUT", "#LUT:0")
                .attr("B6LUT", "#LUT:0")
                .attr("COUTUSED", "0")
                .pin("BX")
                .pin("COUT")
                .test_bel_attr_rename("BCY0", SLICE::MUX_BCY0);
            bctx.mode("SLICEL")
                .attr("C5LUT", "#LUT:0")
                .attr("C6LUT", "#LUT:0")
                .attr("COUTUSED", "0")
                .pin("CX")
                .pin("COUT")
                .test_bel_attr_rename("CCY0", SLICE::MUX_CCY0);
            bctx.mode("SLICEL")
                .attr("D5LUT", "#LUT:0")
                .attr("D6LUT", "#LUT:0")
                .attr("COUTUSED", "0")
                .pin("DX")
                .pin("COUT")
                .test_bel_attr_rename("DCY0", SLICE::MUX_DCY0);
            for (val, vname) in [
                (enums::SLICE_PRECYINIT::AX, "AX"),
                (enums::SLICE_PRECYINIT::CONST_0, "0"),
                (enums::SLICE_PRECYINIT::CONST_1, "1"),
            ] {
                bctx.mode("SLICEL")
                    .attr("COUTUSED", "0")
                    .pin("AX")
                    .pin("COUT")
                    .test_bel_attr_val(SLICE::PRECYINIT, val)
                    .attr("PRECYINIT", vname)
                    .commit();
            }

            bctx.build()
                .test_bel_attr_val(SLICE::CYINIT, enums::SLICE_CYINIT::CIN)
                .related_pip(ClbCinDown, (PinFar, "COUT"), "COUT")
                .commit();

            // misc muxes
            // [ABCD]MUX
            if edev.kind == ChipKind::Virtex5 {
                bctx.mode("SLICEL")
                    .attr("A6LUT", "#LUT:0")
                    .attr("A5LUT", "#LUT:0")
                    .pin("AMUX")
                    .test_bel_attr_subset_rename(
                        "AOUTMUX",
                        SLICE::MUX_AOUT,
                        &[
                            enums::SLICE_MUX_AOUT::O5,
                            enums::SLICE_MUX_AOUT::O6,
                            enums::SLICE_MUX_AOUT::XOR,
                            enums::SLICE_MUX_AOUT::CY,
                            enums::SLICE_MUX_AOUT::F7,
                        ],
                    );
                bctx.mode("SLICEL")
                    .attr("B6LUT", "#LUT:0")
                    .attr("B5LUT", "#LUT:0")
                    .pin("BMUX")
                    .test_bel_attr_subset_rename(
                        "BOUTMUX",
                        SLICE::MUX_BOUT,
                        &[
                            enums::SLICE_MUX_BOUT::O5,
                            enums::SLICE_MUX_BOUT::O6,
                            enums::SLICE_MUX_BOUT::XOR,
                            enums::SLICE_MUX_BOUT::CY,
                            enums::SLICE_MUX_BOUT::F8,
                        ],
                    );
                bctx.mode("SLICEL")
                    .attr("C6LUT", "#LUT:0")
                    .attr("C5LUT", "#LUT:0")
                    .pin("CMUX")
                    .test_bel_attr_subset_rename(
                        "COUTMUX",
                        SLICE::MUX_COUT,
                        &[
                            enums::SLICE_MUX_COUT::O5,
                            enums::SLICE_MUX_COUT::O6,
                            enums::SLICE_MUX_COUT::XOR,
                            enums::SLICE_MUX_COUT::CY,
                            enums::SLICE_MUX_COUT::F7,
                        ],
                    );
                if is_m {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#LUT:0")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5LUT", "#LUT:0")
                        .pin("DMUX")
                        .test_bel_attr_subset_rename(
                            "DOUTMUX",
                            SLICE::MUX_DOUT,
                            &[
                                enums::SLICE_MUX_DOUT::O5,
                                enums::SLICE_MUX_DOUT::O6,
                                enums::SLICE_MUX_DOUT::XOR,
                                enums::SLICE_MUX_DOUT::CY,
                                enums::SLICE_MUX_DOUT::MC31,
                            ],
                        );
                } else {
                    bctx.mode("SLICEL")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5LUT", "#LUT:0")
                        .pin("DMUX")
                        .test_bel_attr_subset_rename(
                            "DOUTMUX",
                            SLICE::MUX_DOUT,
                            &[
                                enums::SLICE_MUX_DOUT::O5,
                                enums::SLICE_MUX_DOUT::O6,
                                enums::SLICE_MUX_DOUT::XOR,
                                enums::SLICE_MUX_DOUT::CY,
                            ],
                        );
                }
            } else {
                bctx.mode("SLICEL")
                    .attr("A6LUT", "#LUT:0")
                    .attr("A5LUT", "#LUT:0")
                    .attr("A5FFMUX", "")
                    .attr("CLKINV", "CLK")
                    .pin("AMUX")
                    .pin("CLK")
                    .test_bel_attr_default_rename(
                        "AOUTMUX",
                        SLICE::MUX_AOUT,
                        enums::SLICE_MUX_AOUT::NONE,
                    );
                bctx.mode("SLICEL")
                    .attr("B6LUT", "#LUT:0")
                    .attr("B5LUT", "#LUT:0")
                    .attr("B5FFMUX", "")
                    .attr("CLKINV", "CLK")
                    .pin("BMUX")
                    .pin("CLK")
                    .test_bel_attr_default_rename(
                        "BOUTMUX",
                        SLICE::MUX_BOUT,
                        enums::SLICE_MUX_BOUT::NONE,
                    );
                bctx.mode("SLICEL")
                    .attr("C6LUT", "#LUT:0")
                    .attr("C5LUT", "#LUT:0")
                    .attr("C5FFMUX", "")
                    .attr("CLKINV", "CLK")
                    .pin("CMUX")
                    .pin("CLK")
                    .test_bel_attr_default_rename(
                        "COUTMUX",
                        SLICE::MUX_COUT,
                        enums::SLICE_MUX_COUT::NONE,
                    );
                if is_m {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#LUT:0")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5LUT", "#LUT:0")
                        .attr("D5FFMUX", "")
                        .attr("CLKINV", "CLK")
                        .pin("DMUX")
                        .pin("CLK")
                        .test_bel_attr_default_rename(
                            "DOUTMUX",
                            SLICE::MUX_DOUT,
                            enums::SLICE_MUX_DOUT::NONE,
                        );
                } else {
                    bctx.mode("SLICEL")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5LUT", "#LUT:0")
                        .attr("D5FFMUX", "")
                        .attr("CLKINV", "CLK")
                        .pin("DMUX")
                        .pin("CLK")
                        .test_bel_attr_subset_rename(
                            "DOUTMUX",
                            SLICE::MUX_DOUT,
                            &[
                                enums::SLICE_MUX_DOUT::O5,
                                enums::SLICE_MUX_DOUT::O6,
                                enums::SLICE_MUX_DOUT::XOR,
                                enums::SLICE_MUX_DOUT::CY,
                                enums::SLICE_MUX_DOUT::D5Q,
                            ],
                        );
                }
            }

            // [ABCD]FF input
            bctx.mode("SLICEL")
                .attr("A6LUT", "#LUT:0")
                .attr("A5LUT", "#LUT:0")
                .attr("AFF", "#FF")
                .attr("CLKINV", "CLK")
                .pin("AX")
                .pin("AQ")
                .pin("CLK")
                .test_bel_attr_default_rename("AFFMUX", SLICE::MUX_FFA, enums::SLICE_MUX_FFA::NONE);
            bctx.mode("SLICEL")
                .attr("B6LUT", "#LUT:0")
                .attr("B5LUT", "#LUT:0")
                .attr("BFF", "#FF")
                .attr("CLKINV", "CLK")
                .pin("BX")
                .pin("BQ")
                .pin("CLK")
                .test_bel_attr_default_rename("BFFMUX", SLICE::MUX_FFB, enums::SLICE_MUX_FFB::NONE);
            bctx.mode("SLICEL")
                .attr("C6LUT", "#LUT:0")
                .attr("C5LUT", "#LUT:0")
                .attr("CFF", "#FF")
                .attr("CLKINV", "CLK")
                .pin("CX")
                .pin("CQ")
                .pin("CLK")
                .test_bel_attr_default_rename("CFFMUX", SLICE::MUX_FFC, enums::SLICE_MUX_FFC::NONE);
            if is_m {
                bctx.mode("SLICEM")
                    .attr("A6LUT", "#LUT:0")
                    .attr("D6LUT", "#LUT:0")
                    .attr("D5LUT", "#LUT:0")
                    .attr("DFF", "#FF")
                    .attr("CLKINV", "CLK")
                    .pin("DX")
                    .pin("DQ")
                    .pin("CLK")
                    .test_bel_attr_default_rename(
                        "DFFMUX",
                        SLICE::MUX_FFD,
                        enums::SLICE_MUX_FFD::NONE,
                    );
            } else {
                bctx.mode("SLICEL")
                    .attr("D6LUT", "#LUT:0")
                    .attr("D5LUT", "#LUT:0")
                    .attr("DFF", "#FF")
                    .attr("CLKINV", "CLK")
                    .pin("DX")
                    .pin("DQ")
                    .pin("CLK")
                    .test_bel_attr_subset_rename(
                        "DFFMUX",
                        SLICE::MUX_FFD,
                        &[
                            enums::SLICE_MUX_FFD::O5,
                            enums::SLICE_MUX_FFD::O6,
                            enums::SLICE_MUX_FFD::XOR,
                            enums::SLICE_MUX_FFD::CY,
                            enums::SLICE_MUX_FFD::DX,
                        ],
                    );
            }
            if matches!(edev.kind, ChipKind::Virtex6 | ChipKind::Virtex7) {
                for (val, vname) in [
                    (enums::SLICE_MUX_FFA5::O5, "IN_A"),
                    (enums::SLICE_MUX_FFA5::AX, "IN_B"),
                ] {
                    bctx.mode("SLICEL")
                        .attr("A6LUT", "#LUT:0")
                        .attr("A5LUT", "#LUT:0")
                        .attr("AOUTMUX", "A5Q")
                        .attr("CLKINV", "CLK")
                        .pin("AX")
                        .pin("AMUX")
                        .pin("CLK")
                        .test_bel_attr_val(SLICE::MUX_FFA5, val)
                        .attr("A5FFMUX", vname)
                        .commit();
                }
                for (val, vname) in [
                    (enums::SLICE_MUX_FFB5::O5, "IN_A"),
                    (enums::SLICE_MUX_FFB5::BX, "IN_B"),
                ] {
                    bctx.mode("SLICEL")
                        .attr("B6LUT", "#LUT:0")
                        .attr("B5LUT", "#LUT:0")
                        .attr("BOUTMUX", "B5Q")
                        .attr("CLKINV", "CLK")
                        .pin("BX")
                        .pin("BMUX")
                        .pin("CLK")
                        .test_bel_attr_val(SLICE::MUX_FFB5, val)
                        .attr("B5FFMUX", vname)
                        .commit();
                }
                for (val, vname) in [
                    (enums::SLICE_MUX_FFC5::O5, "IN_A"),
                    (enums::SLICE_MUX_FFC5::CX, "IN_B"),
                ] {
                    bctx.mode("SLICEL")
                        .attr("C6LUT", "#LUT:0")
                        .attr("C5LUT", "#LUT:0")
                        .attr("COUTMUX", "C5Q")
                        .attr("CLKINV", "CLK")
                        .pin("CX")
                        .pin("CMUX")
                        .pin("CLK")
                        .test_bel_attr_val(SLICE::MUX_FFC5, val)
                        .attr("C5FFMUX", vname)
                        .commit();
                }
                for (val, vname) in [
                    (enums::SLICE_MUX_FFD5::O5, "IN_A"),
                    (enums::SLICE_MUX_FFD5::DX, "IN_B"),
                ] {
                    bctx.mode("SLICEL")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5LUT", "#LUT:0")
                        .attr("DOUTMUX", "D5Q")
                        .attr("CLKINV", "CLK")
                        .pin("DX")
                        .pin("DMUX")
                        .pin("CLK")
                        .test_bel_attr_val(SLICE::MUX_FFD5, val)
                        .attr("D5FFMUX", vname)
                        .commit();
                }
            }

            // FFs
            bctx.mode("SLICEL")
                .attr("AFF", "#FF")
                .pin("AQ")
                .test_bel_attr_bool_rename("SYNC_ATTR", SLICE::FF_SR_SYNC, "ASYNC", "SYNC");
            bctx.mode("SLICEL")
                .attr("AFF", "#FF")
                .pin("AQ")
                .test_bel_input_inv_auto(SLICE::CLK);
            if edev.kind == ChipKind::Virtex5 {
                bctx.mode("SLICEL")
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("DX")
                    .pin("CLK")
                    .test_bel_attr_bits(SLICE::FF_REV_ENABLE)
                    .attr("REVUSED", "0")
                    .commit();
                bctx.mode("SLICEL")
                    .attr("AFFINIT", "INIT1")
                    .attr("BFF", "")
                    .attr("CFF", "")
                    .attr("DFF", "")
                    .pin("AQ")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("AFF", SLICE::FF_LATCH, "#FF", "#LATCH");
                bctx.mode("SLICEL")
                    .attr("BFFINIT", "INIT1")
                    .attr("AFF", "")
                    .attr("CFF", "")
                    .attr("DFF", "")
                    .pin("BQ")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("BFF", SLICE::FF_LATCH, "#FF", "#LATCH");
                bctx.mode("SLICEL")
                    .attr("CFFINIT", "INIT1")
                    .attr("AFF", "")
                    .attr("BFF", "")
                    .attr("DFF", "")
                    .pin("CQ")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("CFF", SLICE::FF_LATCH, "#FF", "#LATCH");
                bctx.mode("SLICEL")
                    .attr("DFFINIT", "INIT1")
                    .attr("AFF", "")
                    .attr("BFF", "")
                    .attr("CFF", "")
                    .pin("DQ")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("DFF", SLICE::FF_LATCH, "#FF", "#LATCH");
            } else {
                let (attr_a, attr_b, attr_c, attr_d) = if edev.kind == ChipKind::Virtex6 {
                    (
                        SLICE::FFA_LATCH,
                        SLICE::FFB_LATCH,
                        SLICE::FFC_LATCH,
                        SLICE::FFD_LATCH,
                    )
                } else {
                    (
                        SLICE::FF_LATCH,
                        SLICE::FF_LATCH,
                        SLICE::FF_LATCH,
                        SLICE::FF_LATCH,
                    )
                };
                for (val, vname) in [
                    (false, "#FF"),
                    (true, "#LATCH"),
                    (true, "AND2L"),
                    (true, "OR2L"),
                ] {
                    bctx.mode("SLICEL")
                        .attr("AFFINIT", "INIT1")
                        .attr("BFF", "")
                        .attr("CFF", "")
                        .attr("DFF", "")
                        .pin("AQ")
                        .pin("CLK")
                        .test_bel_attr_bits_bi(attr_a, val)
                        .attr("AFF", vname)
                        .commit();
                    bctx.mode("SLICEL")
                        .attr("BFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("CFF", "")
                        .attr("DFF", "")
                        .pin("BQ")
                        .pin("CLK")
                        .test_bel_attr_bits_bi(attr_b, val)
                        .attr("BFF", vname)
                        .commit();
                    bctx.mode("SLICEL")
                        .attr("CFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("BFF", "")
                        .attr("DFF", "")
                        .pin("CQ")
                        .pin("CLK")
                        .test_bel_attr_bits_bi(attr_c, val)
                        .attr("CFF", vname)
                        .commit();
                    bctx.mode("SLICEL")
                        .attr("DFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("BFF", "")
                        .attr("CFF", "")
                        .pin("DQ")
                        .pin("CLK")
                        .test_bel_attr_bits_bi(attr_d, val)
                        .attr("DFF", vname)
                        .commit();
                }
            }
            for (attr, aname) in [
                (SLICE::FFA_SRVAL, "AFFSR"),
                (SLICE::FFB_SRVAL, "BFFSR"),
                (SLICE::FFC_SRVAL, "CFFSR"),
                (SLICE::FFD_SRVAL, "DFFSR"),
            ] {
                bctx.mode("SLICEL")
                    .attr("AFF", "#FF")
                    .attr("BFF", "#FF")
                    .attr("CFF", "#FF")
                    .attr("DFF", "#FF")
                    .attr("AFFINIT", "INIT0")
                    .attr("BFFINIT", "INIT0")
                    .attr("CFFINIT", "INIT0")
                    .attr("DFFINIT", "INIT0")
                    .pin("AQ")
                    .pin("BQ")
                    .pin("CQ")
                    .pin("DQ")
                    .pin("CLK")
                    .test_bel_attr_bool_rename(aname, attr, "SRLOW", "SRHIGH");
            }
            for (attr, aname) in [
                (SLICE::FFA_INIT, "AFFINIT"),
                (SLICE::FFB_INIT, "BFFINIT"),
                (SLICE::FFC_INIT, "CFFINIT"),
                (SLICE::FFD_INIT, "DFFINIT"),
            ] {
                bctx.mode("SLICEL")
                    .attr("AFF", "#FF")
                    .attr("BFF", "#FF")
                    .attr("CFF", "#FF")
                    .attr("DFF", "#FF")
                    .attr("AFFSR", "SRLOW")
                    .attr("BFFSR", "SRLOW")
                    .attr("CFFSR", "SRLOW")
                    .attr("DFFSR", "SRLOW")
                    .pin("AQ")
                    .pin("BQ")
                    .pin("CQ")
                    .pin("DQ")
                    .pin("CLK")
                    .test_bel_attr_bool_rename(aname, attr, "INIT0", "INIT1");
            }
            if edev.kind != ChipKind::Virtex5 {
                bctx.mode("SLICEL")
                    .attr("AOUTMUX", "A5Q")
                    .attr("A5LUT", "#LUT:0")
                    .attr("A6LUT", "#LUT:0")
                    .attr("A5FFMUX", "IN_A")
                    .attr("A5FFINIT", "INIT0")
                    .pin("AMUX")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("A5FFSR", SLICE::FFA5_SRVAL, "SRLOW", "SRHIGH");
                bctx.mode("SLICEL")
                    .attr("BOUTMUX", "B5Q")
                    .attr("B5LUT", "#LUT:0")
                    .attr("B6LUT", "#LUT:0")
                    .attr("B5FFMUX", "IN_A")
                    .attr("B5FFINIT", "INIT0")
                    .pin("BMUX")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("B5FFSR", SLICE::FFB5_SRVAL, "SRLOW", "SRHIGH");
                bctx.mode("SLICEL")
                    .attr("COUTMUX", "C5Q")
                    .attr("C5LUT", "#LUT:0")
                    .attr("C6LUT", "#LUT:0")
                    .attr("C5FFMUX", "IN_A")
                    .attr("C5FFINIT", "INIT0")
                    .pin("CMUX")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("C5FFSR", SLICE::FFC5_SRVAL, "SRLOW", "SRHIGH");
                bctx.mode("SLICEL")
                    .attr("DOUTMUX", "D5Q")
                    .attr("D5LUT", "#LUT:0")
                    .attr("D6LUT", "#LUT:0")
                    .attr("D5FFMUX", "IN_A")
                    .attr("D5FFINIT", "INIT0")
                    .pin("DMUX")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("D5FFSR", SLICE::FFD5_SRVAL, "SRLOW", "SRHIGH");
                bctx.mode("SLICEL")
                    .attr("AOUTMUX", "A5Q")
                    .attr("A5LUT", "#LUT:0")
                    .attr("A6LUT", "#LUT:0")
                    .attr("A5FFMUX", "IN_A")
                    .attr("A5FFSR", "SRLOW")
                    .pin("AMUX")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("A5FFINIT", SLICE::FFA5_INIT, "INIT0", "INIT1");
                bctx.mode("SLICEL")
                    .attr("BOUTMUX", "B5Q")
                    .attr("B5LUT", "#LUT:0")
                    .attr("B6LUT", "#LUT:0")
                    .attr("B5FFMUX", "IN_A")
                    .attr("B5FFSR", "SRLOW")
                    .pin("BMUX")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("B5FFINIT", SLICE::FFB5_INIT, "INIT0", "INIT1");
                bctx.mode("SLICEL")
                    .attr("COUTMUX", "C5Q")
                    .attr("C5LUT", "#LUT:0")
                    .attr("C6LUT", "#LUT:0")
                    .attr("C5FFMUX", "IN_A")
                    .attr("C5FFSR", "SRLOW")
                    .pin("CMUX")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("C5FFINIT", SLICE::FFC5_INIT, "INIT0", "INIT1");
                bctx.mode("SLICEL")
                    .attr("DOUTMUX", "D5Q")
                    .attr("D5LUT", "#LUT:0")
                    .attr("D6LUT", "#LUT:0")
                    .attr("D5FFMUX", "IN_A")
                    .attr("D5FFSR", "SRLOW")
                    .pin("DMUX")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("D5FFINIT", SLICE::FFD5_INIT, "INIT0", "INIT1");
            }
            if edev.kind == ChipKind::Virtex5 {
                bctx.mode("SLICEL")
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("CE")
                    .pin("CLK")
                    .test_bel_attr_bits(SLICE::FF_CE_ENABLE)
                    .attr("CEUSED", "0")
                    .commit();
                bctx.mode("SLICEL")
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("SR")
                    .pin("CLK")
                    .test_bel_attr_bits(SLICE::FF_SR_ENABLE)
                    .attr("SRUSED", "0")
                    .commit();
            } else {
                bctx.mode("SLICEL")
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("CE")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("CEUSEDMUX", SLICE::FF_CE_ENABLE, "1", "IN");
                bctx.mode("SLICEL")
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("SR")
                    .pin("CLK")
                    .test_bel_attr_bool_rename("SRUSEDMUX", SLICE::FF_SR_ENABLE, "0", "IN");
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let (tcid_clbll, tcid_clblm) = match edev.kind {
        ChipKind::Virtex5 => (tcls_v5::CLBLL, tcls_v5::CLBLM),
        ChipKind::Virtex6 => (tcls_v6::CLBLL, tcls_v6::CLBLM),
        ChipKind::Virtex7 => (tcls_v7::CLBLL, tcls_v7::CLBLM),
        _ => unreachable!(),
    };
    for tcid in [tcid_clbll, tcid_clblm] {
        for idx in 0..2 {
            let bslot = bslots::SLICE[idx];
            let is_m = idx == 0 && tcid == tcid_clblm;

            // LUTs
            ctx.collect_bel_attr(tcid, bslot, SLICE::A6LUT);
            ctx.collect_bel_attr(tcid, bslot, SLICE::B6LUT);
            ctx.collect_bel_attr(tcid, bslot, SLICE::C6LUT);
            ctx.collect_bel_attr(tcid, bslot, SLICE::D6LUT);

            // LUT RAM
            if is_m {
                ctx.collect_bel_attr(tcid, bslot, SLICE::MUX_WE);
                ctx.collect_bel_attr(tcid, bslot, SLICE::WA7USED);
                ctx.collect_bel_attr(tcid, bslot, SLICE::WA8USED);
                if edev.kind == ChipKind::Virtex5 {
                    ctx.collect_bel_attr_subset(
                        tcid,
                        bslot,
                        SLICE::MUX_ADI1,
                        &[enums::SLICE_MUX_ADI1::ALT, enums::SLICE_MUX_ADI1::AX],
                    );
                    ctx.collect_bel_attr_subset(
                        tcid,
                        bslot,
                        SLICE::MUX_BDI1,
                        &[enums::SLICE_MUX_BDI1::ALT, enums::SLICE_MUX_BDI1::BX],
                    );
                    ctx.collect_bel_attr_subset(
                        tcid,
                        bslot,
                        SLICE::MUX_CDI1,
                        &[enums::SLICE_MUX_CDI1::ALT, enums::SLICE_MUX_CDI1::CX],
                    );
                } else {
                    ctx.collect_bel_attr_subset(
                        tcid,
                        bslot,
                        SLICE::MUX_ADI1,
                        &[enums::SLICE_MUX_ADI1::ALT, enums::SLICE_MUX_ADI1::AI],
                    );
                    ctx.collect_bel_attr_subset(
                        tcid,
                        bslot,
                        SLICE::MUX_BDI1,
                        &[enums::SLICE_MUX_BDI1::ALT, enums::SLICE_MUX_BDI1::BI],
                    );
                    ctx.collect_bel_attr_subset(
                        tcid,
                        bslot,
                        SLICE::MUX_CDI1,
                        &[enums::SLICE_MUX_CDI1::ALT, enums::SLICE_MUX_CDI1::CI],
                    );
                }
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    SLICE::ARAMMODE,
                    enums::SLICE_RAMMODE::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    SLICE::BRAMMODE,
                    enums::SLICE_RAMMODE::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    SLICE::CRAMMODE,
                    enums::SLICE_RAMMODE::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    SLICE::DRAMMODE,
                    enums::SLICE_RAMMODE::NONE,
                );
            }

            // carry chain
            ctx.collect_bel_attr(tcid, bslot, SLICE::MUX_ACY0);
            ctx.collect_bel_attr(tcid, bslot, SLICE::MUX_BCY0);
            ctx.collect_bel_attr(tcid, bslot, SLICE::MUX_CCY0);
            ctx.collect_bel_attr(tcid, bslot, SLICE::MUX_DCY0);
            ctx.collect_bel_attr(tcid, bslot, SLICE::PRECYINIT);
            ctx.collect_bel_attr_default(
                tcid,
                bslot,
                SLICE::CYINIT,
                enums::SLICE_CYINIT::PRECYINIT,
            );

            // misc muxes
            if edev.kind == ChipKind::Virtex5 {
                ctx.collect_bel_attr_subset_default_ocd(
                    tcid,
                    bslot,
                    SLICE::MUX_AOUT,
                    &[
                        enums::SLICE_MUX_AOUT::O6,
                        enums::SLICE_MUX_AOUT::O5,
                        enums::SLICE_MUX_AOUT::XOR,
                        enums::SLICE_MUX_AOUT::CY,
                        enums::SLICE_MUX_AOUT::F7,
                    ],
                    enums::SLICE_MUX_AOUT::NONE,
                    OcdMode::Mux,
                );
                ctx.collect_bel_attr_subset_default_ocd(
                    tcid,
                    bslot,
                    SLICE::MUX_BOUT,
                    &[
                        enums::SLICE_MUX_BOUT::O6,
                        enums::SLICE_MUX_BOUT::O5,
                        enums::SLICE_MUX_BOUT::XOR,
                        enums::SLICE_MUX_BOUT::CY,
                        enums::SLICE_MUX_BOUT::F8,
                    ],
                    enums::SLICE_MUX_BOUT::NONE,
                    OcdMode::Mux,
                );
                ctx.collect_bel_attr_subset_default_ocd(
                    tcid,
                    bslot,
                    SLICE::MUX_COUT,
                    &[
                        enums::SLICE_MUX_COUT::O6,
                        enums::SLICE_MUX_COUT::O5,
                        enums::SLICE_MUX_COUT::XOR,
                        enums::SLICE_MUX_COUT::CY,
                        enums::SLICE_MUX_COUT::F7,
                    ],
                    enums::SLICE_MUX_COUT::NONE,
                    OcdMode::Mux,
                );
                if is_m {
                    ctx.collect_bel_attr_subset_default_ocd(
                        tcid,
                        bslot,
                        SLICE::MUX_DOUT,
                        &[
                            enums::SLICE_MUX_DOUT::O6,
                            enums::SLICE_MUX_DOUT::O5,
                            enums::SLICE_MUX_DOUT::XOR,
                            enums::SLICE_MUX_DOUT::CY,
                            enums::SLICE_MUX_DOUT::MC31,
                        ],
                        enums::SLICE_MUX_DOUT::NONE,
                        OcdMode::Mux,
                    );
                } else {
                    ctx.collect_bel_attr_subset_default_ocd(
                        tcid,
                        bslot,
                        SLICE::MUX_DOUT,
                        &[
                            enums::SLICE_MUX_DOUT::O6,
                            enums::SLICE_MUX_DOUT::O5,
                            enums::SLICE_MUX_DOUT::XOR,
                            enums::SLICE_MUX_DOUT::CY,
                        ],
                        enums::SLICE_MUX_DOUT::NONE,
                        OcdMode::Mux,
                    );
                }
            } else {
                ctx.collect_bel_attr_default_ocd(
                    tcid,
                    bslot,
                    SLICE::MUX_AOUT,
                    enums::SLICE_MUX_AOUT::NONE,
                    OcdMode::Mux,
                );
                ctx.collect_bel_attr_default_ocd(
                    tcid,
                    bslot,
                    SLICE::MUX_BOUT,
                    enums::SLICE_MUX_BOUT::NONE,
                    OcdMode::Mux,
                );
                ctx.collect_bel_attr_default_ocd(
                    tcid,
                    bslot,
                    SLICE::MUX_COUT,
                    enums::SLICE_MUX_COUT::NONE,
                    OcdMode::Mux,
                );
                if is_m {
                    ctx.collect_bel_attr_default_ocd(
                        tcid,
                        bslot,
                        SLICE::MUX_DOUT,
                        enums::SLICE_MUX_DOUT::NONE,
                        OcdMode::Mux,
                    );
                } else {
                    ctx.collect_bel_attr_subset_default_ocd(
                        tcid,
                        bslot,
                        SLICE::MUX_DOUT,
                        &[
                            enums::SLICE_MUX_DOUT::O6,
                            enums::SLICE_MUX_DOUT::O5,
                            enums::SLICE_MUX_DOUT::XOR,
                            enums::SLICE_MUX_DOUT::CY,
                            enums::SLICE_MUX_DOUT::D5Q,
                        ],
                        enums::SLICE_MUX_DOUT::NONE,
                        OcdMode::Mux,
                    );
                }
            }

            ctx.collect_bel_attr_default_ocd(
                tcid,
                bslot,
                SLICE::MUX_FFA,
                enums::SLICE_MUX_FFA::NONE,
                OcdMode::Mux,
            );
            ctx.collect_bel_attr_default_ocd(
                tcid,
                bslot,
                SLICE::MUX_FFB,
                enums::SLICE_MUX_FFB::NONE,
                OcdMode::Mux,
            );
            ctx.collect_bel_attr_default_ocd(
                tcid,
                bslot,
                SLICE::MUX_FFC,
                enums::SLICE_MUX_FFC::NONE,
                OcdMode::Mux,
            );
            if is_m {
                ctx.collect_bel_attr_default_ocd(
                    tcid,
                    bslot,
                    SLICE::MUX_FFD,
                    enums::SLICE_MUX_FFD::NONE,
                    OcdMode::Mux,
                );
            } else {
                ctx.collect_bel_attr_subset_default_ocd(
                    tcid,
                    bslot,
                    SLICE::MUX_FFD,
                    &[
                        enums::SLICE_MUX_FFD::O6,
                        enums::SLICE_MUX_FFD::O5,
                        enums::SLICE_MUX_FFD::XOR,
                        enums::SLICE_MUX_FFD::CY,
                        enums::SLICE_MUX_FFD::DX,
                    ],
                    enums::SLICE_MUX_FFD::NONE,
                    OcdMode::Mux,
                );
            }

            if matches!(edev.kind, ChipKind::Virtex6 | ChipKind::Virtex7) {
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    SLICE::MUX_FFA5,
                    enums::SLICE_MUX_FFA5::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    SLICE::MUX_FFB5,
                    enums::SLICE_MUX_FFB5::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    SLICE::MUX_FFC5,
                    enums::SLICE_MUX_FFC5::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    SLICE::MUX_FFD5,
                    enums::SLICE_MUX_FFD5::NONE,
                );
            }

            // FFs
            ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FF_SR_SYNC);
            ctx.collect_bel_input_inv_bi(tcid, bslot, SLICE::CLK);
            if edev.kind == ChipKind::Virtex5 {
                ctx.collect_bel_attr(tcid, bslot, SLICE::FF_REV_ENABLE);
            }
            if matches!(edev.kind, ChipKind::Virtex5) {
                ctx.collect_bel_attr(tcid, bslot, SLICE::FF_CE_ENABLE);
                ctx.collect_bel_attr(tcid, bslot, SLICE::FF_SR_ENABLE);
            } else {
                ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FF_CE_ENABLE);
                ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FF_SR_ENABLE);
            }
            if edev.kind != ChipKind::Virtex6 {
                ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FF_LATCH);
            } else {
                ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FFA_LATCH);
                ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FFB_LATCH);
                ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FFC_LATCH);
                ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FFD_LATCH);
            }
            for attr in [
                SLICE::FFA_INIT,
                SLICE::FFB_INIT,
                SLICE::FFC_INIT,
                SLICE::FFD_INIT,
                SLICE::FFA_SRVAL,
                SLICE::FFB_SRVAL,
                SLICE::FFC_SRVAL,
                SLICE::FFD_SRVAL,
            ] {
                ctx.collect_bel_attr_bi(tcid, bslot, attr);
            }

            if let ChipKind::Virtex6 | ChipKind::Virtex7 = edev.kind {
                for attr in [
                    SLICE::FFA5_INIT,
                    SLICE::FFB5_INIT,
                    SLICE::FFC5_INIT,
                    SLICE::FFD5_INIT,
                    SLICE::FFA5_SRVAL,
                    SLICE::FFB5_SRVAL,
                    SLICE::FFC5_SRVAL,
                    SLICE::FFD5_SRVAL,
                ] {
                    ctx.collect_bel_attr_bi(tcid, bslot, attr);
                }
            }
        }
    }
}

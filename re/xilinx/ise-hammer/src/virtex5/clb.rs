use prjcombine_entity::EntityId;
use prjcombine_re_collector::{
    diff::{Diff, OcdMode},
    legacy::{xlat_bit_legacy, xlat_enum_legacy},
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::FuzzCtx,
        props::{pip::PinFar, relation::TileRelation},
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Virtex5,
    Virtex6,
    Virtex7,
    Spartan6,
}

#[derive(Copy, Clone, Debug)]
struct ClbCinDown;

impl TileRelation for ClbCinDown {
    fn resolve(
        &self,
        backend: &IseBackend,
        mut tcrd: prjcombine_interconnect::grid::TileCoord,
    ) -> Option<prjcombine_interconnect::grid::TileCoord> {
        loop {
            if tcrd.row.to_idx() == 0 {
                return None;
            }
            tcrd.row -= 1;
            if let Some(ntcrd) = backend.edev.find_tile_by_class(tcrd.cell, |kind| {
                kind.starts_with("CLB") || kind.starts_with("CLEX")
            }) {
                return Some(ntcrd);
            }
            if !matches!(backend.edev, ExpandedDevice::Spartan6(_)) {
                return None;
            }
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mode = match backend.edev {
        ExpandedDevice::Virtex4(edev) => match edev.kind {
            prjcombine_virtex4::chip::ChipKind::Virtex4 => unreachable!(),
            prjcombine_virtex4::chip::ChipKind::Virtex5 => Mode::Virtex5,
            prjcombine_virtex4::chip::ChipKind::Virtex6 => Mode::Virtex6,
            prjcombine_virtex4::chip::ChipKind::Virtex7 => Mode::Virtex7,
        },
        ExpandedDevice::Spartan6(_) => Mode::Spartan6,
        _ => unreachable!(),
    };
    let bels = if mode == Mode::Spartan6 {
        [
            prjcombine_spartan6::defs::bslots::SLICE[0],
            prjcombine_spartan6::defs::bslots::SLICE[1],
        ]
    } else {
        [
            prjcombine_virtex4::defs::bslots::SLICE[0],
            prjcombine_virtex4::defs::bslots::SLICE[1],
        ]
    };

    for tile_name in if mode == Mode::Spartan6 {
        ["CLEXL", "CLEXM"]
    } else {
        ["CLBLL", "CLBLM"]
    } {
        let Some(mut ctx) = FuzzCtx::try_new_legacy(session, backend, tile_name) else {
            continue;
        };
        let bk_x = if mode == Mode::Spartan6 {
            "SLICEX"
        } else {
            "SLICEL"
        };
        for i in 0..2 {
            let bel = bels[i];
            let mut bctx = ctx.bel(bel);
            let is_x = i == 1 && mode == Mode::Spartan6;
            let is_m = i == 0 && tile_name.ends_with('M');

            // LUTs
            for attr in ["A6LUT", "B6LUT", "C6LUT", "D6LUT"] {
                bctx.mode(bk_x).test_multi_attr_lut(attr, 64);
            }

            if is_m {
                // LUT RAM
                bctx.mode("SLICEM")
                    .attr("A6LUT", "#RAM:0")
                    .attr("A6RAMMODE", "SPRAM64")
                    .pin("WE")
                    .pin("CE")
                    .test_enum_legacy("WEMUX", &["WE", "CE"]);
                for attr in ["WA7USED", "WA8USED"] {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#RAM:0")
                        .attr("A6RAMMODE", "SPRAM64")
                        .pin("AX")
                        .pin("BX")
                        .pin("CX")
                        .pin("DX")
                        .test_enum_legacy(attr, &["0"]);
                }
                if matches!(mode, Mode::Virtex5 | Mode::Spartan6) {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#RAM:0")
                        .attr("A6RAMMODE", "SPRAM64")
                        .pin("AX")
                        .test_enum_legacy("ADI1MUX", &["AX", "BMC31", "BDI1"]);
                    bctx.mode("SLICEM")
                        .attr("B6LUT", "#RAM:0")
                        .attr("B6RAMMODE", "SPRAM64")
                        .pin("BX")
                        .pin("DX")
                        .test_enum_legacy("BDI1MUX", &["BX", "CMC31", "DX"]);
                    bctx.mode("SLICEM")
                        .attr("C6LUT", "#RAM:0")
                        .attr("C6RAMMODE", "SPRAM64")
                        .pin("CX")
                        .pin("DX")
                        .test_enum_legacy("CDI1MUX", &["CX", "DMC31", "DX"]);
                } else {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#RAM:0")
                        .attr("A6RAMMODE", "SPRAM64")
                        .pin("AI")
                        .test_enum_legacy("ADI1MUX", &["AI", "BMC31", "BDI1"]);
                    bctx.mode("SLICEM")
                        .attr("B6LUT", "#RAM:0")
                        .attr("B6RAMMODE", "SPRAM64")
                        .pin("BI")
                        .pin("DI")
                        .test_enum_legacy("BDI1MUX", &["BI", "CMC31", "DI"]);
                    bctx.mode("SLICEM")
                        .attr("C6LUT", "#RAM:0")
                        .attr("C6RAMMODE", "SPRAM64")
                        .pin("CI")
                        .pin("DI")
                        .test_enum_legacy("CDI1MUX", &["CI", "DMC31", "DI"]);
                }
                bctx.mode("SLICEM")
                    .attr("A6LUT", "#RAM:0")
                    .test_enum_legacy(
                        "A6RAMMODE",
                        &["SPRAM32", "SPRAM64", "DPRAM32", "DPRAM64", "SRL16", "SRL32"],
                    );
                bctx.mode("SLICEM")
                    .attr("B6LUT", "#RAM:0")
                    .test_enum_legacy(
                        "B6RAMMODE",
                        &["SPRAM32", "SPRAM64", "DPRAM32", "DPRAM64", "SRL16", "SRL32"],
                    );
                bctx.mode("SLICEM")
                    .attr("C6LUT", "#RAM:0")
                    .test_enum_legacy(
                        "C6RAMMODE",
                        &["SPRAM32", "SPRAM64", "DPRAM32", "DPRAM64", "SRL16", "SRL32"],
                    );
                bctx.mode("SLICEM")
                    .attr("D6LUT", "#RAM:0")
                    .test_enum_legacy(
                        "D6RAMMODE",
                        &["SPRAM32", "SPRAM64", "DPRAM32", "DPRAM64", "SRL16", "SRL32"],
                    );
            }

            if !is_x {
                // carry chain
                bctx.mode("SLICEL")
                    .attr("A5LUT", "#LUT:0")
                    .attr("A6LUT", "#LUT:0")
                    .attr("COUTUSED", "0")
                    .pin("AX")
                    .pin("COUT")
                    .test_enum_legacy("ACY0", &["AX", "O5"]);
                bctx.mode("SLICEL")
                    .attr("B5LUT", "#LUT:0")
                    .attr("B6LUT", "#LUT:0")
                    .attr("COUTUSED", "0")
                    .pin("BX")
                    .pin("COUT")
                    .test_enum_legacy("BCY0", &["BX", "O5"]);
                bctx.mode("SLICEL")
                    .attr("C5LUT", "#LUT:0")
                    .attr("C6LUT", "#LUT:0")
                    .attr("COUTUSED", "0")
                    .pin("CX")
                    .pin("COUT")
                    .test_enum_legacy("CCY0", &["CX", "O5"]);
                bctx.mode("SLICEL")
                    .attr("D5LUT", "#LUT:0")
                    .attr("D6LUT", "#LUT:0")
                    .attr("COUTUSED", "0")
                    .pin("DX")
                    .pin("COUT")
                    .test_enum_legacy("DCY0", &["DX", "O5"]);
                bctx.mode("SLICEL")
                    .attr("COUTUSED", "0")
                    .pin("AX")
                    .pin("COUT")
                    .test_enum_legacy("PRECYINIT", &["AX", "1", "0"]);

                bctx.test_manual_legacy("CINUSED", "1")
                    .related_pip(ClbCinDown, (PinFar, "COUT"), "COUT")
                    .commit();
            }

            // misc muxes
            if is_x {
                bctx.mode("SLICEX")
                    .attr("A6LUT", "#LUT:0")
                    .attr("A5LUT", "#LUT:0")
                    .pin("AMUX")
                    .test_enum_legacy("AOUTMUX", &["A5Q", "O5"]);
                bctx.mode("SLICEX")
                    .attr("B6LUT", "#LUT:0")
                    .attr("B5LUT", "#LUT:0")
                    .pin("BMUX")
                    .test_enum_legacy("BOUTMUX", &["B5Q", "O5"]);
                bctx.mode("SLICEX")
                    .attr("C6LUT", "#LUT:0")
                    .attr("C5LUT", "#LUT:0")
                    .pin("CMUX")
                    .test_enum_legacy("COUTMUX", &["C5Q", "O5"]);
                bctx.mode("SLICEX")
                    .attr("D6LUT", "#LUT:0")
                    .attr("D5LUT", "#LUT:0")
                    .pin("DMUX")
                    .test_enum_legacy("DOUTMUX", &["D5Q", "O5"]);
                bctx.mode("SLICEX")
                    .attr("A6LUT", "#LUT:0")
                    .attr("AFF", "#FF")
                    .pin("AX")
                    .pin("AQ")
                    .pin("CLK")
                    .test_enum_legacy("AFFMUX", &["AX", "O6"]);
                bctx.mode("SLICEX")
                    .attr("B6LUT", "#LUT:0")
                    .attr("BFF", "#FF")
                    .pin("BX")
                    .pin("BQ")
                    .pin("CLK")
                    .test_enum_legacy("BFFMUX", &["BX", "O6"]);
                bctx.mode("SLICEX")
                    .attr("C6LUT", "#LUT:0")
                    .attr("CFF", "#FF")
                    .pin("CX")
                    .pin("CQ")
                    .pin("CLK")
                    .test_enum_legacy("CFFMUX", &["CX", "O6"]);
                bctx.mode("SLICEX")
                    .attr("D6LUT", "#LUT:0")
                    .attr("DFF", "#FF")
                    .pin("DX")
                    .pin("DQ")
                    .pin("CLK")
                    .test_enum_legacy("DFFMUX", &["DX", "O6"]);
            } else {
                // [ABCD]MUX
                if mode == Mode::Virtex5 {
                    bctx.mode("SLICEL")
                        .attr("A6LUT", "#LUT:0")
                        .attr("A5LUT", "#LUT:0")
                        .pin("AMUX")
                        .test_enum_legacy("AOUTMUX", &["O5", "O6", "XOR", "CY", "F7"]);
                    bctx.mode("SLICEL")
                        .attr("B6LUT", "#LUT:0")
                        .attr("B5LUT", "#LUT:0")
                        .pin("BMUX")
                        .test_enum_legacy("BOUTMUX", &["O5", "O6", "XOR", "CY", "F8"]);
                    bctx.mode("SLICEL")
                        .attr("C6LUT", "#LUT:0")
                        .attr("C5LUT", "#LUT:0")
                        .pin("CMUX")
                        .test_enum_legacy("COUTMUX", &["O5", "O6", "XOR", "CY", "F7"]);
                    if is_m {
                        bctx.mode("SLICEM")
                            .attr("A6LUT", "#LUT:0")
                            .attr("D6LUT", "#LUT:0")
                            .attr("D5LUT", "#LUT:0")
                            .pin("DMUX")
                            .test_enum_legacy("DOUTMUX", &["O5", "O6", "XOR", "CY", "MC31"]);
                    } else {
                        bctx.mode("SLICEL")
                            .attr("D6LUT", "#LUT:0")
                            .attr("D5LUT", "#LUT:0")
                            .pin("DMUX")
                            .test_enum_legacy("DOUTMUX", &["O5", "O6", "XOR", "CY"]);
                    }
                } else {
                    bctx.mode("SLICEL")
                        .attr("A6LUT", "#LUT:0")
                        .attr("A5LUT", "#LUT:0")
                        .attr("A5FFMUX", "")
                        .attr("CLKINV", "CLK")
                        .pin("AMUX")
                        .pin("CLK")
                        .test_enum_legacy("AOUTMUX", &["O5", "O6", "XOR", "CY", "A5Q", "F7"]);
                    bctx.mode("SLICEL")
                        .attr("B6LUT", "#LUT:0")
                        .attr("B5LUT", "#LUT:0")
                        .attr("B5FFMUX", "")
                        .attr("CLKINV", "CLK")
                        .pin("BMUX")
                        .pin("CLK")
                        .test_enum_legacy("BOUTMUX", &["O5", "O6", "XOR", "CY", "B5Q", "F8"]);
                    bctx.mode("SLICEL")
                        .attr("C6LUT", "#LUT:0")
                        .attr("C5LUT", "#LUT:0")
                        .attr("C5FFMUX", "")
                        .attr("CLKINV", "CLK")
                        .pin("CMUX")
                        .pin("CLK")
                        .test_enum_legacy("COUTMUX", &["O5", "O6", "XOR", "CY", "C5Q", "F7"]);
                    if is_m {
                        bctx.mode("SLICEM")
                            .attr("A6LUT", "#LUT:0")
                            .attr("D6LUT", "#LUT:0")
                            .attr("D5LUT", "#LUT:0")
                            .attr("D5FFMUX", "")
                            .attr("CLKINV", "CLK")
                            .pin("DMUX")
                            .pin("CLK")
                            .test_enum_legacy("DOUTMUX", &["O5", "O6", "XOR", "CY", "D5Q", "MC31"]);
                    } else {
                        bctx.mode("SLICEL")
                            .attr("D6LUT", "#LUT:0")
                            .attr("D5LUT", "#LUT:0")
                            .attr("D5FFMUX", "")
                            .attr("CLKINV", "CLK")
                            .pin("DMUX")
                            .pin("CLK")
                            .test_enum_legacy("DOUTMUX", &["O5", "O6", "XOR", "CY", "D5Q"]);
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
                    .test_enum_legacy("AFFMUX", &["O5", "O6", "XOR", "CY", "AX", "F7"]);
                bctx.mode("SLICEL")
                    .attr("B6LUT", "#LUT:0")
                    .attr("B5LUT", "#LUT:0")
                    .attr("BFF", "#FF")
                    .attr("CLKINV", "CLK")
                    .pin("BX")
                    .pin("BQ")
                    .pin("CLK")
                    .test_enum_legacy("BFFMUX", &["O5", "O6", "XOR", "CY", "BX", "F8"]);
                bctx.mode("SLICEL")
                    .attr("C6LUT", "#LUT:0")
                    .attr("C5LUT", "#LUT:0")
                    .attr("CFF", "#FF")
                    .attr("CLKINV", "CLK")
                    .pin("CX")
                    .pin("CQ")
                    .pin("CLK")
                    .test_enum_legacy("CFFMUX", &["O5", "O6", "XOR", "CY", "CX", "F7"]);
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
                        .test_enum_legacy("DFFMUX", &["O5", "O6", "XOR", "CY", "DX", "MC31"]);
                } else {
                    bctx.mode("SLICEL")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5LUT", "#LUT:0")
                        .attr("DFF", "#FF")
                        .attr("CLKINV", "CLK")
                        .pin("DX")
                        .pin("DQ")
                        .pin("CLK")
                        .test_enum_legacy("DFFMUX", &["O5", "O6", "XOR", "CY", "DX"]);
                }
                if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                    bctx.mode("SLICEL")
                        .attr("A6LUT", "#LUT:0")
                        .attr("A5LUT", "#LUT:0")
                        .attr("AOUTMUX", "A5Q")
                        .attr("CLKINV", "CLK")
                        .pin("AX")
                        .pin("AMUX")
                        .pin("CLK")
                        .test_enum_legacy("A5FFMUX", &["IN_A", "IN_B"]);
                    bctx.mode("SLICEL")
                        .attr("B6LUT", "#LUT:0")
                        .attr("B5LUT", "#LUT:0")
                        .attr("BOUTMUX", "B5Q")
                        .attr("CLKINV", "CLK")
                        .pin("BX")
                        .pin("BMUX")
                        .pin("CLK")
                        .test_enum_legacy("B5FFMUX", &["IN_A", "IN_B"]);
                    bctx.mode("SLICEL")
                        .attr("C6LUT", "#LUT:0")
                        .attr("C5LUT", "#LUT:0")
                        .attr("COUTMUX", "C5Q")
                        .attr("CLKINV", "CLK")
                        .pin("CX")
                        .pin("CMUX")
                        .pin("CLK")
                        .test_enum_legacy("C5FFMUX", &["IN_A", "IN_B"]);
                    bctx.mode("SLICEL")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5LUT", "#LUT:0")
                        .attr("DOUTMUX", "D5Q")
                        .attr("CLKINV", "CLK")
                        .pin("DX")
                        .pin("DMUX")
                        .pin("CLK")
                        .test_enum_legacy("D5FFMUX", &["IN_A", "IN_B"]);
                }
            }

            // FFs
            bctx.mode(bk_x)
                .attr("AFF", "#FF")
                .pin("AQ")
                .test_enum_legacy("SYNC_ATTR", &["SYNC", "ASYNC"]);
            bctx.mode(bk_x)
                .attr("AFF", "#FF")
                .pin("AQ")
                .test_inv_legacy("CLK");
            match mode {
                Mode::Virtex5 => {
                    bctx.mode(bk_x)
                        .attr("AFF", "#FF")
                        .pin("AQ")
                        .pin("DX")
                        .pin("CLK")
                        .test_enum_legacy("REVUSED", &["0"]);
                    bctx.mode(bk_x)
                        .attr("AFFINIT", "INIT1")
                        .attr("BFF", "")
                        .attr("CFF", "")
                        .attr("DFF", "")
                        .pin("AQ")
                        .pin("CLK")
                        .test_enum_legacy("AFF", &["#LATCH", "#FF"]);
                    bctx.mode(bk_x)
                        .attr("BFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("CFF", "")
                        .attr("DFF", "")
                        .pin("BQ")
                        .pin("CLK")
                        .test_enum_legacy("BFF", &["#LATCH", "#FF"]);
                    bctx.mode(bk_x)
                        .attr("CFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("BFF", "")
                        .attr("DFF", "")
                        .pin("CQ")
                        .pin("CLK")
                        .test_enum_legacy("CFF", &["#LATCH", "#FF"]);
                    bctx.mode(bk_x)
                        .attr("DFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("BFF", "")
                        .attr("CFF", "")
                        .pin("DQ")
                        .pin("CLK")
                        .test_enum_legacy("DFF", &["#LATCH", "#FF"]);
                    for attr in ["AFFSR", "BFFSR", "CFFSR", "DFFSR"] {
                        bctx.mode(bk_x)
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
                            .test_enum_legacy(attr, &["SRHIGH", "SRLOW"]);
                    }
                    for attr in ["AFFINIT", "BFFINIT", "CFFINIT", "DFFINIT"] {
                        bctx.mode(bk_x)
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
                            .test_enum_legacy(attr, &["INIT0", "INIT1"]);
                    }
                }
                Mode::Spartan6 => {
                    bctx.mode(bk_x)
                        .attr("BFF", "")
                        .attr("CFF", "")
                        .attr("DFF", "")
                        .pin("AQ")
                        .pin("CLK")
                        .test_enum_legacy("AFF", &["#LATCH", "#FF", "AND2L", "OR2L"]);
                    bctx.mode(bk_x)
                        .attr("AFF", "")
                        .attr("CFF", "")
                        .attr("DFF", "")
                        .pin("BQ")
                        .pin("CLK")
                        .test_enum_legacy("BFF", &["#LATCH", "#FF", "AND2L", "OR2L"]);
                    bctx.mode(bk_x)
                        .attr("AFF", "")
                        .attr("BFF", "")
                        .attr("DFF", "")
                        .pin("CQ")
                        .pin("CLK")
                        .test_enum_legacy("CFF", &["#LATCH", "#FF", "AND2L", "OR2L"]);
                    bctx.mode(bk_x)
                        .attr("AFF", "")
                        .attr("BFF", "")
                        .attr("CFF", "")
                        .pin("DQ")
                        .pin("CLK")
                        .test_enum_legacy("DFF", &["#LATCH", "#FF", "AND2L", "OR2L"]);
                    for attr in ["AFFSRINIT", "BFFSRINIT", "CFFSRINIT", "DFFSRINIT"] {
                        bctx.mode(bk_x)
                            .attr("AFF", "#FF")
                            .attr("BFF", "#FF")
                            .attr("CFF", "#FF")
                            .attr("DFF", "#FF")
                            .pin("AQ")
                            .pin("BQ")
                            .pin("CQ")
                            .pin("DQ")
                            .pin("CLK")
                            .test_enum_legacy(attr, &["SRINIT0", "SRINIT1"]);
                    }
                    bctx.mode(bk_x)
                        .attr("AOUTMUX", "A5Q")
                        .attr("A5LUT", "#LUT:0")
                        .attr("A6LUT", "#LUT:0")
                        .pin("AMUX")
                        .pin("CLK")
                        .test_enum_legacy("A5FFSRINIT", &["SRINIT0", "SRINIT1"]);
                    bctx.mode(bk_x)
                        .attr("BOUTMUX", "B5Q")
                        .attr("B5LUT", "#LUT:0")
                        .attr("B6LUT", "#LUT:0")
                        .pin("BMUX")
                        .pin("CLK")
                        .test_enum_legacy("B5FFSRINIT", &["SRINIT0", "SRINIT1"]);
                    bctx.mode(bk_x)
                        .attr("COUTMUX", "C5Q")
                        .attr("C5LUT", "#LUT:0")
                        .attr("C6LUT", "#LUT:0")
                        .pin("CMUX")
                        .pin("CLK")
                        .test_enum_legacy("C5FFSRINIT", &["SRINIT0", "SRINIT1"]);
                    bctx.mode(bk_x)
                        .attr("DOUTMUX", "D5Q")
                        .attr("D5LUT", "#LUT:0")
                        .attr("D6LUT", "#LUT:0")
                        .pin("DMUX")
                        .pin("CLK")
                        .test_enum_legacy("D5FFSRINIT", &["SRINIT0", "SRINIT1"]);
                }
                Mode::Virtex6 | Mode::Virtex7 => {
                    bctx.mode(bk_x)
                        .attr("AFFINIT", "INIT1")
                        .attr("BFF", "")
                        .attr("CFF", "")
                        .attr("DFF", "")
                        .pin("AQ")
                        .pin("CLK")
                        .test_enum_legacy("AFF", &["#LATCH", "#FF", "AND2L", "OR2L"]);
                    bctx.mode(bk_x)
                        .attr("BFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("CFF", "")
                        .attr("DFF", "")
                        .pin("BQ")
                        .pin("CLK")
                        .test_enum_legacy("BFF", &["#LATCH", "#FF", "AND2L", "OR2L"]);
                    bctx.mode(bk_x)
                        .attr("CFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("BFF", "")
                        .attr("DFF", "")
                        .pin("CQ")
                        .pin("CLK")
                        .test_enum_legacy("CFF", &["#LATCH", "#FF", "AND2L", "OR2L"]);
                    bctx.mode(bk_x)
                        .attr("DFFINIT", "INIT1")
                        .attr("AFF", "")
                        .attr("BFF", "")
                        .attr("CFF", "")
                        .pin("DQ")
                        .pin("CLK")
                        .test_enum_legacy("DFF", &["#LATCH", "#FF", "AND2L", "OR2L"]);

                    for attr in ["AFFSR", "BFFSR", "CFFSR", "DFFSR"] {
                        bctx.mode(bk_x)
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
                            .test_enum_legacy(attr, &["SRHIGH", "SRLOW"]);
                    }
                    for attr in ["AFFINIT", "BFFINIT", "CFFINIT", "DFFINIT"] {
                        bctx.mode(bk_x)
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
                            .test_enum_legacy(attr, &["INIT0", "INIT1"]);
                    }
                    bctx.mode(bk_x)
                        .attr("AOUTMUX", "A5Q")
                        .attr("A5LUT", "#LUT:0")
                        .attr("A6LUT", "#LUT:0")
                        .attr("A5FFMUX", "IN_A")
                        .attr("A5FFINIT", "INIT0")
                        .pin("AMUX")
                        .pin("CLK")
                        .test_enum_legacy("A5FFSR", &["SRLOW", "SRHIGH"]);
                    bctx.mode(bk_x)
                        .attr("BOUTMUX", "B5Q")
                        .attr("B5LUT", "#LUT:0")
                        .attr("B6LUT", "#LUT:0")
                        .attr("B5FFMUX", "IN_A")
                        .attr("B5FFINIT", "INIT0")
                        .pin("BMUX")
                        .pin("CLK")
                        .test_enum_legacy("B5FFSR", &["SRLOW", "SRHIGH"]);
                    bctx.mode(bk_x)
                        .attr("COUTMUX", "C5Q")
                        .attr("C5LUT", "#LUT:0")
                        .attr("C6LUT", "#LUT:0")
                        .attr("C5FFMUX", "IN_A")
                        .attr("C5FFINIT", "INIT0")
                        .pin("CMUX")
                        .pin("CLK")
                        .test_enum_legacy("C5FFSR", &["SRLOW", "SRHIGH"]);
                    bctx.mode(bk_x)
                        .attr("DOUTMUX", "D5Q")
                        .attr("D5LUT", "#LUT:0")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5FFMUX", "IN_A")
                        .attr("D5FFINIT", "INIT0")
                        .pin("DMUX")
                        .pin("CLK")
                        .test_enum_legacy("D5FFSR", &["SRLOW", "SRHIGH"]);
                    bctx.mode(bk_x)
                        .attr("AOUTMUX", "A5Q")
                        .attr("A5LUT", "#LUT:0")
                        .attr("A6LUT", "#LUT:0")
                        .attr("A5FFMUX", "IN_A")
                        .attr("A5FFSR", "SRLOW")
                        .pin("AMUX")
                        .pin("CLK")
                        .test_enum_legacy("A5FFINIT", &["INIT0", "INIT1"]);
                    bctx.mode(bk_x)
                        .attr("BOUTMUX", "B5Q")
                        .attr("B5LUT", "#LUT:0")
                        .attr("B6LUT", "#LUT:0")
                        .attr("B5FFMUX", "IN_A")
                        .attr("B5FFSR", "SRLOW")
                        .pin("BMUX")
                        .pin("CLK")
                        .test_enum_legacy("B5FFINIT", &["INIT0", "INIT1"]);
                    bctx.mode(bk_x)
                        .attr("COUTMUX", "C5Q")
                        .attr("C5LUT", "#LUT:0")
                        .attr("C6LUT", "#LUT:0")
                        .attr("C5FFMUX", "IN_A")
                        .attr("C5FFSR", "SRLOW")
                        .pin("CMUX")
                        .pin("CLK")
                        .test_enum_legacy("C5FFINIT", &["INIT0", "INIT1"]);
                    bctx.mode(bk_x)
                        .attr("DOUTMUX", "D5Q")
                        .attr("D5LUT", "#LUT:0")
                        .attr("D6LUT", "#LUT:0")
                        .attr("D5FFMUX", "IN_A")
                        .attr("D5FFSR", "SRLOW")
                        .pin("DMUX")
                        .pin("CLK")
                        .test_enum_legacy("D5FFINIT", &["INIT0", "INIT1"]);
                }
            }
            if matches!(mode, Mode::Virtex5 | Mode::Spartan6) {
                bctx.mode(bk_x)
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("CE")
                    .pin("CLK")
                    .test_enum_legacy("CEUSED", &["0"]);
                bctx.mode(bk_x)
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("SR")
                    .pin("CLK")
                    .test_enum_legacy("SRUSED", &["0"]);
            } else {
                bctx.mode(bk_x)
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("CE")
                    .pin("CLK")
                    .test_enum_legacy("CEUSEDMUX", &["1", "IN"]);
                bctx.mode(bk_x)
                    .attr("AFF", "#FF")
                    .pin("AQ")
                    .pin("SR")
                    .pin("CLK")
                    .test_enum_legacy("SRUSEDMUX", &["0", "IN"]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let mode = match ctx.edev {
        ExpandedDevice::Virtex4(edev) => match edev.kind {
            prjcombine_virtex4::chip::ChipKind::Virtex4 => unreachable!(),
            prjcombine_virtex4::chip::ChipKind::Virtex5 => Mode::Virtex5,
            prjcombine_virtex4::chip::ChipKind::Virtex6 => Mode::Virtex6,
            prjcombine_virtex4::chip::ChipKind::Virtex7 => Mode::Virtex7,
        },
        ExpandedDevice::Spartan6(_) => Mode::Spartan6,
        _ => unreachable!(),
    };

    for tile in if mode == Mode::Spartan6 {
        ["CLEXL", "CLEXM"]
    } else {
        ["CLBLL", "CLBLM"]
    } {
        let tcls = ctx.edev.db.get_tile_class(tile);
        if ctx.edev.tile_index[tcls].is_empty() {
            continue;
        }
        for (idx, bel) in ["SLICE[0]", "SLICE[1]"].into_iter().enumerate() {
            let is_x = idx == 1 && mode == Mode::Spartan6;
            let is_m = idx == 0 && tile.ends_with('M');

            // LUTs
            ctx.collect_bitvec_legacy(tile, bel, "A6LUT", "#LUT");
            ctx.collect_bitvec_legacy(tile, bel, "B6LUT", "#LUT");
            ctx.collect_bitvec_legacy(tile, bel, "C6LUT", "#LUT");
            ctx.collect_bitvec_legacy(tile, bel, "D6LUT", "#LUT");

            // LUT RAM
            if is_m {
                ctx.collect_enum_legacy(tile, bel, "WEMUX", &["WE", "CE"]);
                for attr in ["WA7USED", "WA8USED"] {
                    let diff = ctx.get_diff_legacy(tile, bel, attr, "0");
                    ctx.insert_legacy(tile, bel, attr, xlat_bit_legacy(diff));
                }
                let di_muxes = match mode {
                    Mode::Virtex5 | Mode::Spartan6 => [
                        ("ADI1MUX", "AX", "BMC31", "BDI1"),
                        ("BDI1MUX", "BX", "CMC31", "DX"),
                        ("CDI1MUX", "CX", "DMC31", "DX"),
                    ],
                    Mode::Virtex6 | Mode::Virtex7 => [
                        ("ADI1MUX", "AI", "BMC31", "BDI1"),
                        ("BDI1MUX", "BI", "CMC31", "DI"),
                        ("CDI1MUX", "CI", "DMC31", "DI"),
                    ],
                };
                for (attr, byp, alt_shift, alt_ram) in di_muxes {
                    let d_byp = ctx.get_diff_legacy(tile, bel, attr, byp);
                    let d_alt = ctx.get_diff_legacy(tile, bel, attr, alt_shift);
                    assert_eq!(d_alt, ctx.get_diff_legacy(tile, bel, attr, alt_ram));
                    ctx.insert_legacy(
                        tile,
                        bel,
                        attr,
                        xlat_enum_legacy(vec![(byp, d_byp), ("ALT", d_alt)]),
                    );
                }
                for (dattr, sattr) in [
                    ("ARAMMODE", "A6RAMMODE"),
                    ("BRAMMODE", "B6RAMMODE"),
                    ("CRAMMODE", "C6RAMMODE"),
                    ("DRAMMODE", "D6RAMMODE"),
                ] {
                    let d_ram32 = ctx.get_diff_legacy(tile, bel, sattr, "SPRAM32");
                    let d_ram64 = ctx.get_diff_legacy(tile, bel, sattr, "SPRAM64");
                    let d_srl16 = ctx.get_diff_legacy(tile, bel, sattr, "SRL16");
                    let d_srl32 = ctx.get_diff_legacy(tile, bel, sattr, "SRL32");
                    assert_eq!(d_ram32, ctx.get_diff_legacy(tile, bel, sattr, "DPRAM32"));
                    assert_eq!(d_ram64, ctx.get_diff_legacy(tile, bel, sattr, "DPRAM64"));
                    ctx.insert_legacy(
                        tile,
                        bel,
                        dattr,
                        xlat_enum_legacy(vec![
                            ("RAM32", d_ram32),
                            ("RAM64", d_ram64),
                            ("SRL16", d_srl16),
                            ("SRL32", d_srl32),
                        ]),
                    );
                }
            }

            // carry chain
            if !is_x {
                ctx.collect_enum_legacy(tile, bel, "ACY0", &["O5", "AX"]);
                ctx.collect_enum_legacy(tile, bel, "BCY0", &["O5", "BX"]);
                ctx.collect_enum_legacy(tile, bel, "CCY0", &["O5", "CX"]);
                ctx.collect_enum_legacy(tile, bel, "DCY0", &["O5", "DX"]);
                ctx.collect_enum_legacy(tile, bel, "PRECYINIT", &["AX", "1", "0"]);
                let item = xlat_enum_legacy(vec![
                    ("CIN", ctx.get_diff_legacy(tile, bel, "CINUSED", "1")),
                    ("PRECYINIT", Diff::default()),
                ]);
                ctx.insert_legacy(tile, bel, "CYINIT", item);
            }

            // misc muxes
            if is_x {
                ctx.collect_enum_legacy(tile, bel, "AOUTMUX", &["O5", "A5Q"]);
                ctx.collect_enum_legacy(tile, bel, "BOUTMUX", &["O5", "B5Q"]);
                ctx.collect_enum_legacy(tile, bel, "COUTMUX", &["O5", "C5Q"]);
                ctx.collect_enum_legacy(tile, bel, "DOUTMUX", &["O5", "D5Q"]);
                ctx.collect_enum_legacy(tile, bel, "AFFMUX", &["O6", "AX"]);
                ctx.collect_enum_legacy(tile, bel, "BFFMUX", &["O6", "BX"]);
                ctx.collect_enum_legacy(tile, bel, "CFFMUX", &["O6", "CX"]);
                ctx.collect_enum_legacy(tile, bel, "DFFMUX", &["O6", "DX"]);
            } else {
                if mode == Mode::Virtex5 {
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "AOUTMUX",
                        &["O6", "O5", "XOR", "CY", "F7"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "BOUTMUX",
                        &["O6", "O5", "XOR", "CY", "F8"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "COUTMUX",
                        &["O6", "O5", "XOR", "CY", "F7"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    if is_m {
                        ctx.collect_enum_default_legacy_ocd(
                            tile,
                            bel,
                            "DOUTMUX",
                            &["O6", "O5", "XOR", "CY", "MC31"],
                            "NONE",
                            OcdMode::Mux,
                        );
                    } else {
                        ctx.collect_enum_default_legacy_ocd(
                            tile,
                            bel,
                            "DOUTMUX",
                            &["O6", "O5", "XOR", "CY"],
                            "NONE",
                            OcdMode::Mux,
                        );
                    }
                } else {
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "AOUTMUX",
                        &["O6", "O5", "XOR", "CY", "A5Q", "F7"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "BOUTMUX",
                        &["O6", "O5", "XOR", "CY", "B5Q", "F8"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "COUTMUX",
                        &["O6", "O5", "XOR", "CY", "C5Q", "F7"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    if is_m {
                        ctx.collect_enum_default_legacy_ocd(
                            tile,
                            bel,
                            "DOUTMUX",
                            &["O6", "O5", "XOR", "CY", "D5Q", "MC31"],
                            "NONE",
                            OcdMode::Mux,
                        );
                    } else {
                        ctx.collect_enum_default_legacy_ocd(
                            tile,
                            bel,
                            "DOUTMUX",
                            &["O6", "O5", "XOR", "CY", "D5Q"],
                            "NONE",
                            OcdMode::Mux,
                        );
                    }
                }
                if mode == Mode::Spartan6 {
                    ctx.collect_enum_legacy(
                        tile,
                        bel,
                        "AFFMUX",
                        &["O6", "O5", "XOR", "CY", "AX", "F7"],
                    );
                    ctx.collect_enum_legacy(
                        tile,
                        bel,
                        "BFFMUX",
                        &["O6", "O5", "XOR", "CY", "BX", "F8"],
                    );
                    ctx.collect_enum_legacy(
                        tile,
                        bel,
                        "CFFMUX",
                        &["O6", "O5", "XOR", "CY", "CX", "F7"],
                    );
                    if is_m {
                        ctx.collect_enum_legacy(
                            tile,
                            bel,
                            "DFFMUX",
                            &["O6", "O5", "XOR", "CY", "DX", "MC31"],
                        );
                    } else {
                        ctx.collect_enum_legacy(
                            tile,
                            bel,
                            "DFFMUX",
                            &["O6", "O5", "XOR", "CY", "DX"],
                        );
                    }
                } else {
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "AFFMUX",
                        &["O6", "O5", "XOR", "CY", "AX", "F7"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "BFFMUX",
                        &["O6", "O5", "XOR", "CY", "BX", "F8"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    ctx.collect_enum_default_legacy_ocd(
                        tile,
                        bel,
                        "CFFMUX",
                        &["O6", "O5", "XOR", "CY", "CX", "F7"],
                        "NONE",
                        OcdMode::Mux,
                    );
                    if is_m {
                        ctx.collect_enum_default_legacy_ocd(
                            tile,
                            bel,
                            "DFFMUX",
                            &["O6", "O5", "XOR", "CY", "DX", "MC31"],
                            "NONE",
                            OcdMode::Mux,
                        );
                    } else {
                        ctx.collect_enum_default_legacy_ocd(
                            tile,
                            bel,
                            "DFFMUX",
                            &["O6", "O5", "XOR", "CY", "DX"],
                            "NONE",
                            OcdMode::Mux,
                        );
                    }
                }
                if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                    for (attr, byp) in [
                        ("A5FFMUX", "AX"),
                        ("B5FFMUX", "BX"),
                        ("C5FFMUX", "CX"),
                        ("D5FFMUX", "DX"),
                    ] {
                        let d_o5 = ctx.get_diff_legacy(tile, bel, attr, "IN_A");
                        let d_byp = ctx.get_diff_legacy(tile, bel, attr, "IN_B");
                        ctx.insert_legacy(
                            tile,
                            bel,
                            attr,
                            xlat_enum_legacy(vec![
                                ("O5", d_o5),
                                (byp, d_byp),
                                ("NONE", Diff::default()),
                            ]),
                        );
                    }
                }
            }

            // FFs
            let ff_sync = ctx.get_diff_legacy(tile, bel, "SYNC_ATTR", "SYNC");
            ctx.get_diff_legacy(tile, bel, "SYNC_ATTR", "ASYNC")
                .assert_empty();
            ctx.insert_legacy(tile, bel, "FF_SR_SYNC", xlat_bit_legacy(ff_sync));
            ctx.collect_inv_legacy(tile, bel, "CLK");
            if mode == Mode::Virtex5 {
                let revused = ctx.get_diff_legacy(tile, bel, "REVUSED", "0");
                ctx.insert_legacy(tile, bel, "FF_REV_ENABLE", xlat_bit_legacy(revused));
            }
            if matches!(mode, Mode::Virtex5 | Mode::Spartan6) {
                let ceused = ctx.get_diff_legacy(tile, bel, "CEUSED", "0");
                ctx.insert_legacy(tile, bel, "FF_CE_ENABLE", xlat_bit_legacy(ceused));
                let srused = ctx.get_diff_legacy(tile, bel, "SRUSED", "0");
                ctx.insert_legacy(tile, bel, "FF_SR_ENABLE", xlat_bit_legacy(srused));
            } else {
                ctx.get_diff_legacy(tile, bel, "CEUSEDMUX", "1")
                    .assert_empty();
                ctx.get_diff_legacy(tile, bel, "SRUSEDMUX", "0")
                    .assert_empty();
                let ceused = ctx.get_diff_legacy(tile, bel, "CEUSEDMUX", "IN");
                ctx.insert_legacy(tile, bel, "FF_CE_ENABLE", xlat_bit_legacy(ceused));
                let srused = ctx.get_diff_legacy(tile, bel, "SRUSEDMUX", "IN");
                ctx.insert_legacy(tile, bel, "FF_SR_ENABLE", xlat_bit_legacy(srused));
            }
            if mode != Mode::Virtex6 {
                let ff_latch = ctx.get_diff_legacy(tile, bel, "AFF", "#LATCH");
                for attr in ["AFF", "BFF", "CFF", "DFF"] {
                    ctx.get_diff_legacy(tile, bel, attr, "#FF").assert_empty();
                    if attr != "AFF" {
                        assert_eq!(ff_latch, ctx.get_diff_legacy(tile, bel, attr, "#LATCH"));
                    }
                    if mode != Mode::Virtex5 {
                        assert_eq!(ff_latch, ctx.get_diff_legacy(tile, bel, attr, "AND2L"));
                        assert_eq!(ff_latch, ctx.get_diff_legacy(tile, bel, attr, "OR2L"));
                    }
                }
                ctx.insert_legacy(tile, bel, "FF_LATCH", xlat_bit_legacy(ff_latch));
            } else {
                for attr in ["AFF", "BFF", "CFF", "DFF"] {
                    ctx.get_diff_legacy(tile, bel, attr, "#FF").assert_empty();
                    let ff_latch = ctx.get_diff_legacy(tile, bel, attr, "#LATCH");
                    assert_eq!(ff_latch, ctx.get_diff_legacy(tile, bel, attr, "AND2L"));
                    assert_eq!(ff_latch, ctx.get_diff_legacy(tile, bel, attr, "OR2L"));
                    ctx.insert_legacy(
                        tile,
                        bel,
                        format!("{attr}_LATCH"),
                        xlat_bit_legacy(ff_latch),
                    );
                }
            }
            match mode {
                Mode::Virtex5 => {
                    for attr in ["AFFINIT", "BFFINIT", "CFFINIT", "DFFINIT"] {
                        ctx.collect_bit_bi_legacy(tile, bel, attr, "INIT0", "INIT1");
                    }
                    for attr in ["AFFSR", "BFFSR", "CFFSR", "DFFSR"] {
                        ctx.collect_bit_bi_legacy(tile, bel, attr, "SRLOW", "SRHIGH");
                    }
                }
                Mode::Virtex6 | Mode::Virtex7 => {
                    for attr in [
                        "AFFINIT", "BFFINIT", "CFFINIT", "DFFINIT", "A5FFINIT", "B5FFINIT",
                        "C5FFINIT", "D5FFINIT",
                    ] {
                        ctx.collect_bit_bi_legacy(tile, bel, attr, "INIT0", "INIT1");
                    }
                    for attr in [
                        "AFFSR", "BFFSR", "CFFSR", "DFFSR", "A5FFSR", "B5FFSR", "C5FFSR", "D5FFSR",
                    ] {
                        ctx.collect_bit_bi_legacy(tile, bel, attr, "SRLOW", "SRHIGH");
                    }
                }
                Mode::Spartan6 => {
                    for attr in [
                        "AFFSRINIT",
                        "BFFSRINIT",
                        "CFFSRINIT",
                        "DFFSRINIT",
                        "A5FFSRINIT",
                        "B5FFSRINIT",
                        "C5FFSRINIT",
                        "D5FFSRINIT",
                    ] {
                        ctx.collect_bit_bi_legacy(tile, bel, attr, "SRINIT0", "SRINIT1");
                    }
                }
            }
        }
    }
}

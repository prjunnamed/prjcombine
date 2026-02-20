use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_collector::diff::OcdMode;
use prjcombine_re_hammer::Session;
use prjcombine_spartan6::defs::{bcls, bslots, enums, tcls, tslots};

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
        loop {
            if tcrd.row.to_idx() == 0 {
                return None;
            }
            tcrd.row -= 1;
            if backend.edev.has_bel(tcrd.bel(bslots::SLICE[0])) {
                return Some(tcrd.tile(tslots::BEL));
            }
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tcid in [tcls::CLEXL, tcls::CLEXM] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for i in 0..2 {
            let bel = bslots::SLICE[i];
            let mut bctx = ctx.bel(bel);
            let is_x = i == 1;
            let is_m = i == 0 && tcid == tcls::CLEXM;

            // LUTs
            for attr in [
                bcls::SLICE::A6LUT,
                bcls::SLICE::B6LUT,
                bcls::SLICE::C6LUT,
                bcls::SLICE::D6LUT,
            ] {
                bctx.mode("SLICEX")
                    .test_bel_attr_multi(attr, MultiValue::Lut);
            }

            if is_m {
                // LUT RAM
                bctx.mode("SLICEM")
                    .attr("A6LUT", "#RAM:0")
                    .attr("A6RAMMODE", "SPRAM64")
                    .pin("WE")
                    .pin("CE")
                    .test_bel_attr_rename("WEMUX", bcls::SLICE::MUX_WE);
                for attr in [bcls::SLICE::WA7USED, bcls::SLICE::WA8USED] {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#RAM:0")
                        .attr("A6RAMMODE", "SPRAM64")
                        .pin("AX")
                        .pin("BX")
                        .pin("CX")
                        .pin("DX")
                        .test_bel_attr_bits(attr)
                        .attr(backend.edev.db[bcls::SLICE].attributes.key(attr), "0")
                        .commit();
                }
                for (val, vname) in [
                    (enums::SLICE_MUX_ADI1::AX, "AX"),
                    (enums::SLICE_MUX_ADI1::ALT, "BDI1"),
                    (enums::SLICE_MUX_ADI1::ALT, "BMC31"),
                ] {
                    bctx.mode("SLICEM")
                        .attr("A6LUT", "#RAM:0")
                        .attr("A6RAMMODE", "SPRAM64")
                        .pin("AX")
                        .test_bel_attr_val(bcls::SLICE::MUX_ADI1, val)
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
                        .test_bel_attr_val(bcls::SLICE::MUX_BDI1, val)
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
                        .test_bel_attr_val(bcls::SLICE::MUX_CDI1, val)
                        .attr("CDI1MUX", vname)
                        .commit();
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
                        .test_bel_attr_val(bcls::SLICE::ARAMMODE, val)
                        .attr("A6RAMMODE", vname)
                        .commit();
                    bctx.mode("SLICEM")
                        .attr("B6LUT", "#RAM:0")
                        .test_bel_attr_val(bcls::SLICE::BRAMMODE, val)
                        .attr("B6RAMMODE", vname)
                        .commit();
                    bctx.mode("SLICEM")
                        .attr("C6LUT", "#RAM:0")
                        .test_bel_attr_val(bcls::SLICE::CRAMMODE, val)
                        .attr("C6RAMMODE", vname)
                        .commit();
                    bctx.mode("SLICEM")
                        .attr("D6LUT", "#RAM:0")
                        .test_bel_attr_val(bcls::SLICE::DRAMMODE, val)
                        .attr("D6RAMMODE", vname)
                        .commit();
                }
            }

            if !is_x {
                // carry chain
                bctx.mode("SLICEL")
                    .attr("A5LUT", "#LUT:0")
                    .attr("A6LUT", "#LUT:0")
                    .attr("COUTUSED", "0")
                    .pin("AX")
                    .pin("COUT")
                    .test_bel_attr_rename("ACY0", bcls::SLICE::MUX_ACY0);
                bctx.mode("SLICEL")
                    .attr("B5LUT", "#LUT:0")
                    .attr("B6LUT", "#LUT:0")
                    .attr("COUTUSED", "0")
                    .pin("BX")
                    .pin("COUT")
                    .test_bel_attr_rename("BCY0", bcls::SLICE::MUX_BCY0);
                bctx.mode("SLICEL")
                    .attr("C5LUT", "#LUT:0")
                    .attr("C6LUT", "#LUT:0")
                    .attr("COUTUSED", "0")
                    .pin("CX")
                    .pin("COUT")
                    .test_bel_attr_rename("CCY0", bcls::SLICE::MUX_CCY0);
                bctx.mode("SLICEL")
                    .attr("D5LUT", "#LUT:0")
                    .attr("D6LUT", "#LUT:0")
                    .attr("COUTUSED", "0")
                    .pin("DX")
                    .pin("COUT")
                    .test_bel_attr_rename("DCY0", bcls::SLICE::MUX_DCY0);
                for (val, vname) in [
                    (enums::SLICE_PRECYINIT::AX, "AX"),
                    (enums::SLICE_PRECYINIT::CONST_0, "0"),
                    (enums::SLICE_PRECYINIT::CONST_1, "1"),
                ] {
                    bctx.mode("SLICEL")
                        .attr("COUTUSED", "0")
                        .pin("AX")
                        .pin("COUT")
                        .test_bel_attr_val(bcls::SLICE::PRECYINIT, val)
                        .attr("PRECYINIT", vname)
                        .commit();
                }

                bctx.build()
                    .test_bel_attr_val(bcls::SLICE::CYINIT, enums::SLICE_CYINIT::CIN)
                    .related_pip(ClbCinDown, (PinFar, "COUT"), "COUT")
                    .commit();
            }

            // misc muxes
            if is_x {
                bctx.mode("SLICEX")
                    .attr("A6LUT", "#LUT:0")
                    .attr("A5LUT", "#LUT:0")
                    .pin("AMUX")
                    .test_bel_attr_subset_rename(
                        "AOUTMUX",
                        bcls::SLICE::MUX_AOUT,
                        &[enums::SLICE_MUX_AOUT::A5Q, enums::SLICE_MUX_AOUT::O5],
                    );
                bctx.mode("SLICEX")
                    .attr("B6LUT", "#LUT:0")
                    .attr("B5LUT", "#LUT:0")
                    .pin("BMUX")
                    .test_bel_attr_subset_rename(
                        "BOUTMUX",
                        bcls::SLICE::MUX_BOUT,
                        &[enums::SLICE_MUX_BOUT::B5Q, enums::SLICE_MUX_BOUT::O5],
                    );
                bctx.mode("SLICEX")
                    .attr("C6LUT", "#LUT:0")
                    .attr("C5LUT", "#LUT:0")
                    .pin("CMUX")
                    .test_bel_attr_subset_rename(
                        "COUTMUX",
                        bcls::SLICE::MUX_COUT,
                        &[enums::SLICE_MUX_COUT::C5Q, enums::SLICE_MUX_COUT::O5],
                    );
                bctx.mode("SLICEX")
                    .attr("D6LUT", "#LUT:0")
                    .attr("D5LUT", "#LUT:0")
                    .pin("DMUX")
                    .test_bel_attr_subset_rename(
                        "DOUTMUX",
                        bcls::SLICE::MUX_DOUT,
                        &[enums::SLICE_MUX_DOUT::D5Q, enums::SLICE_MUX_DOUT::O5],
                    );
                bctx.mode("SLICEX")
                    .attr("A6LUT", "#LUT:0")
                    .attr("AFF", "#FF")
                    .pin("AX")
                    .pin("AQ")
                    .pin("CLK")
                    .test_bel_attr_subset_rename(
                        "AFFMUX",
                        bcls::SLICE::MUX_FFA,
                        &[enums::SLICE_MUX_FFA::AX, enums::SLICE_MUX_FFA::O6],
                    );
                bctx.mode("SLICEX")
                    .attr("B6LUT", "#LUT:0")
                    .attr("BFF", "#FF")
                    .pin("BX")
                    .pin("BQ")
                    .pin("CLK")
                    .test_bel_attr_subset_rename(
                        "BFFMUX",
                        bcls::SLICE::MUX_FFB,
                        &[enums::SLICE_MUX_FFB::BX, enums::SLICE_MUX_FFB::O6],
                    );
                bctx.mode("SLICEX")
                    .attr("C6LUT", "#LUT:0")
                    .attr("CFF", "#FF")
                    .pin("CX")
                    .pin("CQ")
                    .pin("CLK")
                    .test_bel_attr_subset_rename(
                        "CFFMUX",
                        bcls::SLICE::MUX_FFC,
                        &[enums::SLICE_MUX_FFC::CX, enums::SLICE_MUX_FFC::O6],
                    );
                bctx.mode("SLICEX")
                    .attr("D6LUT", "#LUT:0")
                    .attr("DFF", "#FF")
                    .pin("DX")
                    .pin("DQ")
                    .pin("CLK")
                    .test_bel_attr_subset_rename(
                        "DFFMUX",
                        bcls::SLICE::MUX_FFD,
                        &[enums::SLICE_MUX_FFD::DX, enums::SLICE_MUX_FFD::O6],
                    );
            } else {
                // [ABCD]MUX
                bctx.mode("SLICEL")
                    .attr("A6LUT", "#LUT:0")
                    .attr("A5LUT", "#LUT:0")
                    .attr("A5FFMUX", "")
                    .attr("CLKINV", "CLK")
                    .pin("AMUX")
                    .pin("CLK")
                    .test_bel_attr_default_rename(
                        "AOUTMUX",
                        bcls::SLICE::MUX_AOUT,
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
                        bcls::SLICE::MUX_BOUT,
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
                        bcls::SLICE::MUX_COUT,
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
                            bcls::SLICE::MUX_DOUT,
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
                            bcls::SLICE::MUX_DOUT,
                            &[
                                enums::SLICE_MUX_DOUT::O5,
                                enums::SLICE_MUX_DOUT::O6,
                                enums::SLICE_MUX_DOUT::XOR,
                                enums::SLICE_MUX_DOUT::CY,
                                enums::SLICE_MUX_DOUT::D5Q,
                            ],
                        );
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
                    .test_bel_attr_rename("AFFMUX", bcls::SLICE::MUX_FFA);
                bctx.mode("SLICEL")
                    .attr("B6LUT", "#LUT:0")
                    .attr("B5LUT", "#LUT:0")
                    .attr("BFF", "#FF")
                    .attr("CLKINV", "CLK")
                    .pin("BX")
                    .pin("BQ")
                    .pin("CLK")
                    .test_bel_attr_rename("BFFMUX", bcls::SLICE::MUX_FFB);
                bctx.mode("SLICEL")
                    .attr("C6LUT", "#LUT:0")
                    .attr("C5LUT", "#LUT:0")
                    .attr("CFF", "#FF")
                    .attr("CLKINV", "CLK")
                    .pin("CX")
                    .pin("CQ")
                    .pin("CLK")
                    .test_bel_attr_rename("CFFMUX", bcls::SLICE::MUX_FFC);
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
                        .test_bel_attr_rename("DFFMUX", bcls::SLICE::MUX_FFD);
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
                            bcls::SLICE::MUX_FFD,
                            &[
                                enums::SLICE_MUX_FFD::O5,
                                enums::SLICE_MUX_FFD::O6,
                                enums::SLICE_MUX_FFD::XOR,
                                enums::SLICE_MUX_FFD::CY,
                                enums::SLICE_MUX_FFD::DX,
                            ],
                        );
                }
            }

            // FFs
            bctx.mode("SLICEX")
                .attr("AFF", "#FF")
                .pin("AQ")
                .test_bel_attr_bool_rename("SYNC_ATTR", bcls::SLICE::FF_SR_SYNC, "ASYNC", "SYNC");
            bctx.mode("SLICEX")
                .attr("AFF", "#FF")
                .pin("AQ")
                .test_bel_input_inv_auto(bcls::SLICE::CLK);
            for (val, vname) in [
                (false, "#FF"),
                (true, "#LATCH"),
                (true, "AND2L"),
                (true, "OR2L"),
            ] {
                bctx.mode("SLICEX")
                    .attr("BFF", "")
                    .attr("CFF", "")
                    .attr("DFF", "")
                    .pin("AQ")
                    .pin("CLK")
                    .test_bel_attr_bits_bi(bcls::SLICE::FF_LATCH, val)
                    .attr("AFF", vname)
                    .commit();
                bctx.mode("SLICEX")
                    .attr("AFF", "")
                    .attr("CFF", "")
                    .attr("DFF", "")
                    .pin("BQ")
                    .pin("CLK")
                    .test_bel_attr_bits_bi(bcls::SLICE::FF_LATCH, val)
                    .attr("BFF", vname)
                    .commit();
                bctx.mode("SLICEX")
                    .attr("AFF", "")
                    .attr("BFF", "")
                    .attr("DFF", "")
                    .pin("CQ")
                    .pin("CLK")
                    .test_bel_attr_bits_bi(bcls::SLICE::FF_LATCH, val)
                    .attr("CFF", vname)
                    .commit();
                bctx.mode("SLICEX")
                    .attr("AFF", "")
                    .attr("BFF", "")
                    .attr("CFF", "")
                    .pin("DQ")
                    .pin("CLK")
                    .test_bel_attr_bits_bi(bcls::SLICE::FF_LATCH, val)
                    .attr("DFF", vname)
                    .commit();
            }
            for (attr, aname) in [
                (bcls::SLICE::FFA_SRINIT, "AFFSRINIT"),
                (bcls::SLICE::FFB_SRINIT, "BFFSRINIT"),
                (bcls::SLICE::FFC_SRINIT, "CFFSRINIT"),
                (bcls::SLICE::FFD_SRINIT, "DFFSRINIT"),
            ] {
                bctx.mode("SLICEX")
                    .attr("AFF", "#FF")
                    .attr("BFF", "#FF")
                    .attr("CFF", "#FF")
                    .attr("DFF", "#FF")
                    .pin("AQ")
                    .pin("BQ")
                    .pin("CQ")
                    .pin("DQ")
                    .pin("CLK")
                    .test_bel_attr_bool_rename(aname, attr, "SRINIT0", "SRINIT1");
            }
            bctx.mode("SLICEX")
                .attr("AOUTMUX", "A5Q")
                .attr("A5LUT", "#LUT:0")
                .attr("A6LUT", "#LUT:0")
                .pin("AMUX")
                .pin("CLK")
                .test_bel_attr_bool_rename(
                    "A5FFSRINIT",
                    bcls::SLICE::FFA5_SRINIT,
                    "SRINIT0",
                    "SRINIT1",
                );
            bctx.mode("SLICEX")
                .attr("BOUTMUX", "B5Q")
                .attr("B5LUT", "#LUT:0")
                .attr("B6LUT", "#LUT:0")
                .pin("BMUX")
                .pin("CLK")
                .test_bel_attr_bool_rename(
                    "B5FFSRINIT",
                    bcls::SLICE::FFB5_SRINIT,
                    "SRINIT0",
                    "SRINIT1",
                );
            bctx.mode("SLICEX")
                .attr("COUTMUX", "C5Q")
                .attr("C5LUT", "#LUT:0")
                .attr("C6LUT", "#LUT:0")
                .pin("CMUX")
                .pin("CLK")
                .test_bel_attr_bool_rename(
                    "C5FFSRINIT",
                    bcls::SLICE::FFC5_SRINIT,
                    "SRINIT0",
                    "SRINIT1",
                );
            bctx.mode("SLICEX")
                .attr("DOUTMUX", "D5Q")
                .attr("D5LUT", "#LUT:0")
                .attr("D6LUT", "#LUT:0")
                .pin("DMUX")
                .pin("CLK")
                .test_bel_attr_bool_rename(
                    "D5FFSRINIT",
                    bcls::SLICE::FFD5_SRINIT,
                    "SRINIT0",
                    "SRINIT1",
                );
            bctx.mode("SLICEX")
                .attr("AFF", "#FF")
                .pin("AQ")
                .pin("CE")
                .pin("CLK")
                .test_bel_attr_bits(bcls::SLICE::FF_CE_ENABLE)
                .attr("CEUSED", "0")
                .commit();
            bctx.mode("SLICEX")
                .attr("AFF", "#FF")
                .pin("AQ")
                .pin("SR")
                .pin("CLK")
                .test_bel_attr_bits(bcls::SLICE::FF_SR_ENABLE)
                .attr("SRUSED", "0")
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [tcls::CLEXL, tcls::CLEXM] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for idx in 0..2 {
            let bslot = bslots::SLICE[idx];
            let is_x = idx == 1;
            let is_m = idx == 0 && tcid == tcls::CLEXM;

            // LUTs
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::A6LUT);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::B6LUT);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::C6LUT);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::D6LUT);

            // LUT RAM
            if is_m {
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_WE);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::WA7USED);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::WA8USED);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_ADI1);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_BDI1);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_CDI1);
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    bcls::SLICE::ARAMMODE,
                    enums::SLICE_RAMMODE::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    bcls::SLICE::BRAMMODE,
                    enums::SLICE_RAMMODE::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    bcls::SLICE::CRAMMODE,
                    enums::SLICE_RAMMODE::NONE,
                );
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    bcls::SLICE::DRAMMODE,
                    enums::SLICE_RAMMODE::NONE,
                );
            }

            // carry chain
            if !is_x {
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_ACY0);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_BCY0);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_CCY0);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_DCY0);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::PRECYINIT);
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    bcls::SLICE::CYINIT,
                    enums::SLICE_CYINIT::PRECYINIT,
                );
            }

            // misc muxes
            if is_x {
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_AOUT,
                    &[enums::SLICE_MUX_AOUT::O5, enums::SLICE_MUX_AOUT::A5Q],
                );
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_BOUT,
                    &[enums::SLICE_MUX_BOUT::O5, enums::SLICE_MUX_BOUT::B5Q],
                );
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_COUT,
                    &[enums::SLICE_MUX_COUT::O5, enums::SLICE_MUX_COUT::C5Q],
                );
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_DOUT,
                    &[enums::SLICE_MUX_DOUT::O5, enums::SLICE_MUX_DOUT::D5Q],
                );
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_FFA,
                    &[enums::SLICE_MUX_FFA::O6, enums::SLICE_MUX_FFA::AX],
                );
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_FFB,
                    &[enums::SLICE_MUX_FFB::O6, enums::SLICE_MUX_FFB::BX],
                );
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_FFC,
                    &[enums::SLICE_MUX_FFC::O6, enums::SLICE_MUX_FFC::CX],
                );
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_FFD,
                    &[enums::SLICE_MUX_FFD::O6, enums::SLICE_MUX_FFD::DX],
                );
            } else {
                ctx.collect_bel_attr_default_ocd(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_AOUT,
                    enums::SLICE_MUX_AOUT::NONE,
                    OcdMode::Mux,
                );
                ctx.collect_bel_attr_default_ocd(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_BOUT,
                    enums::SLICE_MUX_BOUT::NONE,
                    OcdMode::Mux,
                );
                ctx.collect_bel_attr_default_ocd(
                    tcid,
                    bslot,
                    bcls::SLICE::MUX_COUT,
                    enums::SLICE_MUX_COUT::NONE,
                    OcdMode::Mux,
                );
                if is_m {
                    ctx.collect_bel_attr_default_ocd(
                        tcid,
                        bslot,
                        bcls::SLICE::MUX_DOUT,
                        enums::SLICE_MUX_DOUT::NONE,
                        OcdMode::Mux,
                    );
                } else {
                    ctx.collect_bel_attr_subset_default_ocd(
                        tcid,
                        bslot,
                        bcls::SLICE::MUX_DOUT,
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
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_FFA);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_FFB);
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_FFC);
                if is_m {
                    ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::MUX_FFD);
                } else {
                    ctx.collect_bel_attr_subset(
                        tcid,
                        bslot,
                        bcls::SLICE::MUX_FFD,
                        &[
                            enums::SLICE_MUX_FFD::O6,
                            enums::SLICE_MUX_FFD::O5,
                            enums::SLICE_MUX_FFD::XOR,
                            enums::SLICE_MUX_FFD::CY,
                            enums::SLICE_MUX_FFD::DX,
                        ],
                    );
                }
            }

            // FFs
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::SLICE::FF_SR_SYNC);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::SLICE::CLK);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::FF_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::FF_CE_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::SLICE::FF_LATCH);
            for attr in [
                bcls::SLICE::FFA_SRINIT,
                bcls::SLICE::FFB_SRINIT,
                bcls::SLICE::FFC_SRINIT,
                bcls::SLICE::FFD_SRINIT,
                bcls::SLICE::FFA5_SRINIT,
                bcls::SLICE::FFB5_SRINIT,
                bcls::SLICE::FFC5_SRINIT,
                bcls::SLICE::FFD5_SRINIT,
            ] {
                ctx.collect_bel_attr_bi(tcid, bslot, attr);
            }
        }
    }
}

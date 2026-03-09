use prjcombine_interconnect::db::{BelInfo, BelInputId, WireSlotIdExt};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::{
    chip::ChipKind,
    defs::{
        bcls::DSP_V6 as DSP,
        bslots, enums,
        virtex6::{tcls as tcls_v6, wires as wires_v6},
        virtex7::{tcls as tcls_v7, wires as wires_v7},
    },
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::BaseIntPip,
        props::mutex::WireMutexExclusive,
    },
    virtex4::specials,
};

const DSP48E1_INVPINS: &[BelInputId] = &[
    DSP::CLK,
    DSP::CARRYIN,
    DSP::OPMODE.index_const(0),
    DSP::OPMODE.index_const(1),
    DSP::OPMODE.index_const(2),
    DSP::OPMODE.index_const(3),
    DSP::OPMODE.index_const(4),
    DSP::OPMODE.index_const(5),
    DSP::OPMODE.index_const(6),
    DSP::ALUMODE.index_const(0),
    DSP::ALUMODE.index_const(1),
    DSP::ALUMODE.index_const(2),
    DSP::ALUMODE.index_const(3),
    DSP::INMODE.index_const(0),
    DSP::INMODE.index_const(1),
    DSP::INMODE.index_const(2),
    DSP::INMODE.index_const(3),
    DSP::INMODE.index_const(4),
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let (tcid, gnd) = match edev.kind {
        ChipKind::Virtex6 => (tcls_v6::DSP, wires_v6::TIE_0),
        ChipKind::Virtex7 => (tcls_v7::DSP, wires_v7::TIE_0),
        _ => unreachable!(),
    };
    let mut ctx = FuzzCtx::new(session, backend, tcid);
    for i in 0..2 {
        let bel_other = bslots::DSP[i ^ 1];
        let mut bctx = ctx.bel(bslots::DSP[i]);
        let mode = "DSP48E1";
        bctx.build()
            .null_bits()
            .bel_unused(bel_other)
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for &pin in DSP48E1_INVPINS {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }
        for (attr, aname, acasc, inmode) in [
            (DSP::AREG, "AREG", "ACASCREG", DSP::INMODE[0]),
            (DSP::BREG, "BREG", "BCASCREG", DSP::INMODE[4]),
        ] {
            let BelInfo::Bel(ref bel) = backend.edev.db[tcid].bels[bslots::DSP[i]] else {
                unreachable!()
            };
            let inmode = bel.inputs[inmode].wire();
            for (val, vname, vcasc) in [
                (enums::DSP_REG2_CASC::_0, "0", "0"),
                (enums::DSP_REG2_CASC::_1, "1", "1"),
                (enums::DSP_REG2_CASC::DIRECT_2_CASC_1, "2", "1"),
                (enums::DSP_REG2_CASC::_2, "2", "2"),
            ] {
                bctx.mode(mode)
                    .prop(WireMutexExclusive::new(inmode))
                    .test_bel_attr_val(attr, val)
                    .attr(aname, vname)
                    .attr(acasc, vcasc)
                    .commit();
            }
            bctx.mode(mode)
                .prop(WireMutexExclusive::new(inmode))
                .prop(BaseIntPip::new(inmode, gnd.cell(0)))
                .test_bel_attr_val(attr, enums::DSP_REG2_CASC::_1_INMODE_GND)
                .attr(aname, "1")
                .attr(acasc, "1")
                .commit();
        }
        for attr in [
            DSP::CREG,
            DSP::MREG,
            DSP::PREG,
            DSP::OPMODEREG,
            DSP::ALUMODEREG,
            DSP::INMODEREG,
            DSP::CARRYINREG,
            DSP::CARRYINSELREG,
        ] {
            bctx.mode(mode).test_bel_attr_bool_auto(attr, "0", "1");
        }
        for attr in [DSP::DREG, DSP::ADREG] {
            bctx.mode(mode)
                .attr("USE_MULT", "MULTIPLY")
                .attr("USE_DPORT", "TRUE")
                .test_bel_attr_bool_auto(attr, "0", "1")
        }
        for attr in [DSP::A_INPUT, DSP::B_INPUT] {
            bctx.mode(mode).test_bel_attr_auto(attr);
        }
        for val in ["PATDET", "NO_PATDET"] {
            bctx.mode(mode)
                .null_bits()
                .test_bel_special(specials::DSP_USE_PATTERN_DETECT)
                .attr("USE_PATTERN_DETECT", val)
                .commit();
        }
        bctx.mode(mode).test_bel_attr_auto(DSP::USE_SIMD);
        for (val, vname) in [(false, "NONE"), (true, "MULTIPLY"), (true, "DYNAMIC")] {
            bctx.mode(mode)
                .attr("DREG", "0")
                .attr("ADREG", "0")
                .test_bel_attr_bits_bi(DSP::USE_MULT, val)
                .attr("USE_MULT", vname)
                .commit();
        }
        bctx.mode(mode)
            .attr("DREG", "0")
            .attr("ADREG", "0")
            .test_bel_attr_bool_auto(DSP::USE_DPORT, "FALSE", "TRUE");
        bctx.mode(mode).test_bel_attr_auto(DSP::SEL_PATTERN);
        bctx.mode(mode).test_bel_attr_auto(DSP::SEL_MASK);
        for (val, vname) in [
            (enums::DSP_SEL_ROUNDING_MASK::SEL_MASK, "MASK"),
            (enums::DSP_SEL_ROUNDING_MASK::MODE1, "ROUNDING_MODE1"),
            (enums::DSP_SEL_ROUNDING_MASK::MODE2, "ROUNDING_MODE2"),
        ] {
            bctx.mode(mode)
                .test_bel_attr_val(DSP::SEL_ROUNDING_MASK, val)
                .attr("SEL_MASK", vname)
                .commit();
        }
        bctx.mode(mode).test_bel_attr_bool_rename(
            "AUTORESET_PATDET",
            DSP::AUTORESET_PATTERN_DETECT,
            "NO_RESET",
            "RESET_MATCH",
        );
        bctx.mode(mode)
            .test_bel_attr_bits(DSP::AUTORESET_PATTERN_DETECT_OPTINV)
            .attr_diff("AUTORESET_PATDET", "RESET_MATCH", "RESET_NOT_MATCH")
            .commit();
        bctx.mode(mode)
            .test_bel_attr_multi(DSP::PATTERN, MultiValue::Hex(0));
        bctx.mode(mode)
            .test_bel_attr_multi(DSP::MASK, MultiValue::Hex(0));
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let tcid = match edev.kind {
        ChipKind::Virtex6 => tcls_v6::DSP,
        ChipKind::Virtex7 => tcls_v7::DSP,
        _ => unreachable!(),
    };
    for bslot in bslots::DSP {
        for &pin in DSP48E1_INVPINS {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        ctx.collect_bel_attr_default(tcid, bslot, DSP::AREG, enums::DSP_REG2_CASC::NONE);
        ctx.collect_bel_attr_default(tcid, bslot, DSP::BREG, enums::DSP_REG2_CASC::NONE);
        for attr in [
            DSP::CREG,
            DSP::MREG,
            DSP::PREG,
            DSP::OPMODEREG,
            DSP::ALUMODEREG,
            DSP::INMODEREG,
            DSP::CARRYINREG,
            DSP::CARRYINSELREG,
            DSP::DREG,
            DSP::ADREG,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
        }
        ctx.collect_bel_attr(tcid, bslot, DSP::A_INPUT);
        ctx.collect_bel_attr(tcid, bslot, DSP::B_INPUT);
        ctx.collect_bel_attr(tcid, bslot, DSP::USE_SIMD);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::USE_MULT);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::USE_DPORT);
        ctx.collect_bel_attr(tcid, bslot, DSP::SEL_PATTERN);
        ctx.collect_bel_attr(tcid, bslot, DSP::SEL_MASK);
        ctx.collect_bel_attr(tcid, bslot, DSP::SEL_ROUNDING_MASK);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::AUTORESET_PATTERN_DETECT);
        ctx.collect_bel_attr(tcid, bslot, DSP::AUTORESET_PATTERN_DETECT_OPTINV);

        ctx.collect_bel_attr(tcid, bslot, DSP::PATTERN);
        ctx.collect_bel_attr(tcid, bslot, DSP::MASK);
    }
}

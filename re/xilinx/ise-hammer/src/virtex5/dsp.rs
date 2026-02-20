use prjcombine_interconnect::db::BelInputId;
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs::{bcls::DSP_V5 as DSP, bslots, enums, virtex5::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::FuzzCtx,
    virtex4::specials,
};

const DSP48E_INVPINS: &[BelInputId] = &[
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
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::DSP);
    for i in 0..2 {
        let bel_other = bslots::DSP[i ^ 1];
        let mut bctx = ctx.bel(bslots::DSP[i]);
        let mode = "DSP48E";
        bctx.build()
            .bel_unused(bel_other)
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for &pin in DSP48E_INVPINS {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }
        for (attr, attrdir, attrcasc) in [
            (DSP::AREG, "AREG", "ACASCREG"),
            (DSP::BREG, "BREG", "BCASCREG"),
        ] {
            for (val, valdir, valcasc) in [
                (enums::DSP_REG2_CASC::_0, "0", "0"),
                (enums::DSP_REG2_CASC::_1, "1", "1"),
                (enums::DSP_REG2_CASC::DIRECT_2_CASC_1, "2", "1"),
                (enums::DSP_REG2_CASC::_2, "2", "2"),
            ] {
                bctx.mode(mode)
                    .test_bel_attr_val(attr, val)
                    .attr(attrdir, valdir)
                    .attr(attrcasc, valcasc)
                    .commit();
            }
        }
        for attr in [
            DSP::CREG,
            DSP::MREG,
            DSP::PREG,
            DSP::OPMODEREG,
            DSP::ALUMODEREG,
            DSP::CARRYINREG,
            DSP::CARRYINSELREG,
            DSP::MULTCARRYINREG,
        ] {
            bctx.mode(mode).test_bel_attr_bool_auto(attr, "0", "1");
        }
        for attr in [DSP::A_INPUT, DSP::B_INPUT] {
            bctx.mode(mode).test_bel_attr_auto(attr);
        }
        for attr in [DSP::CLOCK_INVERT_P, DSP::CLOCK_INVERT_M] {
            bctx.mode(mode)
                .test_bel_attr_bool_auto(attr, "SAME_EDGE", "OPPOSITE_EDGE");
        }
        bctx.mode(mode).test_bel_attr_auto(DSP::SEL_ROUNDING_MASK);
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::ROUNDING_LSB_MASK, "0", "1");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::USE_PATTERN_DETECT, "NO_PATDET", "PATDET");
        bctx.mode(mode).test_bel_attr_auto(DSP::USE_SIMD);
        bctx.mode(mode).test_bel_attr_auto(DSP::USE_MULT);
        bctx.mode(mode).test_bel_attr_auto(DSP::SEL_PATTERN);
        bctx.mode(mode).test_bel_attr_auto(DSP::SEL_MASK);
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::AUTORESET_OVER_UNDER_FLOW, "FALSE", "TRUE");
        bctx.mode(mode).test_bel_attr_bool_auto(
            DSP::AUTORESET_PATTERN_DETECT_OPTINV,
            "MATCH",
            "NOT_MATCH",
        );
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::AUTORESET_PATTERN_DETECT, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::SCAN_IN_SET_M, "DONT_SET", "SET");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::SCAN_IN_SET_P, "DONT_SET", "SET");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::SCAN_IN_SETVAL_M, "0", "1");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::SCAN_IN_SETVAL_P, "0", "1");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::TEST_SET_M, "DONT_SET", "SET");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::TEST_SET_P, "DONT_SET", "SET");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::TEST_SETVAL_M, "0", "1");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::TEST_SETVAL_P, "0", "1");
        if i == 0 {
            bctx.mode(mode)
                .bel_mode(bel_other, mode)
                .bel_attr(bel_other, "LFSR_EN_SET", "DONT_SET")
                .test_bel_attr_bool_auto(DSP::LFSR_EN_SET, "DONT_SET", "SET");
        } else {
            bctx.mode(mode)
                .bel_unused(bel_other)
                .test_bel_attr_bool_auto(DSP::LFSR_EN_SET, "DONT_SET", "SET");
        }
        bctx.mode(mode)
            .test_bel_attr_bool_auto(DSP::LFSR_EN_SETVAL, "0", "1");
        bctx.mode(mode)
            .test_bel_attr_multi(DSP::PATTERN, MultiValue::Hex(0));
        bctx.mode(mode)
            .test_bel_attr_multi(DSP::MASK, MultiValue::Hex(0));
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::DSP;
    for bslot in bslots::DSP {
        for &pin in DSP48E_INVPINS {
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
            DSP::CARRYINREG,
            DSP::CARRYINSELREG,
            DSP::MULTCARRYINREG,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
        }
        ctx.collect_bel_attr(tcid, bslot, DSP::A_INPUT);
        ctx.collect_bel_attr(tcid, bslot, DSP::B_INPUT);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::CLOCK_INVERT_M);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::CLOCK_INVERT_P);
        ctx.collect_bel_attr(tcid, bslot, DSP::SEL_ROUNDING_MASK);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::ROUNDING_LSB_MASK);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::USE_PATTERN_DETECT);
        ctx.collect_bel_attr(tcid, bslot, DSP::USE_SIMD);
        ctx.collect_bel_attr(tcid, bslot, DSP::USE_MULT);
        ctx.collect_bel_attr(tcid, bslot, DSP::SEL_PATTERN);
        ctx.collect_bel_attr(tcid, bslot, DSP::SEL_MASK);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::AUTORESET_OVER_UNDER_FLOW);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::AUTORESET_PATTERN_DETECT);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::AUTORESET_PATTERN_DETECT_OPTINV);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::SCAN_IN_SET_M);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::SCAN_IN_SET_P);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::SCAN_IN_SETVAL_M);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::SCAN_IN_SETVAL_P);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::TEST_SET_M);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::TEST_SET_P);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::TEST_SETVAL_M);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::TEST_SETVAL_P);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::LFSR_EN_SET);
        ctx.collect_bel_attr_bi(tcid, bslot, DSP::LFSR_EN_SETVAL);
        ctx.collect_bel_attr(tcid, bslot, DSP::PATTERN);
        ctx.collect_bel_attr(tcid, bslot, DSP::MASK);
    }
    for bslot in bslots::DSP {
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslot, DSP::SCAN_IN_SET_M)]);
        present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslot, DSP::SCAN_IN_SET_P)]);
        present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslot, DSP::TEST_SET_M)]);
        present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslot, DSP::TEST_SET_P)]);
        if bslot == bslots::DSP[0] {
            present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslots::DSP[0], DSP::LFSR_EN_SET)]);
            present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslots::DSP[1], DSP::LFSR_EN_SET)]);
        }
        present.assert_empty();
    }
}

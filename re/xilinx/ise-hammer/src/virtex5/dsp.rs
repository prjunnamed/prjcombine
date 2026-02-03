use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

const DSP48E_INVPINS: &[&str] = &[
    "CLK", "CARRYIN", "OPMODE0", "OPMODE1", "OPMODE2", "OPMODE3", "OPMODE4", "OPMODE5", "OPMODE6",
    "ALUMODE0", "ALUMODE1", "ALUMODE2", "ALUMODE3",
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let tile = "DSP";
    let mut ctx = FuzzCtx::new_legacy(session, backend, tile);
    for i in 0..2 {
        let bel_other = defs::bslots::DSP[i ^ 1];
        let mut bctx = ctx.bel(defs::bslots::DSP[i]);
        let mode = "DSP48E";
        bctx.build()
            .bel_unused(bel_other)
            .test_manual_legacy("PRESENT", "1")
            .mode(mode)
            .commit();
        for &pin in DSP48E_INVPINS {
            bctx.mode(mode).test_inv_legacy(pin);
        }
        for (aname, attr, attrcasc) in [
            ("AREG_ACASCREG", "AREG", "ACASCREG"),
            ("BREG_BCASCREG", "BREG", "BCASCREG"),
        ] {
            for (vname, val, valcasc) in [
                ("0_0", "0", "0"),
                ("1_1", "1", "1"),
                ("2_1", "2", "1"),
                ("2_2", "2", "2"),
            ] {
                bctx.mode(mode)
                    .test_manual_legacy(aname, vname)
                    .attr(attr, val)
                    .attr(attrcasc, valcasc)
                    .commit();
            }
        }
        for attr in [
            "CREG",
            "MREG",
            "PREG",
            "OPMODEREG",
            "ALUMODEREG",
            "CARRYINREG",
            "CARRYINSELREG",
            "MULTCARRYINREG",
        ] {
            bctx.mode(mode).test_enum_legacy(attr, &["0", "1"]);
        }
        for attr in ["A_INPUT", "B_INPUT"] {
            bctx.mode(mode)
                .test_enum_legacy(attr, &["DIRECT", "CASCADE"]);
        }
        for attr in ["CLOCK_INVERT_P", "CLOCK_INVERT_M"] {
            bctx.mode(mode)
                .test_enum_legacy(attr, &["SAME_EDGE", "OPPOSITE_EDGE"]);
        }
        bctx.mode(mode)
            .test_enum_legacy("SEL_ROUNDING_MASK", &["SEL_MASK", "MODE2", "MODE1"]);
        bctx.mode(mode)
            .test_enum_legacy("ROUNDING_LSB_MASK", &["1", "0"]);
        bctx.mode(mode)
            .test_enum_legacy("USE_PATTERN_DETECT", &["PATDET", "NO_PATDET"]);
        bctx.mode(mode)
            .test_enum_legacy("USE_SIMD", &["TWO24", "ONE48", "FOUR12"]);
        bctx.mode(mode)
            .test_enum_legacy("USE_MULT", &["NONE", "MULT", "MULT_S"]);
        bctx.mode(mode)
            .test_enum_legacy("SEL_PATTERN", &["PATTERN", "C"]);
        bctx.mode(mode).test_enum_legacy("SEL_MASK", &["MASK", "C"]);
        bctx.mode(mode)
            .test_enum_legacy("AUTORESET_OVER_UNDER_FLOW", &["TRUE", "FALSE"]);
        bctx.mode(mode)
            .test_enum_legacy("AUTORESET_PATTERN_DETECT_OPTINV", &["NOT_MATCH", "MATCH"]);
        bctx.mode(mode)
            .test_enum_legacy("AUTORESET_PATTERN_DETECT", &["TRUE", "FALSE"]);
        bctx.mode(mode)
            .test_enum_legacy("SCAN_IN_SET_M", &["SET", "DONT_SET"]);
        bctx.mode(mode)
            .test_enum_legacy("SCAN_IN_SET_P", &["SET", "DONT_SET"]);
        bctx.mode(mode)
            .test_enum_legacy("SCAN_IN_SETVAL_M", &["1", "0"]);
        bctx.mode(mode)
            .test_enum_legacy("SCAN_IN_SETVAL_P", &["1", "0"]);
        bctx.mode(mode)
            .test_enum_legacy("TEST_SET_M", &["SET", "DONT_SET"]);
        bctx.mode(mode)
            .test_enum_legacy("TEST_SET_P", &["SET", "DONT_SET"]);
        bctx.mode(mode)
            .test_enum_legacy("TEST_SETVAL_M", &["1", "0"]);
        bctx.mode(mode)
            .test_enum_legacy("TEST_SETVAL_P", &["1", "0"]);
        if i == 0 {
            bctx.mode(mode)
                .bel_mode(bel_other, mode)
                .bel_attr(bel_other, "LFSR_EN_SET", "DONT_SET")
                .test_enum_legacy("LFSR_EN_SET", &["SET", "DONT_SET"]);
        } else {
            bctx.mode(mode)
                .bel_unused(bel_other)
                .test_enum_legacy("LFSR_EN_SET", &["SET", "DONT_SET"]);
        }
        bctx.mode(mode)
            .test_enum_legacy("LFSR_EN_SETVAL", &["1", "0"]);
        bctx.mode(mode).test_multi_attr_hex_legacy("PATTERN", 48);
        bctx.mode(mode).test_multi_attr_hex_legacy("MASK", 48);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "DSP";
    for bel in ["DSP[0]", "DSP[1]"] {
        for &pin in DSP48E_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for attr in ["AREG_ACASCREG", "BREG_BCASCREG"] {
            ctx.collect_enum_legacy(tile, bel, attr, &["0_0", "1_1", "2_1", "2_2"]);
        }
        for attr in [
            "CREG",
            "MREG",
            "PREG",
            "OPMODEREG",
            "ALUMODEREG",
            "CARRYINREG",
            "CARRYINSELREG",
            "MULTCARRYINREG",
        ] {
            ctx.collect_enum_legacy(tile, bel, attr, &["0", "1"]);
        }
        ctx.collect_enum_legacy(tile, bel, "A_INPUT", &["DIRECT", "CASCADE"]);
        ctx.collect_enum_legacy(tile, bel, "B_INPUT", &["DIRECT", "CASCADE"]);
        ctx.collect_enum_legacy(tile, bel, "CLOCK_INVERT_P", &["SAME_EDGE", "OPPOSITE_EDGE"]);
        ctx.collect_enum_legacy(tile, bel, "CLOCK_INVERT_M", &["SAME_EDGE", "OPPOSITE_EDGE"]);
        ctx.collect_enum_legacy(
            tile,
            bel,
            "SEL_ROUNDING_MASK",
            &["SEL_MASK", "MODE2", "MODE1"],
        );
        ctx.collect_bit_bi_legacy(tile, bel, "ROUNDING_LSB_MASK", "0", "1");
        ctx.collect_enum_legacy(tile, bel, "USE_PATTERN_DETECT", &["PATDET", "NO_PATDET"]);
        ctx.collect_enum_legacy(tile, bel, "USE_SIMD", &["TWO24", "ONE48", "FOUR12"]);
        ctx.collect_enum_legacy(tile, bel, "USE_MULT", &["NONE", "MULT", "MULT_S"]);
        ctx.collect_enum_legacy(tile, bel, "SEL_PATTERN", &["PATTERN", "C"]);
        ctx.collect_enum_legacy(tile, bel, "SEL_MASK", &["MASK", "C"]);
        ctx.collect_bit_bi_legacy(tile, bel, "AUTORESET_OVER_UNDER_FLOW", "FALSE", "TRUE");
        ctx.collect_enum_legacy(
            tile,
            bel,
            "AUTORESET_PATTERN_DETECT_OPTINV",
            &["NOT_MATCH", "MATCH"],
        );
        ctx.collect_bit_bi_legacy(tile, bel, "AUTORESET_PATTERN_DETECT", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "SCAN_IN_SET_M", &["SET", "DONT_SET"]);
        ctx.collect_enum_legacy(tile, bel, "SCAN_IN_SET_P", &["SET", "DONT_SET"]);
        ctx.collect_bit_bi_legacy(tile, bel, "SCAN_IN_SETVAL_M", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "SCAN_IN_SETVAL_P", "0", "1");
        ctx.collect_enum_legacy(tile, bel, "TEST_SET_M", &["SET", "DONT_SET"]);
        ctx.collect_enum_legacy(tile, bel, "TEST_SET_P", &["SET", "DONT_SET"]);
        ctx.collect_bit_bi_legacy(tile, bel, "TEST_SETVAL_M", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "TEST_SETVAL_P", "0", "1");
        ctx.collect_enum_legacy(tile, bel, "LFSR_EN_SET", &["SET", "DONT_SET"]);
        ctx.collect_bit_bi_legacy(tile, bel, "LFSR_EN_SETVAL", "0", "1");

        ctx.collect_bitvec_legacy(tile, bel, "PATTERN", "");
        ctx.collect_bitvec_legacy(tile, bel, "MASK", "");
    }
    for bel in ["DSP[0]", "DSP[1]"] {
        let mut present = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");
        present.discard_bits_legacy(ctx.item_legacy(tile, bel, "SCAN_IN_SET_M"));
        present.discard_bits_legacy(ctx.item_legacy(tile, bel, "SCAN_IN_SET_P"));
        present.discard_bits_legacy(ctx.item_legacy(tile, bel, "TEST_SET_M"));
        present.discard_bits_legacy(ctx.item_legacy(tile, bel, "TEST_SET_P"));
        if bel == "DSP[0]" {
            present.discard_bits_legacy(ctx.item_legacy(tile, "DSP[0]", "LFSR_EN_SET"));
            present.discard_bits_legacy(ctx.item_legacy(tile, "DSP[1]", "LFSR_EN_SET"));
        }
        present.assert_empty();
    }
}

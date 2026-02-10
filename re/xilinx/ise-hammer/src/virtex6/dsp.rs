use prjcombine_re_collector::{
    diff::Diff,
    legacy::{xlat_bit_bi_legacy, xlat_enum_legacy},
};
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

const DSP48E1_INVPINS: &[&str] = &[
    "CLK", "CARRYIN", "OPMODE0", "OPMODE1", "OPMODE2", "OPMODE3", "OPMODE4", "OPMODE5", "OPMODE6",
    "ALUMODE0", "ALUMODE1", "ALUMODE2", "ALUMODE3", "INMODE0", "INMODE1", "INMODE2", "INMODE3",
    "INMODE4",
];

const DSP48E1_TIEPINS: &[&str] = &[
    "ALUMODE2",
    "ALUMODE3",
    "CARRYINSEL2",
    "CEAD",
    "CEALUMODE",
    "CED",
    "CEINMODE",
    "INMODE0",
    "INMODE1",
    "INMODE2",
    "INMODE3",
    "INMODE4",
    "OPMODE6",
    "RSTD",
    "D0",
    "D1",
    "D2",
    "D3",
    "D4",
    "D5",
    "D6",
    "D7",
    "D8",
    "D9",
    "D10",
    "D11",
    "D12",
    "D13",
    "D14",
    "D15",
    "D16",
    "D17",
    "D18",
    "D19",
    "D20",
    "D21",
    "D22",
    "D23",
    "D24",
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let tile = "DSP";
    let mut ctx = FuzzCtx::new_legacy(session, backend, tile);
    for i in 0..2 {
        let bel_other = defs::bslots::DSP[i ^ 1];
        let mut bctx = ctx.bel(defs::bslots::DSP[i]);
        let mode = "DSP48E1";
        bctx.build()
            .bel_unused(bel_other)
            .test_manual_legacy("PRESENT", "1")
            .mode(mode)
            .commit();
        for &pin in DSP48E1_INVPINS {
            bctx.mode(mode).test_inv_legacy(pin);
        }
        let bel_tie = defs::bslots::TIEOFF_DSP;
        for &pin in DSP48E1_TIEPINS {
            let name = format!("MUX.{pin}");
            bctx.mode(mode)
                .mutex(&name, "HARD0")
                .attr("AREG", "0")
                .attr("BREG", "0")
                .test_manual_legacy(&name, "GND")
                .pip(pin, (bel_tie, "HARD0"))
                .commit();
            bctx.mode(mode)
                .mutex(&name, "HARD1")
                .attr("AREG", "0")
                .attr("BREG", "0")
                .test_manual_legacy(&name, "VCC")
                .pip(pin, (bel_tie, "HARD1"))
                .commit();
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
        bctx.mode(mode)
            .pip("INMODE0", (bel_tie, "HARD0"))
            .test_manual_legacy("AREG_ACASCREG", "1_1_INMODE0_GND")
            .attr("AREG", "1")
            .attr("ACASCREG", "1")
            .commit();
        bctx.mode(mode)
            .pip("INMODE4", (bel_tie, "HARD0"))
            .test_manual_legacy("BREG_BCASCREG", "1_1_INMODE4_GND")
            .attr("BREG", "1")
            .attr("BCASCREG", "1")
            .commit();
        for attr in [
            "CREG",
            "MREG",
            "PREG",
            "OPMODEREG",
            "ALUMODEREG",
            "INMODEREG",
            "CARRYINREG",
            "CARRYINSELREG",
        ] {
            bctx.mode(mode).test_enum_legacy(attr, &["0", "1"]);
        }
        for attr in ["DREG", "ADREG"] {
            bctx.mode(mode)
                .attr("USE_MULT", "MULTIPLY")
                .attr("USE_DPORT", "TRUE")
                .test_enum_legacy(attr, &["0", "1"]);
        }
        for attr in ["A_INPUT", "B_INPUT"] {
            bctx.mode(mode)
                .test_enum_legacy(attr, &["DIRECT", "CASCADE"]);
        }
        bctx.mode(mode)
            .test_enum_legacy("USE_PATTERN_DETECT", &["PATDET", "NO_PATDET"]);
        bctx.mode(mode)
            .test_enum_legacy("USE_SIMD", &["TWO24", "ONE48", "FOUR12"]);
        bctx.mode(mode)
            .attr("DREG", "0")
            .attr("ADREG", "0")
            .test_enum_legacy("USE_MULT", &["NONE", "MULTIPLY", "DYNAMIC"]);
        bctx.mode(mode)
            .attr("DREG", "0")
            .attr("ADREG", "0")
            .test_enum_legacy("USE_DPORT", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .test_enum_legacy("SEL_PATTERN", &["PATTERN", "C"]);
        bctx.mode(mode).test_enum_legacy(
            "SEL_MASK",
            &["MASK", "C", "ROUNDING_MODE1", "ROUNDING_MODE2"],
        );
        bctx.mode(mode).test_enum_legacy(
            "AUTORESET_PATDET",
            &["RESET_MATCH", "RESET_NOT_MATCH", "NO_RESET"],
        );
        bctx.mode(mode).test_multi_attr_hex_legacy("PATTERN", 48);
        bctx.mode(mode).test_multi_attr_hex_legacy("MASK", 48);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "DSP";
    for bel in ["DSP[0]", "DSP[1]"] {
        for &pin in DSP48E1_INVPINS {
            ctx.collect_inv_legacy(tile, bel, pin);
        }
        for &pin in DSP48E1_TIEPINS {
            let attr = format!("MUX.{pin}");
            let gnd = ctx.get_diff_legacy(tile, bel, &attr, "GND");
            let vcc = ctx.get_diff_legacy(tile, bel, &attr, "VCC");
            ctx.insert_legacy(
                tile,
                bel,
                attr,
                xlat_enum_legacy(vec![("INT", Diff::default()), ("GND", gnd), ("VCC", vcc)]),
            );
        }

        ctx.collect_enum_legacy(
            tile,
            bel,
            "AREG_ACASCREG",
            &["0_0", "1_1_INMODE0_GND", "1_1", "2_1", "2_2"],
        );
        ctx.collect_enum_legacy(
            tile,
            bel,
            "BREG_BCASCREG",
            &["0_0", "1_1_INMODE4_GND", "1_1", "2_1", "2_2"],
        );
        for attr in [
            "CREG",
            "MREG",
            "PREG",
            "OPMODEREG",
            "ALUMODEREG",
            "INMODEREG",
            "CARRYINREG",
            "CARRYINSELREG",
            "DREG",
            "ADREG",
        ] {
            ctx.collect_enum_legacy(tile, bel, attr, &["0", "1"]);
        }
        ctx.collect_enum_legacy(tile, bel, "A_INPUT", &["DIRECT", "CASCADE"]);
        ctx.collect_enum_legacy(tile, bel, "B_INPUT", &["DIRECT", "CASCADE"]);
        ctx.get_diff_legacy(tile, bel, "USE_PATTERN_DETECT", "PATDET")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "USE_PATTERN_DETECT", "NO_PATDET")
            .assert_empty();
        ctx.collect_enum_legacy(tile, bel, "USE_SIMD", &["TWO24", "ONE48", "FOUR12"]);
        let d0 = ctx.get_diff_legacy(tile, bel, "USE_MULT", "NONE");
        let d1 = ctx.get_diff_legacy(tile, bel, "USE_MULT", "MULTIPLY");
        assert_eq!(d1, ctx.get_diff_legacy(tile, bel, "USE_MULT", "DYNAMIC"));
        ctx.insert_legacy(tile, bel, "USE_MULT", xlat_bit_bi_legacy(d0, d1));
        ctx.collect_bit_bi_legacy(tile, bel, "USE_DPORT", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "SEL_PATTERN", &["PATTERN", "C"]);
        ctx.collect_enum_legacy(
            tile,
            bel,
            "SEL_MASK",
            &["MASK", "C", "ROUNDING_MODE1", "ROUNDING_MODE2"],
        );
        ctx.collect_enum_legacy(
            tile,
            bel,
            "AUTORESET_PATDET",
            &["RESET_MATCH", "RESET_NOT_MATCH", "NO_RESET"],
        );

        ctx.collect_bitvec_legacy(tile, bel, "PATTERN", "");
        ctx.collect_bitvec_legacy(tile, bel, "MASK", "");
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
    }
}

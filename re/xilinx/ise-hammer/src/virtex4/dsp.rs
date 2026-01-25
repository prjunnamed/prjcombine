use prjcombine_re_collector::diff::{Diff, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

const DSP48_INVPINS: &[&str] = &[
    "CLK",
    "CEA",
    "CEB",
    "CEM",
    "CEP",
    "CECTRL",
    "CECARRYIN",
    "CECINSUB",
    "RSTA",
    "RSTB",
    "RSTM",
    "RSTP",
    "RSTCTRL",
    "RSTCARRYIN",
    "CARRYINSEL0",
    "CARRYINSEL1",
    "CARRYIN",
    "SUBTRACT",
    "OPMODE0",
    "OPMODE1",
    "OPMODE2",
    "OPMODE3",
    "OPMODE4",
    "OPMODE5",
    "OPMODE6",
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let tile = "DSP";
    let mut ctx = FuzzCtx::new(session, backend, tile);
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::DSP[i]);
        let bel_other = defs::bslots::DSP[i ^ 1];
        let mode = "DSP48";
        bctx.build()
            .bel_unused(bel_other)
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();
        for &pin in DSP48_INVPINS {
            bctx.mode(mode).test_inv(pin);
        }
        for pin in ["CEC", "RSTC"] {
            bctx.mode(mode).bel_unused(bel_other).test_inv(pin);
        }
        for attr in ["AREG", "BREG"] {
            bctx.mode(mode).test_enum(attr, &["0", "1", "2"]);
        }
        bctx.mode(mode)
            .bel_mode(bel_other, mode)
            .bel_attr(bel_other, "CREG", "")
            .test_enum("CREG", &["0", "1"]);
        for attr in [
            "MREG",
            "PREG",
            "OPMODEREG",
            "CARRYINREG",
            "CARRYINSELREG",
            "SUBTRACTREG",
        ] {
            bctx.mode(mode).test_enum(attr, &["0", "1"]);
        }
        bctx.mode(mode).test_enum("B_INPUT", &["DIRECT", "CASCADE"]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "DSP";
    for (pininv, pin, pin_b) in [("CECINV", "CEC", "CEC_B"), ("RSTCINV", "RSTC", "RSTC_B")] {
        let ti0 = ctx.extract_enum_bool(tile, "DSP[0]", pininv, pin, pin_b);
        let ti1 = ctx.extract_enum_bool(tile, "DSP[1]", pininv, pin, pin_b);
        assert_eq!(ti0, ti1);
        ctx.insert_int_inv(&["INT"; 4], tile, "DSP[0]", pin, ti0);
    }
    let d0_0 = ctx.get_diff(tile, "DSP[0]", "CREG", "0");
    let d0_1 = ctx.get_diff(tile, "DSP[0]", "CREG", "1");
    let d1_0 = ctx.get_diff(tile, "DSP[1]", "CREG", "0");
    let d1_1 = ctx.get_diff(tile, "DSP[1]", "CREG", "1");
    let (d0_0, d1_0, dc_0) = Diff::split(d0_0, d1_0);
    let (d0_1, d1_1, dc_1) = Diff::split(d0_1, d1_1);
    ctx.insert(
        tile,
        "DSP_COMMON",
        "CREG",
        xlat_enum(vec![("0", dc_0), ("1", dc_1)]),
    );
    d0_0.assert_empty();
    d1_0.assert_empty();
    ctx.insert(
        tile,
        "DSP_COMMON",
        "CLKC_MUX",
        xlat_enum(vec![("DSP0", d0_1), ("DSP1", d1_1)]),
    );
    for bel in ["DSP[0]", "DSP[1]"] {
        for &pin in DSP48_INVPINS {
            if pin.starts_with("CLK") || pin.starts_with("RST") || pin.starts_with("CE") {
                ctx.collect_int_inv(&["INT"; 4], tile, bel, pin, false);
            } else {
                ctx.collect_inv(tile, bel, pin);
            }
        }
        let mut present = ctx.get_diff(tile, bel, "PRESENT", "1");
        for attr in ["AREG", "BREG"] {
            ctx.collect_enum(tile, bel, attr, &["0", "1", "2"]);
            present.discard_bits(ctx.item(tile, bel, attr));
        }
        for attr in [
            "MREG",
            "PREG",
            "OPMODEREG",
            "CARRYINREG",
            "CARRYINSELREG",
            "SUBTRACTREG",
        ] {
            ctx.collect_enum(tile, bel, attr, &["0", "1"]);
            present.discard_bits(ctx.item(tile, bel, attr));
        }
        ctx.collect_enum(tile, bel, "B_INPUT", &["DIRECT", "CASCADE"]);
        present.discard_bits(ctx.item(tile, "DSP_COMMON", "CREG"));
        present.discard_bits(ctx.item(tile, "DSP_COMMON", "CLKC_MUX"));
        ctx.insert(
            tile,
            bel,
            "UNK_PRESENT",
            xlat_enum(vec![("0", Diff::default()), ("1", present)]),
        );
    }
}

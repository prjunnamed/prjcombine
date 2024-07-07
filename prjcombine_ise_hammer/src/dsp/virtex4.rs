use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{xlat_enum, CollectorCtx, Diff},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_one,
};

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

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let tile = "DSP";
    for (i, bel_name) in [(0, "DSP0"), (1, "DSP1")] {
        let bel_other = BelId::from_idx(i ^ 1);
        let ctx = FuzzCtx::new(session, backend, tile, bel_name, TileBits::MainAuto);
        let bel_kind = "DSP48";
        fuzz_one!(ctx, "PRESENT", "1", [(bel_unused bel_other)], [(mode bel_kind)]);
        for &pin in DSP48_INVPINS {
            fuzz_inv!(ctx, pin, [(mode bel_kind)]);
        }
        for pin in ["CEC", "RSTC"] {
            fuzz_inv!(ctx, pin, [(mode bel_kind), (bel_unused bel_other)]);
        }
        for attr in ["AREG", "BREG"] {
            fuzz_enum!(ctx, attr, ["0", "1", "2"], [(mode bel_kind)]);
        }
        fuzz_enum!(ctx, "CREG", ["0", "1"], [
            (mode bel_kind),
            (bel_mode bel_other, bel_kind),
            (bel_attr bel_other, "CREG", "")
        ]);
        for attr in [
            "MREG",
            "PREG",
            "OPMODEREG",
            "CARRYINREG",
            "CARRYINSELREG",
            "SUBTRACTREG",
        ] {
            fuzz_enum!(ctx, attr, ["0", "1"], [(mode bel_kind)]);
        }
        fuzz_enum!(ctx, "B_INPUT", ["DIRECT", "CASCADE"], [(mode bel_kind)]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "DSP";
    for (pininv, pin, pin_b) in [("CECINV", "CEC", "CEC_B"), ("RSTCINV", "RSTC", "RSTC_B")] {
        let ti0 = ctx.extract_enum_bool(tile, "DSP0", pininv, pin, pin_b);
        let ti1 = ctx.extract_enum_bool(tile, "DSP1", pininv, pin, pin_b);
        assert_eq!(ti0, ti1);
        ctx.insert_int_inv(&["INT"; 4], tile, "DSP0", pin, ti0);
    }
    let d0_0 = ctx.state.get_diff(tile, "DSP0", "CREG", "0");
    let d0_1 = ctx.state.get_diff(tile, "DSP0", "CREG", "1");
    let d1_0 = ctx.state.get_diff(tile, "DSP1", "CREG", "0");
    let d1_1 = ctx.state.get_diff(tile, "DSP1", "CREG", "1");
    let (d0_0, d1_0, dc_0) = Diff::split(d0_0, d1_0);
    let (d0_1, d1_1, dc_1) = Diff::split(d0_1, d1_1);
    ctx.tiledb.insert(
        tile,
        "DSP_COMMON",
        "CREG",
        xlat_enum(vec![("0", dc_0), ("1", dc_1)]),
    );
    d0_0.assert_empty();
    d1_0.assert_empty();
    ctx.tiledb.insert(
        tile,
        "DSP_COMMON",
        "CLKC_MUX",
        xlat_enum(vec![("DSP0", d0_1), ("DSP1", d1_1)]),
    );
    for bel in ["DSP0", "DSP1"] {
        for &pin in DSP48_INVPINS {
            if pin.starts_with("CLK") || pin.starts_with("RST") || pin.starts_with("CE") {
                ctx.collect_int_inv(&["INT"; 4], tile, bel, pin, false);
            } else {
                ctx.collect_inv(tile, bel, pin);
            }
        }
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        for attr in ["AREG", "BREG"] {
            ctx.collect_enum(tile, bel, attr, &["0", "1", "2"]);
            present.discard_bits(ctx.tiledb.item(tile, bel, attr));
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
            present.discard_bits(ctx.tiledb.item(tile, bel, attr));
        }
        ctx.collect_enum(tile, bel, "B_INPUT", &["DIRECT", "CASCADE"]);
        present.discard_bits(ctx.tiledb.item(tile, "DSP_COMMON", "CREG"));
        present.discard_bits(ctx.tiledb.item(tile, "DSP_COMMON", "CLKC_MUX"));
        ctx.tiledb.insert(
            tile,
            bel,
            "UNK_PRESENT",
            xlat_enum(vec![("0", Diff::default()), ("1", present)]),
        );
    }
}

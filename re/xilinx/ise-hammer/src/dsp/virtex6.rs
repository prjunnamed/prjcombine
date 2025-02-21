use prjcombine_re_collector::{xlat_bool, xlat_enum, Diff};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_inv,
    fuzz_multi, fuzz_one,
};

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

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let tile = "DSP";
    for (i, bel) in [(0, "DSP0"), (1, "DSP1")] {
        let bel_other = BelId::from_idx(i ^ 1);
        let ctx = FuzzCtx::new(session, backend, tile, bel, TileBits::MainAuto);
        let bel_kind = "DSP48E1";
        fuzz_one!(ctx, "PRESENT", "1", [(bel_unused bel_other)], [(mode bel_kind)]);
        for &pin in DSP48E1_INVPINS {
            fuzz_inv!(ctx, pin, [(mode bel_kind)]);
        }
        let bel_tie = BelId::from_idx(2);
        for &pin in DSP48E1_TIEPINS {
            let name = format!("MUX.{pin}");
            fuzz_one!(ctx, &name, "GND", [
                (mode bel_kind),
                (mutex &name, "HARD0"),
                (attr "AREG", "0"),
                (attr "BREG", "0")
            ], [
                (pip (bel_pin bel_tie, "HARD0"), (pin pin))
            ]);
            fuzz_one!(ctx, &name, "VCC", [
                (mode bel_kind),
                (mutex &name, "HARD1"),
                (attr "AREG", "0"),
                (attr "BREG", "0")
            ], [
                (pip (bel_pin bel_tie, "HARD1"), (pin pin))
            ]);
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
                fuzz_one!(ctx, aname, vname, [(mode bel_kind)], [
                    (attr attr, val),
                    (attr attrcasc, valcasc)
                ]);
            }
        }
        fuzz_one!(ctx, "AREG_ACASCREG", "1_1_INMODE0_GND", [
            (mode bel_kind),
            (pip (bel_pin bel_tie, "HARD0"), (pin "INMODE0"))
        ], [
            (attr "AREG", "1"),
            (attr "ACASCREG", "1")
        ]);
        fuzz_one!(ctx, "BREG_BCASCREG", "1_1_INMODE4_GND", [
            (mode bel_kind),
            (pip (bel_pin bel_tie, "HARD0"), (pin "INMODE4"))
        ], [
            (attr "BREG", "1"),
            (attr "BCASCREG", "1")
        ]);
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
            fuzz_enum!(ctx, attr, ["0", "1"], [(mode bel_kind)]);
        }
        for attr in ["DREG", "ADREG"] {
            fuzz_enum!(ctx, attr, ["0", "1"], [
                (mode bel_kind),
                (attr "USE_MULT", "MULTIPLY"),
                (attr "USE_DPORT", "TRUE")
            ]);
        }
        for attr in ["A_INPUT", "B_INPUT"] {
            fuzz_enum!(ctx, attr, ["DIRECT", "CASCADE"], [(mode bel_kind)]);
        }
        fuzz_enum!(ctx, "USE_PATTERN_DETECT", ["PATDET", "NO_PATDET"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "USE_SIMD", ["TWO24", "ONE48", "FOUR12"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "USE_MULT", ["NONE", "MULTIPLY", "DYNAMIC"], [
            (mode bel_kind),
            (attr "DREG", "0"),
            (attr "ADREG", "0")
        ]);
        fuzz_enum!(ctx, "USE_DPORT", ["FALSE", "TRUE"], [
            (mode bel_kind),
            (attr "DREG", "0"),
            (attr "ADREG", "0")
        ]);
        fuzz_enum!(ctx, "SEL_PATTERN", ["PATTERN", "C"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "SEL_MASK", ["MASK", "C", "ROUNDING_MODE1", "ROUNDING_MODE2"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "AUTORESET_PATDET", ["RESET_MATCH", "RESET_NOT_MATCH", "NO_RESET"], [(mode bel_kind)]);
        fuzz_multi!(ctx, "PATTERN", "", 48, [(mode bel_kind)], (attr_hex "PATTERN"));
        fuzz_multi!(ctx, "MASK", "", 48, [(mode bel_kind)], (attr_hex "MASK"));
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "DSP";
    for bel in ["DSP0", "DSP1"] {
        for &pin in DSP48E1_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for &pin in DSP48E1_TIEPINS {
            let attr = format!("MUX.{pin}");
            let gnd = ctx.state.get_diff(tile, bel, &attr, "GND");
            let vcc = ctx.state.get_diff(tile, bel, &attr, "VCC");
            ctx.tiledb.insert(
                tile,
                bel,
                attr,
                xlat_enum(vec![("INT", Diff::default()), ("GND", gnd), ("VCC", vcc)]),
            );
        }

        ctx.collect_enum(
            tile,
            bel,
            "AREG_ACASCREG",
            &["0_0", "1_1_INMODE0_GND", "1_1", "2_1", "2_2"],
        );
        ctx.collect_enum(
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
            ctx.collect_enum(tile, bel, attr, &["0", "1"]);
        }
        ctx.collect_enum(tile, bel, "A_INPUT", &["DIRECT", "CASCADE"]);
        ctx.collect_enum(tile, bel, "B_INPUT", &["DIRECT", "CASCADE"]);
        ctx.state
            .get_diff(tile, bel, "USE_PATTERN_DETECT", "PATDET")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "USE_PATTERN_DETECT", "NO_PATDET")
            .assert_empty();
        ctx.collect_enum(tile, bel, "USE_SIMD", &["TWO24", "ONE48", "FOUR12"]);
        let d0 = ctx.state.get_diff(tile, bel, "USE_MULT", "NONE");
        let d1 = ctx.state.get_diff(tile, bel, "USE_MULT", "MULTIPLY");
        assert_eq!(d1, ctx.state.get_diff(tile, bel, "USE_MULT", "DYNAMIC"));
        ctx.tiledb.insert(tile, bel, "USE_MULT", xlat_bool(d0, d1));
        ctx.collect_enum_bool(tile, bel, "USE_DPORT", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SEL_PATTERN", &["PATTERN", "C"]);
        ctx.collect_enum(
            tile,
            bel,
            "SEL_MASK",
            &["MASK", "C", "ROUNDING_MODE1", "ROUNDING_MODE2"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "AUTORESET_PATDET",
            &["RESET_MATCH", "RESET_NOT_MATCH", "NO_RESET"],
        );

        ctx.collect_bitvec(tile, bel, "PATTERN", "");
        ctx.collect_bitvec(tile, bel, "MASK", "");
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    }
}

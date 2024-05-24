use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, State},
    diff::{
        collect_bitvec, collect_enum, collect_enum_bool, collect_inv, extract_enum_bool, xlat_enum,
        Diff,
    },
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_one,
    tiledb::TileDb,
};

const DSP48E_INVPINS: &[&str] = &[
    "CLK", "CARRYIN", "OPMODE0", "OPMODE1", "OPMODE2", "OPMODE3", "OPMODE4", "OPMODE5", "OPMODE6",
    "ALUMODE0", "ALUMODE1", "ALUMODE2", "ALUMODE3",
];

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let node_kind = backend.egrid.db.get_node("DSP");
    let tile = "DSP";
    for (i, bel_name) in [(0, "DSP0"), (1, "DSP1")] {
        let bel = BelId::from_idx(i);
        let bel_other = BelId::from_idx(i ^ 1);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Main(5),
            tile_name: tile,
            bel,
            bel_name,
        };
        let bel_kind = "DSP48E";
        fuzz_one!(ctx, "PRESENT", "1", [(bel_unused bel_other)], [(mode bel_kind)]);
        for &pin in DSP48E_INVPINS {
            let pininv = format!("{pin}INV").leak();
            let pin_b = format!("{pin}_B").leak();
            fuzz_enum!(ctx, pininv, [pin, pin_b], [(mode bel_kind), (pin pin)]);
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
            fuzz_enum!(ctx, attr, ["0", "1"], [(mode bel_kind)]);
        }
        for attr in ["A_INPUT", "B_INPUT"] {
            fuzz_enum!(ctx, attr, ["DIRECT", "CASCADE"], [(mode bel_kind)]);
        }
        for attr in ["CLOCK_INVERT_P", "CLOCK_INVERT_M"] {
            fuzz_enum!(ctx, attr, ["SAME_EDGE", "OPPOSITE_EDGE"], [(mode bel_kind)]);
        }
        fuzz_enum!(ctx, "SEL_ROUNDING_MASK", ["SEL_MASK", "MODE2", "MODE1"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "ROUNDING_LSB_MASK", ["1", "0"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "USE_PATTERN_DETECT", ["PATDET", "NO_PATDET"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "USE_SIMD", ["TWO24", "ONE48", "FOUR12"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "USE_MULT", ["NONE", "MULT", "MULT_S"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "SEL_PATTERN", ["PATTERN", "C"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "SEL_MASK", ["MASK", "C"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "AUTORESET_OVER_UNDER_FLOW", ["TRUE", "FALSE"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "AUTORESET_PATTERN_DETECT_OPTINV", ["NOT_MATCH", "MATCH"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "AUTORESET_PATTERN_DETECT", ["TRUE", "FALSE"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "SCAN_IN_SET_M", ["SET", "DONT_SET"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "SCAN_IN_SET_P", ["SET", "DONT_SET"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "SCAN_IN_SETVAL_M", ["1", "0"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "SCAN_IN_SETVAL_P", ["1", "0"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "TEST_SET_M", ["SET", "DONT_SET"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "TEST_SET_P", ["SET", "DONT_SET"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "TEST_SETVAL_M", ["1", "0"], [(mode bel_kind)]);
        fuzz_enum!(ctx, "TEST_SETVAL_P", ["1", "0"], [(mode bel_kind)]);
        if bel_name == "DSP0" {
            fuzz_enum!(ctx, "LFSR_EN_SET", ["SET", "DONT_SET"], [
                (mode bel_kind),
                (bel_mode bel_other, bel_kind),
                (bel_attr bel_other, "LFSR_EN_SET", "DONT_SET")
            ]);
        } else {
            fuzz_enum!(ctx, "LFSR_EN_SET", ["SET", "DONT_SET"], [
                (mode bel_kind),
                (bel_unused bel_other)
            ]);
        }
        fuzz_enum!(ctx, "LFSR_EN_SETVAL", ["1", "0"], [(mode bel_kind)]);
        fuzz_multi!(ctx, "PATTERN", "", 48, [(mode bel_kind)], (attr_hex "PATTERN"));
        fuzz_multi!(ctx, "MASK", "", 48, [(mode bel_kind)], (attr_hex "MASK"));
    }
}

pub fn collect_fuzzers(state: &mut State, tiledb: &mut TileDb) {
    let tile = "DSP";
    for bel in ["DSP0", "DSP1"] {
        for &pin in DSP48E_INVPINS {
            collect_inv(state, tiledb, tile, bel, pin);
        }
        for attr in ["AREG_ACASCREG", "BREG_BCASCREG"] {
            collect_enum(
                state,
                tiledb,
                tile,
                bel,
                attr,
                &["0_0", "1_1", "2_1", "2_2"],
            );
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
            collect_enum(state, tiledb, tile, bel, attr, &["0", "1"]);
        }
        collect_enum(state, tiledb, tile, bel, "A_INPUT", &["DIRECT", "CASCADE"]);
        collect_enum(state, tiledb, tile, bel, "B_INPUT", &["DIRECT", "CASCADE"]);
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "CLOCK_INVERT_P",
            &["SAME_EDGE", "OPPOSITE_EDGE"],
        );
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "CLOCK_INVERT_M",
            &["SAME_EDGE", "OPPOSITE_EDGE"],
        );
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "SEL_ROUNDING_MASK",
            &["SEL_MASK", "MODE2", "MODE1"],
        );
        collect_enum_bool(state, tiledb, tile, bel, "ROUNDING_LSB_MASK", "0", "1");
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "USE_PATTERN_DETECT",
            &["PATDET", "NO_PATDET"],
        );
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "USE_SIMD",
            &["TWO24", "ONE48", "FOUR12"],
        );
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "USE_MULT",
            &["NONE", "MULT", "MULT_S"],
        );
        collect_enum(state, tiledb, tile, bel, "SEL_PATTERN", &["PATTERN", "C"]);
        collect_enum(state, tiledb, tile, bel, "SEL_MASK", &["MASK", "C"]);
        collect_enum_bool(
            state,
            tiledb,
            tile,
            bel,
            "AUTORESET_OVER_UNDER_FLOW",
            "FALSE",
            "TRUE",
        );
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "AUTORESET_PATTERN_DETECT_OPTINV",
            &["NOT_MATCH", "MATCH"],
        );
        collect_enum_bool(
            state,
            tiledb,
            tile,
            bel,
            "AUTORESET_PATTERN_DETECT",
            "FALSE",
            "TRUE",
        );
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "SCAN_IN_SET_M",
            &["SET", "DONT_SET"],
        );
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "SCAN_IN_SET_P",
            &["SET", "DONT_SET"],
        );
        collect_enum_bool(state, tiledb, tile, bel, "SCAN_IN_SETVAL_M", "0", "1");
        collect_enum_bool(state, tiledb, tile, bel, "SCAN_IN_SETVAL_P", "0", "1");
        collect_enum(state, tiledb, tile, bel, "TEST_SET_M", &["SET", "DONT_SET"]);
        collect_enum(state, tiledb, tile, bel, "TEST_SET_P", &["SET", "DONT_SET"]);
        collect_enum_bool(state, tiledb, tile, bel, "TEST_SETVAL_M", "0", "1");
        collect_enum_bool(state, tiledb, tile, bel, "TEST_SETVAL_P", "0", "1");
        collect_enum(
            state,
            tiledb,
            tile,
            bel,
            "LFSR_EN_SET",
            &["SET", "DONT_SET"],
        );
        collect_enum_bool(state, tiledb, tile, bel, "LFSR_EN_SETVAL", "0", "1");

        collect_bitvec(state, tiledb, tile, bel, "PATTERN", "");
        collect_bitvec(state, tiledb, tile, bel, "MASK", "");
        /*
        tiledb.insert(
            tile,
            bel,
            "UNK_PRESENT",
            xlat_enum(vec![
                ("0".to_string(), Diff::default()),
                ("1".to_string(), present),
                ]),
            );
        */
    }
    for bel in ["DSP0", "DSP1"] {
        let mut present = state.get_diff(tile, bel, "PRESENT", "1");
        present.discard_bits(tiledb.item(tile, bel, "SCAN_IN_SET_M"));
        present.discard_bits(tiledb.item(tile, bel, "SCAN_IN_SET_P"));
        present.discard_bits(tiledb.item(tile, bel, "TEST_SET_M"));
        present.discard_bits(tiledb.item(tile, bel, "TEST_SET_P"));
        if bel == "DSP0" {
            present.discard_bits(tiledb.item(tile, "DSP0", "LFSR_EN_SET"));
            present.discard_bits(tiledb.item(tile, "DSP1", "LFSR_EN_SET"));
        }
        present.assert_empty();
    }
}

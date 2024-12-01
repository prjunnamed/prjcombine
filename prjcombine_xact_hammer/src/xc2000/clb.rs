use bitvec::prelude::*;
use prjcombine_collector::{xlat_bit, xlat_enum, Diff};
use prjcombine_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for tile in backend.egrid.db.nodes.keys() {
        if !tile.starts_with("CLB") {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel("CLB");
        bctx.test_mode("F");
        bctx.test_mode("FG");
        bctx.test_mode("FGM");
        bctx.mode("FG").test_equate("F", &["B", "C", "Q"]);
        bctx.mode("FG").test_equate("G", &["B", "C", "Q"]);
        bctx.mode("FG")
            .test_manual("F", "ABC")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .commit();
        bctx.mode("FG")
            .test_manual("F", "ABD")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "D")
            .commit();
        bctx.mode("FG")
            .test_manual("F", "ACD")
            .cfg("F", "A")
            .cfg("F", "C")
            .cfg("F", "D")
            .commit();
        bctx.mode("FG")
            .test_manual("F", "BCD")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .commit();
        bctx.mode("FG")
            .test_manual("F", "ABQ")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "Q")
            .commit();
        bctx.mode("FG")
            .test_manual("F", "ACQ")
            .cfg("F", "A")
            .cfg("F", "C")
            .cfg("F", "Q")
            .commit();
        bctx.mode("FG")
            .test_manual("F", "BCQ")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "Q")
            .commit();
        bctx.mode("FGM")
            .mutex("FGM", "F")
            .test_manual("F", "MACD")
            .cfg("F", "A")
            .cfg("F", "C")
            .cfg("F", "D")
            .commit();
        bctx.mode("FG")
            .test_manual("G", "ABC")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .commit();
        bctx.mode("FG")
            .test_manual("G", "ABD")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "D")
            .commit();
        bctx.mode("FG")
            .test_manual("G", "ACD")
            .cfg("G", "A")
            .cfg("G", "C")
            .cfg("G", "D")
            .commit();
        bctx.mode("FG")
            .test_manual("G", "BCD")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .commit();
        bctx.mode("FG")
            .test_manual("G", "ABQ")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "Q")
            .commit();
        bctx.mode("FG")
            .test_manual("G", "ACQ")
            .cfg("G", "A")
            .cfg("G", "C")
            .cfg("G", "Q")
            .commit();
        bctx.mode("FG")
            .test_manual("G", "BCQ")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "Q")
            .commit();
        bctx.mode("FGM")
            .mutex("FGM", "G")
            .test_manual("G", "MACD")
            .cfg("G", "A")
            .cfg("G", "C")
            .cfg("G", "D")
            .commit();
        for lut in ["F", "G"] {
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_ABC",
                &["A", "B", "C"],
                bitvec![1, 0, 0, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_ABC_NA",
                &["A", "B", "C"],
                bitvec![0, 1, 0, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_ABC_NB",
                &["A", "B", "C"],
                bitvec![0, 0, 1, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_ABC_NC",
                &["A", "B", "C"],
                bitvec![0, 0, 0, 0, 1, 0, 0, 0],
            );

            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_BCD",
                &["B", "C", "D"],
                bitvec![1, 0, 0, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_BCD_NB",
                &["B", "C", "D"],
                bitvec![0, 1, 0, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_BCD_NC",
                &["B", "C", "D"],
                bitvec![0, 0, 1, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_BCD_ND",
                &["B", "C", "D"],
                bitvec![0, 0, 0, 0, 1, 0, 0, 0],
            );

            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_BCQ",
                &["B", "C", "Q"],
                bitvec![1, 0, 0, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_BCQ_NB",
                &["B", "C", "Q"],
                bitvec![0, 1, 0, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_BCQ_NC",
                &["B", "C", "Q"],
                bitvec![0, 0, 1, 0, 0, 0, 0, 0],
            );
            bctx.mode("FG").test_equate_fixed(
                lut,
                "EQ_BCQ_NQ",
                &["B", "C", "Q"],
                bitvec![0, 0, 0, 0, 1, 0, 0, 0],
            );
        }
        bctx.mode("F").test_equate_fixed(
            "F",
            "EQ_ABCD",
            &["A", "B", "C", "D"],
            bitvec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        bctx.mode("F").test_equate_fixed(
            "F",
            "EQ_ABCD_NA",
            &["A", "B", "C", "D"],
            bitvec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        bctx.mode("F").test_equate_fixed(
            "F",
            "EQ_ABCD_NB",
            &["A", "B", "C", "D"],
            bitvec![0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        bctx.mode("F").test_equate_fixed(
            "F",
            "EQ_ABCD_NC",
            &["A", "B", "C", "D"],
            bitvec![0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        bctx.mode("F").test_equate_fixed(
            "F",
            "EQ_ABCD_ND",
            &["A", "B", "C", "D"],
            bitvec![0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
        );

        bctx.mode("FG").test_enum("SET", &["A", "F"]);
        bctx.mode("FG").test_enum("RES", &["D", "G"]);
        bctx.mode("FG").test_enum("CLK", &["C", "G"]);
        bctx.mode("FG")
            .mutex("Q", "FF")
            .cfg("Q", "FF")
            .mutex("CLK", "C")
            .cfg("CLK", "C")
            .test_cfg("CLK", "NOT");
        bctx.mode("FG")
            .mutex("CLK", "C")
            .cfg("CLK", "C")
            .test_enum("Q", &["LATCH", "FF"]);
        bctx.mode("FG").test_enum("X", &["F", "G", "Q"]);
        bctx.mode("FG").test_enum("Y", &["F", "G", "Q"]);
        bctx.mode("FGM")
            .mutex("FGM", "F")
            .cfg("F", "A")
            .cfg("F", "C")
            .cfg("F", "D")
            .test_cfg("X", "M");
        bctx.mode("FGM")
            .mutex("FGM", "F")
            .cfg("F", "A")
            .cfg("F", "C")
            .cfg("F", "D")
            .test_cfg("Y", "M");
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ctx.edev.egrid.db.nodes.keys() {
        if !tile.starts_with("CLB") {
            continue;
        }
        let bel = "CLB";
        for attr in ["X", "Y"] {
            let item = xlat_enum(vec![
                ("Q", ctx.state.get_diff(tile, bel, attr, "Q")),
                ("F", ctx.state.get_diff(tile, bel, attr, "F")),
                ("G", ctx.state.get_diff(tile, bel, attr, "G")),
                ("F", ctx.state.get_diff(tile, bel, attr, "M")),
            ]);
            ctx.tiledb.insert(tile, bel, format!("MUX.{attr}"), item);
        }
        ctx.collect_enum_default(tile, bel, "RES", &["D", "G"], "NONE");
        ctx.collect_enum_default(tile, bel, "SET", &["A", "F"], "NONE");
        let diff_inv = ctx.state.get_diff(tile, bel, "CLK", "NOT");
        let diff_latch = ctx.state.get_diff(tile, bel, "Q", "LATCH");
        let diff_ff = ctx.state.get_diff(tile, bel, "Q", "FF");
        assert_eq!(diff_latch, diff_inv);
        ctx.tiledb.insert(tile, bel, "INV.K", xlat_bit(diff_inv));
        ctx.tiledb.insert(
            tile,
            bel,
            "FF_MODE",
            xlat_enum(vec![("FF", diff_ff), ("LATCH", Diff::default())]),
        );
        for lut in ["F", "G"] {
            ctx.collect_bitvec(tile, bel, lut, "");
            let diff_abc = ctx.state.get_diff(tile, bel, lut, "ABC");
            let diff_abd = ctx.state.get_diff(tile, bel, lut, "ABD");
            let diff_abq = ctx.state.get_diff(tile, bel, lut, "ABQ");
            let diff_acd = ctx.state.get_diff(tile, bel, lut, "ACD");
            let diff_acq = ctx.state.get_diff(tile, bel, lut, "ACQ");
            let diff_bcd = ctx.state.get_diff(tile, bel, lut, "BCD");
            let diff_bcq = ctx.state.get_diff(tile, bel, lut, "BCQ");
            let diff_macd = ctx.state.get_diff(tile, bel, lut, "MACD");
            let diff_m = diff_macd.combine(&!&diff_acd);
            ctx.tiledb.insert(
                tile,
                bel,
                "MODE",
                xlat_enum(vec![("FG", Diff::default()), ("FGM", diff_m)]),
            );
            let diff_i3_c = diff_abc.combine(&!&diff_abq);
            let diff_i3_d = diff_abd.combine(&!&diff_abq);
            assert_eq!(diff_i3_d, diff_acd.combine(&!&diff_acq));
            assert_eq!(diff_i3_d, diff_bcd.combine(&!&diff_bcq));
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.{lut}3"),
                xlat_enum(vec![
                    ("C", diff_i3_c),
                    ("D", diff_i3_d),
                    ("Q", Diff::default()),
                ]),
            );
            let diff_i1_a = diff_acq.combine(&!&diff_bcq);
            let diff_i2_b = diff_abq.combine(&!&diff_acq);
            diff_bcq.assert_empty();
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.{lut}1"),
                xlat_enum(vec![("A", diff_i1_a), ("B", Diff::default())]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.{lut}2"),
                xlat_enum(vec![("B", diff_i2_b), ("C", Diff::default())]),
            );

            for (val, bits) in [
                ("EQ_ABC", bits![1, 0, 0, 0, 0, 0, 0, 0]),
                ("EQ_ABC_NA", bits![0, 1, 0, 0, 0, 0, 0, 0]),
                ("EQ_ABC_NB", bits![0, 0, 1, 0, 0, 0, 0, 0]),
                ("EQ_ABC_NC", bits![0, 0, 0, 0, 1, 0, 0, 0]),
                ("EQ_BCD", bits![1, 0, 0, 0, 0, 0, 0, 0]),
                ("EQ_BCD_NB", bits![0, 1, 0, 0, 0, 0, 0, 0]),
                ("EQ_BCD_NC", bits![0, 0, 1, 0, 0, 0, 0, 0]),
                ("EQ_BCD_ND", bits![0, 0, 0, 0, 1, 0, 0, 0]),
                ("EQ_BCQ", bits![1, 0, 0, 0, 0, 0, 0, 0]),
                ("EQ_BCQ_NB", bits![0, 1, 0, 0, 0, 0, 0, 0]),
                ("EQ_BCQ_NC", bits![0, 0, 1, 0, 0, 0, 0, 0]),
                ("EQ_BCQ_NQ", bits![0, 0, 0, 0, 1, 0, 0, 0]),
            ] {
                let mut diff = ctx.state.get_diff(tile, bel, lut, val);
                diff.apply_bitvec_diff(ctx.tiledb.item(tile, bel, lut), bits, &bitvec![0; 8]);
                if val.starts_with("EQ_ABC") {
                    diff.apply_enum_diff(
                        ctx.tiledb.item(tile, bel, &format!("MUX.{lut}1")),
                        "A",
                        "B",
                    );
                    diff.apply_enum_diff(
                        ctx.tiledb.item(tile, bel, &format!("MUX.{lut}2")),
                        "B",
                        "C",
                    );
                    diff.apply_enum_diff(
                        ctx.tiledb.item(tile, bel, &format!("MUX.{lut}3")),
                        "C",
                        "Q",
                    );
                }
                if val.starts_with("EQ_BCD") {
                    diff.apply_enum_diff(
                        ctx.tiledb.item(tile, bel, &format!("MUX.{lut}3")),
                        "D",
                        "Q",
                    );
                }
                diff.assert_empty();
            }
        }
        for (val, bits) in [
            (
                "EQ_ABCD",
                bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (
                "EQ_ABCD_NA",
                bits![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (
                "EQ_ABCD_NC",
                bits![0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (
                "EQ_ABCD_ND",
                bits![0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (
                "EQ_ABCD_NB",
                bits![0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
            ),
        ] {
            let mut diff = ctx.state.get_diff(tile, bel, "F", val);
            diff.apply_bitvec_diff(ctx.tiledb.item(tile, bel, "G"), &bits[..8], &bitvec![0; 8]);
            diff.apply_bitvec_diff(ctx.tiledb.item(tile, bel, "F"), &bits[8..], &bitvec![0; 8]);
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.F1"), "A", "B");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.G1"), "A", "B");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.F3"), "D", "Q");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.G3"), "D", "Q");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MODE"), "FGM", "FG");
            diff.assert_empty();
        }
        for val in ["F", "FG", "FGM"] {
            ctx.state.get_diff(tile, bel, "BASE", val).assert_empty();
        }
    }
    for (tile, frame, bit, bel) in [
        ("CLB", 3, 2, "CLB"),
        ("CLB.L", 3, 2, "CLB"),
        ("CLB.L", 18, 5, "LIOB0"),
        ("CLB.L", 20, 0, "LIOB1"),
        ("CLB.R", 12, 2, "CLB"),
        ("CLB.R", 0, 3, "RIOB0"),
        ("CLB.R", 8, 2, "RIOB1"),
        ("CLB.ML", 3, 2, "CLB"),
        ("CLB.ML", 20, 0, "LIOB1"),
        ("CLB.MR", 12, 2, "CLB"),
        ("CLB.MR", 8, 2, "RIOB1"),
        ("CLB.B", 3, 6, "CLB"),
        ("CLB.B", 4, 1, "BIOB0"),
        ("CLB.B", 8, 0, "BIOB1"),
        ("CLB.BR1", 3, 6, "CLB"),
        ("CLB.BR1", 4, 1, "BIOB0"),
        ("CLB.BR1", 8, 0, "BIOB1"),
        ("CLB.BL", 3, 6, "CLB"),
        ("CLB.BL", 4, 1, "BIOB0"),
        ("CLB.BL", 8, 0, "BIOB1"),
        ("CLB.BL", 18, 9, "LIOB0"),
        ("CLB.BR", 12, 6, "CLB"),
        ("CLB.BR", 13, 1, "BIOB0"),
        ("CLB.BR", 17, 0, "BIOB1"),
        ("CLB.BR", 0, 7, "RIOB0"),
        ("CLB.T", 3, 2, "CLB"),
        ("CLB.T", 4, 7, "TIOB0"),
        ("CLB.T", 8, 8, "TIOB1"),
        ("CLB.TR1", 3, 2, "CLB"),
        ("CLB.TR1", 4, 7, "TIOB0"),
        ("CLB.TR1", 8, 8, "TIOB1"),
        ("CLB.TL", 3, 2, "CLB"),
        ("CLB.TL", 4, 7, "TIOB0"),
        ("CLB.TL", 8, 8, "TIOB1"),
        ("CLB.TL", 20, 0, "LIOB1"),
        ("CLB.TR", 12, 2, "CLB"),
        ("CLB.TR", 13, 7, "TIOB0"),
        ("CLB.TR", 17, 8, "TIOB1"),
        ("CLB.TR", 8, 2, "RIOB1"),
    ] {
        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK_Q",
            TileItem::from_bit(TileBit::new(0, frame, bit), true),
        );
    }
}

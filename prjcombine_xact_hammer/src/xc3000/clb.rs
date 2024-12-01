use bitvec::prelude::*;
use prjcombine_collector::{xlat_bitvec, xlat_enum, Diff};
use prjcombine_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for tile in backend.egrid.db.nodes.keys() {
        if !tile.starts_with("CLB") {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let mut bctx = ctx.bel("CLB");
        for lut in ["F", "G"] {
            bctx.mode("FG")
                .test_manual(lut, "ABCD")
                .cfg(lut, "A")
                .cfg(lut, "B")
                .cfg(lut, "C")
                .cfg(lut, "D")
                .commit();
            bctx.mode("FG")
                .test_manual(lut, "ABCE")
                .cfg(lut, "A")
                .cfg(lut, "B")
                .cfg(lut, "C")
                .cfg(lut, "E")
                .commit();
            bctx.mode("FG")
                .test_manual(lut, "AQXCD")
                .cfg(lut, "A")
                .cfg(lut, "QX")
                .cfg(lut, "C")
                .cfg(lut, "D")
                .commit();
            bctx.mode("FG")
                .test_manual(lut, "AQYCD")
                .cfg(lut, "A")
                .cfg(lut, "QY")
                .cfg(lut, "C")
                .cfg(lut, "D")
                .commit();
            bctx.mode("FG")
                .test_manual(lut, "ABQXD")
                .cfg(lut, "A")
                .cfg(lut, "B")
                .cfg(lut, "QX")
                .cfg(lut, "D")
                .commit();
            bctx.mode("FG")
                .test_manual(lut, "ABQYD")
                .cfg(lut, "A")
                .cfg(lut, "B")
                .cfg(lut, "QY")
                .cfg(lut, "D")
                .commit();
            for i in 0..16 {
                let mut bits = bitvec![0; 16];
                bits.set(i, true);
                bctx.mode("FG").test_equate_fixed(
                    lut,
                    format!("EQ_ABCD_{i}"),
                    &["A", "B", "C", "D"],
                    bits,
                );
            }
            for inps in [
                &["A", "B", "C", "E"],
                &["A", "QX", "C", "D"],
                &["A", "QY", "C", "D"],
                &["A", "B", "QX", "D"],
                &["A", "B", "QY", "D"],
            ] {
                for i in [0, 1, 2, 4, 8] {
                    let mut bits = bitvec![0; 16];
                    bits.set(i, true);
                    bctx.mode("FG").test_equate_fixed(
                        lut,
                        format!(
                            "EQ_{a}{b}{c}{d}_{i}",
                            a = inps[0],
                            b = inps[1],
                            c = inps[2],
                            d = inps[3]
                        ),
                        inps,
                        bits,
                    );
                }
            }
        }
        bctx.mode("F")
            .test_manual("F", "ABCDE")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("F", "E")
            .commit();
        for i in [0, 1, 2, 4, 8, 16] {
            let mut bits = bitvec![0; 32];
            bits.set(i, true);
            bctx.mode("F").test_equate_fixed(
                "F",
                format!("EQ_ABCDE_{i}"),
                &["A", "B", "C", "D", "E"],
                bits,
            );
        }
        bctx.mode("FG").test_enum("X", &["F", "QX"]);
        bctx.mode("FG").test_enum("Y", &["G", "QY"]);
        bctx.mode("FG").test_enum("DX", &["F", "G", "DI"]);
        bctx.mode("FG").test_enum("DY", &["F", "G", "DI"]);
        bctx.mode("FGM")
            .mutex("FGM", "FG")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .test_cfg("X", "M");
        bctx.mode("FGM")
            .mutex("FGM", "FG")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .test_cfg("Y", "M");
        bctx.mode("FGM")
            .mutex("FGM", "FG")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .test_cfg("DX", "M");
        bctx.mode("FGM")
            .mutex("FGM", "FG")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .test_cfg("DY", "M");
        bctx.mode("FG").test_cfg("ENCLK", "EC");
        bctx.mode("FG").test_cfg("RSTDIR", "RD");
        bctx.mode("FG").test_cfg("CLK", "K");
        bctx.mode("FG").cfg("CLK", "K").test_cfg("CLK", "NOT");
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ctx.edev.egrid.db.nodes.keys() {
        if !tile.starts_with("CLB") {
            continue;
        }
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "CLB";
        let item = xlat_enum(vec![
            ("QX", ctx.state.get_diff(tile, bel, "X", "QX")),
            ("F", ctx.state.get_diff(tile, bel, "X", "F")),
            ("F", ctx.state.get_diff(tile, bel, "X", "M")),
        ]);
        ctx.tiledb.insert(tile, bel, "MUX.X", item);
        let item = xlat_enum(vec![
            ("QY", ctx.state.get_diff(tile, bel, "Y", "QY")),
            ("G", ctx.state.get_diff(tile, bel, "Y", "G")),
            ("G", ctx.state.get_diff(tile, bel, "Y", "M")),
        ]);
        ctx.tiledb.insert(tile, bel, "MUX.Y", item);
        for attr in ["DX", "DY"] {
            let item = xlat_enum(vec![
                ("DI", ctx.state.get_diff(tile, bel, attr, "DI")),
                ("F", ctx.state.get_diff(tile, bel, attr, "F")),
                ("G", ctx.state.get_diff(tile, bel, attr, "G")),
                ("F", ctx.state.get_diff(tile, bel, attr, "M")),
            ]);
            ctx.tiledb.insert(tile, bel, format!("MUX.{attr}"), item);
        }
        let item = ctx.extract_bit(tile, bel, "ENCLK", "EC");
        ctx.tiledb.insert(tile, bel, "EC_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "RSTDIR", "RD");
        ctx.tiledb.insert(tile, bel, "RD_ENABLE", item);
        ctx.state.get_diff(tile, bel, "CLK", "K").assert_empty();
        let item = ctx.extract_bit(tile, bel, "CLK", "NOT");
        ctx.tiledb.insert(tile, bel, "INV.K", item);
        for lut in ["F", "G"] {
            let diff_abcd = ctx.state.get_diff(tile, bel, lut, "ABCD");
            let diff_abce = ctx.state.get_diff(tile, bel, lut, "ABCE");
            let diff_abqxd = ctx.state.get_diff(tile, bel, lut, "ABQXD");
            let diff_abqyd = ctx.state.get_diff(tile, bel, lut, "ABQYD");
            let diff_aqxcd = ctx.state.get_diff(tile, bel, lut, "AQXCD");
            let diff_aqycd = ctx.state.get_diff(tile, bel, lut, "AQYCD");
            let mut lut_diffs = vec![];
            for i in 0..16 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, lut, format!("EQ_ABCD_{i}"))
                    .combine(&!&diff_abcd);
                lut_diffs.push(diff);
            }
            let diff_i4_e = diff_abce.combine(&!&diff_abcd);
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.{lut}4"),
                xlat_enum(vec![("D", Diff::default()), ("E", diff_i4_e)]),
            );
            let diff_i3_qx = diff_abqxd.combine(&!&diff_abcd);
            let diff_i3_qy = diff_abqyd.combine(&!&diff_abcd);
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.{lut}3"),
                xlat_enum(vec![
                    ("C", Diff::default()),
                    ("QX", diff_i3_qx),
                    ("QY", diff_i3_qy),
                ]),
            );
            let diff_i2_qx = diff_aqxcd.combine(&!&diff_abcd);
            let diff_i2_qy = diff_aqycd.combine(&!&diff_abcd);
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.{lut}2"),
                xlat_enum(vec![
                    ("B", Diff::default()),
                    ("QX", diff_i2_qx),
                    ("QY", diff_i2_qy),
                ]),
            );
            for (name, base) in [
                ("ABQXD", diff_abqxd),
                ("ABQYD", diff_abqyd),
                ("AQXCD", diff_aqxcd),
                ("AQYCD", diff_aqycd),
                ("ABCE", diff_abce),
            ] {
                for i in [0, 1, 2, 4, 8] {
                    let diff = ctx
                        .state
                        .get_diff(tile, bel, lut, format!("EQ_{name}_{i}"))
                        .combine(&!&base);
                    assert_eq!(lut_diffs[i], diff);
                }
            }
            ctx.tiledb.insert(tile, bel, lut, xlat_bitvec(lut_diffs));
            let mut diff = diff_abcd;
            diff.apply_enum_diff(
                ctx.tiledb.item(tile, bel, &format!("MUX.{lut}2")),
                "B",
                "QY",
            );
            diff.apply_enum_diff(
                ctx.tiledb.item(tile, bel, &format!("MUX.{lut}3")),
                "C",
                "QY",
            );
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, &format!("MUX.{lut}4")), "D", "E");
            diff.assert_empty();
        }
        let diff_abcde = ctx.state.get_diff(tile, bel, "F", "ABCDE");
        for i in [0, 1, 2, 4, 8, 16] {
            let mut bits = bitvec![0; 32];
            bits.set(i, true);
            let mut diff = ctx.state.get_diff(tile, bel, "F", format!("EQ_ABCDE_{i}"));
            diff = diff.combine(&!&diff_abcde);
            diff.apply_bitvec_diff(ctx.tiledb.item(tile, bel, "F"), &bits[..16], bits![0; 16]);
            diff.apply_bitvec_diff(ctx.tiledb.item(tile, bel, "G"), &bits[16..], bits![0; 16]);
            diff.assert_empty();
        }
        let mut diff = diff_abcde;
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.F2"), "B", "QY");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.F3"), "C", "QY");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.F4"), "D", "E");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.G2"), "B", "QY");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.G3"), "C", "QY");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.G4"), "D", "E");
        assert_eq!(diff.bits.len(), 1);
        let bit = diff.bits.keys().copied().next().unwrap();
        ctx.tiledb.insert(
            tile,
            bel,
            "MODE",
            xlat_enum(vec![("FG", Diff::default()), ("FGM", diff)]),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK_QY",
            TileItem::from_bit(
                TileBit {
                    bit: bit.bit + 1,
                    ..bit
                },
                true,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK_QX",
            TileItem::from_bit(
                TileBit {
                    frame: bit.frame + 1,
                    bit: bit.bit + 1,
                    ..bit
                },
                true,
            ),
        );
    }
}

use prjcombine_hammer::Session;
use prjcombine_types::TileItem;
use prjcombine_virtex::grid::GridKind;
use prjcombine_xilinx_geom::ExpandedDevice;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{xlat_bitvec, xlat_bool, xlat_enum, CollectorCtx, Diff},
    fgen::{BelKV, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for side in ['L', 'R', 'B', 'T'] {
        let tile = format!("IO.{side}");
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(side, 'B' | 'T')) {
                continue;
            }
            let ctx = FuzzCtx::new(
                session,
                backend,
                &tile,
                format!("IOB{i}"),
                TileBits::MainAuto,
            );
            fuzz_one!(ctx, "PRESENT", "1", [
                (global_mutex "VREF", "NO"),
                (global_opt "SHORTENJTAGCHAIN", "NO"),
                (bel_special BelKV::VirtexIsDllIob(false))
            ], [
                (mode "IOB"),
                (attr "PULL", "PULLDOWN"),
                (attr "TFFATTRBOX", "HIGH"),
                (attr "OFFATTRBOX", "HIGH")
            ]);
            fuzz_one!(ctx, "SHORTEN_JTAG_CHAIN", "0", [
                (global_mutex "VREF", "NO"),
                (global_opt "SHORTENJTAGCHAIN", "YES"),
                (bel_special BelKV::VirtexIsDllIob(false))
            ], [
                (mode "IOB"),
                (attr "PULL", "PULLDOWN"),
                (attr "TFFATTRBOX", "HIGH"),
                (attr "OFFATTRBOX", "HIGH")
            ]);
            fuzz_enum!(ctx, "SRMUX", ["0", "1", "SR", "SR_B"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "IINITMUX", "0"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "ICEMUX", ["0", "1", "ICE", "ICE_B"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (pin "ICE")
            ]);
            fuzz_enum!(ctx, "OCEMUX", ["0", "1", "OCE", "OCE_B"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (pin "OCE")
            ]);
            fuzz_enum!(ctx, "TCEMUX", ["0", "1", "TCE", "TCE_B"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (pin "TCE")
            ]);
            fuzz_enum!(ctx, "TRIMUX", ["0", "1", "T", "T_TB"], [
                (mode "IOB"),
                (attr "TSEL", "1"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "OMUX", ["0", "1", "O", "O_B"], [
                (mode "IOB"),
                (attr "OUTMUX", "1"),
                (pin "O")
            ]);
            fuzz_enum!(ctx, "ICKINV", ["0", "1"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "OCKINV", ["0", "1"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "TCKINV", ["0", "1"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "IFF", ["#FF", "#LATCH"], [
                (mode "IOB"),
                (attr "ICEMUX", "0"),
                (attr "ICKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "OFF", ["#FF", "#LATCH"], [
                (mode "IOB"),
                (attr "OCEMUX", "0"),
                (attr "OCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "TFF", ["#FF", "#LATCH"], [
                (mode "IOB"),
                (attr "TCEMUX", "0"),
                (attr "TCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "IINITMUX", ["0"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "ICKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "OINITMUX", ["0"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (attr "OCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "TINITMUX", ["0"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (attr "TCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "IFFINITATTR", ["LOW", "HIGH"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "ICKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "OFFATTRBOX", ["LOW", "HIGH"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (attr "OCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "TFFATTRBOX", ["LOW", "HIGH"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (attr "TCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "FFATTRBOX", ["SYNC", "ASYNC"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (pin "IQ")
            ]);
            fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "IFFMUX", "1"),
                (pin "IQ"),
                (pin "I")
            ]);
            fuzz_enum!(ctx, "IFFMUX", ["0", "1"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "IMUX", "1"),
                (pin "IQ"),
                (pin "I")
            ]);
            fuzz_enum!(ctx, "TSEL", ["0", "1"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (attr "TRIMUX", "T"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "OUTMUX", ["0", "1"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (attr "OMUX", "O"),
                (attr "TRIMUX", "T"),
                (attr "TSEL", "1"),
                (pin "O"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "PULL", ["PULLDOWN", "PULLUP", "KEEPER"], [
                (mode "IOB"),
                (attr "IMUX", "0"),
                (pin "I")
            ]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    let kind = match edev.grid.kind {
        GridKind::Virtex => "V",
        GridKind::VirtexE | GridKind::VirtexEM => "VE",
    };
    for side in ['L', 'R', 'B', 'T'] {
        let tile = &format!("IO.{side}");
        let tile_iob = &format!("IOB.{side}.{kind}");
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(side, 'B' | 'T')) {
                continue;
            }
            let bel = &format!("IOB{i}");

            // IOI

            let present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
            let diff = ctx.state.get_diff(tile, bel, "SHORTEN_JTAG_CHAIN", "0").combine(&!&present);
            let item = xlat_bitvec(vec![!diff]);
            ctx.tiledb.insert(tile, bel, "SHORTEN_JTAG_CHAIN", item);
            for (pin, pin_b, pinmux) in [
                ("SR", "SR_B", "SRMUX"),
                ("ICE", "ICE_B", "ICEMUX"),
                ("OCE", "OCE_B", "OCEMUX"),
                ("TCE", "TCE_B", "TCEMUX"),
                ("T", "T_TB", "TRIMUX"),
                ("O", "O_B", "OMUX"),
            ] {
                let diff0 = ctx.state.get_diff(tile, bel, pinmux, "1");
                assert_eq!(diff0, ctx.state.get_diff(tile, bel, pinmux, pin));
                let diff1 = ctx.state.get_diff(tile, bel, pinmux, "0");
                assert_eq!(diff1, ctx.state.get_diff(tile, bel, pinmux, pin_b));
                let item = xlat_bool(diff0, diff1);
                ctx.insert_int_inv(&[tile], tile, bel, pin, item);
            }
            for iot in ['I', 'O', 'T'] {
                let item = ctx.extract_enum_bool(tile, bel, &format!("{iot}CKINV"), "1", "0");
                ctx.tiledb
                    .insert(tile, bel, format!("INV.{iot}FF.CLK"), item);
                let item = ctx.extract_bit(tile, bel, &format!("{iot}INITMUX"), "0");
                ctx.tiledb
                    .insert(tile, bel, format!("{iot}FF_SR_ENABLE"), item);
            }
            let item = ctx.extract_enum_bool(tile, bel, "IFFINITATTR", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "IFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFFATTRBOX", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFFATTRBOX", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            ctx.state
                .get_diff(tile, bel, "FFATTRBOX", "ASYNC")
                .assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "FFATTRBOX", "SYNC");
            for iot in ['I', 'O', 'T'] {
                let init = ctx.tiledb.item(tile, bel, &format!("{iot}FF_INIT"));
                let init_bit = init.bits[0];
                let item = xlat_bitvec(vec![diff.split_bits_by(|bit| {
                    bit.tile == init_bit.tile
                        && bit.frame.abs_diff(init_bit.frame) == 1
                        && bit.bit == init_bit.bit
                })]);
                ctx.tiledb.insert(tile, bel, format!("{iot}FF_SYNC"), item);
            }
            diff.assert_empty();
            let item = ctx.extract_enum_bool(tile, bel, "IFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);

            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_IFF",
                TileItem::from_bit(
                    FeatureBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 2,
                            ('R', 2) => 27,
                            ('R', 3) => 32,
                            (_, 1) => 45,
                            (_, 2) => 20,
                            (_, 3) => 15,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_OFF",
                TileItem::from_bit(
                    FeatureBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 8,
                            ('R', 2) => 21,
                            ('R', 3) => 38,
                            (_, 1) => 39,
                            (_, 2) => 26,
                            (_, 3) => 9,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_TFF",
                TileItem::from_bit(
                    FeatureBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 12,
                            ('R', 2) => 17,
                            ('R', 3) => 42,
                            (_, 1) => 35,
                            (_, 2) => 30,
                            (_, 3) => 5,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );

            // IOI + IOB

            ctx.state.get_diff(tile, bel, "TSEL", "1").assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "TSEL", "0");
            let diff_ioi = diff.split_bits_by(|bit| bit.frame < 48 && bit.bit == 16);
            ctx.tiledb.insert(
                tile,
                bel,
                "TMUX",
                xlat_enum(vec![("T", Diff::default()), ("TFF", diff_ioi)]),
            );
            ctx.tiledb.insert(
                tile_iob,
                bel,
                "TMUX",
                xlat_enum(vec![("T", Diff::default()), ("TFF", diff)]),
            );
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "OUTMUX", "0")
                .combine(&!ctx.state.get_diff(tile, bel, "OUTMUX", "1"));
            let diff_ioi = diff.split_bits_by(|bit| bit.frame < 48 && bit.bit == 16);
            ctx.tiledb.insert(
                tile,
                bel,
                "OMUX",
                xlat_enum(vec![("O", Diff::default()), ("OFF", diff_ioi)]),
            );
            ctx.tiledb.insert(
                tile_iob,
                bel,
                "OMUX",
                xlat_enum(vec![("O", Diff::default()), ("OFF", diff)]),
            );

            // IOB

            ctx.tiledb.insert(
                tile_iob,
                bel,
                "READBACK_I",
                TileItem::from_bit(
                    match (side, i) {
                        ('L' | 'R', 1) => FeatureBit::new(0, 50, 13),
                        ('L' | 'R', 2) => FeatureBit::new(0, 50, 12),
                        ('L' | 'R', 3) => FeatureBit::new(0, 50, 2),
                        ('B' | 'T', 1) => FeatureBit::new(0, 25, 17),
                        ('B' | 'T', 2) => FeatureBit::new(0, 21, 17),
                        _ => unreachable!(),
                    },
                    false,
                ),
            );
            let item = ctx.extract_enum_default(
                tile,
                bel,
                "PULL",
                &["PULLDOWN", "PULLUP", "KEEPER"],
                "NONE",
            );
            ctx.tiledb.insert(tile_iob, bel, "PULL", item);
        }
    }
}

use std::collections::BTreeMap;

use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_types::TileItem;

use crate::{
    backend::IseBackend,
    diff::{xlat_bitvec, xlat_bool, xlat_enum, CollectorCtx},
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi, fuzz_multi_attr_hex, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    {
        let ctx = FuzzCtx::new(session, backend, "BRAM", "BRAM_F", TileBits::Bram);

        fuzz_one!(ctx, "PRESENT", "1", [
            (global_mutex "BRAM", "MULTI"),
            (tile_mutex "MODE", "FULL")
        ], [
            (mode "RAMB16BWER")
        ]);
        for pin in [
            "CLKA", "CLKB", "ENA", "ENB", "RSTA", "RSTB", "WEA0", "WEA1", "WEA2", "WEA3", "WEB0",
            "WEB1", "WEB2", "WEB3", "REGCEA", "REGCEB",
        ] {
            fuzz_inv!(ctx, pin, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER")
            ]);
        }
        fuzz_enum!(ctx, "RAM_MODE", ["TDP", "SDP", "SP"], [
            (global_mutex "BRAM", "MULTI"),
            (tile_mutex "MODE", "FULL"),
            (mode "RAMB16BWER")
        ]);
        fuzz_enum!(ctx, "RSTTYPE", ["SYNC", "ASYNC"], [
            (global_mutex "BRAM", "MULTI"),
            (tile_mutex "MODE", "FULL"),
            (mode "RAMB16BWER")
        ]);
        for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
            fuzz_enum!(ctx, attr, ["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER")
            ]);
        }
        for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
            fuzz_enum!(ctx, attr, ["0", "1", "2", "4", "9", "18", "36"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER")
            ]);
        }
        for attr in ["DOA_REG", "DOB_REG"] {
            fuzz_enum!(ctx, attr, ["0", "1"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER")
            ]);
        }
        for attr in ["RST_PRIORITY_A", "RST_PRIORITY_B"] {
            fuzz_enum!(ctx, attr, ["CE", "SR"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER")
            ]);
        }
        for attr in ["EN_RSTRAM_A", "EN_RSTRAM_B"] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER")
            ]);
        }
        for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
            fuzz_multi_attr_hex!(ctx, attr, 36, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER"),
                (attr "DATA_WIDTH_A", "36"),
                (attr "DATA_WIDTH_B", "36")
            ]);
        }
        for i in 0..0x40 {
            fuzz_multi!(ctx, format!("INIT_{i:02X}.NARROW"), "", 256, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER"),
                (attr "DATA_WIDTH_A", "18"),
                (attr "DATA_WIDTH_B", "18")
            ], (attr_hex format!("INIT_{i:02X}")));
            fuzz_multi!(ctx, format!("INIT_{i:02X}.WIDE"), "", 256, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER"),
                (attr "DATA_WIDTH_A", "18"),
                (attr "DATA_WIDTH_B", "36")
            ], (attr_hex format!("INIT_{i:02X}")));
        }
        for i in 0..8 {
            fuzz_multi!(ctx, format!("INITP_{i:02X}.NARROW"), "", 256, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER"),
                (attr "DATA_WIDTH_A", "18"),
                (attr "DATA_WIDTH_B", "18")
            ], (attr_hex format!("INITP_{i:02X}")));
            fuzz_multi!(ctx, format!("INITP_{i:02X}.WIDE"), "", 256, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "FULL"),
                (mode "RAMB16BWER"),
                (attr "DATA_WIDTH_A", "18"),
                (attr "DATA_WIDTH_B", "36")
            ], (attr_hex format!("INITP_{i:02X}")));
        }

        for (attr, val) in [
            ("ENWEAKWRITEA", "ENABLE"),
            ("ENWEAKWRITEB", "ENABLE"),
            ("WEAKWRITEVALA", "1"),
            ("WEAKWRITEVALB", "1"),
        ] {
            fuzz_one!(ctx, attr, val, [
                (global_mutex_site "BRAM"),
                (mode "RAMB16BWER")
            ], [
                (global_opt attr, val)
            ]);
        }
        for attr in [
            "BRAM_DDEL_A_D",
            "BRAM_DDEL_A_U",
            "BRAM_DDEL_B_D",
            "BRAM_DDEL_B_U",
        ] {
            for val in ["0", "1", "11", "111"] {
                fuzz_one!(ctx, attr, val, [
                    (global_mutex_site "BRAM"),
                    (mode "RAMB16BWER")
                ], [
                    (global_opt attr, val)
                ]);
            }
        }
        for attr in [
            "BRAM_WDEL_A_D",
            "BRAM_WDEL_A_U",
            "BRAM_WDEL_B_D",
            "BRAM_WDEL_B_U",
        ] {
            for val in ["0", "1", "10", "11", "100", "101", "110", "111"] {
                fuzz_one!(ctx, attr, val, [
                    (global_mutex_site "BRAM"),
                    (mode "RAMB16BWER")
                ], [
                    (global_opt attr, val)
                ]);
            }
        }
    }
    for bel in ["BRAM_H0", "BRAM_H1"] {
        let ctx = FuzzCtx::new(session, backend, "BRAM", bel, TileBits::Bram);
        fuzz_one!(ctx, "PRESENT", "1", [
            (global_mutex "BRAM", "MULTI"),
            (tile_mutex "MODE", bel)
        ], [
            (mode "RAMB8BWER")
        ]);
        for pin in [
            "CLKAWRCLK",
            "CLKBRDCLK",
            "ENAWREN",
            "ENBRDEN",
            "RSTA",
            "RSTBRST",
            "WEAWEL0",
            "WEAWEL1",
            "WEBWEU0",
            "WEBWEU1",
            "REGCEA",
            "REGCEBREGCE",
        ] {
            fuzz_inv!(ctx, pin, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER")
            ]);
        }
        fuzz_enum!(ctx, "RAM_MODE", ["TDP", "SDP", "SP"], [
            (global_mutex "BRAM", "MULTI"),
            (tile_mutex "MODE", "HALF"),
            (mode "RAMB8BWER")
        ]);
        fuzz_enum!(ctx, "RSTTYPE", ["SYNC", "ASYNC"], [
            (global_mutex "BRAM", "MULTI"),
            (tile_mutex "MODE", "HALF"),
            (mode "RAMB8BWER")
        ]);
        for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
            fuzz_enum!(ctx, attr, ["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER")
            ]);
        }
        for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
            fuzz_enum!(ctx, attr, ["0", "1", "2", "4", "9", "18", "36"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER")
            ]);
        }
        for attr in ["DOA_REG", "DOB_REG"] {
            fuzz_enum!(ctx, attr, ["0", "1"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER")
            ]);
        }
        for attr in ["RST_PRIORITY_A", "RST_PRIORITY_B"] {
            fuzz_enum!(ctx, attr, ["CE", "SR"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER")
            ]);
        }
        for attr in ["EN_RSTRAM_A", "EN_RSTRAM_B"] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER")
            ]);
        }
        for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
            fuzz_multi_attr_hex!(ctx, attr, 18, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER"),
                (attr "DATA_WIDTH_A", "18"),
                (attr "DATA_WIDTH_B", "18")
            ]);
        }
        for i in 0..0x20 {
            fuzz_multi!(ctx, format!("INIT_{i:02X}"), "", 256, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER")
            ], (attr_hex format!("INIT_{i:02X}")));
        }
        for i in 0..4 {
            fuzz_multi!(ctx, format!("INITP_{i:02X}"), "", 256, [
                (global_mutex "BRAM", "MULTI"),
                (tile_mutex "MODE", "HALF"),
                (mode "RAMB8BWER")
            ], (attr_hex format!("INITP_{i:02X}")));
        }
        for (attr, val) in [
            ("ENWEAKWRITEA", "ENABLE"),
            ("ENWEAKWRITEB", "ENABLE"),
            ("WEAKWRITEVALA", "1"),
            ("WEAKWRITEVALB", "1"),
        ] {
            fuzz_one!(ctx, attr, val, [
                (global_mutex_site "BRAM"),
                (mode "RAMB8BWER")
            ], [
                (global_opt attr, val)
            ]);
        }
    }
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    for attr in ["BW_EN_A_D", "BW_EN_A_U", "BW_EN_B_D", "BW_EN_B_U"] {
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::AllBrams,
            "BRAM",
            "BRAM_F",
            attr,
            "1",
        )];
        fuzz_one_extras!(ctx, attr, "1", [
            (global_mutex "BRAM", "NONE")
        ], [
            (global_opt attr, "1")
        ], extras);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "BRAM";

    for (fpin, hbel, hpin) in [
        ("WEA0", "BRAM_H0", "WEAWEL0"),
        ("WEA1", "BRAM_H0", "WEAWEL1"),
        ("WEA2", "BRAM_H1", "WEAWEL0"),
        ("WEA3", "BRAM_H1", "WEAWEL1"),
        ("WEB0", "BRAM_H0", "WEBWEU0"),
        ("WEB1", "BRAM_H0", "WEBWEU1"),
        ("WEB2", "BRAM_H1", "WEBWEU0"),
        ("WEB3", "BRAM_H1", "WEBWEU1"),
    ] {
        let item = ctx.extract_enum_bool(
            tile,
            "BRAM_F",
            &format!("{fpin}INV"),
            fpin,
            &format!("{fpin}_B"),
        );
        assert_eq!(
            item,
            ctx.extract_enum_bool(
                tile,
                hbel,
                &format!("{hpin}INV"),
                hpin,
                &format!("{hpin}_B"),
            )
        );
        ctx.tiledb.insert(tile, hbel, format!("INV.{hpin}"), item);
    }

    for (fpin, hpin) in [
        ("CLKA", "CLKAWRCLK"),
        ("CLKB", "CLKBRDCLK"),
        ("ENA", "ENAWREN"),
        ("ENB", "ENBRDEN"),
        ("RSTA", "RSTA"),
        ("RSTB", "RSTBRST"),
        ("REGCEA", "REGCEA"),
        ("REGCEB", "REGCEBREGCE"),
    ] {
        let diff0_f = ctx
            .state
            .get_diff(tile, "BRAM_F", format!("{fpin}INV"), fpin);
        let diff0_h0 = ctx
            .state
            .get_diff(tile, "BRAM_H0", format!("{hpin}INV"), hpin);
        let diff0_h1 = ctx
            .state
            .get_diff(tile, "BRAM_H1", format!("{hpin}INV"), hpin);
        assert_eq!(diff0_f, diff0_h0.combine(&diff0_h1));
        let diff1_f = ctx
            .state
            .get_diff(tile, "BRAM_F", format!("{fpin}INV"), format!("{fpin}_B"));
        let diff1_h0 =
            ctx.state
                .get_diff(tile, "BRAM_H0", format!("{hpin}INV"), format!("{hpin}_B"));
        let diff1_h1 =
            ctx.state
                .get_diff(tile, "BRAM_H1", format!("{hpin}INV"), format!("{hpin}_B"));
        assert_eq!(diff1_f, diff1_h0.combine(&diff1_h1));
        ctx.tiledb.insert(
            tile,
            "BRAM_H0",
            format!("INV.{hpin}"),
            xlat_bool(diff0_h0, diff1_h0),
        );
        ctx.tiledb.insert(
            tile,
            "BRAM_H1",
            format!("INV.{hpin}"),
            xlat_bool(diff0_h1, diff1_h1),
        );
    }

    for (attr, vals) in [
        ("RAM_MODE", &["TDP", "SDP", "SP"][..]),
        ("RSTTYPE", &["SYNC", "ASYNC"]),
        ("WRITE_MODE_A", &["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]),
        ("WRITE_MODE_B", &["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]),
        ("DOA_REG", &["0", "1"]),
        ("DOB_REG", &["0", "1"]),
        ("RST_PRIORITY_A", &["CE", "SR"]),
        ("RST_PRIORITY_B", &["CE", "SR"]),
        ("DATA_WIDTH_A", &["0", "1", "2", "4", "9", "18", "36"]),
        ("DATA_WIDTH_B", &["0", "1", "2", "4", "9", "18", "36"]),
    ] {
        let mut diffs_h0 = vec![];
        let mut diffs_h1 = vec![];
        for &val in vals {
            let diff_f = ctx.state.get_diff(tile, "BRAM_F", attr, val);
            let diff_h0 = ctx.state.get_diff(tile, "BRAM_H0", attr, val);
            let diff_h1 = ctx.state.get_diff(tile, "BRAM_H1", attr, val);
            assert_eq!(diff_f, diff_h0.combine(&diff_h1));
            diffs_h0.push((val, diff_h0));
            diffs_h1.push((val, diff_h1));
        }
        ctx.tiledb
            .insert(tile, "BRAM_H0", attr, xlat_enum(diffs_h0));
        ctx.tiledb
            .insert(tile, "BRAM_H1", attr, xlat_enum(diffs_h1));
    }
    for (attr, val0, val1) in [
        ("EN_RSTRAM_A", "FALSE", "TRUE"),
        ("EN_RSTRAM_B", "FALSE", "TRUE"),
    ] {
        let diff0_f = ctx.state.get_diff(tile, "BRAM_F", attr, val0);
        let diff0_h0 = ctx.state.get_diff(tile, "BRAM_H0", attr, val0);
        let diff0_h1 = ctx.state.get_diff(tile, "BRAM_H1", attr, val0);
        assert_eq!(diff0_f, diff0_h0.combine(&diff0_h1));
        let diff1_f = ctx.state.get_diff(tile, "BRAM_F", attr, val1);
        let diff1_h0 = ctx.state.get_diff(tile, "BRAM_H0", attr, val1);
        let diff1_h1 = ctx.state.get_diff(tile, "BRAM_H1", attr, val1);
        assert_eq!(diff1_f, diff1_h0.combine(&diff1_h1));
        ctx.tiledb
            .insert(tile, "BRAM_H0", attr, xlat_bool(diff0_h0, diff1_h0));
        ctx.tiledb
            .insert(tile, "BRAM_H1", attr, xlat_bool(diff0_h1, diff1_h1));
    }
    for (attr, val1) in [
        ("ENWEAKWRITEA", "ENABLE"),
        ("ENWEAKWRITEB", "ENABLE"),
        ("WEAKWRITEVALA", "1"),
        ("WEAKWRITEVALB", "1"),
    ] {
        let diff1_f = ctx.state.get_diff(tile, "BRAM_F", attr, val1);
        let diff1_h0 = ctx.state.get_diff(tile, "BRAM_H0", attr, val1);
        let diff1_h1 = ctx.state.get_diff(tile, "BRAM_H1", attr, val1);
        assert_eq!(diff1_f, diff1_h0.combine(&diff1_h1));
        ctx.tiledb
            .insert(tile, "BRAM_H0", attr, xlat_bitvec(vec![diff1_h0]));
        ctx.tiledb
            .insert(tile, "BRAM_H1", attr, xlat_bitvec(vec![diff1_h1]));
    }
    for (attr, bel, sattr) in [
        ("BW_EN_A", "BRAM_H0", "BW_EN_A_D"),
        ("BW_EN_B", "BRAM_H0", "BW_EN_B_D"),
        ("BW_EN_A", "BRAM_H1", "BW_EN_A_U"),
        ("BW_EN_B", "BRAM_H1", "BW_EN_B_U"),
    ] {
        let diff = ctx.state.get_diff(tile, "BRAM_F", sattr, "1");
        ctx.tiledb.insert(tile, bel, attr, xlat_bitvec(vec![diff]));
    }

    for (attr, bel, sattr) in [
        ("DDEL_A", "BRAM_H0", "BRAM_DDEL_A_D"),
        ("DDEL_B", "BRAM_H0", "BRAM_DDEL_B_D"),
        ("DDEL_A", "BRAM_H1", "BRAM_DDEL_A_U"),
        ("DDEL_B", "BRAM_H1", "BRAM_DDEL_B_U"),
    ] {
        let diff0 = ctx.state.get_diff(tile, "BRAM_F", sattr, "0");
        let diff1 = ctx.state.get_diff(tile, "BRAM_F", sattr, "1");
        let diff2 = ctx.state.get_diff(tile, "BRAM_F", sattr, "11");
        let diff3 = ctx.state.get_diff(tile, "BRAM_F", sattr, "111");
        ctx.tiledb.insert(
            tile,
            bel,
            attr,
            xlat_bitvec(vec![
                diff1.combine(&!&diff0),
                diff2.combine(&!&diff1),
                diff3.combine(&!&diff2),
            ]),
        );
    }
    for (attr, bel, sattr) in [
        ("WDEL_A", "BRAM_H0", "BRAM_WDEL_A_D"),
        ("WDEL_B", "BRAM_H0", "BRAM_WDEL_B_D"),
        ("WDEL_A", "BRAM_H1", "BRAM_WDEL_A_U"),
        ("WDEL_B", "BRAM_H1", "BRAM_WDEL_B_U"),
    ] {
        let diff0 = ctx.state.get_diff(tile, "BRAM_F", sattr, "0");
        let diff1 = ctx.state.get_diff(tile, "BRAM_F", sattr, "1");
        let diff2 = ctx.state.get_diff(tile, "BRAM_F", sattr, "10");
        let diff3 = ctx.state.get_diff(tile, "BRAM_F", sattr, "11");
        let diff4 = ctx.state.get_diff(tile, "BRAM_F", sattr, "100");
        let diff5 = ctx.state.get_diff(tile, "BRAM_F", sattr, "101");
        let diff6 = ctx.state.get_diff(tile, "BRAM_F", sattr, "110");
        let diff7 = ctx.state.get_diff(tile, "BRAM_F", sattr, "111");
        let bit0 = diff1.combine(&!&diff0);
        let bit1 = diff2.combine(&!&diff0);
        let bit2 = diff4.combine(&!&diff0);
        assert_eq!(bit0, diff3.combine(&!&diff2));
        assert_eq!(bit0, diff5.combine(&!&diff4));
        assert_eq!(bit0, diff7.combine(&!&diff6));
        assert_eq!(bit1, diff6.combine(&!&diff4));
        diff4.assert_empty();
        ctx.tiledb
            .insert(tile, bel, attr, xlat_bitvec(vec![bit0, bit1, bit2]));
    }

    let mut present_f = ctx.state.get_diff(tile, "BRAM_F", "PRESENT", "1");
    let mut present_h0 = ctx.state.get_diff(tile, "BRAM_H0", "PRESENT", "1");
    let mut present_h1 = ctx.state.get_diff(tile, "BRAM_H1", "PRESENT", "1");

    for pin in [
        "WEAWEL0",
        "WEAWEL1",
        "WEBWEU0",
        "WEBWEU1",
        "REGCEA",
        "REGCEBREGCE",
        "RSTBRST",
        "ENAWREN",
        "ENBRDEN",
    ] {
        let item = ctx.tiledb.item(tile, "BRAM_H0", &format!("INV.{pin}"));
        present_f.apply_bit_diff(item, false, true);
        present_h0.apply_bit_diff(item, false, true);
        let item = ctx.tiledb.item(tile, "BRAM_H1", &format!("INV.{pin}"));
        present_f.apply_bit_diff(item, false, true);
        present_h1.apply_bit_diff(item, false, true);
    }

    for attr in ["BW_EN_A", "BW_EN_B"] {
        let item = ctx.tiledb.item(tile, "BRAM_H0", attr);
        present_f.apply_bit_diff(item, true, false);
        present_h0.apply_bit_diff(item, true, false);
        let item = ctx.tiledb.item(tile, "BRAM_H1", attr);
        present_f.apply_bit_diff(item, true, false);
        present_h1.apply_bit_diff(item, true, false);
    }
    for attr in ["DDEL_A", "DDEL_B"] {
        let val = if ctx.device.name.ends_with("l") { 7 } else { 3 };
        let item = ctx.tiledb.item(tile, "BRAM_H0", attr);
        present_f.apply_bitvec_diff_int(item, val, 0);
        present_h0.apply_bitvec_diff_int(item, val, 0);
        let item = ctx.tiledb.item(tile, "BRAM_H1", attr);
        present_f.apply_bitvec_diff_int(item, val, 0);
        present_h1.apply_bitvec_diff_int(item, val, 0);
    }

    assert_eq!(present_h0, present_h1);
    let mut diffs = present_h0.split_tiles(&[&[0, 1, 2, 3, 4], &[5, 6, 7, 8]]);
    let diff_base = diffs.pop().unwrap();
    let diff_fixup = diffs.pop().unwrap();
    assert_eq!(diff_base, present_f);
    assert_eq!(diff_fixup, present_f);
    assert!(diff_base.bits.values().all(|x| *x));
    let mut bits: Vec<_> = diff_base.bits.keys().copied().collect();
    bits.sort();
    for (bel, bit) in [("BRAM_H0", bits[0]), ("BRAM_H1", bits[1])] {
        ctx.tiledb.insert(
            tile,
            bel,
            "MODE",
            TileItem {
                bits: vec![bit],
                kind: prjcombine_types::TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("RAMB8BWER".to_string(), bitvec![0]),
                        ("RAMB16BWER".to_string(), bitvec![1]),
                    ]),
                },
            },
        );
    }

    for attr in ["SRVAL_A", "SRVAL_B", "INIT_A", "INIT_B"] {
        let diffs_f = ctx.state.get_diffs(tile, "BRAM_F", attr, "");
        let diffs_h0 = ctx.state.get_diffs(tile, "BRAM_H0", attr, "");
        let diffs_h1 = ctx.state.get_diffs(tile, "BRAM_H1", attr, "");
        assert_eq!(diffs_f[0..16], diffs_h0[0..16]);
        assert_eq!(diffs_f[16..32], diffs_h1[0..16]);
        assert_eq!(diffs_f[32..34], diffs_h0[16..18]);
        assert_eq!(diffs_f[34..36], diffs_h1[16..18]);
        ctx.tiledb
            .insert(tile, "BRAM_H0", attr, xlat_bitvec(diffs_h0));
        ctx.tiledb
            .insert(tile, "BRAM_H1", attr, xlat_bitvec(diffs_h1));
    }

    let mut diffs_n = vec![];
    let mut diffs_w = vec![];
    for i in 0..0x40 {
        diffs_n.extend(
            ctx.state
                .get_diffs(tile, "BRAM_F", format!("INIT_{i:02X}.NARROW"), ""),
        );
        diffs_w.extend(
            ctx.state
                .get_diffs(tile, "BRAM_F", format!("INIT_{i:02X}.WIDE"), ""),
        );
    }
    let mut diffs_h0 = vec![];
    let mut diffs_h1 = vec![];
    for i in 0..0x20 {
        diffs_h0.extend(
            ctx.state
                .get_diffs(tile, "BRAM_H0", format!("INIT_{i:02X}"), ""),
        );
        diffs_h1.extend(
            ctx.state
                .get_diffs(tile, "BRAM_H1", format!("INIT_{i:02X}"), ""),
        );
    }
    assert_eq!(diffs_n[..0x2000], diffs_h0);
    assert_eq!(diffs_n[0x2000..], diffs_h1);
    for i in 0..4000 {
        let iw = i & 0xf | i >> 9 & 0x10 | i << 1 & 0x3fe0;
        assert_eq!(diffs_n[i], diffs_w[iw]);
    }
    ctx.tiledb
        .insert(tile, "BRAM_H0", "DATA", xlat_bitvec(diffs_h0));
    ctx.tiledb
        .insert(tile, "BRAM_H1", "DATA", xlat_bitvec(diffs_h1));

    let mut diffs_n = vec![];
    let mut diffs_w = vec![];
    for i in 0..8 {
        diffs_n.extend(
            ctx.state
                .get_diffs(tile, "BRAM_F", format!("INITP_{i:02X}.NARROW"), ""),
        );
        diffs_w.extend(
            ctx.state
                .get_diffs(tile, "BRAM_F", format!("INITP_{i:02X}.WIDE"), ""),
        );
    }
    let mut diffs_h0 = vec![];
    let mut diffs_h1 = vec![];
    for i in 0..4 {
        diffs_h0.extend(
            ctx.state
                .get_diffs(tile, "BRAM_H0", format!("INITP_{i:02X}"), ""),
        );
        diffs_h1.extend(
            ctx.state
                .get_diffs(tile, "BRAM_H1", format!("INITP_{i:02X}"), ""),
        );
    }
    assert_eq!(diffs_n[..0x400], diffs_h0);
    assert_eq!(diffs_n[0x400..], diffs_h1);
    for i in 0..800 {
        let iw = i & 1 | i >> 9 & 2 | i << 1 & 0x7fc;
        assert_eq!(diffs_n[i], diffs_w[iw]);
    }
    ctx.tiledb
        .insert(tile, "BRAM_H0", "DATAP", xlat_bitvec(diffs_h0));
    ctx.tiledb
        .insert(tile, "BRAM_H1", "DATAP", xlat_bitvec(diffs_h1));
}

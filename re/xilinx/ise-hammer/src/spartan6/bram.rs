use std::collections::BTreeMap;

use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_fpga_hammer::{DiffKey, FuzzerProp, xlat_bit, xlat_bitvec, xlat_bool, xlat_enum};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_spartan6::bels;
use prjcombine_types::{
    bits,
    bsdata::{TileItem, TileItemKind},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
};

#[derive(Copy, Clone, Debug)]
struct ExtraBramFixup;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraBramFixup {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let mut feature = fuzzer.info.features[0].clone();
        let DiffKey::Legacy(ref mut id) = feature.key else {
            unreachable!()
        };
        id.attr.push_str(".FIXUP");
        feature.rects = feature
            .rects
            .values()
            .take(4)
            .map(|&rect| rect.to_fixup())
            .collect();
        fuzzer.info.features.push(feature);
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "BRAM");
    {
        let mut bctx = ctx.bel(bels::BRAM_F);
        let mode = "RAMB16BWER";

        bctx.build()
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", "FULL")
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();
        for pin in [
            "CLKA", "CLKB", "ENA", "ENB", "RSTA", "RSTB", "WEA0", "WEA1", "WEA2", "WEA3", "WEB0",
            "WEB1", "WEB2", "WEB3", "REGCEA", "REGCEB",
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_inv(pin);
        }
        bctx.mode(mode)
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", "FULL")
            .test_enum("RAM_MODE", &["TDP", "SDP", "SP"]);
        bctx.mode(mode)
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", "FULL")
            .test_enum("RSTTYPE", &["SYNC", "ASYNC"]);
        for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_enum(attr, &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"]);
        }
        for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_enum(attr, &["0", "1", "2", "4", "9", "18", "36"]);
        }
        for attr in ["DOA_REG", "DOB_REG"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_enum(attr, &["0", "1"]);
        }
        for attr in ["RST_PRIORITY_A", "RST_PRIORITY_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_enum(attr, &["CE", "SR"]);
        }
        for attr in ["EN_RSTRAM_A", "EN_RSTRAM_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_enum(attr, &["FALSE", "TRUE"]);
        }
        for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "36")
                .attr("DATA_WIDTH_B", "36")
                .test_multi_attr_hex(attr, 36)
        }
        for i in 0..0x40 {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "18")
                .test_manual(format!("INIT_{i:02X}.NARROW"), "")
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "36")
                .test_manual(format!("INIT_{i:02X}.WIDE"), "")
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for i in 0..8 {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "18")
                .test_manual(format!("INITP_{i:02X}.NARROW"), "")
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "36")
                .test_manual(format!("INITP_{i:02X}.WIDE"), "")
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
        }

        for (attr, val) in [
            ("ENWEAKWRITEA", "ENABLE"),
            ("ENWEAKWRITEB", "ENABLE"),
            ("WEAKWRITEVALA", "1"),
            ("WEAKWRITEVALB", "1"),
        ] {
            bctx.mode(mode)
                .global_mutex_here("BRAM")
                .test_manual(attr, val)
                .global(attr, val)
                .commit();
        }
        for attr in [
            "BRAM_DDEL_A_D",
            "BRAM_DDEL_A_U",
            "BRAM_DDEL_B_D",
            "BRAM_DDEL_B_U",
        ] {
            for val in ["0", "1", "11", "111"] {
                bctx.mode(mode)
                    .global_mutex_here("BRAM")
                    .test_manual(attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for attr in [
            "BRAM_WDEL_A_D",
            "BRAM_WDEL_A_U",
            "BRAM_WDEL_B_D",
            "BRAM_WDEL_B_U",
        ] {
            for val in ["0", "1", "10", "11", "100", "101", "110", "111"] {
                bctx.mode(mode)
                    .global_mutex_here("BRAM")
                    .test_manual(attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
    }
    for bel in [bels::BRAM_H0, bels::BRAM_H1] {
        let mut bctx = ctx.bel(bel);
        let mode = "RAMB8BWER";
        let bel_name = backend.edev.db.bel_slots.key(bel).as_str();
        bctx.build()
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", bel_name)
            .prop(ExtraBramFixup)
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();
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
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_inv(pin);
        }
        bctx.mode(mode)
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", "HALF")
            .test_enum("RAM_MODE", &["TDP", "SDP", "SP"]);
        bctx.mode(mode)
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", "HALF")
            .test_enum("RSTTYPE", &["SYNC", "ASYNC"]);
        for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_enum(attr, &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"]);
        }
        for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_enum(attr, &["0", "1", "2", "4", "9", "18", "36"]);
        }
        for attr in ["DOA_REG", "DOB_REG"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_enum(attr, &["0", "1"]);
        }
        for attr in ["RST_PRIORITY_A", "RST_PRIORITY_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_enum(attr, &["CE", "SR"]);
        }
        for attr in ["EN_RSTRAM_A", "EN_RSTRAM_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_enum(attr, &["FALSE", "TRUE"]);
        }
        for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "18")
                .test_multi_attr_hex(attr, 18);
        }
        for i in 0..0x20 {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_manual(format!("INIT_{i:02X}"), "")
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for i in 0..4 {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_manual(format!("INITP_{i:02X}"), "")
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for (attr, val) in [
            ("ENWEAKWRITEA", "ENABLE"),
            ("ENWEAKWRITEB", "ENABLE"),
            ("WEAKWRITEVALA", "1"),
            ("WEAKWRITEVALB", "1"),
        ] {
            bctx.mode(mode)
                .global_mutex_here("BRAM")
                .test_manual(attr, val)
                .global(attr, val)
                .commit();
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    for attr in ["BW_EN_A_D", "BW_EN_A_U", "BW_EN_B_D", "BW_EN_B_U"] {
        ctx.build()
            .global_mutex("BRAM", "NONE")
            .extra_tiles_by_bel(bels::BRAM_F, "BRAM_F")
            .test_manual("BRAM", attr, "1")
            .global(attr, "1")
            .commit();
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
        ctx.tiledb.insert(tile, "BRAM_H0", attr, xlat_bit(diff1_h0));
        ctx.tiledb.insert(tile, "BRAM_H1", attr, xlat_bit(diff1_h1));
    }
    for (attr, bel, sattr) in [
        ("BW_EN_A", "BRAM_H0", "BW_EN_A_D"),
        ("BW_EN_B", "BRAM_H0", "BW_EN_B_D"),
        ("BW_EN_A", "BRAM_H1", "BW_EN_A_U"),
        ("BW_EN_B", "BRAM_H1", "BW_EN_B_U"),
    ] {
        let diff = ctx.state.get_diff(tile, "BRAM_F", sattr, "1");
        ctx.tiledb.insert(tile, bel, attr, xlat_bit(diff));
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
    assert_eq!(present_h0, present_f);
    let present_h0_fixup = ctx.state.get_diff(tile, "BRAM_H0", "PRESENT.FIXUP", "1");
    let present_h1_fixup = ctx.state.get_diff(tile, "BRAM_H1", "PRESENT.FIXUP", "1");
    assert_eq!(present_h0_fixup, present_h1_fixup);
    assert_eq!(present_h0_fixup, present_f);
    assert!(present_f.bits.values().all(|x| *x));
    let mut bits: Vec<_> = present_f.bits.keys().copied().collect();
    bits.sort();
    for (bel, bit) in [("BRAM_H0", bits[0]), ("BRAM_H1", bits[1])] {
        ctx.tiledb.insert(
            tile,
            bel,
            "MODE",
            TileItem {
                bits: vec![bit],
                kind: TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("RAMB8BWER".to_string(), bits![0]),
                        ("RAMB16BWER".to_string(), bits![1]),
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

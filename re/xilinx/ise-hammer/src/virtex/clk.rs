use prjcombine_re_fpga_hammer::{Diff, xlat_bitvec, xlat_bool, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_virtex::bels;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

use super::io::VirtexOtherIobInput;

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let package = backend.ebonds.keys().next().unwrap();
    for tile in [
        "CLKB",
        "CLKT",
        "CLKB_4DLL",
        "CLKT_4DLL",
        "CLKB_2DLL",
        "CLKT_2DLL",
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for i in 0..2 {
            let mut bctx = ctx.bel(bels::GCLK_IO[i]);
            let iostds = if !tile.ends_with("DLL") {
                &[
                    "LVTTL", "LVCMOS2", "PCI33_3", "PCI33_5", "PCI66_3", "GTL", "GTLP", "HSTL_I",
                    "HSTL_III", "HSTL_IV", "SSTL3_I", "SSTL3_II", "SSTL2_I", "SSTL2_II", "CTT",
                    "AGP",
                ][..]
            } else {
                &[
                    "LVTTL", "LVCMOS2", "LVCMOS18", "PCI33_3", "PCI66_3", "PCIX66_3", "GTL",
                    "GTLP", "HSTL_I", "HSTL_III", "HSTL_IV", "SSTL3_I", "SSTL3_II", "SSTL2_I",
                    "SSTL2_II", "CTT", "AGP", "LVDS", "LVPECL",
                ][..]
            };
            for &iostd in iostds {
                bctx.build()
                    .global_mutex("GCLKIOB", "YES")
                    .raw(Key::Package, package)
                    .global_mutex("VREF", "YES")
                    .prop(VirtexOtherIobInput(bels::GCLK_IO[i], "GTL".into()))
                    .global("UNUSEDPIN", "PULLNONE")
                    .test_manual("IOATTRBOX", iostd)
                    .mode("GCLKIOB")
                    .attr("IOATTRBOX", iostd)
                    .commit();
            }
            let idx = if tile.starts_with("CLKB") { i } else { 2 + i };
            for val in ["11110", "11101", "11011", "10111", "01111"] {
                bctx.mode("GCLKIOB")
                    .test_manual("DELAY", val)
                    .global(format!("GCLKDEL{idx}"), val)
                    .commit();
            }
        }
        // TODO: IOFB
        for i in 0..2 {
            let mut bctx = ctx.bel(bels::BUFG[i]);
            bctx.mode("GCLK")
                .pin("CE")
                .test_enum("CEMUX", &["0", "1", "CE", "CE_B"]);
            bctx.mode("GCLK")
                .test_enum("DISABLE_ATTR", &["LOW", "HIGH"]);
        }
    }
    for tile in ["CLKV.CLKV", "CLKV.GCLKV", "CLKV.NULL"] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };

        let mut bctx = ctx.bel(bels::CLKV);
        for lr in ['L', 'R'] {
            for i in 0..4 {
                let mut builder = bctx.build();
                if tile == "CLKV.NULL" {
                    builder = builder.null_bits();
                }
                builder
                    .test_manual(format!("BUF.GCLK_{lr}{i}"), "1")
                    .pip(format!("OUT_{lr}{i}"), format!("IN{i}"))
                    .commit();
            }
        }
    }

    // causes a crash on xcv405e. lmao.
    if !backend.device.name.ends_with('e') {
        for (tile, slot) in [
            ("CLKV_BRAM_S", bels::CLKV_BRAM_S),
            ("CLKV_BRAM_N", bels::CLKV_BRAM_N),
        ] {
            let mut ctx = FuzzCtx::new(session, backend, tile);
            let mut bctx = ctx.bel(slot);
            for lr in ['L', 'R'] {
                for i in 0..4 {
                    bctx.build()
                        .tile_mutex("GCLK_DIR", lr)
                        .test_manual(format!("BUF.GCLK_{lr}{i}"), "1")
                        .pip(format!("OUT_{lr}{i}"), format!("IN{i}"))
                        .commit();
                }
            }
        }
    }
    for tile in ["LBRAM", "RBRAM"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(bels::CLKV_BRAM);
        for lr in ['L', 'R'] {
            for i in 0..4 {
                for j in 0..4 {
                    bctx.test_manual(format!("BUF.GCLK_{lr}{i}_{j}"), "1")
                        .pip(format!("OUT_{lr}{j}_{i}"), format!("IN{i}"))
                        .commit();
                }
            }
        }
    }

    for (tile, bel) in [
        ("CLKC", bels::CLKC),
        ("CLKC", bels::GCLKC),
        ("GCLKC", bels::GCLKC),
        ("BRAM_CLKH", bels::BRAM_CLKH),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let mut bctx = ctx.bel(bel);
        for i in 0..4 {
            // TODO null_bits
            bctx.build()
                .null_bits()
                .test_manual(format!("BUF.GCLK{i}"), "1")
                .pip(format!("OUT{i}"), format!("IN{i}"))
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let is_s2 = ctx.device.name.contains("2s") && !ctx.device.name.ends_with('e');
    for tile in [
        "CLKB",
        "CLKB_4DLL",
        "CLKB_2DLL",
        "CLKT",
        "CLKT_4DLL",
        "CLKT_2DLL",
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        for i in 0..2 {
            let bel = format!("GCLK_IO{i}");
            let bel = &bel;
            let mut diffs = vec![];
            for val in ["11110", "11101", "11011", "10111", "01111"] {
                let diff = ctx.state.get_diff(tile, bel, "DELAY", val);
                diffs.push(!diff);
            }
            ctx.tiledb.insert(tile, bel, "DELAY", xlat_bitvec(diffs));
            let iostds = if !tile.ends_with("DLL") {
                &[
                    ("CMOS", "LVTTL"),
                    ("CMOS", "LVCMOS2"),
                    ("CMOS", "PCI33_3"),
                    ("CMOS", "PCI33_5"),
                    ("CMOS", "PCI66_3"),
                    ("VREF_LV", "GTL"),
                    ("VREF_LV", "HSTL_I"),
                    ("VREF_LV", "HSTL_III"),
                    ("VREF_LV", "HSTL_IV"),
                    ("VREF_HV", "GTLP"),
                    ("VREF_HV", "SSTL3_I"),
                    ("VREF_HV", "SSTL3_II"),
                    ("VREF_HV", "SSTL2_I"),
                    ("VREF_HV", "SSTL2_II"),
                    ("VREF_HV", "CTT"),
                    ("VREF_HV", "AGP"),
                ][..]
            } else {
                &[
                    ("CMOS", "LVTTL"),
                    ("CMOS", "LVCMOS2"),
                    ("CMOS", "LVCMOS18"),
                    ("CMOS", "PCI33_3"),
                    ("CMOS", "PCI66_3"),
                    ("CMOS", "PCIX66_3"),
                    ("VREF", "GTL"),
                    ("VREF", "GTLP"),
                    ("VREF", "HSTL_I"),
                    ("VREF", "HSTL_III"),
                    ("VREF", "HSTL_IV"),
                    ("VREF", "SSTL3_I"),
                    ("VREF", "SSTL3_II"),
                    ("VREF", "SSTL2_I"),
                    ("VREF", "SSTL2_II"),
                    ("VREF", "CTT"),
                    ("VREF", "AGP"),
                    ("DIFF", "LVDS"),
                    ("DIFF", "LVPECL"),
                ][..]
            };
            let mut diffs = vec![("NONE", Diff::default())];
            for &(val, iostd) in iostds {
                diffs.push((val, ctx.state.get_diff(tile, bel, "IOATTRBOX", iostd)));
            }
            ctx.tiledb.insert(tile, bel, "IBUF", xlat_enum(diffs));
        }
        for i in 0..2 {
            let bel = format!("BUFG{i}");
            let bel = &bel;
            let d0 = ctx.state.get_diff(tile, bel, "CEMUX", "CE");
            assert_eq!(d0, ctx.state.get_diff(tile, bel, "CEMUX", "1"));
            let d1 = ctx.state.get_diff(tile, bel, "CEMUX", "CE_B");
            assert_eq!(d1, ctx.state.get_diff(tile, bel, "CEMUX", "0"));
            let item = xlat_bool(d0, d1);
            ctx.insert_int_inv(&[tile], tile, bel, "CE", item);
            ctx.collect_enum_bool(tile, bel, "DISABLE_ATTR", "LOW", "HIGH");
        }
    }
    for tile in ["CLKV.CLKV", "CLKV.GCLKV"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "CLKV";
        for lr in ['L', 'R'] {
            for i in 0..4 {
                ctx.collect_bit(tile, bel, &format!("BUF.GCLK_{lr}{i}"), "1");
            }
        }
    }
    if !ctx.device.name.ends_with('e') {
        for tile in ["CLKV_BRAM_S", "CLKV_BRAM_N"] {
            let bel = tile;
            for lr in ['L', 'R'] {
                for i in 0..4 {
                    let item = ctx.extract_bit_wide(tile, bel, &format!("BUF.GCLK_{lr}{i}"), "1");
                    if is_s2 {
                        ctx.tiledb.insert(tile, bel, format!("BUF.GCLK{i}"), item);
                    } else {
                        assert!(item.bits.is_empty());
                    }
                }
            }
        }
    }
    for tile in ["LBRAM", "RBRAM"] {
        let bel = "CLKV_BRAM";
        for lr in ['L', 'R'] {
            for i in 0..4 {
                for j in 0..4 {
                    if tile.starts_with(lr) && !is_s2 {
                        ctx.state
                            .get_diff(tile, bel, format!("BUF.GCLK_{lr}{i}_{j}"), "1")
                            .assert_empty();
                    } else {
                        ctx.collect_bit(tile, bel, &format!("BUF.GCLK_{lr}{i}_{j}"), "1");
                    }
                }
            }
        }
    }
}

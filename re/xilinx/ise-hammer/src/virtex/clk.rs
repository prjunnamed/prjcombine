use prjcombine_re_collector::{
    diff::Diff,
    legacy::{xlat_bit_bi_legacy, xlat_bitvec_legacy, xlat_enum_legacy},
};
use prjcombine_re_hammer::Session;
use prjcombine_virtex::defs;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

use super::io::VirtexOtherIobInput;

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let package = backend.ebonds.keys().next().unwrap();
    for tile in [
        "CLK_S_V",
        "CLK_N_V",
        "CLK_S_VE_4DLL",
        "CLK_N_VE_4DLL",
        "CLK_S_VE_2DLL",
        "CLK_N_VE_2DLL",
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for i in 0..2 {
            let mut bctx = ctx.bel(defs::bslots::GCLK_IO[i]);
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
                    .prop(VirtexOtherIobInput(defs::bslots::GCLK_IO[i], "GTL".into()))
                    .global("UNUSEDPIN", "PULLNONE")
                    .test_manual("IOATTRBOX", iostd)
                    .mode("GCLKIOB")
                    .attr("IOATTRBOX", iostd)
                    .commit();
            }
            let idx = if tile.starts_with("CLK_S") { i } else { 2 + i };
            for val in ["11110", "11101", "11011", "10111", "01111"] {
                bctx.mode("GCLKIOB")
                    .test_manual("DELAY", val)
                    .global(format!("GCLKDEL{idx}"), val)
                    .commit();
            }
        }
        // TODO: IOFB
        for i in 0..2 {
            let mut bctx = ctx.bel(defs::bslots::BUFG[i]);
            bctx.mode("GCLK")
                .pin("CE")
                .test_enum("CEMUX", &["0", "1", "CE", "CE_B"]);
            bctx.mode("GCLK")
                .test_enum("DISABLE_ATTR", &["LOW", "HIGH"]);
        }
    }
    for tile in ["CLKV_CLKV", "CLKV_GCLKV", "CLKV_NULL"] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };

        let mut bctx = ctx.bel(defs::bslots::CLKV);
        for lr in ['L', 'R'] {
            for i in 0..4 {
                let mut builder = bctx.build();
                if tile == "CLKV_NULL" {
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
            ("CLKV_BRAM_S", defs::bslots::CLKV_BRAM_S),
            ("CLKV_BRAM_N", defs::bslots::CLKV_BRAM_N),
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
    for tile in ["BRAM_W", "BRAM_E"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(defs::bslots::CLKV_BRAM);
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
        ("CLKC", defs::bslots::CLKC),
        ("CLKC", defs::bslots::GCLKC),
        ("GCLKC", defs::bslots::GCLKC),
        ("BRAM_CLKH", defs::bslots::BRAM_CLKH),
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
        "CLK_S_V",
        "CLK_S_VE_4DLL",
        "CLK_S_VE_2DLL",
        "CLK_N_V",
        "CLK_N_VE_4DLL",
        "CLK_N_VE_2DLL",
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        for i in 0..2 {
            let bel = format!("GCLK_IO[{i}]");
            let bel = &bel;
            let mut diffs = vec![];
            for val in ["11110", "11101", "11011", "10111", "01111"] {
                let diff = ctx.get_diff_legacy(tile, bel, "DELAY", val);
                diffs.push(!diff);
            }
            ctx.insert(tile, bel, "DELAY", xlat_bitvec_legacy(diffs));
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
                diffs.push((val, ctx.get_diff_legacy(tile, bel, "IOATTRBOX", iostd)));
            }
            ctx.insert(tile, bel, "IBUF", xlat_enum_legacy(diffs));
        }
        for i in 0..2 {
            let bel = format!("BUFG[{i}]");
            let bel = &bel;
            let d0 = ctx.get_diff_legacy(tile, bel, "CEMUX", "CE");
            assert_eq!(d0, ctx.get_diff_legacy(tile, bel, "CEMUX", "1"));
            let d1 = ctx.get_diff_legacy(tile, bel, "CEMUX", "CE_B");
            assert_eq!(d1, ctx.get_diff_legacy(tile, bel, "CEMUX", "0"));
            let item = xlat_bit_bi_legacy(d0, d1);
            ctx.insert(tile, bel, "INV.CE", item);
            ctx.collect_bit_bi_legacy(tile, bel, "DISABLE_ATTR", "LOW", "HIGH");
        }
    }
    for tile in ["CLKV_CLKV", "CLKV_GCLKV"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "CLKV";
        for lr in ['L', 'R'] {
            for i in 0..4 {
                ctx.collect_bit_legacy(tile, bel, &format!("BUF.GCLK_{lr}{i}"), "1");
            }
        }
    }
    if !ctx.device.name.ends_with('e') {
        for tile in ["CLKV_BRAM_S", "CLKV_BRAM_N"] {
            let bel = tile;
            for lr in ['L', 'R'] {
                for i in 0..4 {
                    let item =
                        ctx.extract_bit_wide_legacy(tile, bel, &format!("BUF.GCLK_{lr}{i}"), "1");
                    if is_s2 {
                        ctx.insert(tile, bel, format!("BUF.GCLK{i}"), item);
                    } else {
                        assert!(item.bits.is_empty());
                    }
                }
            }
        }
    }
    for tile in ["BRAM_W", "BRAM_E"] {
        let bel = "CLKV_BRAM";
        for (lr, t) in [('L', "BRAM_W"), ('R', "BRAM_E")] {
            for i in 0..4 {
                for j in 0..4 {
                    if tile == t && !is_s2 {
                        ctx.get_diff_legacy(tile, bel, format!("BUF.GCLK_{lr}{i}_{j}"), "1")
                            .assert_empty();
                    } else {
                        ctx.collect_bit_legacy(tile, bel, &format!("BUF.GCLK_{lr}{i}_{j}"), "1");
                    }
                }
            }
        }
    }
}

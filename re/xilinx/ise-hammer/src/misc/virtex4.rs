use bitvec::prelude::*;
use prjcombine_interconnect::{
    grid::DieId,
    {db::BelId, dir::Dir},
};
use prjcombine_re_collector::{
    Diff, OcdMode, xlat_bit, xlat_bitvec, xlat_enum_ocd, xlat_item_tile,
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_xilinx_bitstream::Reg;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CFG", "MISC", TileBits::Cfg);
    for val in ["0", "1", "2", "3"] {
        fuzz_one!(ctx, "PROBESEL", val, [], [(global_opt "PROBESEL", val)]);
    }
    for attr in ["CCLKPIN", "DONEPIN", "POWERDOWNPIN", "PROGPIN", "INITPIN"] {
        for val in ["PULLUP", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }
    for attr in [
        "HSWAPENPIN",
        "M0PIN",
        "M1PIN",
        "M2PIN",
        "CSPIN",
        "DINPIN",
        "BUSYPIN",
        "RDWRPIN",
        "TCKPIN",
        "TDIPIN",
        "TDOPIN",
        "TMSPIN",
    ] {
        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }

    for i in 0..32 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CFG",
            format!("BUFGCTRL{i}"),
            TileBits::Cfg,
        );
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFGCTRL")]);
        for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
            fuzz_inv!(ctx, pin, [(mode "BUFGCTRL")]);
        }
        fuzz_enum!(ctx, "PRESELECT_I0", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "PRESELECT_I1", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "CREATE_EDGE", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "INIT_OUT", ["0", "1"], [(mode "BUFGCTRL")]);

        for midx in 0..2 {
            let bus = format!("MUXBUS{midx}");
            let mux = format!("MUX.I{midx}");
            let opin = format!("I{midx}MUX");
            for val in ["CKINT0", "CKINT1"] {
                fuzz_one!(ctx, &mux, val, [
                    (mutex "IxMUX", &mux),
                    (mutex &mux, val)
                ], [
                    (pip (pin val), (pin opin))
                ]);
            }
            let obel = BelId::from_idx(0);
            let mb_idx = 2 * (i % 16) + midx;
            let mb_out = format!("MUXBUS_O{mb_idx}");
            fuzz_one!(ctx, &mux, "MUXBUS", [
                (mutex "IxMUX", &mux),
                (mutex &mux, "MUXBUS"),
                (global_mutex "CLK_IOB_MUXBUS", "USE"),
                (related TileRelation::ClkIob(if i >= 16 { Dir::N } else { Dir::S }),
                    (pip (bel_pin obel, "PAD_BUF0"), (bel_pin obel, mb_out)))
            ], [
                (pip (pin bus), (pin opin))
            ]);
            for j in 0..16 {
                let jj = if i < 16 { j } else { j + 16 };
                let obel = BelId::from_idx(jj);
                let val = format!("GFB{j}");
                fuzz_one!(ctx, &mux, &val, [
                    (mutex "IxMUX", &mux),
                    (mutex &mux, val)
                ], [
                    (pip (bel_pin obel, "GFB"), (pin opin))
                ]);
            }
            for val in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
                let obel = BelId::from_idx(45 + i / 16);
                let obel_bufg = BelId::from_idx(i ^ 1);
                fuzz_one!(ctx, &mux, val, [
                    (mutex "IxMUX", &mux),
                    (mutex &mux, val),
                    (global_mutex "BUFG_MGTCLK", "USE"),
                    (bel_mutex obel_bufg, "IxMUX", &mux),
                    (bel_mutex obel_bufg, &mux, val),
                    (pip (bel_pin obel, val), (bel_pin obel_bufg, opin))
                ], [
                    (pip (bel_pin obel, val), (pin opin))
                ]);
            }
        }
        fuzz_one!(ctx, "ENABLE", "1", [
            (global_mutex "BUFGCTRL_OUT", "TEST"),
            (mode "BUFGCTRL")
        ], [(pin "O")]);
        fuzz_one!(ctx, "PIN_O_GFB", "1", [
            (global_mutex "BUFGCTRL_OUT", "TEST"),
            (mode "BUFGCTRL"),
            (pin "O")
        ], [
            (pip (pin "O"), (pin "GFB"))
        ]);
        fuzz_one!(ctx, "PIN_O_GCLK", "1", [
            (global_mutex "BUFGCTRL_OUT", "TEST"),
            (global_mutex "BUFGCTRL_O_GCLK", ctx.bel_name),
            (mode "BUFGCTRL"),
            (pin "O")
        ], [
            (pip (pin "O"), (pin "GCLK"))
        ]);
    }

    for i in 0..4 {
        let ctx = FuzzCtx::new(session, backend, "CFG", format!("BSCAN{i}"), TileBits::Cfg);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BSCAN")]);
    }
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CFG", "BSCAN_COMMON", TileBits::Cfg);
    fuzz_multi!(ctx, "USERID", "", 32, [], (global_hex_prefix "USERID"));

    for i in 0..2 {
        let ctx = FuzzCtx::new(session, backend, "CFG", format!("ICAP{i}"), TileBits::Cfg);
        let obel = BelId::from_idx(36 + (i ^ 1));
        fuzz_one!(ctx, "PRESENT", "1", [(bel_unused obel)], [(mode "ICAP")]);
        fuzz_inv!(ctx, "CLK", [(mode "ICAP")]);
        fuzz_inv!(ctx, "CE", [(mode "ICAP")]);
        fuzz_inv!(ctx, "WRITE", [(mode "ICAP")]);
        fuzz_enum!(ctx, "ICAP_WIDTH", ["X8", "X32"], [(mode "ICAP"), (bel_unused obel)]);
    }

    let ctx = FuzzCtx::new(session, backend, "CFG", "PMV", TileBits::Cfg);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PMV")]);

    let mut ctx = FuzzCtx::new(session, backend, "CFG", "STARTUP", TileBits::Cfg);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "STARTUP")]);
    for pin in [
        "CLK",
        "GTS",
        "GSR",
        "USRCCLKTS",
        "USRCCLKO",
        "USRDONETS",
        "USRDONEO",
    ] {
        fuzz_inv!(ctx, pin, [(mode "STARTUP")]);
    }
    fuzz_one!(ctx, "PIN.GTS", "1", [(mode "STARTUP"), (nopin "GSR")], [(pin "GTS")]);
    fuzz_one!(ctx, "PIN.GSR", "1", [(mode "STARTUP"), (nopin "GTS")], [(pin "GSR")]);
    fuzz_one!(ctx, "PIN.USRCCLKO", "1", [(mode "STARTUP")], [(pin "USRCCLKO")]);
    for attr in ["GSR_SYNC", "GWE_SYNC", "GTS_SYNC"] {
        for val in ["YES", "NO"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }
    ctx.bits = TileBits::Reg(Reg::Cor0);
    ctx.tile_name = "REG.COR".into();
    for val in ["CCLK", "USERCLK", "JTAGCLK"] {
        fuzz_one!(ctx, "STARTUPCLK", val, [(mode "STARTUP"), (pin "CLK")], [(global_opt "STARTUPCLK", val)]);
    }

    let ctx = FuzzCtx::new(session, backend, "CFG", "JTAGPPC", TileBits::Cfg);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "JTAGPPC")]);

    let ctx = FuzzCtx::new(session, backend, "CFG", "FRAME_ECC", TileBits::Cfg);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "FRAME_ECC")]);

    let ctx = FuzzCtx::new(session, backend, "CFG", "DCIRESET", TileBits::Cfg);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "DCIRESET")]);

    let mut ctx = FuzzCtx::new(session, backend, "CFG", "CAPTURE", TileBits::Cfg);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CAPTURE")]);
    fuzz_inv!(ctx, "CLK", [(mode "CAPTURE")]);
    fuzz_inv!(ctx, "CAP", [(mode "CAPTURE")]);
    ctx.bits = TileBits::CfgReg(Reg::Cor0);
    fuzz_enum!(ctx, "ONESHOT", ["FALSE", "TRUE"], [(mode "CAPTURE")]);

    let ctx = FuzzCtx::new(session, backend, "CFG", "USR_ACCESS", TileBits::Cfg);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "USR_ACCESS")]);

    if edev.col_lgt.is_some() {
        for (bel_id, bel) in [
            (BelId::from_idx(47), "BUFG_MGTCLK_B"),
            (BelId::from_idx(48), "BUFG_MGTCLK_T"),
        ] {
            let ctx = FuzzCtx::new_force_bel(session, backend, "CFG", bel, TileBits::Cfg, bel_id);
            for (name, o, i) in [
                ("BUF.MGT_L0", "MGT_L0_O", "MGT_L0_I"),
                ("BUF.MGT_L1", "MGT_L1_O", "MGT_L1_I"),
                ("BUF.MGT_R0", "MGT_R0_O", "MGT_R0_I"),
                ("BUF.MGT_R1", "MGT_R1_O", "MGT_R1_I"),
            ] {
                fuzz_one!(ctx, name, "1", [
                    (global_mutex "BUFG_MGTCLK", "TEST")
                ], [
                    (pip (pin i), (pin o))
                ]);
            }
        }
        for (bel_id, bel, dir_row) in [
            (BelId::from_idx(49), "BUFG_MGTCLK_SRC_B", Dir::S),
            (BelId::from_idx(50), "BUFG_MGTCLK_SRC_T", Dir::N),
        ] {
            let ctx = FuzzCtx::new_force_bel(session, backend, "CFG", bel, TileBits::Null, bel_id);
            for (name, o, i) in [
                ("MGT_L0", "MGT_L0_O", "MGT_L0_I"),
                ("MGT_L1", "MGT_L1_O", "MGT_L1_I"),
                ("MGT_R0", "MGT_R0_O", "MGT_R0_I"),
                ("MGT_R1", "MGT_R1_O", "MGT_R1_I"),
            ] {
                let idx = if name.ends_with('1') { 1 } else { 0 };
                let mut extras = vec![];
                if !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::MgtRepeater(
                            if name.starts_with("MGT_L") {
                                Dir::W
                            } else {
                                Dir::E
                            },
                            Some(dir_row),
                        ),
                        "HCLK_MGT_REPEATER",
                        "HCLK_MGT_REPEATER",
                        format!("BUF.MGT{idx}.CFG"),
                        "1",
                    ));
                }
                fuzz_one_extras!(ctx, name, "1", [
                    (global_mutex "MGT_OUT", "USE")
                ], [
                    (pip (pin i), (pin o))
                ], extras);
            }
        }
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.COR",
        "STARTUP",
        TileBits::Reg(Reg::Cor0),
    );
    for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
        fuzz_one!(ctx, "GWE_CYCLE", val, [], [(global_opt "GWE_CYCLE", val)]);
        fuzz_one!(ctx, "GTS_CYCLE", val, [], [(global_opt "GTS_CYCLE", val)]);
    }
    for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
        fuzz_one!(ctx, "DONE_CYCLE", val, [], [(global_opt "DONE_CYCLE", val)]);
    }
    for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
        fuzz_one!(ctx, "LCK_CYCLE", val, [], [(global_opt "LCK_CYCLE", val)]);
        fuzz_one!(ctx, "MATCH_CYCLE", val, [(global_mutex "GLOBAL_DCI", "NO")], [(global_opt "MATCH_CYCLE", val)]);
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "DRIVE_DONE", val, [], [(global_opt "DRIVEDONE", val)]);
        fuzz_one!(ctx, "DONE_PIPE", val, [], [(global_opt "DONEPIPE", val)]);
    }
    for val in [
        "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51", "55", "60",
        "130",
    ] {
        fuzz_one!(ctx, "CONFIG_RATE", val, [], [(global_opt "CONFIGRATE", val)]);
    }
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "CRC", val, [], [(global_opt "CRC", val)]);
        fuzz_one!(ctx, "DCM_SHUTDOWN", val, [], [(global_opt "DCMSHUTDOWN", val)]);
        fuzz_one!(ctx, "POWERDOWN_STATUS", val, [], [(global_opt "POWERDOWNSTATUS", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.CTL",
        "MISC",
        TileBits::Reg(Reg::Ctl0),
    );
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "GTS_USR_B", val, [], [(global_opt "GTS_USR_B", val)]);
        fuzz_one!(ctx, "VGG_TEST", val, [], [(global_opt "VGG_TEST", val)]);
        fuzz_one!(ctx, "EN_VTEST", val, [], [(global_opt "EN_VTEST", val)]);
        fuzz_one!(ctx, "ENCRYPT", val, [], [(global_opt "ENCRYPT", val)]);
    }
    // persist not fuzzed — too much effort
    for val in ["NONE", "LEVEL1", "LEVEL2"] {
        fuzz_one!(ctx, "SECURITY", val, [], [(global_opt "SECURITY", val)]);
    }

    // TODO: more crap
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "DISABLE_BANDGAP", val, [], [(global_opt "DISABLEBANDGAP", val)]);
    }
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "DCI_SHUTDOWN", val, [], [(global_opt "DCISHUTDOWN", val)]);
    }

    if let Some(ctx) = FuzzCtx::try_new(session, backend, "SYSMON", "SYSMON", TileBits::MainAuto) {
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "MONITOR")]);
        for i in 0x40..0x70 {
            fuzz_multi!(ctx, format!("INIT_{i:02X}"), "", 16, [(mode "MONITOR")], (attr_hex format!("INIT_{i:02X}")));
        }
        fuzz_enum!(ctx, "MONITOR_MODE", ["TEST", "MONITOR", "ADC"], [
            (global_mutex_none "MONITOR_GLOBAL"),
            (mode "MONITOR")
        ]);
        for pin in [
            "DEN",
            // DCLK?
            "DWE",
            "SCANTESTENA",
            "SCANTESTENB",
            // SCANMEMCLK?
            "SCANMEMWE",
            "ROMTESTENABLE",
            "RST",
            "CONVST",
            // SCLK[AB]?
            "SEA",
            "SEB",
        ] {
            fuzz_inv!(ctx, pin, [(mode "MONITOR")]);
        }
        for (attr, len) in [
            ("DCLK_DIVID_2", 1),
            ("LW_DIVID_2_4", 1),
            ("MCCLK_DIVID", 8),
            ("OVER_TEMPERATURE", 10),
            ("OVER_TEMPERATURE_OFF", 1),
            ("OVER_TEMPERATURE_DELAY", 8),
            ("BLOCK_ENABLE", 5),
            ("DCLK_MISSING", 10),
            ("FEATURE_ENABLE", 8),
            ("PROM_DATA", 8),
        ] {
            fuzz_multi!(ctx, attr, "", len, [
                (global_mutex_site "MONITOR_GLOBAL"),
                (mode "MONITOR"),
                (attr "MONITOR_MODE", "ADC")
            ], (global_bin format!("ADC_{attr}")));
        }
        for out in ["CONVST", "CONVST_TEST"] {
            fuzz_one!(ctx, out, "INT_CLK", [
                (mutex "CONVST_OUT", out),
                (mutex "CONVST_IN", "INT_CLK")
            ], [
                (pip (pin "CONVST_INT_CLK"), (pin out))
            ]);
            fuzz_one!(ctx, out, "INT_IMUX", [
                (mutex "CONVST_OUT", out),
                (mutex "CONVST_IN", "INT_IMUX")
            ], [
                (pip (pin "CONVST_INT_IMUX"), (pin out))
            ]);
            for i in 0..16 {
                let obel = BelId::from_idx(0);
                fuzz_one!(ctx, out, format!("GIOB{i}"), [
                    (mutex "CONVST_OUT", out),
                    (mutex "CONVST_IN", format!("GIOB{i}")),
                    (related TileRelation::HclkDcm,
                        (tile_mutex "HCLK_DCM", "USE")),
                    (related TileRelation::HclkDcm,
                        (pip (pin format!("GIOB_I{i}")), (pin format!("GIOB_O_D{i}")))),
                    (related TileRelation::HclkDcm,
                        (pip (bel_pin obel, format!("GIOB_I{i}")), (bel_pin obel, format!("GIOB_O_U{i}"))))
                ], [
                    (pip (pin format!("GIOB{i}")), (pin out))
                ]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let tile = "CFG";
    let reg_tidx = &[34][..];
    let bel = "MISC";
    ctx.collect_enum_default(tile, bel, "PROBESEL", &["0", "1", "2", "3"], "NONE");
    for attr in ["CCLKPIN", "DONEPIN", "POWERDOWNPIN", "PROGPIN", "INITPIN"] {
        ctx.collect_enum(tile, bel, attr, &["PULLUP", "PULLNONE"]);
    }
    for attr in [
        "HSWAPENPIN",
        "M0PIN",
        "M1PIN",
        "M2PIN",
        "CSPIN",
        "DINPIN",
        "BUSYPIN",
        "RDWRPIN",
        "TCKPIN",
        "TDIPIN",
        "TDOPIN",
        "TMSPIN",
    ] {
        ctx.collect_enum(tile, bel, attr, &["PULLUP", "PULLDOWN", "PULLNONE"]);
    }

    for i in 0..32 {
        let bel = format!("BUFGCTRL{i}");
        let bel = &bel;
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
            ctx.collect_inv(tile, bel, pin);
        }
        ctx.collect_enum_bool(tile, bel, "PRESELECT_I0", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "PRESELECT_I1", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CREATE_EDGE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

        let (_, _, ien_diff) = Diff::split(
            ctx.state.peek_diff(tile, bel, "MUX.I0", "CKINT0").clone(),
            ctx.state.peek_diff(tile, bel, "MUX.I1", "CKINT0").clone(),
        );
        let ien_item = xlat_bit(ien_diff);
        for mux in ["MUX.I0", "MUX.I1"] {
            let mut vals = vec![("NONE", Diff::default())];
            for val in [
                "GFB0", "GFB1", "GFB2", "GFB3", "GFB4", "GFB5", "GFB6", "GFB7", "GFB8", "GFB9",
                "GFB10", "GFB11", "GFB12", "GFB13", "GFB14", "GFB15", "CKINT0", "CKINT1", "MGT_L0",
                "MGT_L1", "MGT_R0", "MGT_R1", "MUXBUS",
            ] {
                let mut diff = ctx.state.get_diff(tile, bel, mux, val);
                diff.apply_bit_diff(&ien_item, true, false);
                vals.push((val, diff));
            }
            ctx.tiledb
                .insert(tile, bel, mux, xlat_enum_ocd(vals, OcdMode::Mux));
        }
        ctx.tiledb.insert(tile, bel, "IMUX_ENABLE", ien_item);

        ctx.state
            .get_diff(tile, bel, "PIN_O_GFB", "1")
            .assert_empty();
        ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
        let diff = ctx.state.get_diff(tile, bel, "PIN_O_GCLK", "1");
        if i == 19 || i == 30 {
            // ???? bug?
            diff.assert_empty();
        } else {
            let diffs = diff.split_tiles(&[&[32], &[33]]);
            let [diff_b, diff_t] = diffs.try_into().unwrap();
            ctx.tiledb
                .insert("CLK_TERM_B", "CLK_TERM", "GCLK_ENABLE", xlat_bit(diff_b));
            ctx.tiledb
                .insert("CLK_TERM_T", "CLK_TERM", "GCLK_ENABLE", xlat_bit(diff_t));
        }
    }

    for bel in [
        "BSCAN0", "BSCAN1", "BSCAN2", "BSCAN3", "JTAGPPC", "DCIRESET", "ICAP0", "ICAP1",
    ] {
        let item = ctx.extract_bit(tile, bel, "PRESENT", "1");
        ctx.tiledb.insert(tile, bel, "ENABLE", item);
    }

    let bel = "BSCAN_COMMON";
    let item = xlat_bitvec(ctx.state.get_diffs(tile, bel, "USERID", ""));
    ctx.tiledb.insert(tile, bel, "USERID", item);

    let bel = "STARTUP";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    ctx.collect_enum_bool(tile, bel, "GSR_SYNC", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "GWE_SYNC", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "GTS_SYNC", "NO", "YES");
    for pin in [
        "CLK",
        "GSR",
        "USRDONETS",
        "USRDONEO",
        "USRCCLKTS",
        "USRCCLKO",
    ] {
        ctx.collect_int_inv(&["INT"; 16], tile, bel, pin, false);
    }
    ctx.collect_int_inv(&["INT"; 16], tile, bel, "GTS", true);
    let item0 = ctx.extract_bit(tile, bel, "PIN.GSR", "1");
    let item1 = ctx.extract_bit(tile, bel, "PIN.GTS", "1");
    assert_eq!(item0, item1);
    ctx.tiledb.insert(tile, "STARTUP", "GTS_GSR_ENABLE", item0);
    let item = ctx.extract_bit(tile, bel, "PIN.USRCCLKO", "1");
    ctx.tiledb.insert(tile, "STARTUP", "USRCCLK_ENABLE", item);

    let item0 = ctx.extract_enum(tile, "ICAP0", "ICAP_WIDTH", &["X8", "X32"]);
    let item1 = ctx.extract_enum(tile, "ICAP1", "ICAP_WIDTH", &["X8", "X32"]);
    assert_eq!(item0, item1);
    ctx.tiledb.insert(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);
    for bel in ["ICAP0", "ICAP1"] {
        for pin in ["CLK", "CE", "WRITE"] {
            ctx.collect_int_inv(&["INT"; 16], tile, bel, pin, false);
        }
    }

    let bel = "CAPTURE";
    let item = ctx.extract_enum_bool(tile, bel, "ONESHOT", "FALSE", "TRUE");
    ctx.tiledb
        .insert("REG.COR", bel, "ONESHOT", xlat_item_tile(item, reg_tidx));
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    ctx.collect_int_inv(&["INT"; 16], tile, bel, "CLK", false);
    ctx.collect_int_inv(&["INT"; 16], tile, bel, "CAP", true);

    ctx.state
        .get_diff(tile, "PMV", "PRESENT", "1")
        .assert_empty();
    ctx.state
        .get_diff(tile, "FRAME_ECC", "PRESENT", "1")
        .assert_empty();
    ctx.state
        .get_diff(tile, "USR_ACCESS", "PRESENT", "1")
        .assert_empty();

    if edev.col_lgt.is_some() {
        for bel in ["BUFG_MGTCLK_B", "BUFG_MGTCLK_T"] {
            for attr in ["BUF.MGT_L0", "BUF.MGT_L1", "BUF.MGT_R0", "BUF.MGT_R1"] {
                ctx.collect_bit(tile, bel, attr, "1");
            }
        }
        if !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
            let tile = "HCLK_MGT_REPEATER";
            let bel = "HCLK_MGT_REPEATER";
            let item = ctx.extract_bit(tile, bel, "BUF.MGT0.CFG", "1");
            ctx.tiledb.insert(tile, bel, "BUF.MGT0", item);
            let item = ctx.extract_bit(tile, bel, "BUF.MGT1.CFG", "1");
            ctx.tiledb.insert(tile, bel, "BUF.MGT1", item);
        }
    }

    // config regs

    let tile = "REG.COR";
    let bel = "STARTUP";
    ctx.collect_enum(
        tile,
        bel,
        "GWE_CYCLE",
        &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "GTS_CYCLE",
        &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "DONE_CYCLE",
        &["1", "2", "3", "4", "5", "6", "KEEP"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "LCK_CYCLE",
        &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "MATCH_CYCLE",
        &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
    );
    ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
    ctx.collect_enum_ocd(
        tile,
        bel,
        "CONFIG_RATE",
        &[
            "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51", "55",
            "60", "130",
        ],
        OcdMode::BitOrder,
    );
    ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "DCM_SHUTDOWN", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "POWERDOWN_STATUS", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");

    let tile = "REG.CTL";
    let bel = "MISC";
    ctx.collect_enum_bool(tile, bel, "GTS_USR_B", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "VGG_TEST", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "EN_VTEST", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "ENCRYPT", "NO", "YES");
    ctx.collect_enum(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
    // these are too much trouble to deal with the normal way.
    ctx.tiledb.insert(
        tile,
        bel,
        "PERSIST",
        TileItem {
            bits: vec![TileBit::new(0, 0, 3)],
            kind: TileItemKind::BitVec { invert: bitvec![0] },
        },
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "GLUTMASK",
        TileItem {
            bits: vec![TileBit::new(0, 0, 8)],
            kind: TileItemKind::BitVec { invert: bitvec![1] },
        },
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ICAP_SELECT",
        TileItem {
            bits: vec![TileBit::new(0, 0, 30)],
            kind: TileItemKind::Enum {
                values: [
                    ("TOP".to_string(), bitvec![0]),
                    ("BOTTOM".to_string(), bitvec![1]),
                ]
                .into_iter()
                .collect(),
            },
        },
    );

    let sysmon = edev.egrid.db.get_node("SYSMON");
    if !edev.egrid.node_index[sysmon].is_empty() {
        let tile = "SYSMON";
        let bel = "SYSMON";
        ctx.collect_enum(tile, bel, "MONITOR_MODE", &["TEST", "MONITOR", "ADC"]);
        for i in 0x40..0x70 {
            ctx.collect_bitvec(tile, bel, &format!("INIT_{i:02X}"), "");
        }
        for pin in [
            "DEN",
            "DWE",
            "SCANTESTENA",
            "SCANTESTENB",
            "SCANMEMWE",
            "ROMTESTENABLE",
            "RST",
            "SEA",
            "SEB",
        ] {
            ctx.collect_int_inv(&["INT"; 8], tile, bel, pin, false);
        }
        ctx.collect_inv(tile, bel, "CONVST");
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        for (attr, val) in [
            ("DCLK_DIVID_2", 0),
            ("LW_DIVID_2_4", 0),
            ("MCCLK_DIVID", 0xc8),
            ("OVER_TEMPERATURE", 0x31e),
            ("OVER_TEMPERATURE_OFF", 0),
            ("OVER_TEMPERATURE_DELAY", 0),
            ("BLOCK_ENABLE", 0x1e),
            ("DCLK_MISSING", 0x320),
            ("FEATURE_ENABLE", 0),
            ("PROM_DATA", 0),
        ] {
            ctx.collect_bitvec(tile, bel, attr, "");
            present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, attr), val, 0);
        }
        present.assert_empty();

        let mut diffs = vec![];
        let diff = ctx.state.get_diff(tile, bel, "CONVST", "INT_IMUX");
        assert_eq!(
            diff,
            ctx.state.get_diff(tile, bel, "CONVST_TEST", "INT_IMUX")
        );
        diffs.push(("INT_IMUX".to_string(), diff));
        let mut diff = ctx.state.get_diff(tile, bel, "CONVST", "INT_CLK");
        assert_eq!(
            diff,
            ctx.state.get_diff(tile, bel, "CONVST_TEST", "INT_CLK")
        );
        let item = ctx.item_int_inv(&["INT"; 8], tile, bel, "CONVST_INT_CLK");
        diff.apply_bit_diff(&item, false, true);
        diffs.push(("INT_CLK".to_string(), diff));
        for i in 0..16 {
            let diff = ctx.state.get_diff(tile, bel, "CONVST", format!("GIOB{i}"));
            assert_eq!(
                diff,
                ctx.state
                    .get_diff(tile, bel, "CONVST_TEST", format!("GIOB{i}"))
            );
            diffs.push((format!("GIOB{i}"), diff));
        }
        ctx.tiledb
            .insert(tile, bel, "MUX.CONVST", xlat_enum_ocd(diffs, OcdMode::Mux));
    }
}

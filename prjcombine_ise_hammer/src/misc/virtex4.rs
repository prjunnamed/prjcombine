use bitvec::vec::BitVec;
use prjcombine_hammer::Session;
use prjcombine_int::{db::BelId, grid::DieId};
use prjcombine_types::{TileItem, TileItemKind};
use prjcombine_virtex_bitstream::Reg;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{xlat_bitvec, xlat_enum_ocd, xlat_item_tile, CollectorCtx, Diff, OcdMode},
    fgen::{TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let node_kind = backend.egrid.db.get_node("CFG");
    let mut ctx = FuzzCtx {
        session,
        node_kind,
        bits: TileBits::Cfg,
        tile_name: "CFG",
        bel: BelId::from_idx(0),
        bel_name: "MISC",
    };
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
        "TDOPIN",
        "TMSPIN",
    ] {
        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }

    for i in 0..32 {
        ctx.bel = BelId::from_idx(i);
        ctx.bel_name = &*format!("BUFGCTRL{i}").leak();
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFGCTRL")]);
        for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
            fuzz_inv!(ctx, pin, [(mode "BUFGCTRL")]);
        }
        fuzz_enum!(ctx, "PRESELECT_I0", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "PRESELECT_I1", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "CREATE_EDGE", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "INIT_OUT", ["0", "1"], [(mode "BUFGCTRL")]);

        for (midx, mux, bus) in [(0, "I0MUX", "MUXBUS0"), (1, "I1MUX", "MUXBUS1")] {
            for val in ["CKINT0", "CKINT1"] {
                fuzz_one!(ctx, mux, val, [
                    (mutex "IxMUX", mux),
                    (mutex mux, val)
                ], [
                    (pip (pin val), (pin mux))
                ]);
            }
            let obel = BelId::from_idx(0);
            let mb_idx = 2 * (i % 16) + midx;
            let mb_out = &*format!("MUXBUS_O{mb_idx}").leak();
            fuzz_one!(ctx, mux, "MUXBUS", [
                (mutex "IxMUX", mux),
                (mutex mux, "MUXBUS"),
                (global_mutex "CLK_IOB_MUXBUS", "USE"),
                (related TileRelation::ClkIob(i >= 16),
                    (pip (bel_pin obel, "PAD_BUF0"), (bel_pin obel, mb_out)))
            ], [
                (pip (pin bus), (pin mux))
            ]);
            for j in 0..16 {
                let jj = if i < 16 { j } else { j + 16 };
                let obel = BelId::from_idx(jj);
                let val = &*format!("GFB{j}").leak();
                fuzz_one!(ctx, mux, val, [
                    (mutex "IxMUX", mux),
                    (mutex mux, val)
                ], [
                    (pip (bel_pin obel, "GFB"), (pin mux))
                ]);
            }
            for val in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
                let obel = BelId::from_idx(45 + i / 16);
                let obel_bufg = BelId::from_idx(i ^ 1);
                fuzz_one!(ctx, mux, val, [
                    (mutex "IxMUX", mux),
                    (mutex mux, val),
                    (global_mutex "BUFG_MGTCLK", "USE"),
                    (bel_mutex obel_bufg, "IxMUX", mux),
                    (bel_mutex obel_bufg, mux, val),
                    (pip (bel_pin obel, val), (bel_pin obel_bufg, mux))
                ], [
                    (pip (bel_pin obel, val), (pin mux))
                ]);
            }
        }
        fuzz_one!(ctx, "ENABLE", "1", [(mode "BUFGCTRL")], [(pin "O")]);
        fuzz_one!(ctx, "PIN_O_GFB", "1", [
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
        ctx.bel = BelId::from_idx(32 + i);
        ctx.bel_name = ["BSCAN0", "BSCAN1", "BSCAN2", "BSCAN3"][i];
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BSCAN")]);
    }
    ctx.bel_name = "BSCAN_COMMON";
    fuzz_multi!(ctx, "USERID", "", 32, [], (global_hex_prefix "USERID"));

    for i in 0..2 {
        ctx.bel = BelId::from_idx(36 + i);
        let obel = BelId::from_idx(36 + (i ^ 1));
        ctx.bel_name = ["ICAP0", "ICAP1"][i];
        fuzz_one!(ctx, "PRESENT", "1", [(bel_unused obel)], [(mode "ICAP")]);
        fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [(mode "ICAP"), (pin "CLK")]);
        fuzz_enum!(ctx, "CEINV", ["CE", "CE_B"], [(mode "ICAP"), (pin "CE")]);
        fuzz_enum!(ctx, "WRITEINV", ["WRITE", "WRITE_B"], [(mode "ICAP"), (pin "WRITE")]);
        fuzz_enum!(ctx, "ICAP_WIDTH", ["X8", "X32"], [(mode "ICAP"), (bel_unused obel)]);
    }

    ctx.bel = BelId::from_idx(38);
    ctx.bel_name = "PMV";
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PMV")]);

    ctx.bel = BelId::from_idx(39);
    ctx.bel_name = "STARTUP";
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
        let pin_b = &*format!("{pin}_B").leak();
        let pininv = &*format!("{pin}INV").leak();
        fuzz_enum!(ctx, pininv, [pin, pin_b], [(mode "STARTUP"), (pin pin)]);
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
    ctx.tile_name = "REG.COR";
    for val in ["CCLK", "USERCLK", "JTAGCLK"] {
        fuzz_one!(ctx, "STARTUPCLK", val, [(mode "STARTUP"), (pin "CLK")], [(global_opt "STARTUPCLK", val)]);
    }
    ctx.bits = TileBits::Cfg;
    ctx.tile_name = "CFG";

    ctx.bel = BelId::from_idx(40);
    ctx.bel_name = "JTAGPPC";
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "JTAGPPC")]);

    ctx.bel = BelId::from_idx(41);
    ctx.bel_name = "FRAME_ECC";
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "FRAME_ECC")]);

    ctx.bel = BelId::from_idx(42);
    ctx.bel_name = "DCIRESET";
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "DCIRESET")]);

    ctx.bel = BelId::from_idx(43);
    ctx.bel_name = "CAPTURE";
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CAPTURE")]);
    fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [(mode "CAPTURE"), (pin "CLK")]);
    fuzz_enum!(ctx, "CAPINV", ["CAP", "CAP_B"], [(mode "CAPTURE"), (pin "CAP")]);
    ctx.bits = TileBits::CfgReg(Reg::Cor0);
    fuzz_enum!(ctx, "ONESHOT", ["FALSE", "TRUE"], [(mode "CAPTURE")]);
    ctx.bits = TileBits::Cfg;

    ctx.bel = BelId::from_idx(44);
    ctx.bel_name = "USR_ACCESS";
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "USR_ACCESS")]);

    if edev.col_lgt.is_some() {
        for (bel, bel_name) in [
            (BelId::from_idx(47), "BUFG_MGTCLK_B"),
            (BelId::from_idx(48), "BUFG_MGTCLK_T"),
        ] {
            ctx.bel = bel;
            ctx.bel_name = bel_name;
            for (name, o, i) in [
                ("MGT_L0", "MGT_L0_O", "MGT_L0_I"),
                ("MGT_L1", "MGT_L1_O", "MGT_L1_I"),
                ("MGT_R0", "MGT_R0_O", "MGT_R0_I"),
                ("MGT_R1", "MGT_R1_O", "MGT_R1_I"),
            ] {
                fuzz_one!(ctx, name, "1", [
                    (global_mutex "BUFG_MGTCLK", "TEST")
                ], [
                    (pip (pin i), (pin o))
                ]);
            }
        }
        for (bel, bel_name, row) in [
            (
                BelId::from_idx(49),
                "BUFG_MGTCLK_SRC_B",
                edev.grids[edev.grid_master].row_bufg() - 8,
            ),
            (
                BelId::from_idx(50),
                "BUFG_MGTCLK_SRC_T",
                edev.grids[edev.grid_master].row_bufg() + 8,
            ),
        ] {
            ctx.bel = bel;
            ctx.bel_name = bel_name;
            for (name, o, i) in [
                ("MGT_L0", "MGT_L0_O", "MGT_L0_I"),
                ("MGT_L1", "MGT_L1_O", "MGT_L1_I"),
                ("MGT_R0", "MGT_R0_O", "MGT_R0_I"),
                ("MGT_R1", "MGT_R1_O", "MGT_R1_I"),
            ] {
                let mut btiles = vec![];
                let is_l = name.starts_with("MGT_L");
                for &col in &edev.grids[edev.grid_master].cols_vbrk {
                    if (col < edev.col_cfg) == is_l {
                        btiles.push(edev.btile_hclk(
                            DieId::from_idx(0),
                            if is_l { col } else { col - 1 },
                            row,
                        ));
                    }
                }
                ctx.bits = TileBits::Raw(btiles);
                fuzz_one!(ctx, name, "1", [], [(pip (pin i), (pin o))]);
            }
        }
        ctx.bits = TileBits::Cfg;
    }

    // TODO: global DCI enable

    ctx.bits = TileBits::Reg(Reg::Cor0);
    ctx.tile_name = "REG.COR";
    ctx.bel_name = "STARTUP";
    for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
        fuzz_one!(ctx, "GWE_CYCLE", val, [], [(global_opt "GWE_CYCLE", val)]);
        fuzz_one!(ctx, "GTS_CYCLE", val, [], [(global_opt "GTS_CYCLE", val)]);
    }
    for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
        fuzz_one!(ctx, "DONE_CYCLE", val, [], [(global_opt "DONE_CYCLE", val)]);
    }
    for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
        fuzz_one!(ctx, "LCK_CYCLE", val, [], [(global_opt "LCK_CYCLE", val)]);
        fuzz_one!(ctx, "MATCH_CYCLE", val, [(global_mutex "DCI", "NO")], [(global_opt "MATCH_CYCLE", val)]);
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

    ctx.bits = TileBits::Reg(Reg::Ctl0);
    ctx.tile_name = "REG.CTL";
    ctx.bel_name = "MISC";
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "GTS_USR_B", val, [], [(global_opt "GTS_USR_B", val)]);
        fuzz_one!(ctx, "VGG_TEST", val, [], [(global_opt "VGG_TEST", val)]);
        fuzz_one!(ctx, "EN_VTEST", val, [], [(global_opt "EN_VTEST", val)]);
    }
    // persist not fuzzed — too much effort
    // decrypt not fuzzed — too much effort
    for val in ["NONE", "LEVEL1", "LEVEL2"] {
        fuzz_one!(ctx, "SECURITY", val, [], [(global_opt "SECURITY", val)]);
    }

    // TODO: more crap
    ctx.bits = TileBits::Null;
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "DISABLE_BANDGAP", val, [], [(global_opt "DISABLEBANDGAP", val)]);
    }
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "DCI_SHUTDOWN", val, [], [(global_opt "DCISHUTDOWN", val)]);
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
        "TDOPIN",
        "TMSPIN",
    ] {
        ctx.collect_enum(tile, bel, attr, &["PULLUP", "PULLDOWN", "PULLNONE"]);
    }

    for i in 0..32 {
        let bel = &*format!("BUFGCTRL{i}").leak();
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
            ctx.collect_inv(tile, bel, pin);
        }
        ctx.collect_enum_bool(tile, bel, "PRESELECT_I0", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "PRESELECT_I1", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CREATE_EDGE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

        let (_, _, ien_diff) = Diff::split(
            ctx.state.peek_diff(tile, bel, "I0MUX", "CKINT0").clone(),
            ctx.state.peek_diff(tile, bel, "I1MUX", "CKINT0").clone(),
        );
        let ien_item = xlat_bitvec(vec![ien_diff]);
        for mux in ["I0MUX", "I1MUX"] {
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
            ctx.tiledb.insert(
                "CLK_TERM_B",
                "CLK_TERM",
                "GCLK_ENABLE",
                xlat_bitvec(vec![diff_b]),
            );
            ctx.tiledb.insert(
                "CLK_TERM_T",
                "CLK_TERM",
                "GCLK_ENABLE",
                xlat_bitvec(vec![diff_t]),
            );
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
            for attr in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
                ctx.collect_bit(tile, bel, attr, "1");
            }
        }
        for bel in ["BUFG_MGTCLK_SRC_B", "BUFG_MGTCLK_SRC_T"] {
            for attr in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
                let mut ctr = 0;
                let is_l = attr.starts_with("MGT_L");
                for &col in &edev.grids[edev.grid_master].cols_vbrk {
                    if (col < edev.col_cfg) == is_l {
                        ctr += 1;
                    }
                }
                if ctr != 0 {
                    let diff = ctx.state.get_diff(tile, bel, attr, "1");
                    for i in 0..ctr {
                        let sub_diff = diff.filter_tiles(&[i]);
                        let sub_attr = if attr.ends_with('0') { "MGT0" } else { "MGT1" };
                        ctx.tiledb.insert(
                            "HCLK_MGT_REPEATER",
                            "CLK_MGT_REPEATER",
                            sub_attr,
                            xlat_bitvec(vec![sub_diff]),
                        );
                    }
                }
            }
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
    ctx.collect_enum(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
    // these are too much trouble to deal with the normal way.
    ctx.tiledb.insert(
        tile,
        bel,
        "PERSIST",
        TileItem {
            bits: vec![FeatureBit {
                tile: 0,
                frame: 0,
                bit: 3,
            }],
            kind: TileItemKind::BitVec {
                invert: BitVec::from_iter([false]),
            },
        },
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "DECRYPT",
        TileItem {
            bits: vec![FeatureBit {
                tile: 0,
                frame: 0,
                bit: 6,
            }],
            kind: TileItemKind::BitVec {
                invert: BitVec::from_iter([false]),
            },
        },
    );
}

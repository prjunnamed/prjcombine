use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_virtex2::grid::GridKind;
use prjcombine_virtex_bitstream::Reg;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{
        concat_bitvec, xlat_bitvec, xlat_bool, xlat_bool_default, xlat_item_tile, CollectorCtx,
        Diff,
    },
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };

    let (ll, ul, lr, ur) = match edev.grid.kind {
        prjcombine_virtex2::grid::GridKind::Virtex2 => ("LL", "UL", "LR", "UR.V2"),
        prjcombine_virtex2::grid::GridKind::Virtex2P
        | prjcombine_virtex2::grid::GridKind::Virtex2PX => ("LL", "UL", "LR", "UR.V2P"),
        prjcombine_virtex2::grid::GridKind::Spartan3 => ("LL.S3", "UL.S3", "LR.S3", "UR.S3"),
        prjcombine_virtex2::grid::GridKind::Spartan3E => ("LL.S3E", "UL.S3E", "LR.S3E", "UR.S3E"),
        prjcombine_virtex2::grid::GridKind::Spartan3A
        | prjcombine_virtex2::grid::GridKind::Spartan3ADsp => {
            ("LL.S3A", "UL.S3A", "LR.S3A", "UR.S3A")
        }
    };

    fn fuzz_global(ctx: &mut FuzzCtx, name: &'static str, vals: &'static [&'static str]) {
        for val in vals {
            fuzz_one!(ctx, name, val, [], [(global_opt name, val)]);
        }
    }
    fn fuzz_pull(ctx: &mut FuzzCtx, name: &'static str) {
        fuzz_global(ctx, name, &["PULLNONE", "PULLDOWN", "PULLUP"]);
    }

    if edev.grid.kind == GridKind::Spartan3 {
        for tile in [ll, ul, lr, ur] {
            let node_kind = backend.egrid.db.get_node(tile);
            for i in 0..2 {
                let ctx = FuzzCtx {
                    session,
                    node_kind,
                    bits: TileBits::Corner,
                    tile_name: tile,
                    bel: BelId::from_idx(2 + i),
                    bel_name: ["DCIRESET0", "DCIRESET1"][i],
                };
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "DCIRESET")]);
            }
        }
    }

    // LL
    let node_kind = backend.egrid.db.get_node(ll);
    let mut ctx = FuzzCtx {
        session,
        node_kind,
        bits: TileBits::Corner,
        tile_name: ll,
        bel: BelId::from_idx(0),
        bel_name: "MISC",
    };
    if edev.grid.kind.is_virtex2() {
        fuzz_global(&mut ctx, "DISABLEBANDGAP", &["YES", "NO"]);
        fuzz_global(&mut ctx, "RAISEVGG", &["YES", "NO"]);
        fuzz_global(&mut ctx, "IBCLK_N2", &["1", "0"]);
        fuzz_global(&mut ctx, "IBCLK_N4", &["1", "0"]);
        fuzz_global(&mut ctx, "IBCLK_N8", &["1", "0"]);
        fuzz_global(&mut ctx, "IBCLK_N16", &["1", "0"]);
        fuzz_global(&mut ctx, "IBCLK_N32", &["1", "0"]);
        fuzz_global(&mut ctx, "ZCLK_N2", &["1", "0"]);
        fuzz_global(&mut ctx, "ZCLK_N4", &["1", "0"]);
        fuzz_global(&mut ctx, "ZCLK_N8", &["1", "0"]);
        fuzz_global(&mut ctx, "ZCLK_N16", &["1", "0"]);
        fuzz_global(&mut ctx, "ZCLK_N32", &["1", "0"]);
        if edev.grid.kind.is_virtex2p() {
            fuzz_global(&mut ctx, "DISABLEVGGGENERATION", &["YES", "NO"]);
        }
    } else {
        if edev.grid.kind.is_spartan3a() {
            ctx.bits = TileBits::CornerReg(Reg::Cor1);
        }
        fuzz_global(&mut ctx, "SEND_VGG0", &["1", "0"]);
        fuzz_global(&mut ctx, "SEND_VGG1", &["1", "0"]);
        fuzz_global(&mut ctx, "SEND_VGG2", &["1", "0"]);
        fuzz_global(&mut ctx, "SEND_VGG3", &["1", "0"]);
        fuzz_global(&mut ctx, "VGG_SENDMAX", &["YES", "NO"]);
        fuzz_global(&mut ctx, "VGG_ENABLE_OFFCHIP", &["YES", "NO"]);
        ctx.bits = TileBits::Corner;
    }
    if edev.grid.kind == GridKind::Spartan3 {
        fuzz_global(&mut ctx, "GATE_GHIGH", &["YES", "NO"]);
        fuzz_global(&mut ctx, "IDCI_OSC_SEL0", &["1", "0"]);
        fuzz_global(&mut ctx, "IDCI_OSC_SEL1", &["1", "0"]);
        fuzz_global(&mut ctx, "IDCI_OSC_SEL2", &["1", "0"]);
    }
    if edev.grid.kind.is_spartan3ea() {
        fuzz_global(
            &mut ctx,
            "TEMPSENSOR",
            &["NONE", "PGATE", "CGATE", "BG", "THERM"],
        );
    }
    if edev.grid.kind.is_spartan3a() {
        fuzz_pull(&mut ctx, "CCLK2PIN");
        fuzz_pull(&mut ctx, "MOSI2PIN");
    } else if edev.grid.kind != GridKind::Spartan3E {
        fuzz_pull(&mut ctx, "M0PIN");
        fuzz_pull(&mut ctx, "M1PIN");
        fuzz_pull(&mut ctx, "M2PIN");
    }

    // UL
    let node_kind = backend.egrid.db.get_node(ul);
    let mut ctx = FuzzCtx {
        session,
        node_kind,
        bits: TileBits::Corner,
        tile_name: ul,
        bel: BelId::from_idx(0),
        bel_name: "MISC",
    };
    fuzz_global(&mut ctx, "PROGPIN", &["PULLUP", "PULLNONE"]);
    fuzz_pull(&mut ctx, "TDIPIN");
    if edev.grid.kind.is_spartan3a() {
        fuzz_pull(&mut ctx, "TMSPIN");
    }
    if !edev.grid.kind.is_spartan3ea() {
        fuzz_pull(&mut ctx, "HSWAPENPIN");
    }
    if !edev.grid.kind.is_virtex2() {
        fuzz_global(&mut ctx, "TESTLL", &["NO", "YES"]);
    }
    ctx.bel_name = "PMV";
    ctx.bel = BelId::from_idx(if edev.grid.kind.is_virtex2() {
        2
    } else if !edev.grid.kind.is_spartan3ea() {
        4
    } else {
        0
    });
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PMV")]);
    if edev.grid.kind.is_spartan3a() {
        ctx.bel_name = "DNA_PORT";
        ctx.bel = BelId::from_idx(ctx.bel.to_idx() + 1);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "DNA_PORT")]);
    }

    // LR
    let node_kind = backend.egrid.db.get_node(lr);
    let mut ctx = FuzzCtx {
        session,
        node_kind,
        bits: TileBits::Corner,
        tile_name: lr,
        bel: BelId::from_idx(0),
        bel_name: "MISC",
    };
    fuzz_global(&mut ctx, "DONEPIN", &["PULLUP", "PULLNONE"]);
    if !edev.grid.kind.is_spartan3a() {
        fuzz_global(&mut ctx, "CCLKPIN", &["PULLUP", "PULLNONE"]);
    }
    if edev.grid.kind.is_virtex2() {
        fuzz_global(&mut ctx, "POWERDOWNPIN", &["PULLUP", "PULLNONE"]);
    }
    ctx.bel_name = "STARTUP";
    ctx.bel = BelId::from_idx(if edev.grid.kind.is_virtex2() {
        2
    } else if !edev.grid.kind.is_spartan3ea() {
        4
    } else {
        0
    });
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "STARTUP")]);
    fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [(mode "STARTUP"), (global_opt "STARTUPCLK", "JTAGCLK"), (pin "CLK")]);
    fuzz_enum!(ctx, "GTSINV", ["GTS", "GTS_B"], [(mode "STARTUP"), (pin "GTS"), (nopin "GSR")]);
    fuzz_enum!(ctx, "GSRINV", ["GSR", "GSR_B"], [(mode "STARTUP"), (pin "GSR"), (nopin "GTS")]);
    for attr in ["GTS_SYNC", "GSR_SYNC", "GWE_SYNC"] {
        if !edev.grid.kind.is_virtex2() && attr == "GWE_SYNC" {
            continue;
        }
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, attr, val, [(mode "STARTUP")], [(global_opt attr, val)]);
        }
    }
    ctx.bel_name = "CAPTURE";
    ctx.bel = BelId::from_idx(ctx.bel.to_idx() + 1);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CAPTURE")]);
    fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [(mode "CAPTURE"), (pin "CLK")]);
    fuzz_enum!(ctx, "CAPINV", ["CAP", "CAP_B"], [(mode "CAPTURE"), (pin "CAP")]);
    if edev.grid.kind.is_spartan3a() {
        ctx.bits = TileBits::CornerReg(Reg::Cor2);
        fuzz_enum!(ctx, "ONESHOT", ["FALSE", "TRUE"], [(mode "CAPTURE")]);
    } else {
        ctx.bits = TileBits::CornerReg(Reg::Cor0);
        fuzz_enum!(ctx, "ONESHOT_ATTR", ["ONE_SHOT"], [(mode "CAPTURE")]);
    }
    ctx.bits = TileBits::Corner;
    ctx.bel_name = "ICAP";
    ctx.bel = BelId::from_idx(ctx.bel.to_idx() + 1);
    if edev.grid.kind.is_spartan3a() {
        ctx.bits = TileBits::CornerReg(Reg::Ctl0);
    }
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "ICAP")]);
    fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [(mode "ICAP"), (pin "CLK")]);
    fuzz_enum!(ctx, "CEINV", ["CE", "CE_B"], [(mode "ICAP"), (pin "CE")]);
    fuzz_enum!(ctx, "WRITEINV", ["WRITE", "WRITE_B"], [(mode "ICAP"), (pin "WRITE")]);
    ctx.bits = TileBits::Corner;
    if edev.grid.kind.is_spartan3a() {
        ctx.bel_name = "SPI_ACCESS";
        ctx.bel = BelId::from_idx(ctx.bel.to_idx() + 1);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "SPI_ACCESS")]);
    }

    // UR
    let node_kind = backend.egrid.db.get_node(ur);
    let mut ctx = FuzzCtx {
        session,
        node_kind,
        bits: TileBits::Corner,
        tile_name: ur,
        bel: BelId::from_idx(0),
        bel_name: "MISC",
    };
    fuzz_pull(&mut ctx, "TCKPIN");
    fuzz_pull(&mut ctx, "TDOPIN");
    if !edev.grid.kind.is_spartan3a() {
        fuzz_pull(&mut ctx, "TMSPIN");
    } else {
        fuzz_pull(&mut ctx, "MISO2PIN");
        fuzz_pull(&mut ctx, "CSO2PIN");
    }
    ctx.bel_name = "BSCAN";
    ctx.bel = BelId::from_idx(if edev.grid.kind.is_virtex2() {
        2
    } else if !edev.grid.kind.is_spartan3ea() {
        4
    } else {
        0
    });
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BSCAN")]);
    fuzz_multi!(ctx, "USERID", "", 32, [], (global_hex_prefix "USERID"));
    fuzz_one!(ctx, "TDO1", "1", [(mode "BSCAN"), (nopin "TDO2")], [(pin_full "TDO1")]);
    fuzz_one!(ctx, "TDO2", "1", [(mode "BSCAN"), (nopin "TDO1")], [(pin_full "TDO2")]);
    if edev.grid.kind.is_virtex2p() {
        ctx.bel_name = "JTAGPPC";
        ctx.bel = BelId::from_idx(3);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "JTAGPPC")]);
    }

    // config regs
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let (int_tiles, cnr_tidx, int_tidx, reg_tidx) = if edev.grid.kind.is_virtex2() {
        (&["INT.CNR"], &[0, 1][..], 2, 3)
    } else {
        (&["INT.CLB"], &[0][..], 1, 2)
    };
    let int_tidx = &[int_tidx][..];
    let reg_tidx = &[reg_tidx][..];

    let (ll, ul, lr, ur) = match edev.grid.kind {
        prjcombine_virtex2::grid::GridKind::Virtex2 => ("LL", "UL", "LR", "UR.V2"),
        prjcombine_virtex2::grid::GridKind::Virtex2P
        | prjcombine_virtex2::grid::GridKind::Virtex2PX => ("LL", "UL", "LR", "UR.V2P"),
        prjcombine_virtex2::grid::GridKind::Spartan3 => ("LL.S3", "UL.S3", "LR.S3", "UR.S3"),
        prjcombine_virtex2::grid::GridKind::Spartan3E => ("LL.S3E", "UL.S3E", "LR.S3E", "UR.S3E"),
        prjcombine_virtex2::grid::GridKind::Spartan3A
        | prjcombine_virtex2::grid::GridKind::Spartan3ADsp => {
            ("LL.S3A", "UL.S3A", "LR.S3A", "UR.S3A")
        }
    };

    let get_split_diff = |ctx: &mut CollectorCtx, tile, bel, attr, val| {
        let diff = ctx.state.get_diff(tile, bel, attr, val);
        let mut diffs = diff.split_tiles(&[cnr_tidx, reg_tidx]);
        let diff_reg = diffs.pop().unwrap();
        let diff_cnr = diffs.pop().unwrap();
        (diff_cnr, diff_reg)
    };
    let get_split_bool = |ctx: &mut CollectorCtx, tile, bel, attr, val0, val1| {
        let (d0_cnr, d0_reg) = get_split_diff(ctx, tile, bel, attr, val0);
        let (d1_cnr, d1_reg) = get_split_diff(ctx, tile, bel, attr, val1);
        let (item_cnr, def_cnr) = xlat_bool_default(d0_cnr, d1_cnr);
        let (item_reg, def_reg) = xlat_bool_default(d0_reg, d1_reg);
        assert_eq!(def_cnr, def_reg);
        (item_cnr, item_reg, def_cnr)
    };

    if edev.grid.kind == GridKind::Spartan3 {
        for tile in [ll, ul, lr, ur] {
            for bel in ["DCIRESET0", "DCIRESET1"] {
                let diff = ctx.state.get_diff(tile, bel, "PRESENT", "1");
                ctx.tiledb
                    .insert(tile, bel, "ENABLE", xlat_bitvec(vec![diff]));
            }
        }
    }

    // LL
    let tile = ll;
    let bel = "MISC";
    if edev.grid.kind.is_virtex2() {
        ctx.collect_enum_bool(tile, bel, "DISABLEBANDGAP", "NO", "YES");
        ctx.collect_enum_bool_wide(tile, bel, "RAISEVGG", "NO", "YES");
        ctx.tiledb.insert(
            tile,
            bel,
            "ZCLK_DIV2",
            xlat_bitvec(vec![
                ctx.state.get_diff(tile, bel, "ZCLK_N2", "1"),
                ctx.state.get_diff(tile, bel, "ZCLK_N4", "1"),
                ctx.state.get_diff(tile, bel, "ZCLK_N8", "1"),
                ctx.state.get_diff(tile, bel, "ZCLK_N16", "1"),
                ctx.state.get_diff(tile, bel, "ZCLK_N32", "1"),
            ]),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "BCLK_DIV2",
            xlat_bitvec(vec![
                ctx.state.get_diff(tile, bel, "IBCLK_N2", "1"),
                ctx.state.get_diff(tile, bel, "IBCLK_N4", "1"),
                ctx.state.get_diff(tile, bel, "IBCLK_N8", "1"),
                ctx.state.get_diff(tile, bel, "IBCLK_N16", "1"),
                ctx.state.get_diff(tile, bel, "IBCLK_N32", "1"),
            ]),
        );
        for attr in [
            "ZCLK_N2",
            "ZCLK_N4",
            "ZCLK_N8",
            "ZCLK_N16",
            "ZCLK_N32",
            "IBCLK_N2",
            "IBCLK_N4",
            "IBCLK_N8",
            "IBCLK_N16",
            "IBCLK_N32",
        ] {
            ctx.state.get_diff(tile, bel, attr, "0").assert_empty();
        }
        if edev.grid.kind.is_virtex2p() {
            ctx.collect_enum_bool(tile, bel, "DISABLEVGGGENERATION", "NO", "YES");
        }
    } else {
        if !edev.grid.kind.is_spartan3a() {
            let sendmax = ctx.collect_enum_bool_default(tile, bel, "VGG_SENDMAX", "NO", "YES");
            ctx.tiledb
                .insert_device_data(&ctx.device.name, "MISC:VGG_SENDMAX_DEFAULT", [sendmax]);
            assert!(!ctx.collect_enum_bool_default(tile, bel, "VGG_ENABLE_OFFCHIP", "NO", "YES"));
            let (item0, vgg0) = ctx.extract_enum_bool_default(tile, bel, "SEND_VGG0", "0", "1");
            let (item1, vgg1) = ctx.extract_enum_bool_default(tile, bel, "SEND_VGG1", "0", "1");
            let (item2, vgg2) = ctx.extract_enum_bool_default(tile, bel, "SEND_VGG2", "0", "1");
            let (item3, vgg3) = ctx.extract_enum_bool_default(tile, bel, "SEND_VGG3", "0", "1");
            ctx.tiledb.insert_device_data(
                &ctx.device.name,
                "MISC:SEND_VGG_DEFAULT",
                [vgg0, vgg1, vgg2, vgg3],
            );
            let item = concat_bitvec([item0, item1, item2, item3]);
            ctx.tiledb.insert(tile, bel, "SEND_VGG", item);
        } else {
            ctx.state
                .get_diff(tile, bel, "VGG_ENABLE_OFFCHIP", "NO")
                .assert_empty();
            let (diff_cnr, diff_reg) = get_split_diff(ctx, tile, bel, "VGG_ENABLE_OFFCHIP", "YES");
            ctx.tiledb
                .insert(tile, bel, "VGG_ENABLE_OFFCHIP", xlat_bitvec(vec![diff_cnr]));
            ctx.tiledb.insert(
                "COR1.S3A",
                bel,
                "VGG_ENABLE_OFFCHIP",
                xlat_bitvec(vec![diff_reg]),
            );

            let (item_cnr, item_reg, def) =
                get_split_bool(ctx, tile, bel, "VGG_SENDMAX", "NO", "YES");
            ctx.tiledb.insert(tile, bel, "VGG_SENDMAX", item_cnr);
            ctx.tiledb.insert("COR1.S3A", bel, "VGG_SENDMAX", item_reg);
            ctx.tiledb
                .insert_device_data(&ctx.device.name, "MISC:VGG_SENDMAX_DEFAULT", [def]);
            let (i0_cnr, i0_reg, vgg0) = get_split_bool(ctx, tile, bel, "SEND_VGG0", "0", "1");
            let (i1_cnr, i1_reg, vgg1) = get_split_bool(ctx, tile, bel, "SEND_VGG1", "0", "1");
            let (i2_cnr, i2_reg, vgg2) = get_split_bool(ctx, tile, bel, "SEND_VGG2", "0", "1");
            let (i3_cnr, i3_reg, vgg3) = get_split_bool(ctx, tile, bel, "SEND_VGG3", "0", "1");
            ctx.tiledb.insert_device_data(
                &ctx.device.name,
                "MISC:SEND_VGG_DEFAULT",
                [vgg0, vgg1, vgg2, vgg3],
            );
            let item = concat_bitvec([i0_cnr, i1_cnr, i2_cnr, i3_cnr]);
            ctx.tiledb.insert(tile, bel, "SEND_VGG", item);
            let item = concat_bitvec([i0_reg, i1_reg, i2_reg, i3_reg]);
            ctx.tiledb.insert("COR1.S3A", bel, "SEND_VGG", item);
        }
    }
    if edev.grid.kind == GridKind::Spartan3 {
        ctx.tiledb.insert(
            tile,
            bel,
            "DCI_OSC_SEL",
            xlat_bitvec(vec![
                ctx.state.get_diff(tile, bel, "IDCI_OSC_SEL0", "1"),
                ctx.state.get_diff(tile, bel, "IDCI_OSC_SEL1", "1"),
                ctx.state.get_diff(tile, bel, "IDCI_OSC_SEL2", "1"),
            ]),
        );
        for attr in [
            "IDCI_OSC_SEL0",
            "IDCI_OSC_SEL1",
            "IDCI_OSC_SEL2",
        ] {
            ctx.state.get_diff(tile, bel, attr, "0").assert_empty();
        }
        ctx.collect_enum_bool(tile, bel, "GATE_GHIGH", "NO", "YES");
    }
    if edev.grid.kind.is_spartan3ea() {
        ctx.collect_enum(
            tile,
            bel,
            "TEMPSENSOR",
            &["NONE", "PGATE", "CGATE", "BG", "THERM"],
        );
    }
    if edev.grid.kind.is_spartan3a() {
        ctx.collect_enum(tile, bel, "CCLK2PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "MOSI2PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    } else if edev.grid.kind != GridKind::Spartan3E {
        ctx.collect_enum(tile, bel, "M0PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "M1PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "M2PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    }

    // UL
    let tile = ul;
    let bel = "MISC";
    ctx.collect_enum(tile, bel, "PROGPIN", &["PULLUP", "PULLNONE"]);
    ctx.collect_enum(tile, bel, "TDIPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    if edev.grid.kind.is_spartan3a() {
        ctx.collect_enum(tile, bel, "TMSPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    }
    if !edev.grid.kind.is_spartan3ea() {
        ctx.collect_enum(tile, bel, "HSWAPENPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    }
    if !edev.grid.kind.is_virtex2() {
        ctx.collect_enum_bool(tile, bel, "TESTLL", "NO", "YES");
    }

    ctx.state
        .get_diff(tile, "PMV", "PRESENT", "1")
        .assert_empty();
    if edev.grid.kind.is_spartan3a() {
        ctx.state
            .get_diff(tile, "DNA_PORT", "PRESENT", "1")
            .assert_empty();
    }

    // LR
    let tile = lr;
    let bel = "MISC";
    ctx.collect_enum(tile, bel, "DONEPIN", &["PULLUP", "PULLNONE"]);
    if !edev.grid.kind.is_spartan3a() {
        ctx.collect_enum(tile, bel, "CCLKPIN", &["PULLUP", "PULLNONE"]);
    }
    if edev.grid.kind.is_virtex2() {
        ctx.collect_enum(tile, bel, "POWERDOWNPIN", &["PULLUP", "PULLNONE"]);
    }
    let bel = "STARTUP";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    let item = ctx.extract_enum_bool(tile, bel, "CLKINV", "CLK", "CLK_B");
    ctx.insert_int_inv(int_tiles, tile, bel, "CLK", xlat_item_tile(item, int_tidx));
    let d0 = ctx.state.get_diff(tile, bel, "GTSINV", "GTS");
    let d1 = ctx.state.get_diff(tile, bel, "GTSINV", "GTS_B");
    let (d0, d1, dc_gts) = Diff::split(d0, d1);
    let item = if edev.grid.kind.is_virtex2() {
        // caution: invert
        xlat_bool(d1, d0)
    } else {
        xlat_bool(d0, d1)
    };
    ctx.insert_int_inv(int_tiles, tile, bel, "GTS", xlat_item_tile(item, int_tidx));
    let d0 = ctx.state.get_diff(tile, bel, "GSRINV", "GSR");
    let d1 = ctx.state.get_diff(tile, bel, "GSRINV", "GSR_B");
    let (d0, d1, dc_gsr) = Diff::split(d0, d1);
    let item = if edev.grid.kind.is_virtex2() {
        // caution: invert
        xlat_bool(d1, d0)
    } else {
        xlat_bool(d0, d1)
    };
    ctx.insert_int_inv(int_tiles, tile, bel, "GSR", xlat_item_tile(item, int_tidx));
    assert_eq!(dc_gts, dc_gsr);
    ctx.tiledb
        .insert(tile, bel, "GTS_GSR_ENABLE", xlat_bitvec(vec![dc_gts]));
    ctx.collect_enum_bool(tile, bel, "GTS_SYNC", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "GSR_SYNC", "NO", "YES");
    if edev.grid.kind.is_virtex2() {
        ctx.collect_enum_bool(tile, bel, "GWE_SYNC", "NO", "YES");
    }
    let bel = "CAPTURE";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    let item = ctx.extract_enum_bool(tile, bel, "CLKINV", "CLK", "CLK_B");
    ctx.insert_int_inv(int_tiles, tile, bel, "CLK", xlat_item_tile(item, int_tidx));
    let item = if edev.grid.kind.is_virtex2() {
        // caution: inverted
        ctx.extract_enum_bool(tile, bel, "CAPINV", "CAP_B", "CAP")
    } else {
        ctx.extract_enum_bool(tile, bel, "CAPINV", "CAP", "CAP_B")
    };
    ctx.insert_int_inv(int_tiles, tile, bel, "CAP", xlat_item_tile(item, int_tidx));
    if edev.grid.kind.is_spartan3a() {
        let item = ctx.extract_enum_bool(tile, bel, "ONESHOT", "FALSE", "TRUE");
        ctx.tiledb
            .insert("COR2.S3A", bel, "ONESHOT", xlat_item_tile(item, reg_tidx));
    } else {
        let diff = ctx.state.get_diff(tile, bel, "ONESHOT_ATTR", "ONE_SHOT");
        let item = xlat_bitvec(vec![diff]);
        let reg = if edev.grid.kind.is_virtex2() {
            "COR"
        } else if edev.grid.kind == GridKind::Spartan3 {
            "COR.S3"
        } else {
            "COR.S3E"
        };
        ctx.tiledb
            .insert(reg, bel, "ONESHOT", xlat_item_tile(item, reg_tidx));
    }
    let bel = "ICAP";
    if edev.grid.kind != GridKind::Spartan3E {
        let item = ctx.extract_enum_bool(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.insert_int_inv(int_tiles, tile, bel, "CLK", xlat_item_tile(item, int_tidx));
        let item = if edev.grid.kind.is_virtex2() {
            ctx.extract_enum_bool(tile, bel, "CEINV", "CE", "CE_B")
        } else {
            // caution: inverted
            ctx.extract_enum_bool(tile, bel, "CEINV", "CE_B", "CE")
        };
        ctx.insert_int_inv(int_tiles, tile, bel, "CE", xlat_item_tile(item, int_tidx));
        let item = if edev.grid.kind.is_virtex2() {
            ctx.extract_enum_bool(tile, bel, "WRITEINV", "WRITE", "WRITE_B")
        } else {
            // caution: inverted
            ctx.extract_enum_bool(tile, bel, "WRITEINV", "WRITE_B", "WRITE")
        };
        ctx.insert_int_inv(
            int_tiles,
            tile,
            bel,
            "WRITE",
            xlat_item_tile(item, int_tidx),
        );
        let diff = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        let item = xlat_bitvec(vec![diff]);
        if edev.grid.kind.is_spartan3a() {
            let item = xlat_item_tile(item, reg_tidx);
            ctx.tiledb.insert("CTL0.S3A", "ICAP", "ENABLE", item);
        } else {
            ctx.tiledb.insert(tile, bel, "ENABLE", item);
        }
    } else {
        ctx.state
            .get_diff(tile, bel, "CLKINV", "CLK")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "CLKINV", "CLK_B")
            .assert_empty();
        ctx.state.get_diff(tile, bel, "CEINV", "CE").assert_empty();
        ctx.state
            .get_diff(tile, bel, "CEINV", "CE_B")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "WRITEINV", "WRITE")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "WRITEINV", "WRITE_B")
            .assert_empty();
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    }
    if edev.grid.kind.is_spartan3a() {
        let bel = "SPI_ACCESS";
        let mut diffs = ctx
            .state
            .get_diff(tile, bel, "PRESENT", "1")
            .split_tiles(&[cnr_tidx, int_tidx]);
        let mut diff_int = diffs.pop().unwrap();
        let diff_cnr = diffs.pop().unwrap();
        diff_int.discard_bits(&ctx.item_int_inv(int_tiles, tile, bel, "MOSI"));
        diff_int.assert_empty();
        ctx.tiledb
            .insert(tile, bel, "ENABLE", xlat_bitvec(vec![diff_cnr]));
    }

    // UR
    let tile = ur;
    let bel = "MISC";
    ctx.collect_enum(tile, bel, "TCKPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    ctx.collect_enum(tile, bel, "TDOPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    if !edev.grid.kind.is_spartan3a() {
        ctx.collect_enum(tile, bel, "TMSPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    } else {
        ctx.collect_enum(tile, bel, "MISO2PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "CSO2PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    }
    let bel = "BSCAN";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    ctx.collect_bitvec(tile, bel, "USERID", "");
    let diff = ctx.state.get_diff(tile, bel, "TDO1", "1");
    assert_eq!(diff, ctx.state.get_diff(tile, bel, "TDO2", "1"));
    let mut bits: Vec<_> = diff.bits.into_iter().collect();
    bits.sort();
    ctx.tiledb.insert(
        tile,
        bel,
        "TDO_ENABLE",
        xlat_bitvec(
            bits.into_iter()
                .map(|(k, v)| Diff {
                    bits: [(k, v)].into_iter().collect(),
                })
                .collect(),
        ),
    );

    if edev.grid.kind.is_virtex2p() {
        let bel = "JTAGPPC";
        let diff = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        ctx.tiledb
            .insert(tile, bel, "ENABLE", xlat_bitvec(vec![diff]));
    }

    // config regs
}

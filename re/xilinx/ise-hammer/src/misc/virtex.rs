use prjcombine_re_collector::{xlat_bitvec, xlat_bool, xlat_enum_int, OcdMode};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::{Dir, NodeTileId};
use prjcombine_types::tiledb::{TileBit, TileItem};
use prjcombine_virtex::grid::GridKind;
use prjcombine_xilinx_bitstream::Reg;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileFuzzKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for tile in ["CLKL", "CLKR"] {
        let ctx = FuzzCtx::new(session, backend, tile, "PCILOGIC", TileBits::MainAuto);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PCILOGIC")]);
        fuzz_enum!(ctx, "I1MUX", ["0", "1", "I1", "I1_B"], [(mode "PCILOGIC"), (pin "I1")]);
        fuzz_enum!(ctx, "I2MUX", ["0", "1", "I2", "I2_B"], [(mode "PCILOGIC"), (pin "I2")]);
    }
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    for val in ["00", "01", "10", "11"] {
        fuzz_one_extras!(ctx, "PCIDELAY", val, [], [(global_opt "PCIDELAY", val)], vec![
            ExtraFeature::new(ExtraFeatureKind::Pcilogic(Dir::W), "CLKL", "PCILOGIC", "PCI_DELAY", val),
            ExtraFeature::new(ExtraFeatureKind::Pcilogic(Dir::E), "CLKR", "PCILOGIC", "PCI_DELAY", val),
        ]);
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BL", "MISC", TileBits::MainAuto);
    for attr in ["M0PIN", "M1PIN", "M2PIN"] {
        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }
    for attr in ["POWERDOWNPIN", "PDSTATUSPIN"] {
        for val in ["PULLUP", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "DRIVE_PD_STATUS", val, [], [(global_opt "DRIVEPDSTATUS", val)]);
    }
    for val in ["100US", "200US", "400US"] {
        fuzz_one!(ctx, "POWERUP_DELAY", val, [], [(global_opt "POWERUPDELAY", val)]);
    }

    let ctx = FuzzCtx::new(session, backend, "CNR.BL", "CAPTURE", TileBits::MainAuto);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CAPTURE")]);
    fuzz_enum!(ctx, "CLKINV", ["0", "1"], [(mode "CAPTURE"), (pin "CLK")]);
    fuzz_enum!(ctx, "CAPMUX", ["0", "1", "CAP", "CAP_B"], [(mode "CAPTURE"), (pin "CAP")]);
    fuzz_one_extras!(ctx, "ONESHOT", "1", [(mode "CAPTURE")], [
        (attr "ONESHOT_ATTR", "ONE_SHOT")
    ], vec![
        ExtraFeature::new(
            ExtraFeatureKind::Reg(Reg::Cor0),
            "REG.COR",
            "CAPTURE",
            "ONESHOT",
            "1"
        )
    ]);

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.TL", "MISC", TileBits::MainAuto);
    for attr in ["TMSPIN", "TCKPIN"] {
        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }
    for val in ["INTOSC", "USERCLK", "CCLK"] {
        fuzz_one!(ctx, "POWERUP_CLK", val, [], [(global_opt "POWERUPCLK", val)]);
    }
    for attr in ["IBCLK_N2", "IBCLK_N4", "IBCLK_N8", "IBCLK_N16", "IBCLK_N32"] {
        for val in ["0", "1"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }

    let ctx = FuzzCtx::new(session, backend, "CNR.TL", "STARTUP", TileBits::MainAuto);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "STARTUP")]);
    fuzz_enum!(ctx, "CLKINV", ["0", "1"], [(mode "STARTUP"), (pin "CLK")]);
    fuzz_enum!(ctx, "GWEMUX", ["0", "1", "GWE", "GWE_B"], [(mode "STARTUP"), (pin "GWE")]);
    fuzz_enum!(ctx, "GTSMUX", ["0", "1", "GTS", "GTS_B"], [(mode "STARTUP"), (pin "GTS")]);
    fuzz_enum!(ctx, "GSRMUX", ["0", "1", "GSR", "GSR_B"], [(mode "STARTUP"), (pin "GSR")]);
    let wire_gwe = (
        NodeTileId::from_idx(0),
        backend.egrid.db.get_wire("IMUX.STARTUP.GWE"),
    );
    let wire_gts = (
        NodeTileId::from_idx(0),
        backend.egrid.db.get_wire("IMUX.STARTUP.GTS"),
    );
    let wire_gsr = (
        NodeTileId::from_idx(0),
        backend.egrid.db.get_wire("IMUX.STARTUP.GSR"),
    );
    fuzz_one!(ctx, "GSR", "1", [
        (mode "STARTUP"),
        (nopin "GTS"),
        (nopin "GWE")
    ], [
        (special TileFuzzKV::NodeMutexExclusive(wire_gwe)),
        (special TileFuzzKV::NodeMutexExclusive(wire_gts)),
        (special TileFuzzKV::NodeMutexExclusive(wire_gsr)),
        (pin "GSR"),
        (attr "GSRMUX", "GSR_B")
    ]);
    fuzz_one!(ctx, "GTS", "1", [
        (mode "STARTUP"),
        (nopin "GSR"),
        (nopin "GWE")
    ], [
        (special TileFuzzKV::NodeMutexExclusive(wire_gwe)),
        (special TileFuzzKV::NodeMutexExclusive(wire_gts)),
        (special TileFuzzKV::NodeMutexExclusive(wire_gsr)),
        (pin "GTS"),
        (attr "GTSMUX", "GTS_B")
    ]);
    fuzz_one!(ctx, "GWE", "1", [
        (mode "STARTUP"),
        (nopin "GTS"),
        (nopin "GSR")
    ], [
        (special TileFuzzKV::NodeMutexExclusive(wire_gwe)),
        (special TileFuzzKV::NodeMutexExclusive(wire_gts)),
        (special TileFuzzKV::NodeMutexExclusive(wire_gsr)),
        (pin "GWE"),
        (attr "GWEMUX", "GWE")
    ]);
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "GWE_SYNC", val, [], [(global_opt "GWE_SYNC", val)]);
        fuzz_one!(ctx, "GTS_SYNC", val, [], [(global_opt "GTS_SYNC", val)]);
        fuzz_one!(ctx, "GSR_SYNC", val, [], [(global_opt "GSR_SYNC", val)]);
    }
    for val in ["CCLK", "USERCLK", "JTAGCLK"] {
        fuzz_one_extras!(ctx, "STARTUPCLK", val, [
            (mode "STARTUP"),
            (pin "CLK")
        ], [
            (global_opt "STARTUPCLK", val)
        ], vec![
            ExtraFeature::new(ExtraFeatureKind::Reg(Reg::Cor0), "REG.COR", "STARTUP", "STARTUPCLK", val)
        ]);
    }

    let ctx = FuzzCtx::new(session, backend, "CNR.TL", "BSCAN", TileBits::MainAuto);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BSCAN")]);
    fuzz_enum!(ctx, "TDO1MUX", ["0", "1", "TDO1", "TDO1_B"], [(mode "BSCAN"), (pin "TDO1")]);
    fuzz_enum!(ctx, "TDO2MUX", ["0", "1", "TDO2", "TDO2_B"], [(mode "BSCAN"), (pin "TDO2")]);
    fuzz_multi!(ctx, "USERID", "", 32, [], (global_hex_prefix "USERID"));

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BR", "MISC", TileBits::MainAuto);
    for attr in ["DONEPIN", "PROGPIN"] {
        for val in ["PULLUP", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.TR", "MISC", TileBits::MainAuto);
    for attr in ["TDIPIN", "TDOPIN"] {
        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }
    for val in ["PULLUP", "PULLNONE"] {
        fuzz_one!(ctx, "CCLKPIN", val, [], [(global_opt "CCLKPIN", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.COR",
        "STARTUP",
        TileBits::Reg(Reg::Cor0),
    );
    for attr in ["GSR_CYCLE", "GWE_CYCLE", "GTS_CYCLE"] {
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }
    for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
        fuzz_one!(ctx, "DONE_CYCLE", val, [], [(global_opt "DONE_CYCLE", val)]);
    }
    for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
        fuzz_one!(ctx, "LCK_CYCLE", val, [], [(global_opt "LCK_CYCLE", val)]);
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

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.CTL",
        "MISC",
        TileBits::Reg(Reg::Ctl0),
    );
    // persist not fuzzed — too much effort
    for val in ["NONE", "LEVEL1", "LEVEL2"] {
        fuzz_one!(ctx, "SECURITY", val, [], [(global_opt "SECURITY", val)]);
    }
    for val in ["0", "1"] {
        for attr in ["DISPMP1", "DISPMP2"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    for tile in ["CLKL", "CLKR"] {
        let bel = "PCILOGIC";
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        for (pinmux, pin, pin_b) in [("I1MUX", "I1", "I1_B"), ("I2MUX", "I2", "I2_B")] {
            // this is different from other virtex muxes!
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "0"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let item = xlat_bool(d0, d1);
            present.discard_bits(&item);
            ctx.insert_int_inv(&[tile], tile, bel, pin, item);
        }
        present.assert_empty();
        if edev.grid.kind == GridKind::Virtex {
            let d0 = ctx.state.get_diff(tile, bel, "PCI_DELAY", "00");
            let d1 = ctx.state.get_diff(tile, bel, "PCI_DELAY", "01");
            let d2 = ctx.state.get_diff(tile, bel, "PCI_DELAY", "10");
            let d3 = ctx.state.get_diff(tile, bel, "PCI_DELAY", "11");
            // bug? bug.
            assert_eq!(d0, d1);
            ctx.tiledb.insert(
                tile,
                bel,
                "PCI_DELAY",
                xlat_enum_int(vec![(0, d0), (2, d2), (3, d3)]),
            );
        } else {
            for val in ["00", "01", "10", "11"] {
                ctx.state
                    .get_diff(tile, bel, "PCI_DELAY", val)
                    .assert_empty();
            }
        }
    }
    {
        let tile = "CNR.BL";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "M0PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "M1PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "M2PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        if edev.grid.kind == GridKind::Virtex && ctx.device.name.contains("2s") {
            ctx.collect_enum(tile, bel, "POWERDOWNPIN", &["PULLUP", "PULLNONE"]);
            ctx.collect_enum(tile, bel, "PDSTATUSPIN", &["PULLUP", "PULLNONE"]);
            ctx.collect_enum(tile, bel, "POWERUP_DELAY", &["100US", "200US", "400US"]);
            ctx.collect_enum_bool(tile, bel, "DRIVE_PD_STATUS", "NO", "YES");
        } else {
            for (attr, val) in [
                ("POWERDOWNPIN", "PULLUP"),
                ("POWERDOWNPIN", "PULLNONE"),
                ("PDSTATUSPIN", "PULLUP"),
                ("PDSTATUSPIN", "PULLNONE"),
                ("POWERUP_DELAY", "100US"),
                ("POWERUP_DELAY", "200US"),
                ("POWERUP_DELAY", "400US"),
                ("DRIVE_PD_STATUS", "YES"),
                ("DRIVE_PD_STATUS", "NO"),
            ] {
                ctx.state.get_diff(tile, bel, attr, val).assert_empty();
            }
        }

        let bel = "CAPTURE";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        let d0 = ctx.state.get_diff(tile, bel, "CAPMUX", "CAP");
        assert_eq!(d0, ctx.state.get_diff(tile, bel, "CAPMUX", "1"));
        let d1 = ctx.state.get_diff(tile, bel, "CAPMUX", "CAP_B");
        assert_eq!(d1, ctx.state.get_diff(tile, bel, "CAPMUX", "0"));
        let item = xlat_bool(d0, d1);
        ctx.insert_int_inv(&[tile], tile, bel, "CAP", item);
        let item = ctx.extract_enum_bool(tile, bel, "CLKINV", "1", "0");
        ctx.insert_int_inv(&[tile], tile, bel, "CLK", item);
        ctx.state.get_diff(tile, bel, "ONESHOT", "1").assert_empty();
    }
    {
        let tile = "CNR.TL";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "TMSPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "TCKPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        let item = xlat_bitvec(vec![
            !ctx.state.get_diff(tile, bel, "IBCLK_N2", "0"),
            !ctx.state.get_diff(tile, bel, "IBCLK_N4", "0"),
            !ctx.state.get_diff(tile, bel, "IBCLK_N8", "0"),
            !ctx.state.get_diff(tile, bel, "IBCLK_N16", "0"),
            !ctx.state.get_diff(tile, bel, "IBCLK_N32", "0"),
        ]);
        ctx.tiledb.insert(tile, bel, "BCLK_DIV2", item);
        for attr in ["IBCLK_N2", "IBCLK_N4", "IBCLK_N8", "IBCLK_N16", "IBCLK_N32"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
        if edev.grid.kind == GridKind::Virtex && ctx.device.name.contains("2s") {
            ctx.collect_enum(tile, bel, "POWERUP_CLK", &["USERCLK", "INTOSC", "CCLK"]);
        } else {
            for (attr, val) in [
                ("POWERUP_CLK", "USERCLK"),
                ("POWERUP_CLK", "INTOSC"),
                ("POWERUP_CLK", "CCLK"),
            ] {
                ctx.state.get_diff(tile, bel, attr, val).assert_empty();
            }
        }

        let bel = "STARTUP";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for attr in ["GWE_SYNC", "GSR_SYNC", "GTS_SYNC"] {
            ctx.collect_enum_bool(tile, bel, attr, "NO", "YES");
        }
        for (pinmux, pin, pin_b) in [
            ("GWEMUX", "GWE", "GWE_B"),
            ("GTSMUX", "GTS", "GTS_B"),
            ("GSRMUX", "GSR", "GSR_B"),
        ] {
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "0"));
            let item = xlat_bool(d0, d1);
            ctx.insert_int_inv(&[tile], tile, bel, pin, item);
        }
        let item = ctx.extract_enum_bool(tile, bel, "CLKINV", "1", "0");
        ctx.insert_int_inv(&[tile], tile, bel, "CLK", item);
        let item = ctx.extract_bit(tile, bel, "GSR", "1");
        ctx.tiledb.insert(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "GWE", "1");
        ctx.tiledb.insert(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "GTS", "1");
        ctx.tiledb.insert(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        for val in ["JTAGCLK", "CCLK", "USERCLK"] {
            ctx.state
                .get_diff(tile, bel, "STARTUPCLK", val)
                .assert_empty();
        }

        let bel = "BSCAN";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for (pinmux, pin, pin_b) in [("TDO1MUX", "TDO1", "TDO1_B"), ("TDO2MUX", "TDO2", "TDO2_B")] {
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "0"));
            let item = xlat_bool(d0, d1);
            ctx.insert_int_inv(&[tile], tile, bel, pin, item);
        }
        ctx.collect_bitvec(tile, bel, "USERID", "");
    }
    {
        let tile = "CNR.BR";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "DONEPIN", &["PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "PROGPIN", &["PULLUP", "PULLNONE"]);
    }
    {
        let tile = "CNR.TR";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "CCLKPIN", &["PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "TDIPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "TDOPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    }
    {
        let tile = "REG.COR";
        let bel = "STARTUP";
        ctx.collect_enum(
            tile,
            bel,
            "GSR_CYCLE",
            &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
        );
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
        ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
        ctx.collect_enum_ocd(
            tile,
            bel,
            "CONFIG_RATE",
            &[
                "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51",
                "55", "60", "130",
            ],
            OcdMode::BitOrder,
        );
        ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
        ctx.tiledb.insert(
            tile,
            bel,
            "SHUTDOWN",
            TileItem::from_bit(TileBit::new(0, 0, 15), false),
        );

        let bel = "CAPTURE";
        ctx.collect_bit(tile, bel, "ONESHOT", "1");

        let tile = "REG.CTL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "DISPMP1", "0", "1");
        ctx.collect_enum_bool(tile, bel, "DISPMP2", "0", "1");
        ctx.collect_enum(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
        // these are too much trouble to deal with the normal way.
        ctx.tiledb.insert(
            tile,
            bel,
            "PERSIST",
            TileItem::from_bit(TileBit::new(0, 0, 6), false),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "GTS_USR_B",
            TileItem::from_bit(TileBit::new(0, 0, 0), false),
        );
    }
}

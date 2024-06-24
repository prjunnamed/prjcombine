use std::collections::HashSet;

use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_int::{db::BelId, grid::DieId};
use prjcombine_types::{TileItem, TileItemKind};
use prjcombine_virtex2::{
    expanded::{IoDiffKind, IoPadKind},
    grid::GridKind,
};
use prjcombine_virtex_bitstream::{BitTile, Reg};
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, IseBackend, Key, SimpleFeatureId},
    diff::{
        concat_bitvec, xlat_bit_wide, xlat_bitvec, xlat_bool, xlat_bool_default, xlat_enum,
        xlat_enum_ocd, xlat_item_tile, CollectorCtx, Diff, OcdMode,
    },
    fgen::{get_bonded_ios_v2_pkg, TileBits, TileFuzzKV, TileFuzzerGen, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_one,
    io::virtex2::{get_iostds, DciKind, DiffKind},
};

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    skip_io: bool,
) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };

    let (ll, ul, lr, ur) = match edev.grid.kind {
        prjcombine_virtex2::grid::GridKind::Virtex2 => ("LL.V2", "UL.V2", "LR.V2", "UR.V2"),
        prjcombine_virtex2::grid::GridKind::Virtex2P
        | prjcombine_virtex2::grid::GridKind::Virtex2PX => ("LL.V2P", "UL.V2P", "LR.V2P", "UR.V2P"),
        prjcombine_virtex2::grid::GridKind::Spartan3 => ("LL.S3", "UL.S3", "LR.S3", "UR.S3"),
        prjcombine_virtex2::grid::GridKind::Spartan3E => ("LL.S3E", "UL.S3E", "LR.S3E", "UR.S3E"),
        prjcombine_virtex2::grid::GridKind::Spartan3A
        | prjcombine_virtex2::grid::GridKind::Spartan3ADsp => {
            ("LL.S3A", "UL.S3A", "LR.S3A", "UR.S3A")
        }
    };

    let reg_cor = if edev.grid.kind.is_virtex2() {
        "REG.COR"
    } else if edev.grid.kind == GridKind::Spartan3 {
        "REG.COR.S3"
    } else {
        "REG.COR.S3E"
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
                    bits: TileBits::Cfg,
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
        bits: TileBits::Cfg,
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
        for attr in ["ZCLK_N2", "ZCLK_N4", "ZCLK_N8", "ZCLK_N16", "ZCLK_N32"] {
            for val in ["1", "0"] {
                fuzz_one!(ctx, attr, val, [(global_mutex "DCI", "NO")], [(global_opt attr, val)]);
            }
        }
        if edev.grid.kind.is_virtex2p() {
            fuzz_global(&mut ctx, "DISABLEVGGGENERATION", &["YES", "NO"]);
        }
    } else {
        if edev.grid.kind.is_spartan3a() {
            ctx.bits = TileBits::CfgReg(Reg::Cor1);
        }
        fuzz_global(&mut ctx, "SEND_VGG0", &["1", "0"]);
        fuzz_global(&mut ctx, "SEND_VGG1", &["1", "0"]);
        fuzz_global(&mut ctx, "SEND_VGG2", &["1", "0"]);
        fuzz_global(&mut ctx, "SEND_VGG3", &["1", "0"]);
        fuzz_global(&mut ctx, "VGG_SENDMAX", &["YES", "NO"]);
        fuzz_global(&mut ctx, "VGG_ENABLE_OFFCHIP", &["YES", "NO"]);
        ctx.bits = TileBits::Cfg;
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
        bits: TileBits::Cfg,
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
    if edev.grid.kind.is_virtex2() {
        ctx.bits = TileBits::TestLL;
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "TEST_LL", val, [], [(global_opt "TESTLL", val)]);
    }
    ctx.bits = TileBits::Cfg;

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
        bits: TileBits::Cfg,
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
    if edev.grid.kind.is_spartan3a() {
        ctx.bits = TileBits::Reg(Reg::Cor1);
        ctx.tile_name = "REG.COR1.S3A";
    } else {
        ctx.bits = TileBits::Reg(Reg::Cor0);
        ctx.tile_name = reg_cor;
    }
    if edev.grid.kind == GridKind::Spartan3E {
        fuzz_one!(ctx, "MULTIBOOT_ENABLE", "1", [(mode "STARTUP")], [(pin "MBT")]);
    }
    for val in ["CCLK", "USERCLK", "JTAGCLK"] {
        fuzz_one!(ctx, "STARTUPCLK", val, [(mode "STARTUP"), (pin "CLK")], [(global_opt "STARTUPCLK", val)]);
    }
    ctx.bits = TileBits::Cfg;
    ctx.tile_name = lr;
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
        ctx.bits = TileBits::Reg(Reg::Cor2);
        ctx.tile_name = "REG.COR2.S3A";
        fuzz_enum!(ctx, "ONESHOT", ["FALSE", "TRUE"], [(mode "CAPTURE")]);
    } else {
        ctx.bits = TileBits::Reg(Reg::Cor0);
        ctx.tile_name = reg_cor;
        fuzz_enum!(ctx, "ONESHOT_ATTR", ["ONE_SHOT"], [(mode "CAPTURE")]);
    }
    ctx.bits = TileBits::Cfg;
    ctx.tile_name = lr;
    ctx.bel_name = "ICAP";
    ctx.bel = BelId::from_idx(ctx.bel.to_idx() + 1);
    if edev.grid.kind.is_spartan3a() {
        ctx.bits = TileBits::CfgReg(Reg::Ctl0);
    }
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "ICAP")]);
    fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [(mode "ICAP"), (pin "CLK")]);
    fuzz_enum!(ctx, "CEINV", ["CE", "CE_B"], [(mode "ICAP"), (pin "CE")]);
    fuzz_enum!(ctx, "WRITEINV", ["WRITE", "WRITE_B"], [(mode "ICAP"), (pin "WRITE")]);
    ctx.bits = TileBits::Cfg;
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
        bits: TileBits::Cfg,
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

    // I/O bank misc control
    if !skip_io {
        let package = backend
            .device
            .bonds
            .values()
            .max_by_key(|bond| {
                let bdata = &backend.db.bonds[bond.bond];
                let prjcombine_xilinx_geom::Bond::Virtex2(bdata) = bdata else {
                    unreachable!();
                };
                bdata.pins.len()
            })
            .unwrap();
        let bonded_io = get_bonded_ios_v2_pkg(backend, &package.name);
        if !edev.grid.kind.is_spartan3ea() {
            for (tile_name, bel, bank) in [
                (ul, 0, 7),
                (ul, 1, 0),
                (ur, 1, 1),
                (ur, 0, 2),
                (lr, 0, 3),
                (lr, 1, 4),
                (ll, 1, 5),
                (ll, 0, 6),
            ] {
                let bel_name = ["DCI0", "DCI1"][bel];
                let bel = BelId::from_idx(bel);
                let node_kind = backend.egrid.db.get_node(tile_name);
                let col = if tile_name == ul || tile_name == ll {
                    edev.grid.col_left()
                } else {
                    edev.grid.col_right()
                };
                let row = if tile_name == ll || tile_name == lr {
                    edev.grid.row_bot()
                } else {
                    edev.grid.row_top()
                };
                let mut btiles = vec![edev.btile_lrterm(col, row)];
                if edev.grid.kind.is_virtex2() {
                    btiles.push(edev.btile_btterm(col, row));
                }
                let mut site = None;
                let mut site_other = None;
                let mut coords = HashSet::new();
                let other_bank = if bank == 4 { 5 } else { 4 };
                let mut io_vr = None;
                if let Some(&(vrp, vrn)) = edev.grid.dci_io.get(&bank) {
                    if bonded_io.contains(&vrp) && bonded_io.contains(&vrn) {
                        io_vr = Some((vrp, vrn));
                    }
                }
                if io_vr.is_none() {
                    io_vr = Some(edev.grid.dci_io_alt[&bank]);
                }
                let (io_vrp, io_vrn) = io_vr.unwrap();
                let site_vrp = edev.get_io_bel(io_vrp).unwrap().3;
                let site_vrn = edev.get_io_bel(io_vrn).unwrap().3;
                for &io in edev.bonded_ios.iter().rev() {
                    let ioinfo = edev.get_io(io);
                    if ioinfo.bank == bank && coords.insert((io.col, io.row)) {
                        btiles.push(edev.btile_main(io.col, io.row));
                        if io.col == edev.grid.col_left() || io.col == edev.grid.col_right() {
                            btiles.push(edev.btile_lrterm(io.col, io.row));
                        } else {
                            btiles.push(edev.btile_btterm(io.col, io.row));
                        }
                    }
                    if bonded_io.contains(&io)
                        && matches!(ioinfo.diff, IoDiffKind::P(_))
                        && ioinfo.pad_kind == IoPadKind::Io
                        && io != io_vrp
                        && io != io_vrn
                    {
                        if ioinfo.bank == bank && site.is_none() {
                            site = Some(edev.get_io_bel(io).unwrap().3);
                        }
                        if ioinfo.bank == other_bank && site_other.is_none() {
                            site_other = Some(edev.get_io_bel(io).unwrap().3);
                        }
                    }
                }
                let bits = TileBits::Raw(btiles);
                let site = site.unwrap();
                let site_other = site_other.unwrap();
                for std in get_iostds(edev, false) {
                    if std.diff == DiffKind::True {
                        session.add_fuzzer(Box::new(TileFuzzerGen {
                            node: node_kind,
                            bits: bits.clone(),
                            feature: SimpleFeatureId {
                                tile: tile_name,
                                bel: bel_name,
                                attr: "LVDSBIAS",
                                val: std.name,
                            },
                            base: vec![
                                TileKV::Package(&package.name),
                                TileKV::GlobalMutex("DIFF", "BANK"),
                                TileKV::GlobalMutex("VREF", "NO"),
                                TileKV::GlobalMutex("DCI", "YES"),
                            ],
                            fuzz: vec![
                                TileFuzzKV::Raw(Key::SiteMode(site), None.into(), "DIFFM".into()),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site, "OMUX"),
                                    None.into(),
                                    "O1".into(),
                                ),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site, "O1INV"),
                                    None.into(),
                                    "O1".into(),
                                ),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site, "IOATTRBOX"),
                                    None.into(),
                                    std.name.into(),
                                ),
                                TileFuzzKV::Raw(Key::SitePin(site, "O1"), None.into(), true.into()),
                            ],
                        }));
                    }
                    if matches!(
                        std.dci,
                        DciKind::InputSplit | DciKind::BiSplit | DciKind::InputVcc | DciKind::BiVcc
                    ) && std.diff == DiffKind::None
                    {
                        session.add_fuzzer(Box::new(TileFuzzerGen {
                            node: node_kind,
                            bits: bits.clone(),
                            feature: SimpleFeatureId {
                                tile: tile_name,
                                bel: bel_name,
                                attr: "DCI_TERM",
                                val: std.name,
                            },
                            base: vec![
                                TileKV::Package(&package.name),
                                TileKV::GlobalMutex("VREF", "NO"),
                                TileKV::GlobalMutex("DCI", "BANK_TERM"),
                                TileKV::Raw(Key::SiteMode(site_other), "IOB".into()),
                                TileKV::Raw(Key::SiteAttr(site_other, "OMUX"), "O1".into()),
                                TileKV::Raw(Key::SiteAttr(site_other, "O1INV"), "O1".into()),
                                TileKV::Raw(
                                    Key::SiteAttr(site_other, "IOATTRBOX"),
                                    "LVDCI_33".into(),
                                ),
                                TileKV::Raw(Key::SitePin(site_other, "O1"), true.into()),
                                TileKV::Raw(Key::SiteMode(site_vrp), None.into()),
                                TileKV::Raw(Key::SiteMode(site_vrn), None.into()),
                                TileKV::Raw(Key::SiteAttr(site, "IMUX"), "1".into()),
                                TileKV::Raw(Key::SitePin(site, "I"), true.into()),
                            ],
                            fuzz: vec![
                                TileFuzzKV::Raw(Key::SiteMode(site), "IOB".into(), "IOB".into()),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site, "IOATTRBOX"),
                                    "GTL".into(),
                                    std.name.into(),
                                ),
                            ],
                        }));
                    }
                }
                if edev.grid.kind == GridKind::Spartan3 {
                    for val in ["ASREQUIRED", "CONTINUOUS", "QUIET"] {
                        session.add_fuzzer(Box::new(TileFuzzerGen {
                            node: node_kind,
                            bits: bits.clone(),
                            feature: SimpleFeatureId {
                                tile: tile_name,
                                bel: bel_name,
                                attr: "DCI_OUT",
                                val,
                            },
                            base: vec![
                                TileKV::Package(&package.name),
                                TileKV::GlobalMutex("VREF", "NO"),
                                TileKV::GlobalMutex("DCI", "BANK"),
                                TileKV::Raw(Key::SiteMode(site_other), "IOB".into()),
                                TileKV::Raw(Key::SiteAttr(site_other, "OMUX"), "O1".into()),
                                TileKV::Raw(Key::SiteAttr(site_other, "O1INV"), "O1".into()),
                                TileKV::Raw(
                                    Key::SiteAttr(site_other, "IOATTRBOX"),
                                    "LVDCI_33".into(),
                                ),
                                TileKV::Raw(Key::SitePin(site_other, "O1"), true.into()),
                                TileKV::Raw(Key::SiteMode(site_vrp), None.into()),
                                TileKV::Raw(Key::SiteMode(site_vrn), None.into()),
                                TileKV::GlobalOpt("DCIUPDATEMODE", val),
                            ],
                            fuzz: vec![
                                TileFuzzKV::Raw(Key::SiteMode(site), None.into(), "IOB".into()),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site, "OMUX"),
                                    None.into(),
                                    "O1".into(),
                                ),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site, "O1INV"),
                                    None.into(),
                                    "O1".into(),
                                ),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site, "IOATTRBOX"),
                                    None.into(),
                                    "LVDCI_33".into(),
                                ),
                                TileFuzzKV::Raw(Key::SitePin(site, "O1"), None.into(), true.into()),
                            ],
                        }));
                    }
                } else {
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: SimpleFeatureId {
                            tile: tile_name,
                            bel: bel_name,
                            attr: "DCI_OUT",
                            val: "1",
                        },
                        base: vec![
                            TileKV::Package(&package.name),
                            TileKV::GlobalMutex("VREF", "NO"),
                            TileKV::GlobalMutex("DCI", "BANK"),
                            TileKV::Raw(Key::SiteMode(site_other), "IOB".into()),
                            TileKV::Raw(Key::SiteAttr(site_other, "OMUX"), "O1".into()),
                            TileKV::Raw(Key::SiteAttr(site_other, "O1INV"), "O1".into()),
                            TileKV::Raw(Key::SiteAttr(site_other, "IOATTRBOX"), "LVDCI_33".into()),
                            TileKV::Raw(Key::SitePin(site_other, "O1"), true.into()),
                            TileKV::Raw(Key::SiteMode(site_vrp), None.into()),
                            TileKV::Raw(Key::SiteMode(site_vrn), None.into()),
                        ],
                        fuzz: vec![
                            TileFuzzKV::Raw(Key::SiteMode(site), None.into(), "IOB".into()),
                            TileFuzzKV::Raw(Key::SiteAttr(site, "OMUX"), None.into(), "O1".into()),
                            TileFuzzKV::Raw(Key::SiteAttr(site, "O1INV"), None.into(), "O1".into()),
                            TileFuzzKV::Raw(
                                Key::SiteAttr(site, "IOATTRBOX"),
                                None.into(),
                                "LVDCI_33".into(),
                            ),
                            TileFuzzKV::Raw(Key::SitePin(site, "O1"), None.into(), true.into()),
                        ],
                    }));
                }
                if bank == 6 {
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: SimpleFeatureId {
                            tile: tile_name,
                            bel: bel_name,
                            attr: "DCI_OUT_ALONE",
                            val: "1",
                        },
                        base: vec![
                            TileKV::Package(&package.name),
                            TileKV::GlobalMutex("VREF", "NO"),
                            TileKV::GlobalMutex("DCI", "GLOBAL"),
                            TileKV::GlobalOpt("MATCH_CYCLE", "NOWAIT"),
                            if edev.grid.kind == GridKind::Spartan3 {
                                TileKV::Nop
                            } else {
                                TileKV::GlobalOpt("FREEZEDCI", "NO")
                            },
                            TileKV::Raw(Key::SiteMode(site_vrp), None.into()),
                            TileKV::Raw(Key::SiteMode(site_vrn), None.into()),
                        ],
                        fuzz: vec![
                            TileFuzzKV::Raw(Key::SiteMode(site), None.into(), "IOB".into()),
                            TileFuzzKV::Raw(Key::SiteAttr(site, "OMUX"), None.into(), "O1".into()),
                            TileFuzzKV::Raw(Key::SiteAttr(site, "O1INV"), None.into(), "O1".into()),
                            TileFuzzKV::Raw(
                                Key::SiteAttr(site, "IOATTRBOX"),
                                None.into(),
                                "LVDCI_33".into(),
                            ),
                            TileFuzzKV::Raw(Key::SitePin(site, "O1"), None.into(), true.into()),
                        ],
                    }));
                } else if bank == 5 && edev.grid.dci_io_alt.contains_key(&5) {
                    let (io_alt_vrp, io_alt_vrn) = edev.grid.dci_io_alt[&5];
                    let site_alt_vrp = edev.get_io_bel(io_alt_vrp).unwrap().3;
                    let site_alt_vrn = edev.get_io_bel(io_alt_vrn).unwrap().3;
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: SimpleFeatureId {
                            tile: tile_name,
                            bel: bel_name,
                            attr: "DCI_OUT_ALONE",
                            val: "1",
                        },
                        base: vec![
                            TileKV::Package(&package.name),
                            TileKV::AltVr(true),
                            TileKV::GlobalMutex("VREF", "NO"),
                            TileKV::GlobalMutex("DCI", "GLOBAL_ALT"),
                            TileKV::GlobalOpt("MATCH_CYCLE", "NOWAIT"),
                            if site == site_alt_vrp {
                                TileKV::Nop
                            } else {
                                TileKV::Raw(Key::SiteMode(site_alt_vrp), None.into())
                            },
                            if site == site_alt_vrn {
                                TileKV::Nop
                            } else {
                                TileKV::Raw(Key::SiteMode(site_alt_vrn), None.into())
                            },
                        ],
                        fuzz: vec![
                            TileFuzzKV::Raw(Key::SiteMode(site), None.into(), "IOB".into()),
                            TileFuzzKV::Raw(Key::SiteAttr(site, "OMUX"), None.into(), "O1".into()),
                            TileFuzzKV::Raw(Key::SiteAttr(site, "O1INV"), None.into(), "O1".into()),
                            TileFuzzKV::Raw(
                                Key::SiteAttr(site, "IOATTRBOX"),
                                None.into(),
                                "LVDCI_33".into(),
                            ),
                            TileFuzzKV::Raw(Key::SitePin(site, "O1"), None.into(), true.into()),
                        ],
                    }));
                }
                let ctx = FuzzCtx {
                    session,
                    node_kind,
                    bits: TileBits::Cfg,
                    tile_name,
                    bel,
                    bel_name,
                };
                if edev.grid.kind == GridKind::Spartan3 {
                    fuzz_one!(ctx, "PRESENT", "1", [
                        (global_mutex "DCI", "PRESENT")
                    ], [
                        (mode "DCI")
                    ]);
                    fuzz_one!(ctx, "SELECT", "1", [
                        (global_mutex "DCI", "PRESENT"),
                        (global_mutex "DCI_SELECT", bel_name),
                        (mode "DCI")
                    ], [
                        (pip (pin "DATA"), (pin_far "DATA"))
                    ]);
                    for i in 0..13 {
                        let name = &*format!("LVDSBIAS_OPT{i}").leak();
                        let gname = &*format!("LVDSBIAS_OPT{i}_{bank}").leak();
                        fuzz_one!(ctx, name, "1", [
                            (global_mutex "DIFF", "MANUAL")
                        ], [
                            (global_opt_diff gname, "0", "1")
                        ]);
                    }
                } else {
                    fuzz_one!(ctx, "PRESENT", "1", [
                        (global_mutex "DCI", "PRESENT")
                    ], [
                        (mode "DCI")
                    ]);
                    fuzz_one!(ctx, "PRESENT", "TEST", [
                        (global_mutex "DCI", "PRESENT_TEST"),
                        (global_opt "TESTDCI", "YES")
                    ], [
                        (mode "DCI")
                    ]);
                }
                // ???
                fuzz_one!(ctx, "FORCE_DONE_HIGH", "#OFF", [
                    (global_mutex "DCI", "PRESENT"),
                    (mode "DCI")
                ], [
                    (attr "FORCE_DONE_HIGH", "#OFF")
                ]);
            }
            let tile_name = ll;
            let node_kind = backend.egrid.db.get_node(tile_name);
            let mut ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Cfg,
                tile_name,
                bel: BelId::from_idx(0),
                bel_name: "MISC",
            };

            if edev.grid.kind.is_virtex2p()
                && !backend.device.name.ends_with("2vp4")
                && !backend.device.name.ends_with("2vp7")
            {
                ctx.bits = TileBits::Raw(vec![
                    edev.btile_btterm(edev.grid.col_left(), edev.grid.row_top()),
                    edev.btile_btterm(edev.grid.col_right(), edev.grid.row_top()),
                    edev.btile_lrterm(edev.grid.col_right(), edev.grid.row_top()),
                    edev.btile_lrterm(edev.grid.col_right(), edev.grid.row_bot()),
                    edev.btile_btterm(edev.grid.col_right(), edev.grid.row_bot()),
                    edev.btile_btterm(edev.grid.col_left(), edev.grid.row_bot()),
                    edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_bot()),
                    edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_top()),
                ]);
                for val in ["ASREQUIRED", "CONTINUOUS", "QUIET"] {
                    fuzz_one!(ctx, "DCIUPDATEMODE", val, [
                        (global_mutex "DCI", "GLOBAL_MODE")
                    ], [
                        (global_opt "DCIUPDATEMODE", val)
                    ]);
                }
            }
        } else {
            let banks = if edev.grid.kind == GridKind::Spartan3E {
                &[
                    (
                        ul,
                        edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_top()),
                        0,
                    ),
                    (
                        ur,
                        edev.btile_lrterm(edev.grid.col_right(), edev.grid.row_top()),
                        1,
                    ),
                    (
                        lr,
                        edev.btile_lrterm(edev.grid.col_right(), edev.grid.row_bot()),
                        2,
                    ),
                    (
                        ll,
                        edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_bot()),
                        3,
                    ),
                ][..]
            } else {
                &[
                    (
                        ul,
                        edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_top()),
                        0,
                    ),
                    (
                        ll,
                        edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_bot()),
                        2,
                    ),
                ][..]
            };
            for &(tile_name, btile, bank) in banks {
                let node_kind = backend.egrid.db.get_node(tile_name);
                let mut btiles = vec![btile];
                match bank {
                    0 => {
                        let row = edev.grid.row_top();
                        for col in edev.grid.columns.ids() {
                            if col != edev.grid.col_left() && col != edev.grid.col_right() {
                                btiles.push(edev.btile_main(col, row));
                                btiles.push(edev.btile_btterm(col, row));
                            }
                        }
                    }
                    1 => {
                        let col = edev.grid.col_right();
                        for row in edev.grid.rows.ids() {
                            if row != edev.grid.row_bot() && row != edev.grid.row_top() {
                                btiles.push(edev.btile_main(col, row));
                                btiles.push(edev.btile_lrterm(col, row));
                            }
                        }
                    }
                    2 => {
                        let row = edev.grid.row_bot();
                        for col in edev.grid.columns.ids() {
                            if col != edev.grid.col_left() && col != edev.grid.col_right() {
                                btiles.push(edev.btile_main(col, row));
                                btiles.push(edev.btile_btterm(col, row));
                            }
                        }
                    }
                    3 => {
                        let col = edev.grid.col_left();
                        for row in edev.grid.rows.ids() {
                            if row != edev.grid.row_bot() && row != edev.grid.row_top() {
                                btiles.push(edev.btile_main(col, row));
                                btiles.push(edev.btile_lrterm(col, row));
                            }
                        }
                    }
                    _ => unreachable!(),
                }
                let bits = TileBits::Raw(btiles);
                let mut ios = vec![];
                for &io in edev.bonded_ios.iter().rev() {
                    let ioinfo = edev.get_io(io);
                    if bonded_io.contains(&io)
                        && matches!(ioinfo.diff, IoDiffKind::P(_))
                        && ioinfo.pad_kind == IoPadKind::Io
                        && ioinfo.bank == bank
                    {
                        ios.push(io)
                    }
                }
                assert!(ios.len() >= 2);
                if edev.grid.kind == GridKind::Spartan3ADsp {
                    ios.reverse();
                }
                let site_a = edev.get_io_bel(ios[0]).unwrap().3;
                let site_b = edev.get_io_bel(ios[1]).unwrap().3;
                let diffm = if edev.grid.kind == GridKind::Spartan3E {
                    "DIFFM"
                } else {
                    "DIFFMTB"
                };
                for std in get_iostds(edev, false) {
                    if std.diff != DiffKind::True {
                        continue;
                    }
                    if std.name != "LVDS_25" || edev.grid.kind.is_spartan3a() {
                        session.add_fuzzer(Box::new(TileFuzzerGen {
                            node: node_kind,
                            bits: bits.clone(),
                            feature: SimpleFeatureId {
                                tile: tile_name,
                                bel: "BANK",
                                attr: "LVDSBIAS_0",
                                val: std.name,
                            },
                            base: vec![
                                TileKV::Package(&package.name),
                                TileKV::GlobalMutex("DIFF", "BANK"),
                                TileKV::GlobalMutex("VREF", "NO"),
                            ],
                            fuzz: vec![
                                TileFuzzKV::Raw(Key::SiteMode(site_a), None.into(), diffm.into()),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site_a, "OMUX"),
                                    None.into(),
                                    "O1".into(),
                                ),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site_a, "O1INV"),
                                    None.into(),
                                    "O1".into(),
                                ),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site_a, "IOATTRBOX"),
                                    None.into(),
                                    std.name.into(),
                                ),
                                TileFuzzKV::Raw(
                                    Key::SiteAttr(site_a, "SUSPEND"),
                                    None.into(),
                                    if edev.grid.kind.is_spartan3a() {"3STATE"} else {""}.into(),
                                ),
                                TileFuzzKV::Raw(
                                    Key::SitePin(site_a, "O1"),
                                    None.into(),
                                    true.into(),
                                ),
                            ],
                        }));
                    }
                    let alt_std = if std.name == "RSDS_25" {
                        "MINI_LVDS_25"
                    } else {
                        "RSDS_25"
                    };
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: SimpleFeatureId {
                            tile: tile_name,
                            bel: "BANK",
                            attr: "LVDSBIAS_1",
                            val: std.name,
                        },
                        base: vec![
                            TileKV::Package(&package.name),
                            TileKV::GlobalMutex("DIFF", "BANK"),
                            TileKV::Raw(Key::SiteMode(site_a), diffm.into()),
                            TileKV::Raw(Key::SiteAttr(site_a, "OMUX"), "O1".into()),
                            TileKV::Raw(Key::SiteAttr(site_a, "O1INV"), "O1".into()),
                            TileKV::Raw(Key::SiteAttr(site_a, "IOATTRBOX"), alt_std.into()),
                            TileKV::Raw(
                                Key::SiteAttr(site_a, "SUSPEND"),
                                if edev.grid.kind.is_spartan3a() {"3STATE"} else {""}.into(),
                            ),
                            TileKV::Raw(Key::SitePin(site_a, "O1"), true.into()),
                        ],
                        fuzz: vec![
                            TileFuzzKV::Raw(Key::SiteMode(site_b), None.into(), diffm.into()),
                            TileFuzzKV::Raw(
                                Key::SiteAttr(site_b, "OMUX"),
                                None.into(),
                                "O1".into(),
                            ),
                            TileFuzzKV::Raw(
                                Key::SiteAttr(site_b, "O1INV"),
                                None.into(),
                                "O1".into(),
                            ),
                            TileFuzzKV::Raw(
                                Key::SiteAttr(site_b, "IOATTRBOX"),
                                None.into(),
                                std.name.into(),
                            ),
                            TileFuzzKV::Raw(
                                Key::SiteAttr(site_b, "SUSPEND"),
                                None.into(),
                                if edev.grid.kind.is_spartan3a() {"3STATE"} else {""}.into(),
                            ),
                            TileFuzzKV::Raw(Key::SitePin(site_b, "O1"), None.into(), true.into()),
                        ],
                    }));
                }
            }
        }
    }

    // config regs; pick dummy corner to anchor the fuzzers
    let node_kind = backend.egrid.db.get_node(ll);
    let mut ctx = FuzzCtx {
        session,
        node_kind,
        bits: TileBits::Cfg,
        tile_name: ll,
        bel: BelId::from_idx(0),
        bel_name: "MISC",
    };
    if !edev.grid.kind.is_spartan3a() {
        ctx.bits = TileBits::Reg(Reg::Cor0);
        ctx.tile_name = reg_cor;
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
            if edev.grid.kind != GridKind::Spartan3E {
                // option is accepted on S3E, but doesn't do anything
                fuzz_one!(ctx, "MATCH_CYCLE", val, [(global_mutex "DCI", "NO")], [(global_opt "MATCH_CYCLE", val)]);
            }
        }
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "DRIVE_DONE", val, [], [(global_opt "DRIVEDONE", val)]);
            fuzz_one!(ctx, "DONE_PIPE", val, [], [(global_opt "DONEPIPE", val)]);
        }
        for val in ["ENABLE", "DISABLE"] {
            fuzz_one!(ctx, "DCM_SHUTDOWN", val, [], [(global_opt "DCMSHUTDOWN", val)]);
            if edev.grid.kind.is_virtex2() {
                fuzz_one!(ctx, "DCI_SHUTDOWN", val, [], [(global_opt "DCISHUTDOWN", val)]);
                fuzz_one!(ctx, "POWERDOWN_STATUS", val, [], [(global_opt "POWERDOWNSTATUS", val)]);
            }
        }
        let vals = if edev.grid.kind.is_virtex2() {
            &[
                "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51",
                "55", "60", "130",
            ][..]
        } else if edev.grid.kind == GridKind::Spartan3 {
            &["6", "12", "25", "50", "3", "100"][..]
        } else {
            &["1", "3", "6", "12", "25", "50"][..]
        };
        for &val in vals {
            fuzz_one!(ctx, "CONFIG_RATE", val, [], [(global_opt "CONFIGRATE", val)]);
        }
        for val in ["DISABLE", "ENABLE"] {
            fuzz_one!(ctx, "CRC", val, [], [(global_opt "CRC", val)]);
        }
        if !edev.grid.kind.is_virtex2() {
            for val in ["100", "25", "50", "200"] {
                fuzz_one!(ctx, "BUSCLK_FREQ", val, [], [(global_opt "BUSCLKFREQ", val)]);
            }
            let vals = if edev.grid.kind == GridKind::Spartan3 {
                &["80", "90", "95", "100"]
            } else {
                &["70", "75", "80", "90"]
            };
            for &val in vals {
                fuzz_one!(ctx, "VRDSEL", val, [], [(global_opt "VRDSEL", val)]);
            }
        }
        ctx.bits = TileBits::Reg(Reg::Ctl0);
        ctx.tile_name = if edev.grid.kind.is_virtex2() {
            "REG.CTL"
        } else {
            "REG.CTL.S3"
        };
        ctx.bel_name = "MISC";
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "GTS_USR_B", val, [], [(global_opt "GTS_USR_B", val)]);
            fuzz_one!(ctx, "VGG_TEST", val, [], [(global_opt "VGG_TEST", val)]);
            fuzz_one!(ctx, "BCLK_TEST", val, [], [(global_opt "BCLK_TEST", val)]);
        }
        // persist not fuzzed — too much effort
        // decrypt not fuzzed — too much effort
        for val in ["NONE", "LEVEL1", "LEVEL2"] {
            // disables FreezeDCI?
            fuzz_one!(ctx, "SECURITY", val, [(global_mutex "DCI", "NO")], [(global_opt "SECURITY", val)]);
        }
    } else {
        ctx.bel_name = "STARTUP";
        ctx.bits = TileBits::Reg(Reg::Cor1);
        ctx.tile_name = "REG.COR1.S3A";
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "DRIVE_DONE", val, [], [(global_opt "DRIVEDONE", val)]);
            fuzz_one!(ctx, "DONE_PIPE", val, [], [(global_opt "DONEPIPE", val)]);
            fuzz_one!(ctx, "DRIVE_AWAKE", val, [], [(global_opt "DRIVE_AWAKE", val)]);
        }
        for val in ["DISABLE", "ENABLE"] {
            fuzz_one!(ctx, "CRC", val, [], [(global_opt "CRC", val)]);
        }
        fuzz_multi!(ctx, "VRDSEL", "", 3, [], (global_bin "VRDSEL"));

        ctx.bits = TileBits::Reg(Reg::Cor2);
        ctx.tile_name = "REG.COR2.S3A";
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            fuzz_one!(ctx, "GWE_CYCLE", val, [], [(global_opt "GWE_CYCLE", val)]);
            fuzz_one!(ctx, "GTS_CYCLE", val, [], [(global_opt "GTS_CYCLE", val)]);
        }
        for val in ["1", "2", "3", "4", "5", "6"] {
            fuzz_one!(ctx, "DONE_CYCLE", val, [], [(global_opt "DONE_CYCLE", val)]);
        }
        for val in ["1", "2", "3", "4", "5", "6", "NOWAIT"] {
            fuzz_one!(ctx, "LCK_CYCLE", val, [], [(global_opt "LCK_CYCLE", val)]);
        }
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "BPI_DIV8", val, [], [(global_opt "BPI_DIV8", val)]);
            fuzz_one!(ctx, "RESET_ON_ERR", val, [], [(global_opt "RESET_ON_ERR", val)]);
        }
        ctx.bel_name = "ICAP";
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "BYPASS", val, [], [(global_opt "ICAP_BYPASS", val)]);
        }

        ctx.bits = TileBits::Reg(Reg::Ctl0);
        ctx.tile_name = "REG.CTL.S3A";
        ctx.bel_name = "MISC";
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "GTS_USR_B", val, [], [(global_opt "GTS_USR_B", val)]);
            fuzz_one!(ctx, "VGG_TEST", val, [], [(global_opt "VGG_TEST", val)]);
            fuzz_one!(ctx, "MULTIBOOT_ENABLE", val, [], [(global_opt "MULTIBOOTMODE", val)]);
        }
        // persist not fuzzed — too much effort
        for val in ["NONE", "LEVEL1", "LEVEL2", "LEVEL3"] {
            fuzz_one!(ctx, "SECURITY", val, [], [(global_opt "SECURITY", val)]);
        }

        ctx.bits = TileBits::Reg(Reg::CclkFrequency);
        ctx.tile_name = "REG.CCLK_FREQ";
        ctx.bel_name = "STARTUP";
        for val in [
            "6", "1", "3", "7", "8", "10", "12", "13", "17", "22", "25", "27", "33", "44", "50",
            "100",
        ] {
            fuzz_one!(ctx, "CONFIG_RATE", val, [], [(global_opt "CONFIGRATE", val)]);
        }
        for val in ["0", "1", "2", "3"] {
            fuzz_one!(ctx, "CCLK_DLY", val, [], [(global_opt "CCLK_DLY", val)]);
            fuzz_one!(ctx, "CCLK_SEP", val, [], [(global_opt "CCLK_SEP", val)]);
            fuzz_one!(ctx, "CLK_SWITCH_OPT", val, [], [(global_opt "CLK_SWITCH_OPT", val)]);
        }

        ctx.bits = TileBits::Reg(Reg::HcOpt);
        ctx.tile_name = "REG.HC_OPT";
        ctx.bel_name = "MISC";
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "BRAM_SKIP", val, [], [(global_opt "BRAM_SKIP", val)]);
            fuzz_one!(ctx, "TWO_ROUND", val, [], [(global_opt "TWO_ROUND", val)]);
        }
        for i in 1..16 {
            let val = &*format!("{i}").leak();
            fuzz_one!(ctx, "HC_CYCLE", val, [], [(global_opt "HC_CYCLE", val)]);
        }

        ctx.bits = TileBits::Reg(Reg::Powerdown);
        ctx.tile_name = "REG.POWERDOWN";
        for val in ["STARTUPCLK", "INTERNALCLK"] {
            fuzz_one!(ctx, "SW_CLK", val, [], [(global_opt "SW_CLK", val)]);
        }
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "EN_SUSPEND", val, [], [(global_opt "EN_SUSPEND", val)]);
            fuzz_one!(ctx, "EN_PORB", val, [], [(global_opt "EN_PORB", val)]);
            fuzz_one!(ctx, "SUSPEND_FILTER", val, [], [(global_opt "SUSPEND_FILTER", val)]);
            fuzz_one!(ctx, "EN_SW_GSR", val, [], [(global_opt "EN_SW_GSR", val)]);
        }
        for i in 1..8 {
            let val = &*format!("{i}").leak();
            fuzz_one!(ctx, "WAKE_DELAY1", val, [], [(global_opt "WAKE_DELAY1", val)]);
        }
        for i in 1..32 {
            let val = &*format!("{i}").leak();
            fuzz_one!(ctx, "WAKE_DELAY2", val, [], [(global_opt "WAKE_DELAY2", val)]);
        }

        ctx.bits = TileBits::Reg(Reg::PuGwe);
        ctx.tile_name = "REG.PU_GWE";
        fuzz_multi!(ctx, "SW_GWE_CYCLE", "", 10, [], (global_dec "SW_GWE_CYCLE"));

        ctx.bits = TileBits::Reg(Reg::PuGts);
        ctx.tile_name = "REG.PU_GTS";
        fuzz_multi!(ctx, "SW_GTS_CYCLE", "", 10, [], (global_dec "SW_GTS_CYCLE"));

        ctx.bits = TileBits::Reg(Reg::Mode);
        ctx.tile_name = "REG.MODE";
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "TESTMODE_EN", val, [], [(global_opt "TESTMODE_EN", val)]);
            fuzz_one!(ctx, "NEXT_CONFIG_NEW_MODE", val, [], [(global_opt "NEXT_CONFIG_NEW_MODE", val)]);
        }
        fuzz_multi!(ctx, "NEXT_CONFIG_BOOT_MODE", "", 3, [], (global_bin "NEXT_CONFIG_BOOT_MODE"));
        fuzz_multi!(ctx, "BOOTVSEL", "", 3, [], (global_bin "BOOTVSEL"));

        ctx.bits = TileBits::Raw(vec![
            BitTile::Reg(DieId::from_idx(0), Reg::General1),
            BitTile::Reg(DieId::from_idx(0), Reg::General2),
        ]);
        ctx.tile_name = "REG.GENERAL";
        fuzz_multi!(ctx, "NEXT_CONFIG_ADDR", "", 32, [], (global_hex_prefix "NEXT_CONFIG_ADDR"));

        ctx.bits = TileBits::Reg(Reg::SeuOpt);
        ctx.tile_name = "REG.SEU_OPT";
        ctx.bel_name = "MISC";
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "GLUTMASK", val, [], [(global_opt "GLUTMASK", val)]);
            fuzz_one!(ctx, "POST_CRC_KEEP", val, [], [(global_opt "POST_CRC_KEEP", val)]);
        }
        for val in [
            "6", "1", "3", "7", "8", "10", "12", "13", "17", "22", "25", "27", "33", "44", "50",
            "100",
        ] {
            fuzz_one!(ctx, "POST_CRC_FREQ", val, [], [(global_opt "POST_CRC_FREQ", val)]);
        }

        ctx.bits = TileBits::Null;
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, "SPI2_EN", val, [], [(global_opt "SPI2_EN", val)]);
            fuzz_one!(ctx, "BRAMMASK", val, [], [(global_opt "BRAMMASK", val)]);
        }

        // TODO
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, skip_io: bool) {
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
        prjcombine_virtex2::grid::GridKind::Virtex2 => ("LL.V2", "UL.V2", "LR.V2", "UR.V2"),
        prjcombine_virtex2::grid::GridKind::Virtex2P
        | prjcombine_virtex2::grid::GridKind::Virtex2PX => ("LL.V2P", "UL.V2P", "LR.V2P", "UR.V2P"),
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
                "REG.COR1.S3A",
                bel,
                "VGG_ENABLE_OFFCHIP",
                xlat_bitvec(vec![diff_reg]),
            );

            let (item_cnr, item_reg, def) =
                get_split_bool(ctx, tile, bel, "VGG_SENDMAX", "NO", "YES");
            ctx.tiledb.insert(tile, bel, "VGG_SENDMAX", item_cnr);
            ctx.tiledb
                .insert("REG.COR1.S3A", bel, "VGG_SENDMAX", item_reg);
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
            ctx.tiledb.insert("REG.COR1.S3A", bel, "SEND_VGG", item);
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
        for attr in ["IDCI_OSC_SEL0", "IDCI_OSC_SEL1", "IDCI_OSC_SEL2"] {
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
        ctx.collect_enum_bool(tile, bel, "TEST_LL", "NO", "YES");
    } else {
        ctx.state
            .get_diff(tile, bel, "TEST_LL", "NO")
            .assert_empty();
        let diff = ctx.state.get_diff(tile, bel, "TEST_LL", "YES");
        let mut diffs = diff.split_tiles(&[&[0, 1], &[2, 3]]);
        let diff_ur = diffs.pop().unwrap();
        let diff_ul = diffs.pop().unwrap();
        ctx.tiledb
            .insert(tile, bel, "TEST_LL", xlat_bitvec(vec![diff_ul]));
        ctx.tiledb
            .insert(ur, bel, "TEST_LL", xlat_bitvec(vec![diff_ur]));
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
            ctx.tiledb.insert("REG.CTL.S3A", "ICAP", "ENABLE", item);
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

    // I/O bank misc control
    if !skip_io {
        if !edev.grid.kind.is_spartan3ea() {
            for (tile, bel) in [
                (ul, "DCI0"),
                (ul, "DCI1"),
                (ur, "DCI1"),
                (ur, "DCI0"),
                (lr, "DCI0"),
                (lr, "DCI1"),
                (ll, "DCI1"),
                (ll, "DCI0"),
            ] {
                // LVDS
                let mut vals = vec![];
                for std in get_iostds(edev, false) {
                    if std.diff != DiffKind::True {
                        continue;
                    }
                    let diff = ctx.state.get_diff(tile, bel, "LVDSBIAS", std.name);
                    vals.push((
                        std.name,
                        diff.filter_tiles(if edev.grid.kind.is_virtex2() {
                            &[0, 1][..]
                        } else {
                            &[0][..]
                        }),
                    ));
                }
                vals.push(("NONE", Diff::default()));
                let prefix = match edev.grid.kind {
                    GridKind::Virtex2 => "IOSTD:V2:LVDSBIAS",
                    GridKind::Virtex2P | GridKind::Virtex2PX => "IOSTD:V2P:LVDSBIAS",
                    GridKind::Spartan3 => "IOSTD:S3:LVDSBIAS",
                    _ => unreachable!(),
                };

                if edev.grid.kind == GridKind::Spartan3 {
                    let diffs = (0..13)
                        .map(|i| {
                            ctx.state
                                .get_diff(tile, bel, &*format!("LVDSBIAS_OPT{i}").leak(), "1")
                        })
                        .collect();
                    let item = xlat_bitvec(diffs);
                    let base = BitVec::repeat(false, 13);
                    for (name, diff) in vals {
                        let val = crate::diff::extract_bitvec_val(&item, &base, diff);
                        ctx.tiledb.insert_misc_data(format!("{prefix}:{name}"), val)
                    }
                    ctx.tiledb.insert(tile, bel, "LVDSBIAS", item);
                } else {
                    let mut item = xlat_enum(vals);
                    let TileItemKind::Enum { values } = item.kind else {
                        unreachable!()
                    };
                    for (name, val) in values {
                        ctx.tiledb.insert_misc_data(format!("{prefix}:{name}"), val)
                    }
                    let invert = BitVec::repeat(false, item.bits.len());
                    item.kind = TileItemKind::BitVec { invert };
                    ctx.tiledb.insert(tile, bel, "LVDSBIAS", item);
                }

                // DCI
                let diff_fdh = !ctx.state.get_diff(tile, bel, "FORCE_DONE_HIGH", "#OFF");
                if edev.grid.kind.is_virtex2() {
                    let diff = ctx
                        .state
                        .get_diff(tile, bel, "DCI_OUT", "1")
                        .filter_tiles(&[0, 1]);
                    let diff_p = ctx.state.get_diff(tile, bel, "PRESENT", "1");
                    let diff_t = ctx.state.get_diff(tile, bel, "PRESENT", "TEST");
                    assert_eq!(diff_p, diff.combine(&diff_fdh));
                    ctx.tiledb
                        .insert(tile, bel, "ENABLE", xlat_bitvec(vec![diff]));
                    let diff_t = diff_t.combine(&!diff_p);
                    ctx.tiledb
                        .insert(tile, bel, "TEST_ENABLE", xlat_bitvec(vec![diff_t]));
                } else {
                    let diff_ar = ctx
                        .state
                        .get_diff(tile, bel, "DCI_OUT", "ASREQUIRED")
                        .filter_tiles(&[0]);
                    let diff_c = ctx
                        .state
                        .get_diff(tile, bel, "DCI_OUT", "CONTINUOUS")
                        .filter_tiles(&[0]);
                    let diff_q = ctx
                        .state
                        .get_diff(tile, bel, "DCI_OUT", "QUIET")
                        .filter_tiles(&[0]);
                    let diff_p = ctx.state.get_diff(tile, bel, "PRESENT", "1");
                    assert_eq!(diff_c, diff_ar);
                    let diff_q = diff_q.combine(&!&diff_c);
                    let diff_p = diff_p.combine(&!&diff_c).combine(&!&diff_fdh);
                    ctx.tiledb
                        .insert(tile, bel, "ENABLE", xlat_bitvec(vec![diff_c]));
                    ctx.tiledb
                        .insert(tile, bel, "QUIET", xlat_bitvec(vec![diff_q]));
                    ctx.tiledb
                        .insert(tile, bel, "TEST_ENABLE", xlat_bitvec(vec![diff_p]));
                }
                ctx.tiledb
                    .insert(tile, bel, "FORCE_DONE_HIGH", xlat_bitvec(vec![diff_fdh]));

                // DCI TERM stuff
                let mut vals_split = vec![("NONE", Diff::default())];
                let mut vals_vcc = vec![("NONE", Diff::default())];
                let item_en = ctx.tiledb.item(tile, bel, "ENABLE");
                for std in get_iostds(edev, false) {
                    if std.name.starts_with("DIFF_") {
                        continue;
                    }
                    match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => (),
                        DciKind::InputVcc | DciKind::BiVcc => {
                            let mut diff = ctx.state.get_diff(tile, bel, "DCI_TERM", std.name);
                            diff.apply_bit_diff(item_en, true, false);
                            vals_vcc.push((
                                std.name,
                                diff.filter_tiles(if edev.grid.kind.is_virtex2() {
                                    &[0, 1][..]
                                } else {
                                    &[0][..]
                                }),
                            ));
                        }
                        DciKind::InputSplit | DciKind::BiSplit => {
                            if std.diff == DiffKind::True {
                                vals_split.push((std.name, Diff::default()));
                            } else {
                                let mut diff = ctx.state.get_diff(tile, bel, "DCI_TERM", std.name);
                                diff.apply_bit_diff(item_en, true, false);
                                vals_split.push((
                                    std.name,
                                    diff.filter_tiles(if edev.grid.kind.is_virtex2() {
                                        &[0, 1][..]
                                    } else {
                                        &[0][..]
                                    }),
                                ));
                            }
                        }
                    }
                }
                let prefix = match edev.grid.kind {
                    GridKind::Virtex2 => "IOSTD:V2",
                    GridKind::Virtex2P | GridKind::Virtex2PX => "IOSTD:V2P",
                    GridKind::Spartan3 => "IOSTD:S3",
                    _ => unreachable!(),
                };
                for (attr, vals) in [("TERM_SPLIT", vals_split), ("TERM_VCC", vals_vcc)] {
                    let mut item = xlat_enum(vals);
                    let TileItemKind::Enum { values } = item.kind else {
                        unreachable!()
                    };
                    for (name, val) in values {
                        ctx.tiledb
                            .insert_misc_data(format!("{prefix}:{attr}:{name}"), val)
                    }
                    let invert = BitVec::repeat(false, item.bits.len());
                    item.kind = TileItemKind::BitVec { invert };
                    ctx.tiledb.insert(tile, bel, attr, item);
                }
            }

            if edev.grid.kind == GridKind::Spartan3 {
                for tile in [ll, ul, lr, ur] {
                    ctx.tiledb.insert(
                        tile,
                        "MISC",
                        "DCI_TEST_MUX",
                        xlat_enum(vec![
                            ("DCI0", ctx.state.get_diff(tile, "DCI0", "SELECT", "1")),
                            ("DCI1", ctx.state.get_diff(tile, "DCI1", "SELECT", "1")),
                        ]),
                    );
                }
            }
            if edev.grid.kind.is_virtex2p()
                && !ctx.device.name.ends_with("2vp4")
                && !ctx.device.name.ends_with("2vp7")
            {
                ctx.state
                    .get_diff(ll, "MISC", "DCIUPDATEMODE", "ASREQUIRED")
                    .assert_empty();
                ctx.state
                    .get_diff(ll, "MISC", "DCIUPDATEMODE", "CONTINUOUS")
                    .assert_empty();
                let diff = ctx.state.get_diff(ll, "MISC", "DCIUPDATEMODE", "QUIET");
                let diff0 = diff.filter_tiles(&[8, 0]);
                let diff1 = diff.filter_tiles(&[8, 1]);
                let diff2 = diff.filter_tiles(&[2]);
                let diff3 = diff.filter_tiles(&[3]);
                let diff4 = diff.filter_tiles(&[8, 4]);
                let diff5 = diff.filter_tiles(&[8, 5]);
                let diff6 = diff.filter_tiles(&[6]);
                let diff7 = diff.filter_tiles(&[7]);
                ctx.tiledb
                    .insert(ul, "DCI1", "QUIET", xlat_bitvec(vec![diff0]));
                ctx.tiledb
                    .insert(ur, "DCI1", "QUIET", xlat_bitvec(vec![diff1]));
                ctx.tiledb
                    .insert(ur, "DCI0", "QUIET", xlat_bitvec(vec![diff2]));
                ctx.tiledb
                    .insert(lr, "DCI0", "QUIET", xlat_bitvec(vec![diff3]));
                ctx.tiledb
                    .insert(lr, "DCI1", "QUIET", xlat_bitvec(vec![diff4]));
                ctx.tiledb
                    .insert(ll, "DCI1", "QUIET", xlat_bitvec(vec![diff5]));
                ctx.tiledb
                    .insert(ll, "DCI0", "QUIET", xlat_bitvec(vec![diff6]));
                ctx.tiledb
                    .insert(ul, "DCI0", "QUIET", xlat_bitvec(vec![diff7]));
            }

            let tile = ll;
            let bel = "DCI0";
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "DCI_OUT_ALONE", "1")
                .filter_tiles(if edev.grid.kind.is_virtex2() {
                    &[0, 1][..]
                } else {
                    &[0][..]
                });
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
            if edev.grid.dci_io_alt.contains_key(&5) {
                let bel = "DCI1";
                let mut alt_diff = ctx
                    .state
                    .get_diff(tile, bel, "DCI_OUT_ALONE", "1")
                    .filter_tiles(if edev.grid.kind.is_virtex2() {
                        &[0, 1][..]
                    } else {
                        &[0][..]
                    });
                alt_diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
                alt_diff = alt_diff.combine(&!&diff);
                ctx.tiledb
                    .insert(tile, "MISC", "DCI_ALTVR", xlat_bitvec(vec![alt_diff]));
            }
            if edev.grid.kind.is_virtex2() {
                diff.apply_bitvec_diff(
                    ctx.tiledb.item(tile, "MISC", "ZCLK_DIV2"),
                    &bitvec![0, 0, 0, 1, 0],
                    &BitVec::repeat(false, 5),
                );
            }
            ctx.tiledb
                .insert(tile, "MISC", "DCI_CLK_ENABLE", xlat_bitvec(vec![diff]));
        } else {
            let banks = if edev.grid.kind == GridKind::Spartan3E {
                &[ul, ur, lr, ll][..]
            } else {
                &[ul, ll][..]
            };
            for &tile in banks {
                let bel = "BANK";
                let mut vals_0 = vec![];
                let mut vals_1 = vec![];
                for std in get_iostds(edev, false) {
                    if std.diff != DiffKind::True {
                        continue;
                    }
                    if std.name != "LVDS_25" || edev.grid.kind.is_spartan3a() {
                        let diff_0 = ctx.state.get_diff(tile, bel, "LVDSBIAS_0", std.name);
                        vals_0.push((std.name, diff_0.filter_tiles(&[0])));
                    }
                    let diff_1 = ctx.state.get_diff(tile, bel, "LVDSBIAS_1", std.name);
                    vals_1.push((std.name, diff_1.filter_tiles(&[0])));
                }
                vals_0.push(("NONE", Diff::default()));
                vals_1.push(("NONE", Diff::default()));
                if edev.grid.kind == GridKind::Spartan3E {
                    // move LVDS_25 to back in LVDSBIAS_1 so that the other values are aligned
                    let idx = vals_1.iter().position(|x| x.0 == "LVDS_25").unwrap();
                    let lvds = vals_1.remove(idx);
                    vals_1.push(lvds);
                }
                let item_0 = xlat_enum(vals_0);
                let item_1 = xlat_enum(vals_1);
                for (attr, mut item) in [("LVDSBIAS_0", item_0), ("LVDSBIAS_1", item_1)] {
                    let TileItemKind::Enum { values } = item.kind else {
                        unreachable!()
                    };
                    let prefix = if edev.grid.kind == GridKind::Spartan3E {
                        format!("IOSTD:S3E:{attr}")
                    } else {
                        "IOSTD:S3A.TB:LVDSBIAS".to_string()
                    };
                    for (name, val) in values {
                        ctx.tiledb.insert_misc_data(format!("{prefix}:{name}"), val)
                    }
                    let invert = BitVec::repeat(false, item.bits.len());
                    item.kind = TileItemKind::BitVec { invert };
                    ctx.tiledb.insert(tile, bel, attr, item);
                }
            }
        }

        if edev.grid.kind.is_spartan3ea() {
            for (tile, btile) in [
                (
                    ll,
                    edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_bot()),
                ),
                (
                    ul,
                    edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_top()),
                ),
                (
                    lr,
                    edev.btile_lrterm(edev.grid.col_right(), edev.grid.row_bot()),
                ),
                (
                    ur,
                    edev.btile_lrterm(edev.grid.col_right(), edev.grid.row_top()),
                ),
            ] {
                let bel = "MISC";
                let mut diff = Diff::default();
                let BitTile::Main(_, _, width, _, height, _) = btile else {
                    unreachable!()
                };
                for tframe in 0..width {
                    for tbit in 0..height {
                        let bit = btile.xlat_pos_fwd((tframe, tbit));
                        if ctx.empty_bs.get_bit(bit) {
                            diff.bits.insert(
                                FeatureBit {
                                    tile: 0,
                                    frame: tframe,
                                    bit: tbit,
                                },
                                true,
                            );
                        }
                    }
                }
                if tile == ll {
                    for attr in ["SEND_VGG", "VGG_SENDMAX"] {
                        diff.discard_bits(ctx.tiledb.item(tile, bel, attr));
                    }
                }
                if edev.grid.kind == GridKind::Spartan3E {
                    for attr in ["LVDSBIAS_0", "LVDSBIAS_1"] {
                        diff.discard_bits(ctx.tiledb.item(tile, "BANK", attr));
                    }
                }
                if !diff.bits.is_empty() {
                    ctx.tiledb
                        .insert(tile, bel, "UNK_ALWAYS_SET", xlat_bit_wide(diff));
                }
            }
        }
    }

    // config regs
    if !edev.grid.kind.is_spartan3a() {
        let tile = if edev.grid.kind.is_virtex2() {
            "REG.COR"
        } else if edev.grid.kind == GridKind::Spartan3 {
            "REG.COR.S3"
        } else {
            "REG.COR.S3E"
        };
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
        if edev.grid.kind != GridKind::Spartan3E {
            ctx.collect_enum(
                tile,
                bel,
                "MATCH_CYCLE",
                &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
            );
        }
        ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
        if edev.grid.kind == GridKind::Spartan3E {
            ctx.collect_bit(tile, bel, "MULTIBOOT_ENABLE", "1");
        }
        let vals = if edev.grid.kind.is_virtex2() {
            &[
                "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51",
                "55", "60", "130",
            ][..]
        } else if edev.grid.kind == GridKind::Spartan3 {
            &["3", "6", "12", "25", "50", "100"][..]
        } else {
            &["1", "3", "6", "12", "25", "50"][..]
        };
        ctx.collect_enum_ocd(tile, bel, "CONFIG_RATE", vals, OcdMode::BitOrder);
        if !edev.grid.kind.is_virtex2() {
            ctx.collect_enum(tile, bel, "BUSCLK_FREQ", &["25", "50", "100", "200"]);
        }
        ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DCM_SHUTDOWN", "DISABLE", "ENABLE");
        if edev.grid.kind.is_virtex2() {
            ctx.collect_enum_bool(tile, bel, "POWERDOWN_STATUS", "DISABLE", "ENABLE");
            ctx.state
                .get_diff(tile, bel, "DCI_SHUTDOWN", "ENABLE")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "DCI_SHUTDOWN", "DISABLE")
                .assert_empty();
        }
        ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
        if edev.grid.kind == GridKind::Spartan3 {
            ctx.collect_enum(tile, bel, "VRDSEL", &["100", "95", "90", "80"]);
        } else if edev.grid.kind == GridKind::Spartan3E {
            // ??? 70 == 75?
            let d70 = ctx.state.get_diff(tile, bel, "VRDSEL", "70");
            let d75 = ctx.state.get_diff(tile, bel, "VRDSEL", "75");
            let d80 = ctx.state.get_diff(tile, bel, "VRDSEL", "80");
            let d90 = ctx.state.get_diff(tile, bel, "VRDSEL", "90");
            assert_eq!(d70, d75);
            ctx.tiledb.insert(
                tile,
                bel,
                "VRDSEL",
                xlat_enum_ocd(
                    vec![("70_75", d70), ("80", d80), ("90", d90)],
                    OcdMode::BitOrder,
                ),
            );
        }

        let bel = "CAPTURE";
        let item = ctx.extract_bit(tile, bel, "ONESHOT_ATTR", "ONE_SHOT");
        ctx.tiledb.insert(tile, bel, "ONESHOT", item);

        let tile = if edev.grid.kind.is_virtex2() {
            "REG.CTL"
        } else {
            "REG.CTL.S3"
        };
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "GTS_USR_B", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "VGG_TEST", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "BCLK_TEST", "NO", "YES");
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
        if edev.grid.kind.is_virtex2() {
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
    } else {
        let tile = "REG.COR1.S3A";
        let bel = "STARTUP";
        ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
        ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DRIVE_AWAKE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
        ctx.collect_bitvec(tile, bel, "VRDSEL", "");

        let tile = "REG.COR2.S3A";
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
        ctx.collect_enum(tile, bel, "DONE_CYCLE", &["1", "2", "3", "4", "5", "6"]);
        ctx.collect_enum(
            tile,
            bel,
            "LCK_CYCLE",
            &["1", "2", "3", "4", "5", "6", "NOWAIT"],
        );
        ctx.collect_enum_bool(tile, "CAPTURE", "ONESHOT", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "BPI_DIV8", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "RESET_ON_ERR", "NO", "YES");
        ctx.collect_enum_bool(tile, "ICAP", "BYPASS", "NO", "YES");

        let tile = "REG.CTL.S3A";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "GTS_USR_B", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "VGG_TEST", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "MULTIBOOT_ENABLE", "NO", "YES");
        ctx.collect_enum(
            tile,
            bel,
            "SECURITY",
            &["NONE", "LEVEL1", "LEVEL2", "LEVEL3"],
        );
        // too much trouble to deal with in normal ways.
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

        let tile = "REG.CCLK_FREQ";
        let bel = "STARTUP";
        let mut item = ctx.extract_enum_ocd(
            tile,
            bel,
            "CONFIG_RATE",
            &[
                "6", "1", "3", "7", "8", "10", "12", "13", "17", "22", "25", "27", "33", "44",
                "50", "100",
            ],
            OcdMode::BitOrder,
        );
        // a little fixup.
        assert_eq!(item.bits.len(), 9);
        assert_eq!(
            item.bits[8],
            FeatureBit {
                tile: 0,
                frame: 0,
                bit: 8
            }
        );
        item.bits.push(FeatureBit {
            tile: 0,
            frame: 0,
            bit: 9,
        });
        let TileItemKind::Enum { ref mut values } = item.kind else {
            unreachable!()
        };
        for val in values.values_mut() {
            val.push(false);
        }
        ctx.tiledb.insert(tile, bel, "CONFIG_RATE", item);
        ctx.collect_enum(tile, bel, "CCLK_DLY", &["0", "1", "2", "3"]);
        ctx.collect_enum(tile, bel, "CCLK_SEP", &["0", "1", "2", "3"]);
        ctx.collect_enum(tile, bel, "CLK_SWITCH_OPT", &["0", "1", "2", "3"]);

        let tile = "REG.HC_OPT";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "BRAM_SKIP", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "TWO_ROUND", "NO", "YES");
        ctx.collect_enum(
            tile,
            bel,
            "HC_CYCLE",
            &Vec::from_iter((1..16).map(|x| &*format!("{x}").leak())),
        );

        let tile = "REG.POWERDOWN";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "SW_CLK", &["STARTUPCLK", "INTERNALCLK"]);
        ctx.collect_enum_bool(tile, bel, "EN_SUSPEND", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "EN_PORB", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "EN_SW_GSR", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "SUSPEND_FILTER", "NO", "YES");
        ctx.collect_enum(
            tile,
            bel,
            "WAKE_DELAY1",
            &Vec::from_iter((1..8).map(|x| &*format!("{x}").leak())),
        );
        ctx.collect_enum(
            tile,
            bel,
            "WAKE_DELAY2",
            &Vec::from_iter((1..32).map(|x| &*format!("{x}").leak())),
        );

        let tile = "REG.PU_GWE";
        ctx.collect_bitvec(tile, bel, "SW_GWE_CYCLE", "");

        let tile = "REG.PU_GTS";
        ctx.collect_bitvec(tile, bel, "SW_GTS_CYCLE", "");

        let tile = "REG.MODE";
        let bel = "MISC";
        ctx.collect_bitvec(tile, bel, "BOOTVSEL", "");
        ctx.collect_bitvec(tile, bel, "NEXT_CONFIG_BOOT_MODE", "");
        ctx.collect_enum_bool(tile, bel, "NEXT_CONFIG_NEW_MODE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "TESTMODE_EN", "NO", "YES");

        let tile = "REG.GENERAL";
        ctx.collect_bitvec(tile, bel, "NEXT_CONFIG_ADDR", "");

        let tile = "REG.SEU_OPT";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "GLUTMASK", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "POST_CRC_KEEP", "NO", "YES");

        // too much effort to include in the automatic fuzzer
        ctx.tiledb.insert(
            tile,
            bel,
            "POST_CRC_EN",
            TileItem {
                bits: vec![FeatureBit {
                    tile: 0,
                    frame: 0,
                    bit: 0,
                }],
                kind: TileItemKind::BitVec {
                    invert: BitVec::from_iter([false]),
                },
            },
        );

        let mut item = ctx.extract_enum_ocd(
            tile,
            bel,
            "POST_CRC_FREQ",
            &[
                "6", "1", "3", "7", "8", "10", "12", "13", "17", "22", "25", "27", "33", "44",
                "50", "100",
            ],
            OcdMode::BitOrder,
        );
        // a little fixup.
        assert_eq!(item.bits.len(), 9);
        assert_eq!(
            item.bits[8],
            FeatureBit {
                tile: 0,
                frame: 0,
                bit: 12
            }
        );
        item.bits.push(FeatureBit {
            tile: 0,
            frame: 0,
            bit: 13,
        });
        let TileItemKind::Enum { ref mut values } = item.kind else {
            unreachable!()
        };
        for val in values.values_mut() {
            val.push(false);
        }
        ctx.tiledb.insert(tile, bel, "POST_CRC_FREQ", item);

        // TODO
    }
}

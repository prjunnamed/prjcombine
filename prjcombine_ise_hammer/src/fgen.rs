use prjcombine_int::{
    db::{Dir, IntfWireInNaming, IntfWireOutNaming, NodeRawTileId, NodeTileId, NodeWireId},
    grid::{ColId, DieId, LayerId, RowId},
};
use prjcombine_virtex_bitstream::{BitTile, Reg};
use prjcombine_xilinx_geom::ExpandedDevice;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use unnamed_entity::EntityId;

use prjcombine_hammer::{BatchValue, Fuzzer, FuzzerGen};
use prjcombine_int::db::{BelId, NodeKindId};

use crate::backend::{FuzzerInfo, IseBackend, Key, MultiValue, SimpleFeatureId, State};

pub type Loc = (DieId, ColId, RowId, LayerId);

#[derive(Debug, Copy, Clone)]
pub enum TileWire<'a> {
    BelPinNear(BelId, &'a str),
    BelPinFar(BelId, &'a str),
    IntWire(NodeWireId),
}

fn resolve_tile_relation(
    backend: &IseBackend,
    mut loc: Loc,
    relation: TileRelation,
) -> Option<Loc> {
    match relation {
        TileRelation::ClbTbusRight => loop {
            if loc.1 == backend.egrid.die(loc.0).cols().last().unwrap() {
                return None;
            }
            loc.1 += 1;
            if let Some((layer, _)) = backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                backend.egrid.db.nodes.key(node.kind) == "CLB"
            }) {
                loc.3 = layer;
                if let ExpandedDevice::Virtex2(edev) = backend.edev {
                    if loc.1 == edev.grid.col_right() - 1 {
                        return None;
                    }
                }
                return Some(loc);
            }
        },
        TileRelation::ClbCinDown => loop {
            if loc.2 == backend.egrid.die(loc.0).rows().last().unwrap() {
                return None;
            }
            loc.2 -= 1;
            if let Some((layer, _)) = backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                backend.egrid.db.nodes.key(node.kind).starts_with("CLB")
                    || backend.egrid.db.nodes.key(node.kind).starts_with("CLEX")
            }) {
                loc.3 = layer;
                return Some(loc);
            }
        },
    }
}

fn resolve_tile_wire<'a>(
    backend: &IseBackend<'a>,
    loc: Loc,
    wire: TileWire,
) -> Option<(&'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let intdb = backend.egrid.db;
    let node_naming = &intdb.node_namings[node.naming];
    Some(match wire {
        TileWire::BelPinNear(bel, pin) => {
            let bel_naming = &node_naming.bels[bel];
            (&node.names[bel_naming.tile], &bel_naming.pins[pin].name)
        }
        TileWire::BelPinFar(bel, pin) => {
            let bel_naming = &node_naming.bels[bel];
            (&node.names[bel_naming.tile], &bel_naming.pins[pin].name_far)
        }
        TileWire::IntWire(w) => {
            backend.egrid.resolve_wire((loc.0, node.tiles[w.0], w.1))?;
            (
                &node.names[NodeRawTileId::from_idx(0)],
                node_naming.wires.get(&w)?,
            )
        }
    })
}

fn resolve_int_pip<'a>(
    backend: &IseBackend<'a>,
    loc: Loc,
    wire_from: NodeWireId,
    wire_to: NodeWireId,
) -> Option<(&'a str, &'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let intdb = backend.egrid.db;
    let node_naming = &intdb.node_namings[node.naming];
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire_to.0], wire_to.1))?;
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire_from.0], wire_from.1))?;
    Some(
        if let Some(ext) = node_naming.ext_pips.get(&(wire_to, wire_from)) {
            (&node.names[ext.tile], &ext.wire_from, &ext.wire_to)
        } else {
            (
                &node.names[NodeRawTileId::from_idx(0)],
                node_naming.wires.get(&wire_from)?,
                node_naming.wires.get(&wire_to)?,
            )
        },
    )
}

fn resolve_intf_test_pip<'a>(
    backend: &IseBackend<'a>,
    loc: Loc,
    wire_from: NodeWireId,
    wire_to: NodeWireId,
) -> Option<(&'a str, &'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let intdb = backend.egrid.db;
    let node_naming = &intdb.node_namings[node.naming];
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire_to.0], wire_to.1))?;
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire_from.0], wire_from.1))?;
    if let ExpandedDevice::Virtex4(edev) = backend.edev {
        if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex5
            && intdb.node_namings.key(node.naming) == "INTF.PPC_R"
            && intdb.wires.key(wire_from.1).starts_with("TEST")
        {
            // ISE.
            return None;
        }
    }
    Some((
        &node.names[NodeRawTileId::from_idx(0)],
        match node_naming.intf_wires_in.get(&wire_from)? {
            IntfWireInNaming::Simple { name } => name,
            IntfWireInNaming::Buf { name_in, .. } => name_in,
            IntfWireInNaming::TestBuf { name_out, .. } => name_out,
            IntfWireInNaming::Delay { name_out, .. } => name_out,
            _ => unreachable!(),
        },
        match node_naming.intf_wires_out.get(&wire_to)? {
            IntfWireOutNaming::Simple { name } => name,
            IntfWireOutNaming::Buf { name_out, .. } => name_out,
        },
    ))
}

fn resolve_intf_delay<'a>(
    backend: &IseBackend<'a>,
    loc: Loc,
    wire: NodeWireId,
) -> Option<(&'a str, &'a str, &'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let intdb = backend.egrid.db;
    let node_naming = &intdb.node_namings[node.naming];
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
    let IntfWireInNaming::Delay {
        name_out,
        name_in,
        name_delay,
    } = node_naming.intf_wires_in.get(&wire)?
    else {
        unreachable!()
    };
    Some((
        &node.names[NodeRawTileId::from_idx(0)],
        name_in,
        name_delay,
        name_out,
    ))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TileRelation {
    ClbTbusRight,
    ClbCinDown,
}

#[derive(Debug)]
pub enum TileKV<'a> {
    SiteMode(BelId, &'a str),
    SiteUnused(BelId),
    SiteAttr(BelId, &'a str, &'a str),
    SitePin(BelId, &'a str, bool),
    GlobalOpt(&'a str, &'a str),
    GlobalMutexNone(&'a str),
    GlobalMutex(&'a str, &'a str),
    GlobalMutexSite(&'a str, BelId),
    RowMutexSite(&'a str, BelId),
    RowMutex(&'a str, &'a str),
    SiteMutex(BelId, &'a str, &'a str),
    TileMutex(&'a str, &'a str),
    Pip(TileWire<'a>, TileWire<'a>),
    IntPip(NodeWireId, NodeWireId),
    NodeMutexShared(NodeWireId),
    IntMutexShared(&'a str),
    NodeIntDstFilter(NodeWireId),
    NodeIntSrcFilter(NodeWireId),
    NodeIntDistinct(NodeWireId, NodeWireId),
    DriveLLH(NodeWireId),
    DriveLLV(NodeWireId),
    StabilizeGclkc,
}

impl<'a> TileKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        Some(match *self {
            TileKV::SiteMode(bel, mode) => {
                let node = backend.egrid.node(loc);
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), "MAIN");
                }
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMode(site), mode)
            }
            TileKV::SiteUnused(bel) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMode(site), None)
            }
            TileKV::SiteAttr(bel, attr, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteAttr(site, attr), val)
            }
            TileKV::SitePin(bel, pin, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SitePin(site, pin), val)
            }
            TileKV::GlobalOpt(opt, val) => fuzzer.base(Key::GlobalOpt(opt), val),
            TileKV::GlobalMutexNone(name) => fuzzer.base(Key::GlobalMutex(name), None),
            TileKV::GlobalMutex(name, val) => fuzzer.base(Key::GlobalMutex(name), val),
            TileKV::GlobalMutexSite(name, bel) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::GlobalMutex(name), &site[..])
            }
            TileKV::RowMutexSite(name, bel) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::RowMutex(name, loc.2), &site[..])
            }
            TileKV::RowMutex(name, val) => fuzzer.base(Key::RowMutex(name, loc.2), val),
            TileKV::SiteMutex(bel, name, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMutex(&site[..], name), val)
            }
            TileKV::TileMutex(name, val) => fuzzer.base(Key::TileMutex(loc, name), val),
            TileKV::Pip(wa, wb) => {
                let (ta, wa) = resolve_tile_wire(backend, loc, wa)?;
                let (tb, wb) = resolve_tile_wire(backend, loc, wb)?;
                assert_eq!(ta, tb);
                fuzzer.base(Key::Pip(ta, wa, wb), true)
            }
            TileKV::IntPip(wa, wb) => {
                let (tile, wa, wb) = resolve_int_pip(backend, loc, wa, wb)?;
                fuzzer.base(Key::Pip(tile, wa, wb), true)
            }
            TileKV::NodeMutexShared(wire) => {
                let node = backend.egrid.node(loc);
                let node = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                fuzzer.base(Key::NodeMutex(node), "SHARED")
            }
            TileKV::IntMutexShared(val) => {
                let node = backend.egrid.node(loc);
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), val);
                }
                fuzzer
            }
            TileKV::NodeIntDstFilter(wire) => {
                let intdb = backend.egrid.db;
                let wire_name = intdb.wires.key(wire.1);
                match backend.edev {
                    ExpandedDevice::Virtex2(edev) => {
                        let node = backend.egrid.node(loc);
                        if backend
                            .egrid
                            .db
                            .nodes
                            .key(node.kind)
                            .starts_with("INT.BRAM")
                        {
                            let mut tgt = None;
                            for i in 0..4 {
                                if let Some(bram_node) =
                                    backend.egrid.find_node(loc.0, (loc.1, loc.2 - i), |node| {
                                        intdb.nodes.key(node.kind).starts_with("BRAM")
                                            || intdb.nodes.key(node.kind) == "DSP"
                                    })
                                {
                                    tgt = Some((bram_node, i));
                                    break;
                                }
                            }
                            let (bram_node, idx) = tgt.unwrap();
                            let node_tile = NodeTileId::from_idx(idx);
                            let bram_node_kind = &intdb.nodes[bram_node.kind];
                            if (edev.grid.kind.is_virtex2()
                                || edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3)
                                && (wire_name.starts_with("IMUX.CLK")
                                    || wire_name.starts_with("IMUX.SR")
                                    || wire_name.starts_with("IMUX.CE")
                                    || wire_name.starts_with("IMUX.TS"))
                            {
                                let mut found = false;
                                for bel in bram_node_kind.bels.values() {
                                    for pin in bel.pins.values() {
                                        if pin.wires.contains(&(node_tile, wire.1)) {
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                                if !found {
                                    return None;
                                }
                            }
                        }
                        if backend.egrid.db.nodes.key(node.kind) == "INT.IOI.S3E"
                            || backend.egrid.db.nodes.key(node.kind) == "INT.IOI.S3A.LR"
                        {
                            if matches!(
                                &wire_name[..],
                                "IMUX.DATA3"
                                    | "IMUX.DATA7"
                                    | "IMUX.DATA11"
                                    | "IMUX.DATA15"
                                    | "IMUX.DATA19"
                                    | "IMUX.DATA23"
                                    | "IMUX.DATA27"
                                    | "IMUX.DATA31"
                            ) && loc.2 != edev.grid.row_mid() - 1
                                && loc.2 != edev.grid.row_mid()
                            {
                                return None;
                            }
                            if wire_name == "IMUX.DATA13"
                                && edev.grid.kind
                                    == prjcombine_virtex2::grid::GridKind::Spartan3ADsp
                                && loc.1 == edev.grid.col_left()
                            {
                                // ISE bug. sigh.
                                return None;
                            }
                            if matches!(
                                &wire_name[..],
                                "IMUX.DATA12" | "IMUX.DATA13" | "IMUX.DATA14"
                            ) && loc.2 != edev.grid.row_mid()
                            {
                                return None;
                            }
                        }
                        if backend.egrid.db.nodes.key(node.kind) == "INT.IOI.S3A.TB"
                            && wire_name == "IMUX.DATA15"
                            && loc.2 == edev.grid.row_top()
                        {
                            // also ISE bug.
                            return None;
                        }
                        if edev.grid.kind.is_spartan3a()
                            && backend.egrid.db.nodes.key(node.kind) == "INT.CLB"
                        {
                            // avoid SR in corners — it causes the inverter bit to be auto-set
                            let is_lr =
                                loc.1 == edev.grid.col_left() || loc.1 == edev.grid.col_right();
                            let is_bt =
                                loc.2 == edev.grid.row_bot() || loc.2 == edev.grid.row_top();
                            if intdb.wires.key(wire.1).starts_with("IMUX.SR") && is_lr && is_bt {
                                return None;
                            }
                        }
                        if matches!(&wire_name[..], "IMUX.DATA15" | "IMUX.DATA31")
                            && intdb.node_namings.key(node.naming).starts_with("INT.MACC")
                        {
                            // ISE bug.
                            return None;
                        }

                        // TODO
                    }
                    ExpandedDevice::Virtex4(edev) => {
                        if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex4 {
                            // avoid CLK in center column — using it on DCM tiles causes the inverter bit to be auto-set
                            if intdb.wires.key(wire.1).starts_with("IMUX.CLK")
                                && loc.1 == edev.col_clk
                            {
                                return None;
                            }
                        }
                    }
                    _ => (),
                }
                fuzzer
            }
            TileKV::NodeIntSrcFilter(wire) => {
                let intdb = backend.egrid.db;
                let wire_name = intdb.wires.key(wire.1);
                let node = backend.egrid.node(loc);
                #[allow(clippy::single_match)]
                match backend.edev {
                    ExpandedDevice::Virtex2(edev) => {
                        if (edev.grid.kind.is_virtex2()
                            || edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3)
                            && wire_name.starts_with("OUT")
                            && intdb.nodes.key(node.kind).starts_with("INT.DCM")
                        {
                            let dcm = backend
                                .egrid
                                .find_node(loc.0, (loc.1, loc.2), |node| {
                                    intdb.nodes.key(node.kind).starts_with("DCM.")
                                })
                                .unwrap();
                            let site = &dcm.bels[BelId::from_idx(0)];
                            fuzzer = fuzzer.base(Key::SiteMode(site), "DCM");
                            for pin in [
                                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
                                "CLKFX", "CLKFX180", "CONCUR", "STATUS1", "STATUS7",
                            ] {
                                fuzzer = fuzzer.base(Key::SitePin(site, pin), true);
                            }
                        }
                        if wire_name == "OUT.PCI0"
                            && loc.2 != edev.grid.row_pci.unwrap() - 2
                            && loc.2 != edev.grid.row_pci.unwrap() - 1
                            && loc.2 != edev.grid.row_pci.unwrap()
                            && loc.2 != edev.grid.row_pci.unwrap() + 1
                        {
                            return None;
                        }
                        if wire_name == "OUT.PCI1"
                            && loc.2 != edev.grid.row_pci.unwrap() - 1
                            && loc.2 != edev.grid.row_pci.unwrap()
                        {
                            return None;
                        }
                        if (backend.egrid.db.nodes.key(node.kind) == "INT.IOI.S3E"
                            || backend.egrid.db.nodes.key(node.kind) == "INT.IOI.S3A.LR")
                            && matches!(
                                &wire_name[..],
                                "OUT.FAN3" | "OUT.FAN7" | "OUT.SEC11" | "OUT.SEC15"
                            )
                            && loc.2 != edev.grid.row_mid() - 1
                            && loc.2 != edev.grid.row_mid()
                        {
                            return None;
                        }
                        if wire_name.starts_with("GCLK")
                            && matches!(
                                &intdb.node_namings.key(node.naming)[..],
                                "INT.BRAM.BRK" | "INT.BRAM.S3ADSP.BRK" | "INT.MACC.BRK"
                            )
                        {
                            // ISE bug.
                            return None;
                        }
                    }
                    _ => (),
                }
                fuzzer
            }
            TileKV::NodeIntDistinct(a, b) => {
                let node = backend.egrid.node(loc);
                let a = backend.egrid.resolve_wire((loc.0, node.tiles[a.0], a.1))?;
                let b = backend.egrid.resolve_wire((loc.0, node.tiles[b.0], b.1))?;
                if a == b {
                    return None;
                }
                fuzzer
            }
            TileKV::DriveLLH(wire) => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                let node = backend.egrid.node(loc);
                let wnode = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let mut src_col = node.tiles[wire.0].0;
                loop {
                    if let Some((src_layer, src_node)) =
                        backend
                            .egrid
                            .find_node_loc(loc.0, (src_col, loc.2), |src_node| {
                                backend.egrid.db.nodes.key(src_node.kind).starts_with("INT")
                            })
                    {
                        let src_node_kind = &backend.egrid.db.nodes[src_node.kind];
                        for (&dwire, mux) in &src_node_kind.muxes {
                            if !backend.egrid.db.wires.key(dwire.1).starts_with("LH") {
                                continue;
                            }
                            let Some(dnode) = backend.egrid.resolve_wire((
                                loc.0,
                                src_node.tiles[dwire.0],
                                dwire.1,
                            )) else {
                                continue;
                            };
                            if dnode != wnode {
                                continue;
                            }
                            let swire = *mux.ins.first().unwrap();
                            let (tile, wa, wb) = resolve_int_pip(
                                backend,
                                (loc.0, src_col, loc.2, src_layer),
                                swire,
                                dwire,
                            )?;
                            return Some(fuzzer.base(Key::Pip(tile, wa, wb), true));
                        }
                    }
                    if src_col == edev.grid.col_left() || src_col == edev.grid.col_right() {
                        return None;
                    }
                    if wire.0.to_idx() == 0 {
                        src_col -= 1;
                    } else {
                        src_col += 1;
                    }
                }
            }
            TileKV::DriveLLV(wire) => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                let node = backend.egrid.node(loc);
                let wnode = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let mut src_row = node.tiles[wire.0].1;
                loop {
                    if let Some((src_layer, src_node)) =
                        backend
                            .egrid
                            .find_node_loc(loc.0, (loc.1, src_row), |src_node| {
                                backend.egrid.db.nodes.key(src_node.kind).starts_with("INT")
                            })
                    {
                        let src_node_kind = &backend.egrid.db.nodes[src_node.kind];
                        for (&dwire, mux) in &src_node_kind.muxes {
                            if !backend.egrid.db.wires.key(dwire.1).starts_with("LV") {
                                continue;
                            }
                            let Some(dnode) = backend.egrid.resolve_wire((
                                loc.0,
                                src_node.tiles[dwire.0],
                                dwire.1,
                            )) else {
                                continue;
                            };
                            if dnode != wnode {
                                continue;
                            }
                            let swire = *mux.ins.first().unwrap();
                            let (tile, wa, wb) = resolve_int_pip(
                                backend,
                                (loc.0, loc.1, src_row, src_layer),
                                swire,
                                dwire,
                            )?;
                            return Some(fuzzer.base(Key::Pip(tile, wa, wb), true));
                        }
                    }
                    if src_row == edev.grid.row_bot() || src_row == edev.grid.row_top() {
                        return None;
                    }
                    if wire.0.to_idx() == 0 {
                        src_row -= 1;
                    } else {
                        src_row += 1;
                    }
                }
            }
            TileKV::StabilizeGclkc => {
                for (node_kind, node_name, _) in &backend.egrid.db.nodes {
                    if !node_name.starts_with("GCLKC") {
                        continue;
                    }
                    for &loc in &backend.egrid.node_index[node_kind] {
                        let bel = BelId::from_idx(0);
                        for (o, i) in [
                            ("OUT_L0", "IN_B0"),
                            ("OUT_R0", "IN_B0"),
                            ("OUT_L1", "IN_B1"),
                            ("OUT_R1", "IN_B1"),
                            ("OUT_L2", "IN_B2"),
                            ("OUT_R2", "IN_B2"),
                            ("OUT_L3", "IN_B3"),
                            ("OUT_R3", "IN_B3"),
                            ("OUT_L4", "IN_B4"),
                            ("OUT_R4", "IN_B4"),
                            ("OUT_L5", "IN_B5"),
                            ("OUT_R5", "IN_B5"),
                            ("OUT_L6", "IN_B6"),
                            ("OUT_R6", "IN_B6"),
                            ("OUT_L7", "IN_B7"),
                            ("OUT_R7", "IN_B7"),
                        ] {
                            fuzzer = TileKV::TileMutex(o, i).apply(backend, loc, fuzzer)?;
                            fuzzer = TileKV::Pip(
                                TileWire::BelPinNear(bel, i),
                                TileWire::BelPinNear(bel, o),
                            )
                            .apply(backend, loc, fuzzer)?;
                        }
                    }
                }
                fuzzer
            }
        })
    }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum TileFuzzKV<'a> {
    SiteMode(BelId, &'a str),
    SiteAttr(BelId, &'a str, &'a str),
    SiteAttrDiff(BelId, &'a str, &'a str, &'a str),
    #[allow(dead_code)]
    SitePin(BelId, &'a str),
    SitePinFull(BelId, &'a str),
    GlobalOpt(&'a str, &'a str),
    GlobalOptDiff(&'a str, &'a str, &'a str),
    Pip(TileWire<'a>, TileWire<'a>),
    IntPip(NodeWireId, NodeWireId),
    IntfTestPip(NodeWireId, NodeWireId),
    IntfDelay(NodeWireId, bool),
    NodeMutexExclusive(NodeWireId),
    TileMutexExclusive(&'a str),
    RowMutexExclusive(&'a str),
    TileRelated(TileRelation, Box<TileFuzzKV<'a>>),
}

impl<'a> TileFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        let node = backend.egrid.node(loc);
        let node_data = &backend.egrid.db.nodes[node.kind];
        let node_naming = &backend.egrid.db.node_namings[node.naming];
        Some(match *self {
            TileFuzzKV::SiteMode(bel, val) => {
                let node = backend.egrid.node(loc);
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), "MAIN");
                }
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteMode(site), None, val)
            }
            TileFuzzKV::SiteAttr(bel, attr, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteAttr(site, attr), None, val)
            }
            TileFuzzKV::SiteAttrDiff(bel, attr, va, vb) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteAttr(site, attr), va, vb)
            }
            TileFuzzKV::SitePin(bel, pin) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SitePin(site, pin), false, true)
            }
            TileFuzzKV::SitePinFull(bel, pin) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer = fuzzer.fuzz(Key::SitePin(site, pin), false, true);
                let bel_data = &node_data.bels[bel];
                let pin_data = &bel_data.pins[pin];
                let bel_naming = &node_naming.bels[bel];
                let pin_naming = &bel_naming.pins[pin];
                assert_eq!(pin_data.wires.len(), 1);
                let wire = *pin_data.wires.first().unwrap();
                if let Some(pip) = pin_naming.int_pips.get(&wire) {
                    fuzzer = fuzzer.fuzz(
                        Key::Pip(&node.names[pip.tile], &pip.wire_from, &pip.wire_to),
                        false,
                        true,
                    );
                }
                fuzzer
            }
            TileFuzzKV::GlobalOpt(opt, val) => fuzzer.fuzz(Key::GlobalOpt(opt), None, val),
            TileFuzzKV::GlobalOptDiff(opt, vala, valb) => {
                fuzzer.fuzz(Key::GlobalOpt(opt), vala, valb)
            }
            TileFuzzKV::Pip(wa, wb) => {
                let (ta, wa) = resolve_tile_wire(backend, loc, wa)?;
                let (tb, wb) = resolve_tile_wire(backend, loc, wb)?;
                assert_eq!(ta, tb);
                fuzzer.fuzz(Key::Pip(ta, wa, wb), None, true)
            }
            TileFuzzKV::IntPip(wa, wb) => {
                let (tile, wa, wb) = resolve_int_pip(backend, loc, wa, wb)?;
                fuzzer.fuzz(Key::Pip(tile, wa, wb), None, true)
            }
            TileFuzzKV::IntfTestPip(wa, wb) => {
                let (tile, wa, wb) = resolve_intf_test_pip(backend, loc, wa, wb)?;
                fuzzer.fuzz(Key::Pip(tile, wa, wb), None, true)
            }
            TileFuzzKV::IntfDelay(wire, state) => {
                let (tile, wa, wb, wc) = resolve_intf_delay(backend, loc, wire)?;
                if state {
                    fuzzer.fuzz(Key::Pip(tile, wa, wb), None, true).fuzz(
                        Key::Pip(tile, wb, wc),
                        None,
                        true,
                    )
                } else {
                    fuzzer.fuzz(Key::Pip(tile, wa, wc), None, true)
                }
            }
            TileFuzzKV::NodeMutexExclusive(wire) => {
                let node = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                fuzzer.fuzz(Key::NodeMutex(node), None, "EXCLUSIVE")
            }
            TileFuzzKV::TileMutexExclusive(name) => {
                fuzzer.fuzz(Key::TileMutex(loc, name), None, "EXCLUSIVE")
            }
            TileFuzzKV::RowMutexExclusive(name) => {
                fuzzer.fuzz(Key::RowMutex(name, loc.2), None, "EXCLUSIVE")
            }
            TileFuzzKV::TileRelated(relation, ref chain) => {
                let loc = resolve_tile_relation(backend, loc, relation)?;
                chain.apply(backend, loc, fuzzer)?
            }
        })
    }
}

#[derive(Debug)]
pub enum TileMultiFuzzKV<'a> {
    SiteAttr(BelId, &'a str, MultiValue),
    GlobalOpt(&'a str, MultiValue),
}

impl<'a> TileMultiFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Fuzzer<IseBackend<'a>> {
        match *self {
            TileMultiFuzzKV::SiteAttr(bel, attr, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz_multi(Key::SiteAttr(site, attr), val)
            }
            TileMultiFuzzKV::GlobalOpt(attr, val) => fuzzer.fuzz_multi(Key::GlobalOpt(attr), val),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TileBits {
    Null,
    Main(usize),
    MainUp,
    MainDown,
    Bram,
    Spine,
    BTTerm,
    LRTerm,
    BTSpine,
    #[allow(clippy::upper_case_acronyms)]
    LLV,
    #[allow(clippy::upper_case_acronyms)]
    PPC,
    Gclkc,
    ClkLR,
    Hclk,
    Gclkvm,
    Clkc,
    Corner,
    CornerReg(Reg),
}

impl TileBits {
    fn get_bits(&self, backend: &IseBackend, loc: (DieId, ColId, RowId, LayerId)) -> Vec<BitTile> {
        let (die, col, row, _) = loc;
        match *self {
            TileBits::Null => vec![],
            TileBits::Main(n) => match backend.edev {
                ExpandedDevice::Xc4k(_) => todo!(),
                ExpandedDevice::Xc5200(_) => todo!(),
                ExpandedDevice::Virtex(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                ExpandedDevice::Virtex2(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                ExpandedDevice::Spartan6(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                ExpandedDevice::Virtex4(edev) => (0..n)
                    .map(|idx| edev.btile_main(die, col, row + idx))
                    .collect(),
                ExpandedDevice::Ultrascale(_) => todo!(),
                ExpandedDevice::Versal(_) => todo!(),
            },
            TileBits::Bram => match backend.edev {
                ExpandedDevice::Xc4k(_) => unreachable!(),
                ExpandedDevice::Xc5200(_) => unreachable!(),
                ExpandedDevice::Virtex(edev) => {
                    vec![
                        edev.btile_main(col, row),
                        edev.btile_main(col, row + 1),
                        edev.btile_main(col, row + 2),
                        edev.btile_main(col, row + 3),
                        edev.btile_bram(col, row),
                    ]
                }
                ExpandedDevice::Virtex2(edev) => {
                    vec![
                        edev.btile_main(col, row),
                        edev.btile_main(col, row + 1),
                        edev.btile_main(col, row + 2),
                        edev.btile_main(col, row + 3),
                        edev.btile_bram(col, row),
                    ]
                }
                ExpandedDevice::Spartan6(edev) => {
                    todo!()
                }
                ExpandedDevice::Virtex4(edev) => {
                    if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex4 {
                        vec![
                            edev.btile_main(die, col, row),
                            edev.btile_main(die, col, row + 1),
                            edev.btile_main(die, col, row + 2),
                            edev.btile_main(die, col, row + 3),
                            edev.btile_bram(die, col, row),
                        ]
                    } else {
                        vec![
                            edev.btile_main(die, col, row),
                            edev.btile_main(die, col, row + 1),
                            edev.btile_main(die, col, row + 2),
                            edev.btile_main(die, col, row + 3),
                            edev.btile_main(die, col, row + 4),
                            edev.btile_bram(die, col, row),
                        ]
                    }
                }
                ExpandedDevice::Ultrascale(_) => {
                    todo!()
                }
                ExpandedDevice::Versal(_) => {
                    todo!()
                }
            },
            TileBits::Spine => match backend.edev {
                ExpandedDevice::Virtex2(edev) => {
                    vec![edev.btile_spine(row)]
                }
                _ => unreachable!(),
            },
            TileBits::BTTerm => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_btterm(col, row)]
            }
            TileBits::LRTerm => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_lrterm(col, row)]
            }
            TileBits::MainUp => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_main(col, row + 1)]
            }
            TileBits::MainDown => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_main(col, row - 1)]
            }
            TileBits::BTSpine => match backend.edev {
                ExpandedDevice::Virtex2(edev) => {
                    vec![edev.btile_spine(row), edev.btile_btspine(row)]
                }
                ExpandedDevice::Spartan6(edev) => {
                    let dir = if row == edev.grid.row_bio_outer() {
                        Dir::S
                    } else if row == edev.grid.row_tio_outer() {
                        Dir::N
                    } else {
                        unreachable!()
                    };
                    vec![edev.btile_reg(dir)]
                }
                _ => unreachable!(),
            },
            TileBits::LLV => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3E {
                    vec![edev.btile_llv_b(col), edev.btile_llv_t(col)]
                } else {
                    vec![edev.btile_llv(col)]
                }
            }
            TileBits::PPC => match backend.edev {
                ExpandedDevice::Virtex2(edev) => {
                    let mut res = vec![];
                    for i in 0..16 {
                        res.push(edev.btile_main(col, row + i));
                    }
                    for i in 0..16 {
                        res.push(edev.btile_main(col + 9, row + i));
                    }
                    for i in 1..9 {
                        res.push(edev.btile_main(col + i, row));
                    }
                    for i in 1..9 {
                        res.push(edev.btile_main(col + i, row + 15));
                    }
                    res
                }
                ExpandedDevice::Virtex4(edev) => match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            TileBits::Gclkc => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if row == edev.grid.row_bot() + 1 {
                    vec![
                        edev.btile_btspine(row - 1),
                        edev.btile_spine(row - 1),
                        edev.btile_spine(row),
                        edev.btile_spine(row + 1),
                    ]
                } else if row == edev.grid.row_top() {
                    vec![
                        edev.btile_spine(row - 2),
                        edev.btile_spine(row - 1),
                        edev.btile_spine(row),
                        edev.btile_btspine(row),
                    ]
                } else {
                    vec![
                        edev.btile_spine(row - 2),
                        edev.btile_spine(row - 1),
                        edev.btile_spine(row),
                        edev.btile_spine(row + 1),
                    ]
                }
            }
            TileBits::ClkLR => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![
                    edev.btile_main(col, row - 1),
                    edev.btile_main(col, row),
                    edev.btile_lrterm(col, row - 2),
                    edev.btile_lrterm(col, row - 1),
                    edev.btile_lrterm(col, row),
                    edev.btile_lrterm(col, row + 1),
                ]
            }
            TileBits::Hclk => match backend.edev {
                ExpandedDevice::Virtex2(edev) => {
                    vec![edev.btile_hclk(col, row)]
                }
                ExpandedDevice::Spartan6(_) => todo!(),
                ExpandedDevice::Virtex4(_) => todo!(),
                ExpandedDevice::Ultrascale(_) => todo!(),
                ExpandedDevice::Versal(_) => todo!(),
                _ => unreachable!(),
            },
            TileBits::Gclkvm => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_clkv(col, row - 1), edev.btile_clkv(col, row)]
            }
            TileBits::Clkc => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_spine(row - 1)]
            }
            TileBits::Corner => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if edev.grid.kind.is_virtex2() {
                    vec![
                        edev.btile_lrterm(col, row),
                        edev.btile_btterm(col, row),
                        edev.btile_main(col, row),
                    ]
                } else {
                    vec![edev.btile_lrterm(col, row), edev.btile_main(col, row)]
                }
            }
            TileBits::CornerReg(reg) => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if edev.grid.kind.is_virtex2() {
                    vec![
                        edev.btile_lrterm(col, row),
                        edev.btile_btterm(col, row),
                        edev.btile_main(col, row),
                        BitTile::Reg(DieId::from_idx(0), reg),
                    ]
                } else {
                    vec![
                        edev.btile_lrterm(col, row),
                        edev.btile_main(col, row),
                        BitTile::Reg(DieId::from_idx(0), reg),
                    ]
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TileFuzzerGen<'a> {
    pub node: NodeKindId,
    pub bits: TileBits,
    pub feature: SimpleFeatureId<'a>,
    pub base: Vec<TileKV<'a>>,
    pub fuzz: Vec<TileFuzzKV<'a>>,
}

impl<'b> TileFuzzerGen<'b> {
    fn try_gen(
        &self,
        backend: &IseBackend<'b>,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
        loc: (DieId, ColId, RowId, LayerId),
    ) -> Option<Fuzzer<IseBackend<'b>>> {
        let bits = self.bits.get_bits(backend, loc);
        let mut fuzzer = Fuzzer::new(FuzzerInfo::Simple(bits, self.feature));
        for x in &self.base {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        for x in &self.fuzz {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        if fuzzer.is_ok(kv) {
            Some(fuzzer)
        } else {
            None
        }
    }
}

impl<'b> FuzzerGen<IseBackend<'b>> for TileFuzzerGen<'b> {
    fn gen<'a>(
        &self,
        backend: &'a IseBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
    ) -> Option<(
        Fuzzer<IseBackend<'b>>,
        Option<Box<dyn FuzzerGen<IseBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.node];
        let mut rng = thread_rng();
        if locs.len() > 20 {
            for &loc in locs.choose_multiple(&mut rng, 20) {
                if let Some(res) = self.try_gen(backend, kv, loc) {
                    return Some((res, None));
                }
            }
        }
        for &loc in locs.choose_multiple(&mut rng, locs.len()) {
            if let Some(res) = self.try_gen(backend, kv, loc) {
                return Some((res, None));
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct TileMultiFuzzerGen<'a> {
    pub node: NodeKindId,
    pub bits: TileBits,
    pub feature: SimpleFeatureId<'a>,
    pub base: Vec<TileKV<'a>>,
    pub width: usize,
    pub fuzz: TileMultiFuzzKV<'a>,
}

impl<'b> TileMultiFuzzerGen<'b> {
    fn try_gen(
        &self,
        backend: &IseBackend<'b>,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
        loc: (DieId, ColId, RowId, LayerId),
    ) -> Option<Fuzzer<IseBackend<'b>>> {
        let bits = self.bits.get_bits(backend, loc);
        let mut fuzzer = Fuzzer::new(FuzzerInfo::Simple(bits, self.feature));
        for x in &self.base {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        fuzzer = fuzzer.bits(self.width);
        fuzzer = self.fuzz.apply(backend, loc, fuzzer);
        if fuzzer.is_ok(kv) {
            Some(fuzzer)
        } else {
            None
        }
    }
}

impl<'b> FuzzerGen<IseBackend<'b>> for TileMultiFuzzerGen<'b> {
    fn gen<'a>(
        &self,
        backend: &'a IseBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
    ) -> Option<(
        Fuzzer<IseBackend<'b>>,
        Option<Box<dyn FuzzerGen<IseBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.node];
        let mut rng = thread_rng();
        if locs.len() > 20 {
            for &loc in locs.choose_multiple(&mut rng, 20) {
                if let Some(res) = self.try_gen(backend, kv, loc) {
                    return Some((res, None));
                }
            }
        }
        for &loc in locs.choose_multiple(&mut rng, locs.len()) {
            if let Some(res) = self.try_gen(backend, kv, loc) {
                return Some((res, None));
            }
        }
        None
    }
}

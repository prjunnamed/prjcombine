use prjcombine_int::{
    db::{Dir, IntfWireInNaming, IntfWireOutNaming, NodeRawTileId, NodeTileId, NodeWireId},
    grid::{ColId, DieId, LayerId, RowId},
};
use prjcombine_virtex2::expanded::IoPadKind;
use prjcombine_virtex_bitstream::{BitTile, Reg};
use prjcombine_xilinx_geom::{Bond, ExpandedDevice};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::{HashMap, HashSet};
use unnamed_entity::EntityId;

use prjcombine_hammer::{BatchValue, Fuzzer, FuzzerGen, FuzzerValue};
use prjcombine_int::db::{BelId, NodeKindId};

use crate::backend::{FuzzerInfo, IseBackend, Key, MultiValue, SimpleFeatureId, State, Value};

pub type Loc = (DieId, ColId, RowId, LayerId);

#[derive(Debug, Copy, Clone)]
pub enum TileWire<'a> {
    BelPinNear(BelId, &'a str),
    BelPinFar(BelId, &'a str),
    RelatedBelPinNear(BelId, BelRelation, &'a str),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TileRelation {
    ClbTbusRight,
    ClbCinDown,
    IoiBrefclk,
    IobBrefclkClkBT,
    ClkIob(bool),
    ClkDcm,
    ClkHrow,
    Rclk,
    Ioclk(Dir),
    Cfg,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BelRelation {
    Rclk,
    Ioclk(Dir),
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
            if loc.2.to_idx() == 0 {
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
        TileRelation::IoiBrefclk => {
            loc.1 += 1;
            let Some((layer, _)) = backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                backend.egrid.db.nodes.key(node.kind).starts_with("IOI")
            }) else {
                unreachable!()
            };
            loc.3 = layer;
            Some(loc)
        }
        TileRelation::IobBrefclkClkBT => {
            let ExpandedDevice::Virtex2(edev) = backend.edev else {
                unreachable!()
            };
            if loc.1 != edev.grid.col_clk && loc.1 != edev.grid.col_clk - 2 {
                return None;
            }
            loc.1 = edev.grid.col_clk;
            let Some((layer, _)) = backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                backend.egrid.db.nodes.key(node.kind).starts_with("CLK")
            }) else {
                unreachable!()
            };
            loc.3 = layer;
            Some(loc)
        }
        TileRelation::ClkIob(is_t) => {
            let ExpandedDevice::Virtex4(edev) = backend.edev else {
                unreachable!()
            };
            match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    loc.2 = if is_t {
                        edev.row_iobdcm.unwrap() - 16
                    } else {
                        edev.row_dcmiob.unwrap()
                    };
                    let Some((layer, _)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                            backend.egrid.db.nodes.key(node.kind).starts_with("CLK_IOB")
                        })
                    else {
                        unreachable!()
                    };
                    loc.3 = layer;
                    Some(loc)
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex6
                | prjcombine_virtex4::grid::GridKind::Virtex7 => unreachable!(),
            }
        }
        TileRelation::ClkDcm => {
            let ExpandedDevice::Virtex4(edev) = backend.edev else {
                unreachable!()
            };
            match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    let is_t = loc.2 > edev.grids[loc.0].row_bufg();
                    loop {
                        if is_t {
                            loc.2 += 8;
                            if loc.2.to_idx() >= edev.grids[loc.0].rows().len() {
                                return None;
                            }
                        } else {
                            if loc.2.to_idx() == 0 {
                                return None;
                            }
                            loc.2 -= 8;
                        }
                        if let Some((layer, _)) =
                            backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                                backend.egrid.db.nodes.key(node.kind).starts_with("CLK_DCM")
                            })
                        {
                            loc.3 = layer;
                            return Some(loc);
                        }
                    }
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex6
                | prjcombine_virtex4::grid::GridKind::Virtex7 => unreachable!(),
            }
        }
        TileRelation::ClkHrow => match backend.edev {
            ExpandedDevice::Xc4k(_) => todo!(),
            ExpandedDevice::Xc5200(_) => todo!(),
            ExpandedDevice::Virtex(_) => todo!(),
            ExpandedDevice::Virtex2(_) => todo!(),
            ExpandedDevice::Spartan6(_) => todo!(),
            ExpandedDevice::Virtex4(edev) => {
                loc.1 = edev.col_clk;
                let Some((layer, _)) = backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                    backend
                        .egrid
                        .db
                        .nodes
                        .key(node.kind)
                        .starts_with("CLK_HROW")
                }) else {
                    unreachable!()
                };
                loc.3 = layer;
                Some(loc)
            }
            ExpandedDevice::Ultrascale(_) => todo!(),
            ExpandedDevice::Versal(_) => todo!(),
        },
        TileRelation::Cfg => {
            let ExpandedDevice::Virtex4(edev) = backend.edev else {
                unreachable!()
            };
            match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    loc.1 = edev.col_cfg;
                    loc.2 = edev.grids[loc.0].row_bufg() - 8;
                    let Some((layer, _)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                            backend.egrid.db.nodes.key(node.kind) == "CFG"
                        })
                    else {
                        unreachable!()
                    };
                    loc.3 = layer;
                    Some(loc)
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex6
                | prjcombine_virtex4::grid::GridKind::Virtex7 => unreachable!(),
            }
        }
        TileRelation::Rclk => {
            Some(resolve_bel_relation(backend, loc, BelId::from_idx(0), BelRelation::Rclk)?.0)
        }
        TileRelation::Ioclk(dir) => {
            Some(resolve_bel_relation(backend, loc, BelId::from_idx(0), BelRelation::Ioclk(dir))?.0)
        }
    }
}

fn resolve_bel_relation(
    backend: &IseBackend,
    mut loc: Loc,
    mut bel: BelId,
    relation: BelRelation,
) -> Option<(Loc, BelId)> {
    match relation {
        BelRelation::Rclk => {
            let ExpandedDevice::Virtex4(edev) = backend.edev else {
                unreachable!()
            };
            match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    loc.1 = if loc.1 <= edev.col_clk {
                        edev.col_lio.unwrap()
                    } else {
                        edev.col_rio.unwrap()
                    };
                    let Some((layer, node)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                            backend
                                .egrid
                                .db
                                .nodes
                                .key(node.kind)
                                .starts_with("HCLK_IOIS")
                        })
                    else {
                        unreachable!()
                    };
                    loc.3 = layer;
                    bel = backend.egrid.db.nodes[node.kind]
                        .bels
                        .get("RCLK")
                        .unwrap()
                        .0;
                    Some((loc, bel))
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex6 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
            }
        }
        BelRelation::Ioclk(dir) => {
            let ExpandedDevice::Virtex4(edev) = backend.edev else {
                unreachable!()
            };
            match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    match dir {
                        Dir::S => {
                            if loc.1 == edev.col_cfg && loc.2 == edev.grids[loc.0].row_bufg() + 8 {
                                return None;
                            }
                            if loc.2.to_idx() < 16 {
                                return None;
                            }
                            loc.2 -= 16;
                        }
                        Dir::N => {
                            if loc.1 == edev.col_cfg && loc.2 == edev.grids[loc.0].row_bufg() - 8 {
                                return None;
                            }
                            loc.2 += 16;
                            if loc.2.to_idx() >= edev.grids[loc.0].rows().len() {
                                return None;
                            }
                        }
                        _ => unreachable!(),
                    }
                    let (layer, node) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                            matches!(
                                &backend.egrid.db.nodes.key(node.kind)[..],
                                "HCLK_IOIS_DCI"
                                    | "HCLK_IOIS_LVDS"
                                    | "HCLK_CENTER"
                                    | "HCLK_CENTER_ABOVE_CFG"
                                    | "HCLK_DCMIOB"
                                    | "HCLK_IOBDCM"
                            )
                        })?;
                    loc.3 = layer;
                    bel = backend.egrid.db.nodes[node.kind]
                        .bels
                        .get("IOCLK")
                        .unwrap()
                        .0;
                    Some((loc, bel))
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex6 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
            }
        }
    }
}

fn find_ioi(backend: &IseBackend, loc: Loc, tile: usize) -> Loc {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };
    let (col, row) = if loc.1 == edev.grid.col_left() || loc.1 == edev.grid.col_right() {
        (loc.1, loc.2 + tile)
    } else {
        (loc.1 + tile, loc.2)
    };
    let layer = backend
        .egrid
        .find_node_loc(loc.0, (col, row), |node| {
            backend.egrid.db.nodes.key(node.kind).starts_with("IOI")
        })
        .unwrap()
        .0;
    (loc.0, col, row, layer)
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
        TileWire::RelatedBelPinNear(bel, relation, pin) => {
            let (loc, bel) = resolve_bel_relation(backend, loc, bel, relation)?;
            let node = backend.egrid.node(loc);
            let intdb = backend.egrid.db;
            let node_naming = &intdb.node_namings[node.naming];
            let bel_naming = &node_naming.bels[bel];
            (&node.names[bel_naming.tile], &bel_naming.pins[pin].name)
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

pub fn get_bonded_ios_v2_pkg(
    backend: &IseBackend,
    pkg: &str,
) -> HashSet<prjcombine_virtex2::grid::IoCoord> {
    let bond_id = backend
        .device
        .bonds
        .values()
        .find(|bond| bond.name == *pkg)
        .unwrap()
        .bond;
    let Bond::Virtex2(ref bond) = backend.db.bonds[bond_id] else {
        unreachable!()
    };
    bond.pins
        .values()
        .filter_map(|pin| {
            if let prjcombine_virtex2::bond::BondPin::Io(io) = pin {
                Some(*io)
            } else {
                None
            }
        })
        .collect()
}

fn get_bonded_ios_v2(
    backend: &IseBackend,
    fuzzer: &Fuzzer<IseBackend>,
) -> HashSet<prjcombine_virtex2::grid::IoCoord> {
    let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
        unreachable!()
    };
    get_bonded_ios_v2_pkg(backend, pkg)
}

#[derive(Debug, Clone)]
pub enum TileKV<'a> {
    Nop,
    Bel(BelId, BelKV<'a>),
    IobBel(usize, BelId, BelKV<'a>),
    Package(&'a str),
    AltVr(bool),
    GlobalOpt(&'a str, &'a str),
    NoGlobalOpt(&'a str),
    VccAux(&'a str),
    GlobalMutexNone(&'a str),
    GlobalMutex(&'a str, &'a str),
    RowMutex(&'a str, &'a str),
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
    IsLeftRandor(bool),
    Raw(Key<'a>, Value<'a>),
    TileRelated(TileRelation, Box<TileKV<'a>>),
}

#[derive(Debug, Clone)]
pub enum BelKV<'a> {
    Nop,
    Mode(&'a str),
    Unused,
    Attr(&'a str, &'a str),
    Pin(&'a str, bool),
    GlobalMutexHere(&'a str),
    RowMutexHere(&'a str),
    Mutex(&'a str, &'a str),
    IsVref,
    IsVr,
    OtherIobInput(&'a str),
    OtherIobDiffOutput(&'a str),
    BankDiffOutput(&'a str, Option<&'a str>),
    NotIbuf,
}

impl<'a> TileKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        Some(match *self {
            TileKV::Nop => fuzzer,
            TileKV::Bel(bel, ref inner) => inner.apply(backend, loc, bel, fuzzer)?,
            TileKV::IobBel(tile, bel, ref inner) => {
                let ioi_loc = find_ioi(backend, loc, tile);
                inner.apply(backend, ioi_loc, bel, fuzzer)?
            }
            TileKV::Package(pkg) => fuzzer.base(Key::Package, pkg),
            TileKV::AltVr(alt) => fuzzer.base(Key::AltVr, alt),
            TileKV::GlobalOpt(opt, val) => fuzzer.base(Key::GlobalOpt(opt), val),
            TileKV::NoGlobalOpt(opt) => fuzzer.base(Key::GlobalOpt(opt), None),
            TileKV::VccAux(val) => fuzzer.base(Key::VccAux, val),
            TileKV::GlobalMutexNone(name) => fuzzer.base(Key::GlobalMutex(name), None),
            TileKV::GlobalMutex(name, val) => fuzzer.base(Key::GlobalMutex(name), val),
            TileKV::RowMutex(name, val) => fuzzer.base(Key::RowMutex(name, loc.2), val),
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
            TileKV::IsLeftRandor(val) => {
                if val != (loc.1.to_idx() == 1) {
                    return None;
                }
                if loc.2.to_idx() == 0 {
                    return None;
                }
                fuzzer
            }
            TileKV::Raw(ref key, ref val) => fuzzer.base(key.clone(), val.clone()),
            TileKV::TileRelated(relation, ref chain) => {
                let loc = resolve_tile_relation(backend, loc, relation)?;
                chain.apply(backend, loc, fuzzer)?
            }
        })
    }
}

impl<'a> BelKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        bel: BelId,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        Some(match *self {
            BelKV::Nop => fuzzer,
            BelKV::Mode(mode) => {
                let node = backend.egrid.node(loc);
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), "MAIN");
                }
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMode(site), mode)
            }
            BelKV::Unused => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMode(site), None)
            }
            BelKV::Attr(attr, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteAttr(site, attr), val)
            }
            BelKV::Pin(pin, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SitePin(site, pin), val)
            }
            BelKV::GlobalMutexHere(name) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::GlobalMutex(name), &site[..])
            }
            BelKV::RowMutexHere(name) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::RowMutex(name, loc.2), &site[..])
            }
            BelKV::Mutex(name, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMutex(&site[..], name), val)
            }
            BelKV::IsVref => match backend.edev {
                ExpandedDevice::Xc4k(_) => todo!(),
                ExpandedDevice::Xc5200(_) => todo!(),
                ExpandedDevice::Virtex(_) => todo!(),
                ExpandedDevice::Virtex2(edev) => {
                    let crd = prjcombine_virtex2::grid::IoCoord {
                        col: loc.1,
                        row: loc.2,
                        iob: prjcombine_virtex2::grid::TileIobId::from_idx(bel.to_idx()),
                    };
                    if !edev.grid.vref.contains(&crd) {
                        return None;
                    }
                    let bonded_ios = get_bonded_ios_v2(backend, &fuzzer);
                    if !bonded_ios.contains(&crd) {
                        return None;
                    }
                    fuzzer
                }
                ExpandedDevice::Spartan6(_) => todo!(),
                ExpandedDevice::Virtex4(_) => todo!(),
                ExpandedDevice::Ultrascale(_) => todo!(),
                ExpandedDevice::Versal(_) => todo!(),
            },
            BelKV::IsVr => match backend.edev {
                ExpandedDevice::Xc4k(_) => todo!(),
                ExpandedDevice::Xc5200(_) => todo!(),
                ExpandedDevice::Virtex(_) => todo!(),
                ExpandedDevice::Virtex2(edev) => {
                    let crd = prjcombine_virtex2::grid::IoCoord {
                        col: loc.1,
                        row: loc.2,
                        iob: prjcombine_virtex2::grid::TileIobId::from_idx(bel.to_idx()),
                    };
                    let mut is_vr = false;
                    for (bank, vr) in &edev.grid.dci_io {
                        if vr.0 == crd || vr.1 == crd {
                            if edev.grid.dci_io_alt.contains_key(bank) {
                                let &FuzzerValue::Base(Value::Bool(alt)) = &fuzzer.kv[&Key::AltVr]
                                else {
                                    unreachable!()
                                };
                                is_vr = !alt;
                            } else {
                                is_vr = true;
                            }
                        }
                    }
                    for (bank, vr) in &edev.grid.dci_io_alt {
                        if vr.0 == crd || vr.1 == crd {
                            if edev.grid.dci_io.contains_key(bank) {
                                let &FuzzerValue::Base(Value::Bool(alt)) = &fuzzer.kv[&Key::AltVr]
                                else {
                                    unreachable!()
                                };
                                is_vr = alt;
                            } else {
                                is_vr = true;
                            }
                        }
                    }
                    if !is_vr {
                        return None;
                    }
                    let bonded_ios = get_bonded_ios_v2(backend, &fuzzer);
                    if !bonded_ios.contains(&crd) {
                        return None;
                    }
                    fuzzer
                }
                ExpandedDevice::Spartan6(_) => todo!(),
                ExpandedDevice::Virtex4(_) => todo!(),
                ExpandedDevice::Ultrascale(_) => todo!(),
                ExpandedDevice::Versal(_) => todo!(),
            },
            BelKV::OtherIobInput(iostd) | BelKV::OtherIobDiffOutput(iostd) => {
                let is_diff = !matches!(*self, BelKV::OtherIobInput(_));
                let is_out = matches!(*self, BelKV::OtherIobDiffOutput(_));
                match backend.edev {
                    ExpandedDevice::Xc4k(_) => todo!(),
                    ExpandedDevice::Xc5200(_) => todo!(),
                    ExpandedDevice::Virtex(_) => todo!(),
                    ExpandedDevice::Virtex2(edev) => {
                        let bonded_ios = get_bonded_ios_v2(backend, &fuzzer);
                        let crd = prjcombine_virtex2::grid::IoCoord {
                            col: loc.1,
                            row: loc.2,
                            iob: prjcombine_virtex2::grid::TileIobId::from_idx(bel.to_idx()),
                        };
                        let orig_io = edev.get_io(crd);
                        for &io in &edev.bonded_ios {
                            if io != crd
                                && orig_io.bank == edev.get_io(io).bank
                                && edev.get_io(io).pad_kind != IoPadKind::Clk
                                && (!is_diff
                                    || edev.get_io(io).diff
                                        != prjcombine_virtex2::expanded::IoDiffKind::None)
                                && bonded_ios.contains(&io)
                            {
                                let node = backend
                                    .egrid
                                    .find_node(loc.0, (io.col, io.row), |node| {
                                        backend.egrid.db.nodes.key(node.kind).starts_with("IOI")
                                    })
                                    .unwrap();
                                let obel = BelId::from_idx(io.iob.to_idx());
                                let site = &node.bels[obel][..];

                                fuzzer = fuzzer.base(
                                    Key::SiteMode(site),
                                    if is_diff {
                                        match edev.get_io(io).diff {
                                            prjcombine_virtex2::expanded::IoDiffKind::P(_) => {
                                                if edev.grid.kind.is_spartan3a() {
                                                    "DIFFMI_NDT"
                                                } else if edev.grid.kind.is_spartan3ea() {
                                                    "DIFFMI"
                                                } else {
                                                    "DIFFM"
                                                }
                                            }
                                            prjcombine_virtex2::expanded::IoDiffKind::N(_) => {
                                                if edev.grid.kind.is_spartan3a() {
                                                    "DIFFSI_NDT"
                                                } else if edev.grid.kind.is_spartan3ea() {
                                                    "DIFFSI"
                                                } else {
                                                    "DIFFS"
                                                }
                                            }
                                            prjcombine_virtex2::expanded::IoDiffKind::None => {
                                                unreachable!()
                                            }
                                        }
                                    } else {
                                        if edev.grid.kind.is_spartan3ea() {
                                            "IBUF"
                                        } else {
                                            "IOB"
                                        }
                                    },
                                );
                                fuzzer = fuzzer.base(Key::SiteAttr(site, "IOATTRBOX"), iostd);
                                if !is_out {
                                    fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX"), "1");
                                    fuzzer = fuzzer.base(Key::SitePin(site, "I"), true);
                                    if edev.grid.kind.is_spartan3a() {
                                        fuzzer = fuzzer
                                            .base(Key::SiteAttr(site, "IBUF_DELAY_VALUE"), "DLY0");
                                        fuzzer = fuzzer.base(
                                            Key::SiteAttr(site, "DELAY_ADJ_ATTRBOX"),
                                            "FIXED",
                                        );
                                        fuzzer = fuzzer.base(Key::SiteAttr(site, "SEL_MUX"), "0");
                                    }
                                } else {
                                    fuzzer = fuzzer.base(Key::SiteAttr(site, "OMUX"), "O1");
                                    fuzzer = fuzzer.base(Key::SiteAttr(site, "O1INV"), "O1");
                                    fuzzer = fuzzer.base(Key::SitePin(site, "O1"), true);
                                }
                                return Some(fuzzer);
                            }
                        }
                        return None;
                    }
                    ExpandedDevice::Spartan6(_) => todo!(),
                    ExpandedDevice::Virtex4(_) => todo!(),
                    ExpandedDevice::Ultrascale(_) => todo!(),
                    ExpandedDevice::Versal(_) => todo!(),
                }
            }
            BelKV::BankDiffOutput(stda, stdb) => {
                match backend.edev {
                    ExpandedDevice::Xc4k(_) => todo!(),
                    ExpandedDevice::Xc5200(_) => todo!(),
                    ExpandedDevice::Virtex(_) => todo!(),
                    ExpandedDevice::Virtex2(edev) => {
                        let bonded_ios = get_bonded_ios_v2(backend, &fuzzer);
                        let crd = prjcombine_virtex2::grid::IoCoord {
                            col: loc.1,
                            row: loc.2,
                            iob: prjcombine_virtex2::grid::TileIobId::from_idx(bel.to_idx()),
                        };
                        let stds = if let Some(stdb) = stdb {
                            &[stda, stdb][..]
                        } else {
                            &[stda][..]
                        };
                        let bank = edev.get_io(crd).bank;
                        let mut done = 0;
                        let mut ios: Vec<_> = edev.bonded_ios.iter().collect();
                        if edev.grid.kind != prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                            ios.reverse();
                        }
                        for &io in ios {
                            if io == crd {
                                if edev.grid.kind.is_spartan3ea() {
                                    // too much thinking. just pick a different loc.
                                    return None;
                                } else {
                                    continue;
                                }
                            }
                            let io_info = edev.get_io(io);
                            if !bonded_ios.contains(&io)
                                || io_info.bank != bank
                                || io_info.pad_kind != IoPadKind::Io
                            {
                                continue;
                            }
                            let prjcombine_virtex2::expanded::IoDiffKind::P(other_iob) =
                                io_info.diff
                            else {
                                continue;
                            };
                            // okay, got a pair.
                            let node = backend
                                .egrid
                                .find_node(loc.0, (io.col, io.row), |node| {
                                    backend.egrid.db.nodes.key(node.kind).starts_with("IOI")
                                })
                                .unwrap();
                            let bel_p = BelId::from_idx(io.iob.to_idx());
                            let bel_n = BelId::from_idx(other_iob.to_idx());
                            let site_p = &node.bels[bel_p][..];
                            let site_n = &node.bels[bel_n][..];
                            let std = stds[done];
                            fuzzer = fuzzer
                                .base(
                                    Key::SiteMode(site_p),
                                    if edev.grid.kind.is_spartan3a() {
                                        "DIFFMTB"
                                    } else {
                                        "DIFFM"
                                    },
                                )
                                .base(
                                    Key::SiteMode(site_n),
                                    if edev.grid.kind.is_spartan3a() {
                                        "DIFFSTB"
                                    } else {
                                        "DIFFS"
                                    },
                                )
                                .base(Key::SiteAttr(site_p, "IOATTRBOX"), std)
                                .base(Key::SiteAttr(site_n, "IOATTRBOX"), std)
                                .base(Key::SiteAttr(site_p, "OMUX"), "O1")
                                .base(Key::SiteAttr(site_p, "O1INV"), "O1")
                                .base(Key::SitePin(site_p, "O1"), true)
                                .base(Key::SitePin(site_p, "DIFFO_OUT"), true)
                                .base(Key::SitePin(site_n, "DIFFO_IN"), true)
                                .base(Key::SiteAttr(site_n, "DIFFO_IN_USED"), "0");
                            if edev.grid.kind.is_spartan3a() {
                                fuzzer = fuzzer
                                    .base(Key::SiteAttr(site_p, "SUSPEND"), "3STATE")
                                    .base(Key::SiteAttr(site_n, "SUSPEND"), "3STATE");
                            }
                            done += 1;
                            if done == stds.len() {
                                break;
                            }
                        }
                        if done != stds.len() {
                            return None;
                        }
                        fuzzer
                    }
                    ExpandedDevice::Spartan6(_) => todo!(),
                    ExpandedDevice::Virtex4(_) => todo!(),
                    ExpandedDevice::Ultrascale(_) => todo!(),
                    ExpandedDevice::Versal(_) => todo!(),
                }
            }
            BelKV::NotIbuf => {
                let node = backend.egrid.node(loc);
                if !node.names[NodeRawTileId::from_idx(0)].contains("IOIS") {
                    return None;
                }
                fuzzer
            }
        })
    }
}

#[derive(Debug)]
pub enum TileFuzzKV<'a> {
    Bel(BelId, BelFuzzKV<'a>),
    IobBel(usize, BelId, BelFuzzKV<'a>),
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
    Raw(Key<'a>, Value<'a>, Value<'a>),
}

#[derive(Debug)]
pub enum BelFuzzKV<'a> {
    Mode(&'a str),
    ModeDiff(&'a str, &'a str),
    Attr(&'a str, &'a str),
    AttrDiff(&'a str, &'a str, &'a str),
    Pin(&'a str),
    PinFull(&'a str),
}

impl<'a> TileFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        let node = backend.egrid.node(loc);
        Some(match *self {
            TileFuzzKV::Bel(bel, ref inner) => inner.apply(backend, loc, bel, fuzzer)?,
            TileFuzzKV::IobBel(tile, bel, ref inner) => {
                let ioi_loc = find_ioi(backend, loc, tile);
                inner.apply(backend, ioi_loc, bel, fuzzer)?
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
                if let ExpandedDevice::Virtex4(edev) = backend.edev {
                    if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex4
                        && backend.egrid.db.wires.key(wa.1).starts_with("TEST")
                        && loc.1 == edev.col_cfg
                    {
                        // interference.
                        return None;
                    }
                }
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
            TileFuzzKV::Raw(ref key, ref vala, ref valb) => {
                fuzzer.fuzz(key.clone(), vala.clone(), valb.clone())
            }
        })
    }
}

impl<'a> BelFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        bel: BelId,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        let node = backend.egrid.node(loc);
        let node_data = &backend.egrid.db.nodes[node.kind];
        let node_naming = &backend.egrid.db.node_namings[node.naming];
        Some(match *self {
            BelFuzzKV::Mode(val) => {
                let node = backend.egrid.node(loc);
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), "MAIN");
                }
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteMode(site), None, val)
            }
            BelFuzzKV::ModeDiff(vala, valb) => {
                let node = backend.egrid.node(loc);
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), "MAIN");
                }
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteMode(site), vala, valb)
            }
            BelFuzzKV::Attr(attr, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteAttr(site, attr), None, val)
            }
            BelFuzzKV::AttrDiff(attr, va, vb) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteAttr(site, attr), va, vb)
            }
            BelFuzzKV::Pin(pin) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SitePin(site, pin), false, true)
            }
            BelFuzzKV::PinFull(pin) => {
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
        })
    }
}

#[derive(Debug)]
pub enum TileMultiFuzzKV<'a> {
    SiteAttr(BelId, &'a str, MultiValue),
    IobSiteAttr(usize, BelId, &'a str, MultiValue),
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
            TileMultiFuzzKV::IobSiteAttr(tile, bel, attr, val) => {
                let ioi_loc = find_ioi(backend, loc, tile);
                let site = &backend.egrid.node(ioi_loc).bels[bel];
                fuzzer.fuzz_multi(Key::SiteAttr(site, attr), val)
            }
            TileMultiFuzzKV::GlobalOpt(attr, val) => fuzzer.fuzz_multi(Key::GlobalOpt(attr), val),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TileBits {
    Null,
    Main(usize),
    Reg(Reg),
    Raw(Vec<BitTile>),
    MainUp,
    MainDown,
    Bram,
    Spine(usize),
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
    Cfg,
    CfgReg(Reg),
    Iob(usize),
    TestLL,
    RandorLeft,
    MainAuto,
    ClkIob,
    ClkHrow,
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
            TileBits::Reg(reg) => vec![BitTile::Reg(die, reg)],
            TileBits::Raw(ref raw) => raw.clone(),
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
                ExpandedDevice::Spartan6(_edev) => {
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
            TileBits::Spine(n) => match backend.edev {
                ExpandedDevice::Virtex2(edev) => {
                    (0..n).map(|idx| edev.btile_spine(row + idx)).collect()
                }
                ExpandedDevice::Virtex4(edev) => {
                    (0..n).map(|idx| edev.btile_spine(die, row + idx)).collect()
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
                    prjcombine_virtex4::grid::GridKind::Virtex4 => {
                        let mut res = vec![];
                        for i in 0..24 {
                            res.push(edev.btile_main(die, col, row + i));
                        }
                        for i in 0..24 {
                            res.push(edev.btile_main(die, col + 8, row + i));
                        }
                        for i in 1..8 {
                            res.push(edev.btile_main(die, col + i, row));
                        }
                        for i in 1..8 {
                            res.push(edev.btile_main(die, col + i, row + 23));
                        }
                        res
                    }
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
                ExpandedDevice::Virtex4(edev) => {
                    vec![edev.btile_hclk(die, col, row)]
                }
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
            TileBits::Cfg => match backend.edev {
                ExpandedDevice::Xc4k(_) | ExpandedDevice::Xc5200(_) | ExpandedDevice::Virtex(_) => {
                    unreachable!()
                }
                ExpandedDevice::Virtex2(edev) => {
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
                ExpandedDevice::Spartan6(_) => todo!(),
                ExpandedDevice::Virtex4(edev) => match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => {
                        let mut res = vec![];
                        for i in 0..16 {
                            res.push(edev.btile_main(die, col, row + i));
                        }
                        for i in 0..16 {
                            res.push(edev.btile_spine(die, row + i));
                        }
                        // hmmmmm.
                        res.push(edev.btile_spine(die, RowId::from_idx(0)));
                        res.push(edev.btile_spine(
                            die,
                            RowId::from_idx(
                                edev.grids[die].regs * edev.grids[die].rows_per_reg() - 1,
                            ),
                        ));
                        res
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex6 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
                },
                ExpandedDevice::Ultrascale(_) => todo!(),
                ExpandedDevice::Versal(_) => todo!(),
            },
            TileBits::CfgReg(reg) => {
                let mut res = TileBits::Cfg.get_bits(backend, loc);
                res.push(BitTile::Reg(DieId::from_idx(0), reg));
                res
            }
            TileBits::TestLL => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![
                    edev.btile_lrterm(edev.grid.col_left(), edev.grid.row_top()),
                    edev.btile_btterm(edev.grid.col_left(), edev.grid.row_top()),
                    edev.btile_lrterm(edev.grid.col_right(), edev.grid.row_top()),
                    edev.btile_btterm(edev.grid.col_right(), edev.grid.row_top()),
                ]
            }
            TileBits::RandorLeft => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_main(col, row), edev.btile_main(col - 1, row)]
            }
            TileBits::Iob(n) => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if col == edev.grid.col_left() || col == edev.grid.col_right() {
                    (0..n)
                        .map(|idx| edev.btile_lrterm(col, row + idx))
                        .chain((0..n).map(|idx| edev.btile_main(col, row + idx)))
                        .collect()
                } else {
                    (0..n)
                        .map(|idx| edev.btile_btterm(col + idx, row))
                        .chain((0..n).map(|idx| edev.btile_main(col + idx, row)))
                        .collect()
                }
            }
            TileBits::MainAuto => {
                let node = backend.egrid.node(loc);
                match backend.edev {
                    ExpandedDevice::Xc4k(_) => todo!(),
                    ExpandedDevice::Xc5200(_) => todo!(),
                    ExpandedDevice::Virtex(_) => todo!(),
                    ExpandedDevice::Virtex2(edev) => node
                        .tiles
                        .values()
                        .map(|&(col, row)| edev.btile_main(col, row))
                        .collect(),
                    ExpandedDevice::Spartan6(edev) => node
                        .tiles
                        .values()
                        .map(|&(col, row)| edev.btile_main(col, row))
                        .collect(),
                    ExpandedDevice::Virtex4(edev) => node
                        .tiles
                        .values()
                        .map(|&(col, row)| edev.btile_main(die, col, row))
                        .collect(),
                    ExpandedDevice::Ultrascale(_) => todo!(),
                    ExpandedDevice::Versal(_) => todo!(),
                }
            }
            TileBits::ClkIob => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => {
                        let mut res = vec![];
                        for i in 0..16 {
                            res.push(edev.btile_spine(die, row + i));
                        }
                        if row < edev.grids[die].row_bufg() {
                            res.push(edev.btile_spine(die, RowId::from_idx(0)))
                        } else {
                            res.push(edev.btile_spine(die, edev.grids[die].rows().last().unwrap()))
                        }
                        res
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex6
                    | prjcombine_virtex4::grid::GridKind::Virtex7 => unreachable!(),
                }
            }
            TileBits::ClkHrow => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => {
                        vec![
                            edev.btile_spine(die, row - 1),
                            edev.btile_spine(die, row),
                            edev.btile_spine_hclk(die, row),
                            edev.btile_hclk(die, ColId::from_idx(0), row),
                            edev.btile_hclk(die, edev.grids[die].columns.last_id().unwrap(), row),
                        ]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex6
                    | prjcombine_virtex4::grid::GridKind::Virtex7 => unreachable!(),
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

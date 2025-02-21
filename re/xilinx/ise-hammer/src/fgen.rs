use prjcombine_interconnect::{
    db::{BelId, Dir, NodeKindId, NodeTileId, NodeWireId},
    grid::{ColId, DieId, IntWire, LayerId, NodeLoc, RowId, TileIobId},
};
use prjcombine_re_collector::{FeatureId, State};
use prjcombine_re_xilinx_geom::{ExpandedBond, ExpandedDevice, ExpandedNamedDevice};
use prjcombine_re_xilinx_naming::db::{IntfWireInNaming, IntfWireOutNaming, NodeRawTileId};
use prjcombine_virtex2::iob::IobKind;
use prjcombine_xilinx_bitstream::{BitTile, Reg};
use rand::prelude::*;
use std::collections::{BTreeSet, HashMap};
use unnamed_entity::EntityId;

use prjcombine_re_hammer::{BatchValue, Fuzzer, FuzzerGen, FuzzerValue};

use crate::backend::{FuzzerFeature, FuzzerInfo, IseBackend, Key, MultiValue, PinFromKind, Value};

#[derive(Debug, Clone)]
pub enum TileWire {
    IntWire(NodeWireId),
    BelPinNear(BelId, String),
    BelPinFar(BelId, String),
    RelatedBelPinNear(BelId, BelRelation, String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TileRelation {
    ClbTbusRight,
    ClbCinDown,
    IoiBrefclk,
    IobBrefclkClkBT,
    ClkIob(Dir),
    ClkDcm,
    ClkHrow(isize),
    Rclk,
    Ioclk(Dir),
    Cfg,
    HclkDcm,
    Mgt(Dir),
    Delta(isize, isize, NodeKindId),
    ColPair(isize, NodeKindId),
    Hclk(NodeKindId),
    ClkRebuf(Dir),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BelRelation {
    Rclk,
    Ioclk(Dir),
}

fn resolve_tile_relation(
    backend: &IseBackend,
    mut loc: NodeLoc,
    relation: TileRelation,
) -> Option<NodeLoc> {
    match relation {
        TileRelation::ClbTbusRight => loop {
            if loc.1 == backend.egrid.die(loc.0).cols().next_back().unwrap() {
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
            if !matches!(backend.edev, ExpandedDevice::Spartan6(_)) {
                return None;
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
        TileRelation::ClkIob(dir) => {
            let ExpandedDevice::Virtex4(edev) = backend.edev else {
                unreachable!()
            };
            match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    loc.2 = match dir {
                        Dir::N => edev.row_iobdcm.unwrap() - 16,
                        Dir::S => edev.row_dcmiob.unwrap(),
                        _ => unreachable!(),
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
        TileRelation::ClkHrow(delta) => match backend.edev {
            ExpandedDevice::Virtex4(edev) => {
                loc.1 = edev.col_clk;
                loc.2 += delta;
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
            _ => todo!(),
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
        TileRelation::HclkDcm => match backend.edev {
            ExpandedDevice::Virtex4(edev) => {
                loc.1 = edev.col_clk;
                loc.2 = edev.grids[loc.0].row_hclk(loc.2);
                let Some((layer, _)) = backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                    backend.egrid.db.nodes.key(node.kind) == "HCLK_DCM"
                }) else {
                    unreachable!()
                };
                loc.3 = layer;
                Some(loc)
            }
            _ => todo!(),
        },
        TileRelation::Mgt(dir) => {
            let ExpandedDevice::Virtex4(edev) = backend.edev else {
                unreachable!()
            };
            match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    match dir {
                        Dir::S => {
                            if loc.2.to_idx() == 0 {
                                return None;
                            }
                            loc.2 -= 32
                        }
                        Dir::N => {
                            loc.2 += 32;
                            if loc.2.to_idx() >= edev.grids[loc.0].rows().len() {
                                return None;
                            }
                        }
                        _ => unreachable!(),
                    }
                    let Some((layer, _)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                            backend.egrid.db.nodes.key(node.kind) == "MGT"
                        })
                    else {
                        unreachable!()
                    };
                    loc.3 = layer;
                    Some(loc)
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex6 => todo!(),
                prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
            }
        }
        TileRelation::Delta(dx, dy, kind) => {
            if dx < 0 {
                if loc.1.to_idx() < (-dx) as usize {
                    return None;
                }
                loc.1 -= (-dx) as usize;
            } else {
                loc.1 += dx as usize;
                if loc.1.to_idx() >= backend.egrid.die(loc.0).cols().len() {
                    return None;
                }
            }
            if dy < 0 {
                if loc.2.to_idx() < (-dy) as usize {
                    return None;
                }
                loc.2 -= (-dy) as usize;
            } else {
                loc.2 += dy as usize;
                if loc.2.to_idx() >= backend.egrid.die(loc.0).rows().len() {
                    return None;
                }
            }
            let (layer, _) = backend
                .egrid
                .find_node_loc(loc.0, (loc.1, loc.2), |node| node.kind == kind)?;
            loc.3 = layer;
            Some(loc)
        }
        TileRelation::ColPair(dy, kind) => {
            if loc.1.to_idx() % 2 == 0 {
                loc.1 += 1;
            } else {
                loc.1 -= 1;
            }
            if dy < 0 {
                if loc.2.to_idx() < (-dy) as usize {
                    return None;
                }
                loc.2 -= (-dy) as usize;
            } else {
                loc.2 += dy as usize;
                if loc.2.to_idx() >= backend.egrid.die(loc.0).rows().len() {
                    return None;
                }
            }
            let (layer, _) = backend
                .egrid
                .find_node_loc(loc.0, (loc.1, loc.2), |node| node.kind == kind)?;
            loc.3 = layer;
            Some(loc)
        }
        TileRelation::Hclk(kind) => {
            let ExpandedDevice::Virtex4(edev) = backend.edev else {
                unreachable!()
            };
            loc.2 = edev.grids[loc.0].row_reg_hclk(edev.grids[loc.0].row_to_reg(loc.2));
            let (layer, _) = backend
                .egrid
                .find_node_loc(loc.0, (loc.1, loc.2), |node| node.kind == kind)?;
            loc.3 = layer;
            Some(loc)
        }
        TileRelation::ClkRebuf(dir) => loop {
            match dir {
                Dir::S => {
                    if loc.2.to_idx() == 0 {
                        if loc.0.to_idx() == 0 {
                            return None;
                        }
                        loc.0 -= 1;
                        loc.2 = backend.egrid.die(loc.0).rows().next_back().unwrap();
                    } else {
                        loc.2 -= 1;
                    }
                }
                Dir::N => {
                    if loc.2 == backend.egrid.die(loc.0).rows().next_back().unwrap() {
                        loc.2 = RowId::from_idx(0);
                        loc.0 += 1;
                        if loc.0 == backend.egrid.die.next_id() {
                            return None;
                        }
                    } else {
                        loc.2 += 1;
                    }
                }
                _ => unreachable!(),
            }
            if let Some((layer, _)) = backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                matches!(
                    &backend.egrid.db.nodes.key(node.kind)[..],
                    "CLK_BUFG_REBUF" | "CLK_BALI_REBUF"
                )
            }) {
                loc.3 = layer;
                return Some(loc);
            }
        },
    }
}

fn resolve_bel_relation(
    backend: &IseBackend,
    mut loc: NodeLoc,
    _bel: BelId,
    relation: BelRelation,
) -> Option<(NodeLoc, BelId)> {
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
                    let bel = backend.egrid.db.nodes[node.kind]
                        .bels
                        .get("RCLK")
                        .unwrap()
                        .0;
                    Some((loc, bel))
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    loc.1 = if loc.1 <= edev.col_clk {
                        edev.col_lio?
                    } else {
                        edev.col_rio?
                    };
                    let Some((layer, node)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                            backend.egrid.db.nodes.key(node.kind) == "HCLK_IOI"
                        })
                    else {
                        unreachable!()
                    };
                    loc.3 = layer;
                    let bel = backend.egrid.db.nodes[node.kind]
                        .bels
                        .get("IOCLK")
                        .unwrap()
                        .0;
                    Some((loc, bel))
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    loc.1 = edev.col_clk;
                    let Some((layer, node)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |node| {
                            backend.egrid.db.nodes.key(node.kind) == "CMT"
                        })
                    else {
                        unreachable!()
                    };
                    loc.3 = layer;
                    let bel = backend.egrid.db.nodes[node.kind].bels.get("CMT").unwrap().0;
                    Some((loc, bel))
                }
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
                    let bel = backend.egrid.db.nodes[node.kind]
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

fn find_ioi(backend: &IseBackend, loc: NodeLoc, tile: usize) -> NodeLoc {
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
    loc: NodeLoc,
    wire: &TileWire,
) -> Option<(&'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let ndb = backend.ngrid.db;
    let nnode = &backend.ngrid.nodes[&loc];
    let node_naming = &ndb.node_namings[nnode.naming];
    Some(match wire {
        TileWire::IntWire(wire) => {
            backend
                .egrid
                .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
            (
                &nnode.names[NodeRawTileId::from_idx(0)],
                node_naming.wires.get(wire)?,
            )
        }
        TileWire::BelPinNear(bel, pin) => {
            let bel_naming = &node_naming.bels[*bel];
            (
                &nnode.names[bel_naming.tile],
                &bel_naming
                    .pins
                    .get(pin)
                    .unwrap_or_else(|| {
                        panic!(
                            "missing pin {pin} in bel {bel} tile {tile}",
                            bel = backend.egrid.db.nodes[node.kind].bels.key(*bel),
                            tile = backend.egrid.db.nodes.key(node.kind),
                        )
                    })
                    .name,
            )
        }
        TileWire::BelPinFar(bel, pin) => {
            let bel_naming = &node_naming.bels[*bel];
            (
                &nnode.names[bel_naming.tile],
                &bel_naming
                    .pins
                    .get(pin)
                    .unwrap_or_else(|| {
                        panic!(
                            "missing pin {pin} in bel {bel} tile {tile}",
                            bel = backend.egrid.db.nodes[node.kind].bels.key(*bel),
                            tile = backend.egrid.db.nodes.key(node.kind),
                        )
                    })
                    .name_far,
            )
        }
        TileWire::RelatedBelPinNear(bel, relation, pin) => {
            let (loc, bel) = resolve_bel_relation(backend, loc, *bel, *relation)?;
            let nnode = &backend.ngrid.nodes[&loc];
            let node_naming = &ndb.node_namings[nnode.naming];
            let bel_naming = &node_naming.bels[bel];
            (&nnode.names[bel_naming.tile], &bel_naming.pins[pin].name)
        }
    })
}

fn resolve_int_pip<'a>(
    backend: &IseBackend<'a>,
    loc: NodeLoc,
    wire_from: NodeWireId,
    wire_to: NodeWireId,
) -> Option<(&'a str, &'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let nnode = &backend.ngrid.nodes[&loc];
    let ndb = backend.ngrid.db;
    let node_naming = &ndb.node_namings[nnode.naming];
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire_to.0], wire_to.1))?;
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire_from.0], wire_from.1))?;
    Some(
        if let Some(ext) = node_naming.ext_pips.get(&(wire_to, wire_from)) {
            (&nnode.names[ext.tile], &ext.wire_from, &ext.wire_to)
        } else {
            (
                &nnode.names[NodeRawTileId::from_idx(0)],
                node_naming.wires.get(&wire_from)?,
                node_naming.wires.get(&wire_to)?,
            )
        },
    )
}

fn resolve_intf_test_pip<'a>(
    backend: &IseBackend<'a>,
    loc: NodeLoc,
    wire_from: NodeWireId,
    wire_to: NodeWireId,
) -> Option<(&'a str, &'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let nnode = &backend.ngrid.nodes[&loc];
    let intdb = backend.egrid.db;
    let ndb = backend.ngrid.db;
    let node_naming = &ndb.node_namings[nnode.naming];
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire_to.0], wire_to.1))?;
    backend
        .egrid
        .resolve_wire((loc.0, node.tiles[wire_from.0], wire_from.1))?;
    if let ExpandedDevice::Virtex4(edev) = backend.edev {
        if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex5
            && ndb.node_namings.key(nnode.naming) == "INTF.PPC_R"
            && intdb.wires.key(wire_from.1).starts_with("TEST")
        {
            // ISE.
            return None;
        }
    }
    Some((
        &nnode.names[NodeRawTileId::from_idx(0)],
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
    loc: NodeLoc,
    wire: NodeWireId,
) -> Option<(&'a str, &'a str, &'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let nnode = &backend.ngrid.nodes[&loc];
    let ndb = backend.ngrid.db;
    let node_naming = &ndb.node_namings[nnode.naming];
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
        &nnode.names[NodeRawTileId::from_idx(0)],
        name_in,
        name_delay,
        name_out,
    ))
}

#[derive(Debug, Clone)]
pub enum TileKV<'a> {
    Nop,
    Bel(BelId, BelKV),
    IobBel(usize, BelId, BelKV),
    Package(String),
    AltVr(bool),
    GlobalOpt(String, String),
    NoGlobalOpt(String),
    VccAux(String),
    GlobalMutexNone(String),
    GlobalMutex(String, String),
    RowMutex(String, String),
    TileMutex(String, String),
    Pip(TileWire, TileWire),
    IntPip(NodeWireId, NodeWireId),
    NodeMutexShared(NodeWireId),
    IntMutexShared(String),
    NodeIntDstFilter(NodeWireId),
    NodeIntSrcFilter(NodeWireId),
    NodeIntDistinct(NodeWireId, NodeWireId),
    DriveLLH(NodeWireId),
    DriveLLV(NodeWireId),
    StabilizeGclkc,
    IsLeftRandor(bool),
    HclkHasDcm(Dir),
    HclkHasInt(Dir),
    HclkHasCmt,
    Raw(Key<'a>, Value<'a>),
    TileRelated(TileRelation, Box<TileKV<'a>>),
    NoTileRelated(TileRelation),
    VirtexPinBramLv(NodeWireId),
    VirtexPinLh(NodeWireId),
    VirtexPinIoLh(NodeWireId),
    VirtexPinHexH(NodeWireId),
    VirtexPinHexV(NodeWireId),
    VirtexDriveHexH(NodeWireId),
    VirtexDriveHexV(NodeWireId),
    Xc4000BiPip(NodeWireId, NodeWireId),
    Xc4000DoublePip(NodeWireId, NodeWireId, NodeWireId),
    DeviceSide(Dir),
    HclkSide(Dir),
    CenterDci(u32),
    CascadeDci(u32, u32),
    TouchHout(usize),
    PinPair(BelId, String, BelId, String),
}

#[derive(Debug, Clone)]
pub enum BelKV {
    Nop,
    Mode(String),
    Unused,
    Attr(String, String),
    AttrAny(String, Vec<String>),
    Global(BelGlobalKind, String, String),
    Pin(String, bool),
    PinFrom(String, PinFromKind),
    PinPips(String),
    PinNodeMutexShared(String),
    GlobalMutexHere(String),
    RowMutexHere(String),
    Mutex(String, String),
    IsBonded,
    IsBank(u32),
    IsDiff,
    IsVref,
    IsVr,
    PrepVref,
    PrepVrefInternal(u32),
    PrepDci,
    PrepDiffOut,
    OtherIobInput(String),
    OtherIobDiffOutput(String),
    BankDiffOutput(String, Option<String>),
    NotIbuf,
    VirtexIsDllIob(bool),
    Xc4000TbufSplitter(Dir, bool),
    Xc4000DriveImux(&'static str, bool),
}

impl<'a> TileKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        Some(match self {
            TileKV::Nop => fuzzer,
            TileKV::Bel(bel, inner) => inner.apply(backend, loc, *bel, fuzzer)?,
            TileKV::IobBel(tile, bel, inner) => {
                let ioi_loc = find_ioi(backend, loc, *tile);
                inner.apply(backend, ioi_loc, *bel, fuzzer)?
            }
            TileKV::Package(pkg) => fuzzer.base(Key::Package, pkg.clone()),
            TileKV::AltVr(alt) => fuzzer.base(Key::AltVr, *alt),
            TileKV::GlobalOpt(opt, val) => fuzzer.base(Key::GlobalOpt(opt.clone()), val),
            TileKV::NoGlobalOpt(opt) => fuzzer.base(Key::GlobalOpt(opt.clone()), None),
            TileKV::VccAux(val) => fuzzer.base(Key::VccAux, val),
            TileKV::GlobalMutexNone(name) => fuzzer.base(Key::GlobalMutex(name.clone()), None),
            TileKV::GlobalMutex(name, val) => fuzzer.base(Key::GlobalMutex(name.clone()), val),
            TileKV::RowMutex(name, val) => fuzzer.base(Key::RowMutex(name.clone(), loc.2), val),
            TileKV::TileMutex(name, val) => fuzzer.base(Key::TileMutex(loc, name.clone()), val),
            TileKV::Pip(wa, wb) => {
                let (ta, wa) = resolve_tile_wire(backend, loc, wa)?;
                let (tb, wb) = resolve_tile_wire(backend, loc, wb)?;
                assert_eq!(ta, tb);
                fuzzer.base(Key::Pip(ta, wa, wb), true)
            }
            TileKV::IntPip(wa, wb) => {
                let (tile, wa, wb) = resolve_int_pip(backend, loc, *wa, *wb)?;
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
                let ndb = backend.ngrid.db;
                let wire_name = intdb.wires.key(wire.1);
                match backend.edev {
                    ExpandedDevice::Virtex2(edev) => {
                        let node = backend.egrid.node(loc);
                        let nnode = &backend.ngrid.nodes[&loc];
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
                            && ndb.node_namings.key(nnode.naming).starts_with("INT.MACC")
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
                let ndb = backend.ngrid.db;
                let wire_name = intdb.wires.key(wire.1);
                let node = backend.egrid.node(loc);
                let nnode = &backend.ngrid.nodes[&loc];
                #[allow(clippy::single_match)]
                match backend.edev {
                    ExpandedDevice::Virtex2(edev) => {
                        if (edev.grid.kind.is_virtex2()
                            || edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3)
                            && wire_name.starts_with("OUT")
                            && intdb.nodes.key(node.kind).starts_with("INT.DCM")
                        {
                            let (layer, _) = backend
                                .egrid
                                .find_node_loc(loc.0, (loc.1, loc.2), |node| {
                                    intdb.nodes.key(node.kind).starts_with("DCM.")
                                })
                                .unwrap();
                            let ndcm = &backend.ngrid.nodes[&(loc.0, loc.1, loc.2, layer)];
                            let site = &ndcm.bels[BelId::from_idx(0)];
                            fuzzer = fuzzer.base(Key::SiteMode(site), "DCM").base(
                                Key::BelMutex(
                                    (loc.0, loc.1, loc.2, layer, BelId::from_idx(0)),
                                    "MODE".into(),
                                ),
                                "INT",
                            );
                            for pin in [
                                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
                                "CLKFX", "CLKFX180", "CONCUR", "STATUS1", "STATUS7",
                            ] {
                                fuzzer = fuzzer.base(Key::SitePin(site, pin.into()), true);
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
                                &ndb.node_namings.key(nnode.naming)[..],
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
            TileKV::DriveLLH(wire) => match backend.edev {
                ExpandedDevice::Xc2000(edev) => {
                    assert_eq!(edev.grid.kind, prjcombine_xc2000::grid::GridKind::Xc5200);
                    let node = backend.egrid.node(loc);
                    let wnode = backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                    let mut src_col = if node.tiles[wire.0].0 < edev.grid.col_mid() {
                        edev.grid.col_mid() - 1
                    } else {
                        edev.grid.col_mid()
                    };
                    loop {
                        if let Some((src_layer, src_node)) =
                            backend
                                .egrid
                                .find_node_loc(loc.0, (src_col, loc.2), |src_node| {
                                    backend.egrid.db.nodes.key(src_node.kind).starts_with("IO")
                                        || backend.egrid.db.nodes.key(src_node.kind) == "CLB"
                                })
                        {
                            let dwire = (NodeTileId::from_idx(0), wire.1);
                            let src_node_kind = &backend.egrid.db.nodes[src_node.kind];
                            if let Some(mux) = src_node_kind.muxes.get(&dwire) {
                                let Some(dnode) = backend.egrid.resolve_wire((
                                    loc.0,
                                    src_node.tiles[dwire.0],
                                    dwire.1,
                                )) else {
                                    continue;
                                };
                                assert_eq!(dnode, wnode);
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
                        if src_col == edev.grid.col_lio() || src_col == edev.grid.col_rio() {
                            return None;
                        }
                        if src_col < edev.grid.col_mid() {
                            src_col -= 1;
                        } else {
                            src_col += 1;
                        }
                    }
                }
                ExpandedDevice::Virtex2(edev) => {
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
                _ => todo!(),
            },
            TileKV::DriveLLV(wire) => match backend.edev {
                ExpandedDevice::Xc2000(edev) => {
                    assert_eq!(edev.grid.kind, prjcombine_xc2000::grid::GridKind::Xc5200);
                    let node = backend.egrid.node(loc);
                    let wnode = backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                    let mut src_row = if node.tiles[wire.0].1 < edev.grid.row_mid() {
                        edev.grid.row_mid() - 1
                    } else {
                        edev.grid.row_mid()
                    };
                    loop {
                        if let Some((src_layer, src_node)) =
                            backend
                                .egrid
                                .find_node_loc(loc.0, (loc.1, src_row), |src_node| {
                                    backend.egrid.db.nodes.key(src_node.kind).starts_with("IO")
                                        || backend.egrid.db.nodes.key(src_node.kind) == "CLB"
                                })
                        {
                            let dwire = (NodeTileId::from_idx(0), wire.1);
                            let src_node_kind = &backend.egrid.db.nodes[src_node.kind];
                            if let Some(mux) = src_node_kind.muxes.get(&dwire) {
                                let Some(dnode) = backend.egrid.resolve_wire((
                                    loc.0,
                                    src_node.tiles[dwire.0],
                                    dwire.1,
                                )) else {
                                    continue;
                                };
                                assert_eq!(dnode, wnode);
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
                        if src_row == edev.grid.row_bio() || src_row == edev.grid.row_tio() {
                            return None;
                        }
                        if src_row < edev.grid.row_mid() {
                            src_row -= 1;
                        } else {
                            src_row += 1;
                        }
                    }
                }
                ExpandedDevice::Virtex2(edev) => {
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
                _ => todo!(),
            },
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
                            fuzzer = TileKV::TileMutex(o.into(), i.into())
                                .apply(backend, loc, fuzzer)?;
                            fuzzer = TileKV::Pip(
                                TileWire::BelPinNear(bel, i.into()),
                                TileWire::BelPinNear(bel, o.into()),
                            )
                            .apply(backend, loc, fuzzer)?;
                        }
                    }
                }
                fuzzer
            }
            TileKV::IsLeftRandor(val) => {
                if *val != (loc.1.to_idx() == 1) {
                    return None;
                }
                if loc.2.to_idx() == 0 {
                    return None;
                }
                fuzzer
            }
            TileKV::HclkHasDcm(dir) => {
                let row = match dir {
                    Dir::N => loc.2,
                    Dir::S => loc.2 - 8,
                    _ => unreachable!(),
                };
                backend.egrid.find_node(loc.0, (loc.1, row), |node| {
                    matches!(&backend.egrid.db.nodes.key(node.kind)[..], "CCM" | "DCM")
                })?;
                fuzzer
            }
            TileKV::HclkHasInt(dir) => {
                let row = match dir {
                    Dir::N => loc.2,
                    Dir::S => loc.2 - 1,
                    _ => unreachable!(),
                };
                backend.egrid.find_node(loc.0, (loc.1, row), |node| {
                    backend.egrid.db.nodes.key(node.kind).starts_with("INT")
                })?;
                fuzzer
            }
            TileKV::HclkHasCmt => {
                let ExpandedDevice::Spartan6(edev) = backend.edev else {
                    unreachable!()
                };
                if loc.2 == edev.grid.row_clk() {
                    return None;
                }
                fuzzer
            }
            TileKV::Raw(key, val) => fuzzer.base(key.clone(), val.clone()),
            TileKV::TileRelated(relation, chain) => {
                let loc = resolve_tile_relation(backend, loc, *relation)?;
                chain.apply(backend, loc, fuzzer)?
            }
            TileKV::NoTileRelated(relation) => {
                match resolve_tile_relation(backend, loc, *relation) {
                    Some(_) => return None,
                    None => fuzzer,
                }
            }
            TileKV::VirtexPinBramLv(wire) => {
                let node = backend.egrid.node(loc);
                let wire = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let mut loc = loc;
                loc.2 = RowId::from_idx(1);
                loc.3 = LayerId::from_idx(0);
                for i in 0..12 {
                    let wire_pin = (
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("LV.{i}")),
                    );

                    let resolved_pin = backend
                        .egrid
                        .resolve_wire((loc.0, (loc.1, loc.2), wire_pin.1))
                        .unwrap();
                    let wire_clk = (
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire("IMUX.BRAM.CLKA"),
                    );
                    let resolved_clk = backend
                        .egrid
                        .resolve_wire((loc.0, (loc.1, loc.2), wire_clk.1))
                        .unwrap();
                    if resolved_pin == wire {
                        let (tile, wa, wb) =
                            resolve_int_pip(backend, loc, wire_pin, wire_clk).unwrap();
                        fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                        fuzzer = fuzzer.fuzz(Key::NodeMutex(resolved_clk), None, "EXCLUSIVE");
                        return Some(fuzzer);
                    }
                }
                panic!("UMM FAILED TO PIN BRAM LV");
            }
            TileKV::VirtexPinLh(wire) => {
                let node = backend.egrid.node(loc);
                let resolved_wire =
                    backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let mut loc = (loc.0, ColId::from_idx(0), node.tiles[wire.0].1, loc.3);
                let (layer, node) = backend
                    .egrid
                    .find_node_loc(loc.0, (loc.1, loc.2), |n| {
                        backend.egrid.db.nodes.key(n.kind) == "IO.L"
                    })
                    .unwrap();
                loc.3 = layer;
                let node_data = &backend.egrid.db.nodes[node.kind];
                for i in 0..12 {
                    let wire_pin = (
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("LH.{i}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire((loc.0, (loc.1, loc.2), wire_pin.1))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for (&wire_out, mux_data) in &node_data.muxes {
                        if mux_data.ins.contains(&wire_pin) {
                            // FOUND
                            let resolved_out = backend
                                .egrid
                                .resolve_wire((loc.0, (loc.1, loc.2), wire_out.1))
                                .unwrap();
                            let (tile, wa, wb) =
                                resolve_int_pip(backend, loc, wire_pin, wire_out).unwrap();
                            fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                            fuzzer = fuzzer.fuzz(Key::NodeMutex(resolved_out), None, "EXCLUSIVE");
                            return Some(fuzzer);
                        }
                    }
                }
                unreachable!()
            }
            TileKV::VirtexPinIoLh(wire) => {
                let node = backend.egrid.node(loc);
                let resolved_wire =
                    backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let mut loc = (loc.0, ColId::from_idx(0), node.tiles[wire.0].1, loc.3);
                loop {
                    if let Some((layer, _)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |n| {
                            matches!(&backend.egrid.db.nodes.key(n.kind)[..], "IO.B" | "IO.T")
                        })
                    {
                        loc.3 = layer;
                        for i in [0, 6] {
                            let wire_pin = (
                                NodeTileId::from_idx(0),
                                backend.egrid.db.get_wire(&format!("LH.{i}")),
                            );
                            let resolved_pin = backend
                                .egrid
                                .resolve_wire((loc.0, (loc.1, loc.2), wire_pin.1))
                                .unwrap();
                            if resolved_pin != resolved_wire {
                                continue;
                            }
                            // FOUND
                            let wire_buf = (
                                NodeTileId::from_idx(0),
                                backend.egrid.db.get_wire(&format!("LH.{i}.FAKE")),
                            );
                            let resolved_buf = backend
                                .egrid
                                .resolve_wire((loc.0, (loc.1, loc.2), wire_buf.1))
                                .unwrap();
                            let (tile, wa, wb) =
                                resolve_int_pip(backend, loc, wire_pin, wire_buf).unwrap();
                            fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                            fuzzer = fuzzer.fuzz(Key::NodeMutex(resolved_buf), None, "EXCLUSIVE");
                            return Some(fuzzer);
                        }
                    }
                    loc.1 += 1;
                }
            }
            TileKV::VirtexPinHexH(wire) => {
                let node = backend.egrid.node(loc);
                let resolved_wire =
                    backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let wire_name = backend.egrid.db.wires.key(wire.1);
                let h = wire_name[4..5].chars().next().unwrap();
                let i: usize = wire_name[5..6].parse().unwrap();
                let mut loc = (loc.0, node.tiles[wire.0].0, node.tiles[wire.0].1, loc.3);
                if loc.1.to_idx() >= 8 {
                    loc.1 -= 8;
                } else {
                    loc.1 = ColId::from_idx(0)
                };
                loop {
                    if let Some((layer, node)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |n| {
                            matches!(
                                &backend.egrid.db.nodes.key(n.kind)[..],
                                "IO.L" | "IO.R" | "IO.B" | "IO.T" | "CLB" | "CNR.BR" | "CNR.TR"
                            )
                        })
                    {
                        loc.3 = layer;
                        let node_data = &backend.egrid.db.nodes[node.kind];
                        for j in 0..=6 {
                            let wire_pin = (
                                NodeTileId::from_idx(0),
                                backend.egrid.db.get_wire(&format!("HEX.{h}{i}.{j}")),
                            );
                            let resolved_pin = backend
                                .egrid
                                .resolve_wire((loc.0, (loc.1, loc.2), wire_pin.1))
                                .unwrap();
                            if resolved_pin != resolved_wire {
                                continue;
                            }
                            for (&wire_out, mux_data) in &node_data.muxes {
                                if mux_data.ins.contains(&wire_pin) {
                                    let out_name = backend.egrid.db.wires.key(wire_out.1);
                                    if out_name.starts_with("SINGLE")
                                        || (out_name.starts_with("LV") && i >= 4)
                                        || (out_name.starts_with("HEX.E")
                                            && backend.egrid.db.nodes.key(node.kind) == "IO.L")
                                        || (out_name.starts_with("HEX.W")
                                            && backend.egrid.db.nodes.key(node.kind) == "IO.R")
                                    {
                                        // FOUND
                                        let resolved_out = backend
                                            .egrid
                                            .resolve_wire((loc.0, (loc.1, loc.2), wire_out.1))
                                            .unwrap();
                                        let (tile, wa, wb) =
                                            resolve_int_pip(backend, loc, wire_pin, wire_out)
                                                .unwrap();
                                        fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                                        fuzzer = fuzzer.fuzz(
                                            Key::NodeMutex(resolved_out),
                                            None,
                                            "EXCLUSIVE",
                                        );
                                        return Some(fuzzer);
                                    }
                                }
                            }
                        }
                    }
                    loc.1 += 1;
                }
            }
            TileKV::VirtexPinHexV(wire) => {
                let node = backend.egrid.node(loc);
                let resolved_wire =
                    backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let wire_name = backend.egrid.db.wires.key(wire.1);
                let v = wire_name[4..5].chars().next().unwrap();
                let i: usize = wire_name[5..6].parse().unwrap();
                let mut loc = (loc.0, node.tiles[wire.0].0, node.tiles[wire.0].1, loc.3);
                if loc.2.to_idx() >= 6 {
                    loc.2 -= 6;
                } else {
                    loc.2 = RowId::from_idx(0)
                };
                loop {
                    if let Some((layer, node)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |n| {
                            matches!(
                                &backend.egrid.db.nodes.key(n.kind)[..],
                                "IO.L" | "IO.R" | "CLB" | "IO.B" | "IO.T"
                            )
                        })
                    {
                        loc.3 = layer;
                        let node_data = &backend.egrid.db.nodes[node.kind];
                        for j in 0..=6 {
                            let wire_pin = (
                                NodeTileId::from_idx(0),
                                backend.egrid.db.get_wire(&format!("HEX.{v}{i}.{j}")),
                            );
                            let resolved_pin = backend
                                .egrid
                                .resolve_wire((loc.0, (loc.1, loc.2), wire_pin.1))
                                .unwrap();
                            if resolved_pin != resolved_wire {
                                continue;
                            }
                            for (&wire_out, mux_data) in &node_data.muxes {
                                if mux_data.ins.contains(&wire_pin) {
                                    let out_name = backend.egrid.db.wires.key(wire_out.1);
                                    if out_name.starts_with("SINGLE")
                                        || (out_name.starts_with("HEX.N")
                                            && backend.egrid.db.nodes.key(node.kind) == "IO.B")
                                        || (out_name.starts_with("HEX.S")
                                            && backend.egrid.db.nodes.key(node.kind) == "IO.T")
                                    {
                                        // FOUND
                                        let resolved_out = backend
                                            .egrid
                                            .resolve_wire((loc.0, (loc.1, loc.2), wire_out.1))
                                            .unwrap();
                                        let (tile, wa, wb) =
                                            resolve_int_pip(backend, loc, wire_pin, wire_out)
                                                .unwrap();
                                        fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                                        fuzzer = fuzzer.fuzz(
                                            Key::NodeMutex(resolved_out),
                                            None,
                                            "EXCLUSIVE",
                                        );
                                        return Some(fuzzer);
                                    }
                                }
                            }
                        }
                    }
                    loc.2 += 1;
                }
            }
            TileKV::VirtexDriveHexH(wire) => {
                let node = backend.egrid.node(loc);
                let resolved_wire =
                    backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let wire_name = backend.egrid.db.wires.key(wire.1);
                let h = wire_name[4..5].chars().next().unwrap();
                let i: usize = wire_name[5..6].parse().unwrap();
                let mut loc = (loc.0, node.tiles[wire.0].0, node.tiles[wire.0].1, loc.3);
                if loc.1.to_idx() >= 8 {
                    loc.1 -= 8;
                } else {
                    loc.1 = ColId::from_idx(0)
                };
                loop {
                    if let Some((layer, node)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |n| {
                            matches!(
                                &backend.egrid.db.nodes.key(n.kind)[..],
                                "IO.L" | "IO.R" | "IO.B" | "IO.T" | "CLB"
                            )
                        })
                    {
                        loc.3 = layer;
                        let node_data = &backend.egrid.db.nodes[node.kind];
                        for j in 0..=6 {
                            let wire_pin = (
                                NodeTileId::from_idx(0),
                                backend.egrid.db.get_wire(&format!("HEX.{h}{i}.{j}")),
                            );
                            let resolved_pin = backend
                                .egrid
                                .resolve_wire((loc.0, (loc.1, loc.2), wire_pin.1))
                                .unwrap();
                            if resolved_pin != resolved_wire {
                                continue;
                            }
                            if let Some(mux_data) = node_data.muxes.get(&wire_pin) {
                                for &inp in &mux_data.ins {
                                    let inp_name = backend.egrid.db.wires.key(inp.1);
                                    if inp_name.starts_with("OMUX")
                                        || inp_name.starts_with("OUT")
                                        || (h == 'E'
                                            && backend.egrid.db.nodes.key(node.kind) == "IO.L"
                                            && inp_name.starts_with("HEX"))
                                        || (h == 'W'
                                            && backend.egrid.db.nodes.key(node.kind) == "IO.R"
                                            && inp_name.starts_with("HEX"))
                                    {
                                        // FOUND
                                        let resolved_inp = backend
                                            .egrid
                                            .resolve_wire((loc.0, (loc.1, loc.2), inp.1))
                                            .unwrap();
                                        let (tile, wa, wb) =
                                            resolve_int_pip(backend, loc, inp, wire_pin).unwrap();
                                        fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                                        fuzzer = fuzzer.fuzz(
                                            Key::NodeMutex(resolved_inp),
                                            None,
                                            "EXCLUSIVE",
                                        );
                                        fuzzer = fuzzer.fuzz(
                                            Key::NodeMutex(resolved_pin),
                                            None,
                                            "EXCLUSIVE",
                                        );
                                        return Some(fuzzer);
                                    }
                                }
                            }
                        }
                    }
                    loc.1 += 1;
                }
            }
            TileKV::VirtexDriveHexV(wire) => {
                let node = backend.egrid.node(loc);
                let resolved_wire =
                    backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                let wire_name = backend.egrid.db.wires.key(wire.1);
                let v = wire_name[4..5].chars().next().unwrap();
                let i: usize = wire_name[5..6].parse().unwrap();
                let mut loc = (loc.0, node.tiles[wire.0].0, node.tiles[wire.0].1, loc.3);
                if loc.2.to_idx() >= 6 {
                    loc.2 -= 6;
                } else {
                    loc.2 = RowId::from_idx(0)
                };
                loop {
                    if let Some((layer, node)) =
                        backend.egrid.find_node_loc(loc.0, (loc.1, loc.2), |n| {
                            matches!(
                                &backend.egrid.db.nodes.key(n.kind)[..],
                                "IO.L" | "IO.R" | "CLB" | "IO.B" | "IO.T"
                            )
                        })
                    {
                        loc.3 = layer;
                        let node_data = &backend.egrid.db.nodes[node.kind];
                        for j in 0..=6 {
                            let wire_pin = (
                                NodeTileId::from_idx(0),
                                backend.egrid.db.get_wire(&format!("HEX.{v}{i}.{j}")),
                            );
                            let resolved_pin = backend
                                .egrid
                                .resolve_wire((loc.0, (loc.1, loc.2), wire_pin.1))
                                .unwrap();
                            if resolved_pin != resolved_wire {
                                continue;
                            }
                            if let Some(mux_data) = node_data.muxes.get(&wire_pin) {
                                for &inp in &mux_data.ins {
                                    let inp_name = backend.egrid.db.wires.key(inp.1);
                                    if inp_name.starts_with("OMUX")
                                        || inp_name.starts_with("OUT")
                                        || (v == 'N'
                                            && backend.egrid.db.nodes.key(node.kind) == "IO.B"
                                            && inp_name.starts_with("HEX"))
                                        || (v == 'S'
                                            && backend.egrid.db.nodes.key(node.kind) == "IO.T"
                                            && inp_name.starts_with("HEX"))
                                    {
                                        // FOUND
                                        let resolved_inp = backend
                                            .egrid
                                            .resolve_wire((loc.0, (loc.1, loc.2), inp.1))
                                            .unwrap();
                                        let (tile, wa, wb) =
                                            resolve_int_pip(backend, loc, inp, wire_pin).unwrap();
                                        fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                                        fuzzer = fuzzer.fuzz(
                                            Key::NodeMutex(resolved_inp),
                                            None,
                                            "EXCLUSIVE",
                                        );
                                        fuzzer = fuzzer.fuzz(
                                            Key::NodeMutex(resolved_pin),
                                            None,
                                            "EXCLUSIVE",
                                        );
                                        return Some(fuzzer);
                                    }
                                }
                            }
                        }
                    }
                    loc.2 += 1;
                }
            }
            TileKV::Xc4000BiPip(wire_from, wire_to) => {
                let node = backend.egrid.node(loc);
                let res_from = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire_from.0], wire_from.1))
                    .unwrap();
                let res_to = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire_to.0], wire_to.1))
                    .unwrap();
                let fuzzer = fuzzer.fuzz(Key::NodeMutex(res_to), None, "EXCLUSIVE-TGT");
                let (fuzzer, src_site, src_pin) =
                    drive_xc4000_wire(backend, fuzzer, res_from, Some((loc, *wire_from)), res_to);
                let (tile, wa, wb) = resolve_int_pip(backend, loc, *wire_from, *wire_to)?;
                fuzzer.fuzz(
                    Key::Pip(tile, wa, wb),
                    None,
                    Value::FromPin(src_site, src_pin.into()),
                )
            }
            TileKV::Xc4000DoublePip(wire_from, wire_mid, wire_to) => {
                let node = backend.egrid.node(loc);
                let res_from = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire_from.0], wire_from.1))
                    .unwrap();
                let res_mid = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire_mid.0], wire_mid.1))
                    .unwrap();
                let res_to = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire_to.0], wire_to.1))
                    .unwrap();
                let fuzzer = fuzzer
                    .fuzz(Key::NodeMutex(res_to), None, "EXCLUSIVE-TGT")
                    .fuzz(Key::NodeMutex(res_mid), None, "EXCLUSIVE-MID");
                let (fuzzer, src_site, src_pin) =
                    drive_xc4000_wire(backend, fuzzer, res_from, Some((loc, *wire_from)), res_to);
                let (tile0, wa0, wb0) = resolve_int_pip(backend, loc, *wire_from, *wire_mid)?;
                let (tile1, wa1, wb1) = resolve_int_pip(backend, loc, *wire_mid, *wire_to)?;
                fuzzer
                    .fuzz(
                        Key::Pip(tile0, wa0, wb0),
                        None,
                        Value::FromPin(src_site, src_pin.into()),
                    )
                    .fuzz(
                        Key::Pip(tile1, wa1, wb1),
                        None,
                        Value::FromPin(src_site, src_pin.into()),
                    )
            }
            TileKV::DeviceSide(dir) => match backend.edev {
                ExpandedDevice::Virtex(edev) => {
                    match dir {
                        Dir::W => {
                            if loc.1 >= edev.grid.col_clk() {
                                return None;
                            }
                        }
                        Dir::E => {
                            if loc.1 < edev.grid.col_clk() {
                                return None;
                            }
                        }
                        Dir::S => {
                            if loc.2 >= edev.grid.row_mid() {
                                return None;
                            }
                        }
                        Dir::N => {
                            if loc.2 < edev.grid.row_mid() {
                                return None;
                            }
                        }
                    }
                    fuzzer
                }
                ExpandedDevice::Virtex2(edev) => {
                    match dir {
                        Dir::W => {
                            if loc.1 >= edev.grid.col_clk {
                                return None;
                            }
                        }
                        Dir::E => {
                            if loc.1 < edev.grid.col_clk {
                                return None;
                            }
                        }
                        Dir::S => {
                            if loc.2 >= edev.grid.row_mid() {
                                return None;
                            }
                        }
                        Dir::N => {
                            if loc.2 < edev.grid.row_mid() {
                                return None;
                            }
                        }
                    }
                    fuzzer
                }
                ExpandedDevice::Spartan6(edev) => {
                    match dir {
                        Dir::W => {
                            if loc.1 >= edev.grid.col_clk {
                                return None;
                            }
                        }
                        Dir::E => {
                            if loc.1 < edev.grid.col_clk {
                                return None;
                            }
                        }
                        Dir::S => {
                            if loc.2 >= edev.grid.row_clk() {
                                return None;
                            }
                        }
                        Dir::N => {
                            if loc.2 < edev.grid.row_clk() {
                                return None;
                            }
                        }
                    }
                    fuzzer
                }
                _ => todo!(),
            },
            TileKV::HclkSide(dir) => match backend.edev {
                ExpandedDevice::Virtex4(edev) => {
                    match dir {
                        Dir::S => {
                            if loc.2 >= edev.grids[loc.0].row_hclk(loc.2) {
                                return None;
                            }
                        }
                        Dir::N => {
                            if loc.2 < edev.grids[loc.0].row_hclk(loc.2) {
                                return None;
                            }
                        }
                        _ => unreachable!(),
                    }
                    fuzzer
                }
                _ => todo!(),
            },
            TileKV::CenterDci(bank) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => {
                        let (vr_row, io_row) = get_v4_center_dci_rows(edev, *bank);
                        // Ensure nothing is placed in VR.
                        if let Some(vr_row) = vr_row {
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(loc.0, edev.col_cfg, vr_row, bel)
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                        }
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, edev.col_cfg, io_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "INBUFUSED".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OPROGRAMMING".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
                        fuzzer =
                            fuzzer.fuzz(Key::SiteAttr(site, "IOATTRBOX".into()), None, "LVDCI_33");
                        fuzzer =
                            fuzzer.fuzz(Key::SiteAttr(site, "DRIVE_0MA".into()), "DRIVE_0MA", None);
                        // Take exclusive mutex on global DCI.
                        fuzzer =
                            fuzzer.fuzz(Key::GlobalMutex("GLOBAL_DCI".into()), None, "EXCLUSIVE");
                        // Avoid interference.
                        fuzzer = fuzzer.base(Key::GlobalOpt("MATCH_CYCLE".into()), "NOWAIT");
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex5 => {
                        let (vr_row, io_row) = get_v4_center_dci_rows(edev, *bank);
                        // Ensure nothing is placed in VR.
                        if let Some(vr_row) = vr_row {
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(loc.0, edev.col_cfg, vr_row, bel)
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                        }
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, edev.col_cfg, io_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OPROGRAMMING".into()), None);
                        fuzzer = fuzzer.fuzz(Key::SiteAttr(site, "OUSED".into()), None, "0");
                        fuzzer =
                            fuzzer.fuzz(Key::SiteAttr(site, "OSTANDARD".into()), None, "LVDCI_33");
                        // Take exclusive mutex on global DCI.
                        fuzzer =
                            fuzzer.fuzz(Key::GlobalMutex("GLOBAL_DCI".into()), None, "EXCLUSIVE");
                        // Avoid interference.
                        fuzzer = fuzzer.base(Key::GlobalOpt("MATCH_CYCLE".into()), "NOWAIT");
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex6 => {
                        assert_eq!(*bank, 25);
                        let (vr_row, io_row) = get_v4_center_dci_rows(edev, *bank);
                        // Ensure nothing is placed in VR.
                        if let Some(vr_row) = vr_row {
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(loc.0, edev.col_lcio.unwrap(), vr_row, bel)
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                        }
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, edev.col_lcio.unwrap(), io_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IUSED".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OPROGRAMMING".into()), None);
                        fuzzer = fuzzer.fuzz(Key::SiteAttr(site, "OUSED".into()), None, "0");
                        fuzzer =
                            fuzzer.fuzz(Key::SiteAttr(site, "OSTANDARD".into()), None, "LVDCI_25");
                        // Make note of anchor VCCO.
                        let (layer, _) = edev
                            .egrid
                            .find_node_loc(
                                loc.0,
                                (edev.col_lcio.unwrap(), edev.grids[loc.0].row_bufg() + 20),
                                |node| edev.egrid.db.nodes.key(node.kind) == "HCLK_IOI",
                            )
                            .unwrap();
                        fuzzer = fuzzer.base(
                            Key::TileMutex(
                                (
                                    loc.0,
                                    edev.col_lcio.unwrap(),
                                    edev.grids[loc.0].row_bufg() + 20,
                                    layer,
                                ),
                                "VCCO".to_string(),
                            ),
                            "2500",
                        );
                        // Take exclusive mutex on global DCI.
                        fuzzer =
                            fuzzer.fuzz(Key::GlobalMutex("GLOBAL_DCI".into()), None, "EXCLUSIVE");
                        // Avoid interference.
                        fuzzer = fuzzer.base(Key::GlobalOpt("MATCH_CYCLE".into()), "NOWAIT");
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex7 => {
                        let anchor_reg = if edev.grids[loc.0].has_ps {
                            prjcombine_virtex4::grid::RegId::from_idx(
                                edev.grids[loc.0].regs - 2 + *bank as usize,
                            )
                        } else {
                            prjcombine_virtex4::grid::RegId::from_idx(*bank as usize)
                        };
                        // Ensure nothing is placed in VR.
                        for row in [
                            edev.grids[loc.0].row_reg_hclk(anchor_reg) - 25,
                            edev.grids[loc.0].row_reg_hclk(anchor_reg) + 24,
                        ] {
                            let site = backend
                                .ngrid
                                .get_bel_name(loc.0, edev.col_rio.unwrap(), row, "IOB")
                                .unwrap();
                            fuzzer = fuzzer.base(Key::SiteMode(site), None);
                        }
                        let io_row = edev.grids[loc.0].row_reg_hclk(anchor_reg) - 24;
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, edev.col_rio.unwrap(), io_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::GlobalOpt("UNCONSTRAINEDPINS".into()), "ALLOW");
                        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IUSED".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OPROGRAMMING".into()), None);
                        fuzzer = fuzzer.fuzz(Key::SiteAttr(site, "OUSED".into()), None, "0");
                        fuzzer = fuzzer.fuzz(
                            Key::SiteAttr(site, "OSTANDARD".into()),
                            None,
                            "HSLVDCI_18",
                        );
                        // Make note of anchor VCCO.
                        let (layer, _) = edev
                            .egrid
                            .find_node_loc(
                                loc.0,
                                (
                                    edev.col_rio.unwrap(),
                                    edev.grids[loc.0].row_reg_hclk(anchor_reg),
                                ),
                                |node| edev.egrid.db.nodes.key(node.kind) == "HCLK_IOI_HP",
                            )
                            .unwrap();
                        fuzzer = fuzzer.base(
                            Key::TileMutex(
                                (
                                    loc.0,
                                    edev.col_rio.unwrap(),
                                    edev.grids[loc.0].row_reg_hclk(anchor_reg),
                                    layer,
                                ),
                                "VCCO".to_string(),
                            ),
                            "1800",
                        );
                        // Take exclusive mutex on global DCI.
                        fuzzer =
                            fuzzer.fuzz(Key::GlobalMutex("GLOBAL_DCI".into()), None, "EXCLUSIVE");
                        // Avoid interference.
                        fuzzer = fuzzer.base(Key::GlobalOpt("MATCH_CYCLE".into()), "NOWAIT");
                    }
                }
                fuzzer
            }
            TileKV::CascadeDci(bank_a, bank_b) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex5
                    | prjcombine_virtex4::grid::GridKind::Virtex6 => {
                        let col = if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex6 {
                            edev.col_lcio.unwrap()
                        } else {
                            edev.col_cfg
                        };
                        let (_, io_row) = get_v4_center_dci_rows(edev, *bank_a);
                        // Ensure nothing else in the bank.
                        let bot =
                            edev.grids[loc.0].row_reg_bot(edev.grids[loc.0].row_to_reg(io_row));
                        for i in 0..edev.grids[loc.0].rows_per_reg() {
                            let row = bot + i;
                            for bel in ["IOB0", "IOB1"] {
                                if row == io_row && bel == "IOB0" {
                                    continue;
                                }
                                if let Some(site) = backend.ngrid.get_bel_name(loc.0, col, row, bel)
                                {
                                    fuzzer = fuzzer.base(Key::SiteMode(site), None);
                                }
                            }
                        }
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, col, io_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IUSED".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OPROGRAMMING".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
                        fuzzer = fuzzer.base(
                            Key::SiteAttr(site, "OSTANDARD".into()),
                            if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex6 {
                                "LVDCI_25"
                            } else {
                                "LVDCI_33"
                            },
                        );
                        // Take shared mutex on global DCI.
                        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
                        let (_, io_row) = get_v4_center_dci_rows(edev, *bank_b);
                        // Ensure nothing else in the bank.
                        let bot =
                            edev.grids[loc.0].row_reg_bot(edev.grids[loc.0].row_to_reg(io_row));
                        for i in 0..edev.grids[loc.0].rows_per_reg() {
                            let row = bot + i;
                            for bel in ["IOB0", "IOB1"] {
                                if row == io_row && bel == "IOB0" {
                                    continue;
                                }
                                if let Some(site) = backend.ngrid.get_bel_name(loc.0, col, row, bel)
                                {
                                    fuzzer = fuzzer.base(Key::SiteMode(site), None);
                                }
                            }
                        }
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, col, io_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IUSED".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OPROGRAMMING".into()), None);
                        fuzzer = fuzzer.fuzz(Key::SiteAttr(site, "OUSED".into()), None, "0");
                        fuzzer = fuzzer.fuzz(
                            Key::SiteAttr(site, "OSTANDARD".into()),
                            None,
                            if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex6 {
                                "LVDCI_25"
                            } else {
                                "LVDCI_33"
                            },
                        );
                        fuzzer = fuzzer.fuzz(Key::DciCascade(*bank_b), None, *bank_a);
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex7 => {
                        fuzzer = fuzzer.base(Key::GlobalOpt("UNCONSTRAINEDPINS".into()), "ALLOW");
                        let (anchor_reg_a, anchor_reg_b) = if edev.grids[loc.0].has_ps {
                            (
                                prjcombine_virtex4::grid::RegId::from_idx(
                                    edev.grids[loc.0].regs - 2 + *bank_a as usize,
                                ),
                                prjcombine_virtex4::grid::RegId::from_idx(
                                    edev.grids[loc.0].regs - 2 + *bank_b as usize,
                                ),
                            )
                        } else {
                            (
                                prjcombine_virtex4::grid::RegId::from_idx(*bank_a as usize),
                                prjcombine_virtex4::grid::RegId::from_idx(*bank_b as usize),
                            )
                        };
                        let hclk_a = edev.grids[loc.0].row_reg_hclk(anchor_reg_a);
                        let hclk_b = edev.grids[loc.0].row_reg_hclk(anchor_reg_b);
                        let io_row = hclk_a - 24;
                        let col = edev.col_rio.unwrap();
                        // Ensure nothing else in the bank.
                        for i in 0..50 {
                            let row = hclk_a - 25 + i;
                            for bel in ["IOB", "IOB0", "IOB1"] {
                                if row == io_row && bel == "IOB0" {
                                    continue;
                                }
                                if let Some(site) = backend.ngrid.get_bel_name(loc.0, col, row, bel)
                                {
                                    fuzzer = fuzzer.base(Key::SiteMode(site), None);
                                }
                            }
                        }
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, col, io_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IUSED".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OPROGRAMMING".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "HSLVDCI_18");
                        // Take shared mutex on global DCI.
                        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
                        let io_row = hclk_b - 24;
                        // Ensure nothing else in the bank.
                        for i in 0..50 {
                            let row = hclk_b - 25 + i;
                            for bel in ["IOB", "IOB0", "IOB1"] {
                                if row == io_row && bel == "IOB0" {
                                    continue;
                                }
                                if let Some(site) = backend.ngrid.get_bel_name(loc.0, col, row, bel)
                                {
                                    fuzzer = fuzzer.base(Key::SiteMode(site), None);
                                }
                            }
                        }
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, col, io_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "IUSED".into()), None);
                        fuzzer = fuzzer.base(Key::SiteAttr(site, "OPROGRAMMING".into()), None);
                        fuzzer = fuzzer.fuzz(Key::SiteAttr(site, "OUSED".into()), None, "0");
                        fuzzer = fuzzer.fuzz(
                            Key::SiteAttr(site, "OSTANDARD".into()),
                            None,
                            "HSLVDCI_18",
                        );
                        let actual_bank_a = edev
                            .get_io_info(prjcombine_virtex4::expanded::IoCoord {
                                die: loc.0,
                                col,
                                row: hclk_a - 24,
                                iob: EntityId::from_idx(0),
                            })
                            .bank;
                        let actual_bank_b = edev
                            .get_io_info(prjcombine_virtex4::expanded::IoCoord {
                                die: loc.0,
                                col,
                                row: hclk_b - 24,
                                iob: EntityId::from_idx(0),
                            })
                            .bank;
                        fuzzer = fuzzer.fuzz(Key::DciCascade(actual_bank_b), None, actual_bank_a);
                    }
                    _ => unreachable!(),
                }
                fuzzer
            }
            TileKV::TouchHout(idx) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let mut tgt_col_cmt = None;
                let mut tgt_col_gt = None;
                if loc.1 < edev.col_clk {
                    if let Some(col_io) = edev.col_lio {
                        if loc.1 < col_io {
                            tgt_col_cmt = Some(col_io + 1);
                        }
                    }
                    if let Some((col_gt, _)) = edev.col_mgt {
                        let gtcol = edev.grids[loc.0].get_col_gt(col_gt).unwrap();
                        if loc.1 > col_gt
                            && gtcol.regs[edev.grids[loc.0].row_to_reg(loc.2)].is_some()
                        {
                            tgt_col_gt = Some(col_gt);
                        }
                    }
                } else {
                    if let Some(col_io) = edev.col_rio {
                        if loc.1 > col_io {
                            tgt_col_cmt = Some(col_io - 1);
                        }
                    }
                    if let Some((_, col_gt)) = edev.col_mgt {
                        let gtcol = edev.grids[loc.0].get_col_gt(col_gt).unwrap();
                        if loc.1 > col_gt
                            && gtcol.regs[edev.grids[loc.0].row_to_reg(loc.2)].is_some()
                        {
                            tgt_col_gt = Some(col_gt);
                        }
                    }
                }
                if let Some(_col) = tgt_col_cmt {
                    todo!();
                } else if tgt_col_gt.is_some() {
                    // nope.
                    return None;
                } else {
                    let lr = if loc.1 < edev.col_clk { 'L' } else { 'R' };
                    let mut loc = loc;
                    loc.1 = edev.col_clk;
                    let (layer, _) = edev
                        .egrid
                        .find_node_loc(loc.0, (loc.1, loc.2), |node| {
                            edev.egrid.db.nodes.key(node.kind) == "CLK_HROW"
                        })
                        .unwrap();
                    loc.3 = layer;
                    let bel = BelId::from_idx(58);
                    let (ta, wa) = resolve_tile_wire(
                        backend,
                        loc,
                        &TileWire::BelPinNear(bel, format!("HIN{idx}_{lr}")),
                    )?;
                    let (tb, wb) = resolve_tile_wire(
                        backend,
                        loc,
                        &TileWire::BelPinNear(bel, format!("CASCO{idx}")),
                    )?;
                    assert_eq!(ta, tb);

                    fuzzer = fuzzer
                        .base(Key::TileMutex(loc, format!("HIN{idx}_{lr}")), "USE")
                        .base(
                            Key::BelMutex(
                                (loc.0, loc.1, loc.2, loc.3, bel),
                                format!("MUX.CASCO{idx}"),
                            ),
                            format!("HIN{idx}_{lr}"),
                        )
                        .base(
                            Key::BelMutex((loc.0, loc.1, loc.2, loc.3, bel), "CASCO".into()),
                            "CASCO",
                        )
                        .base(Key::Pip(ta, wa, wb), true);
                }
                fuzzer
            }
            TileKV::PinPair(bel_a, pin_a, bel_b, pin_b) => {
                let site_a = &backend.ngrid.nodes[&loc].bels[*bel_a];
                let site_b = &backend.ngrid.nodes[&loc].bels[*bel_b];
                fuzzer.base(
                    Key::SitePin(site_a, pin_a.clone()),
                    Value::FromPin(site_b, pin_b.clone()),
                )
            }
        })
    }
}

fn drive_xc4000_wire<'a>(
    backend: &IseBackend<'a>,
    fuzzer: Fuzzer<IseBackend<'a>>,
    wire_target: IntWire,
    orig_target: Option<(NodeLoc, NodeWireId)>,
    wire_avoid: IntWire,
) -> (Fuzzer<IseBackend<'a>>, &'a str, &'a str) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };
    let wname = backend.egrid.db.wires.key(wire_target.2);
    let aname = backend.egrid.db.wires.key(wire_avoid.2);
    let (die, (mut col, mut row), mut wt) = wire_target;
    let (_, (acol, arow), _) = wire_avoid;
    let fuzzer = fuzzer.fuzz(Key::NodeMutex(wire_target), None, "EXCLUSIVE");
    // println!("DRIVING {wire_target:?} {wname}");
    if wire_target.1.1 != edev.grid.row_bio()
        && wire_target.1.1 != edev.grid.row_tio()
        && (wname == "LONG.H2" || wname == "LONG.H3")
    {
        let bel = if wname == "LONG.H3" { "TBUF1" } else { "TBUF0" };
        let nloc = (die, col, row, LayerId::from_idx(0));
        let node = backend.egrid.node(nloc);
        let nnode = &backend.ngrid.nodes[&nloc];
        let node_info = &backend.egrid.db.nodes[node.kind];
        let node_naming = &backend.ngrid.db.node_namings[nnode.naming];
        let bel = node_info.bels.get(bel).unwrap().0;
        let bel_naming = &node_naming.bels[bel];
        let pin_naming = &bel_naming.pins["O"];
        let site_name = &nnode.bels[bel];
        let fuzzer = fuzzer
            .base(Key::SiteMode(site_name), "TBUF")
            .base(Key::SitePin(site_name, "O".into()), true)
            .base(
                Key::Pip(
                    &nnode.names[bel_naming.tile],
                    &pin_naming.name,
                    &pin_naming.name_far,
                ),
                Value::FromPin(site_name, "O".into()),
            );
        (fuzzer, site_name, "O")
    } else if wname == "GND" {
        let nloc = (die, col, row, LayerId::from_idx(0));
        let nnode = &backend.ngrid.nodes[&nloc];
        let site_name = nnode.tie_name.as_ref().unwrap();
        let fuzzer = fuzzer
            .base(Key::SiteMode(site_name), "TIE")
            .base(Key::SitePin(site_name, "O".into()), true);
        (fuzzer, site_name, "O")
    } else if wname.starts_with("OUT.CLB") && (wname.ends_with(".V") || wname.ends_with(".H")) {
        let owname = &wname[..(wname.len() - 2)];
        let nwt = (die, (col, row), backend.egrid.db.get_wire(owname));
        let (fuzzer, site_name, pin_name) =
            drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
        let (tile, wa, wb) = resolve_int_pip(
            backend,
            (die, col, row, LayerId::from_idx(0)),
            (NodeTileId::from_idx(0), nwt.2),
            (NodeTileId::from_idx(0), wt),
        )
        .unwrap();
        let fuzzer = fuzzer.base(
            Key::Pip(tile, wa, wb),
            Value::FromPin(site_name, pin_name.into()),
        );
        (fuzzer, site_name, pin_name)
    } else if wname.starts_with("OUT.CLB") {
        let nloc = (die, col, row, LayerId::from_idx(0));
        let node = backend.egrid.node(nloc);
        let nnode = &backend.ngrid.nodes[&nloc];
        let node_info = &backend.egrid.db.nodes[node.kind];
        let bel = node_info.bels.get("CLB").unwrap().0;
        let site_name = &nnode.bels[bel];
        let (pin, fuzzer) = match &wname[..] {
            "OUT.CLB.FX" => (
                "X",
                fuzzer
                    .base(Key::SiteAttr(site_name, "F".into()), "#LUT:F=0x0000")
                    .base(Key::SiteAttr(site_name, "XMUX".into()), "F"),
            ),
            "OUT.CLB.GY" => (
                "Y",
                fuzzer
                    .base(Key::SiteAttr(site_name, "G".into()), "#LUT:G=0x0000")
                    .base(Key::SiteAttr(site_name, "YMUX".into()), "G"),
            ),
            "OUT.CLB.FXQ" => (
                "XQ",
                if edev.grid.kind.is_clb_xl() {
                    fuzzer
                        .base(Key::SiteAttr(site_name, "CLKX".into()), "CLK")
                        .base(Key::SiteAttr(site_name, "XQMUX".into()), "QX")
                        .base(Key::SiteAttr(site_name, "FFX".into()), "#LATCH")
                        .base(Key::SiteAttr(site_name, "DX".into()), "DIN")
                        .base(Key::SiteAttr(site_name, "DIN".into()), "C1")
                } else {
                    fuzzer
                        .base(Key::SiteAttr(site_name, "CLKX".into()), "CLK")
                        .base(Key::SiteAttr(site_name, "XQMUX".into()), "QX")
                },
            ),
            "OUT.CLB.GYQ" => (
                "YQ",
                if edev.grid.kind.is_clb_xl() {
                    fuzzer
                        .base(Key::SiteAttr(site_name, "CLKY".into()), "CLK")
                        .base(Key::SiteAttr(site_name, "YQMUX".into()), "QY")
                        .base(Key::SiteAttr(site_name, "FFY".into()), "#LATCH")
                        .base(Key::SiteAttr(site_name, "DY".into()), "DIN")
                        .base(Key::SiteAttr(site_name, "DIN".into()), "C1")
                } else {
                    fuzzer
                        .base(Key::SiteAttr(site_name, "CLKY".into()), "CLK")
                        .base(Key::SiteAttr(site_name, "YQMUX".into()), "QY")
                },
            ),
            _ => unreachable!(),
        };
        let fuzzer = fuzzer
            .base(Key::SiteMode(site_name), "CLB")
            .base(Key::SitePin(site_name, pin.into()), true);
        (fuzzer, site_name, pin)
    } else if let Some(idx) = wname.strip_prefix("SINGLE.H") {
        let idx: u8 = idx.parse().unwrap();
        assert_ne!(row, edev.grid.row_tio());
        if col == edev.grid.col_lio()
            || (col == edev.grid.col_lio() + 1
                && (row == edev.grid.row_bio() || row == edev.grid.row_tio() - 1))
        {
            let nwt = (die, (col + 1, row), wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col + 1, row, LayerId::from_idx(0)),
                (NodeTileId::from_idx(0), wt),
                (
                    NodeTileId::from_idx(0),
                    backend.egrid.db.get_wire(&format!("SINGLE.H{idx}.E")),
                ),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if col == edev.grid.col_rio() {
            let nwt = (die, (col - 1, row), wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row, LayerId::from_idx(0)),
                (
                    NodeTileId::from_idx(0),
                    backend.egrid.db.get_wire(&format!("SINGLE.H{idx}.E")),
                ),
                (NodeTileId::from_idx(0), wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if row == edev.grid.row_bio() {
            let nwt = (
                die,
                (col, row + 1),
                backend.egrid.db.get_wire(&format!("SINGLE.V{idx}")),
            );
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row, LayerId::from_idx(0)),
                (
                    NodeTileId::from_idx(0),
                    backend.egrid.db.get_wire(&format!("SINGLE.V{idx}.S")),
                ),
                (NodeTileId::from_idx(0), wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if row == edev.grid.row_tio() - 1 {
            let nwt = (
                die,
                (col, row),
                backend.egrid.db.get_wire(&format!("SINGLE.V{idx}")),
            );
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row, LayerId::from_idx(0)),
                (NodeTileId::from_idx(0), nwt.2),
                (NodeTileId::from_idx(0), wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else {
            let (out, sout, srow) = match (
                idx,
                edev.grid.kind == prjcombine_xc2000::grid::GridKind::Xc4000E,
            ) {
                (0 | 4, true) => ("OUT.CLB.GY", "OUT.CLB.GY", row),
                (1 | 5, true) => ("OUT.CLB.GYQ", "OUT.CLB.GYQ", row),
                (2 | 6, true) => ("OUT.CLB.FXQ.S", "OUT.CLB.FXQ", row + 1),
                (3 | 7, true) => ("OUT.CLB.FX.S", "OUT.CLB.FX", row + 1),
                (0 | 4, false) => ("OUT.CLB.GY.V", "OUT.CLB.GY.V", row),
                (1 | 5, false) => ("OUT.CLB.GYQ.V", "OUT.CLB.GYQ.V", row),
                (2 | 6, false) => ("OUT.CLB.FXQ.S", "OUT.CLB.FXQ.V", row + 1),
                (3 | 7, false) => ("OUT.CLB.FX.S", "OUT.CLB.FX.V", row + 1),
                _ => unreachable!(),
            };
            let nwt = (die, (col, srow), backend.egrid.db.get_wire(sout));
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row, LayerId::from_idx(0)),
                (NodeTileId::from_idx(0), backend.egrid.db.get_wire(out)),
                (NodeTileId::from_idx(0), wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        }
    } else if let Some(idx) = wname.strip_prefix("SINGLE.V") {
        let idx: u8 = idx.parse().unwrap();
        assert_ne!(col, edev.grid.col_lio());
        if row == edev.grid.row_bio() {
            let nwt = (die, (col, row + 1), wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row, LayerId::from_idx(0)),
                (
                    NodeTileId::from_idx(0),
                    backend.egrid.db.get_wire(&format!("SINGLE.V{idx}.S")),
                ),
                (NodeTileId::from_idx(0), wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if row == edev.grid.row_tio()
            || (row == edev.grid.row_tio() - 1
                && (col == edev.grid.col_lio() + 1 || col == edev.grid.col_rio()))
        {
            let nwt = (die, (col, row - 1), wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row - 1, LayerId::from_idx(0)),
                (NodeTileId::from_idx(0), wt),
                (
                    NodeTileId::from_idx(0),
                    backend.egrid.db.get_wire(&format!("SINGLE.V{idx}.S")),
                ),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if col == edev.grid.col_lio() + 1 {
            let nwt = (
                die,
                (col, row),
                backend.egrid.db.get_wire(&format!("SINGLE.H{idx}")),
            );
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row, LayerId::from_idx(0)),
                (NodeTileId::from_idx(0), nwt.2),
                (NodeTileId::from_idx(0), wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if col == edev.grid.col_rio() {
            let nwt = (
                die,
                (col - 1, row),
                backend.egrid.db.get_wire(&format!("SINGLE.H{idx}")),
            );
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row, LayerId::from_idx(0)),
                (
                    NodeTileId::from_idx(0),
                    backend.egrid.db.get_wire(&format!("SINGLE.H{idx}.E")),
                ),
                (NodeTileId::from_idx(0), wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else {
            let (out, sout, scol) = match (
                idx,
                edev.grid.kind == prjcombine_xc2000::grid::GridKind::Xc4000E,
            ) {
                (0 | 4, true) => ("OUT.CLB.FXQ", "OUT.CLB.FXQ", col),
                (1 | 5, true) => ("OUT.CLB.FX", "OUT.CLB.FX", col),
                (2 | 6, true) => ("OUT.CLB.GY.E", "OUT.CLB.GY", col - 1),
                (3 | 7, true) => ("OUT.CLB.GYQ.E", "OUT.CLB.GYQ", col - 1),
                (0 | 4, false) => ("OUT.CLB.FXQ.H", "OUT.CLB.FXQ.H", col),
                (1 | 5, false) => ("OUT.CLB.FX.H", "OUT.CLB.FX.H", col),
                (2 | 6, false) => ("OUT.CLB.GY.E", "OUT.CLB.GY.H", col - 1),
                (3 | 7, false) => ("OUT.CLB.GYQ.E", "OUT.CLB.GYQ.H", col - 1),
                _ => unreachable!(),
            };
            let nwt = (die, (scol, row), backend.egrid.db.get_wire(sout));
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(
                backend,
                (die, col, row, LayerId::from_idx(0)),
                (NodeTileId::from_idx(0), backend.egrid.db.get_wire(out)),
                (NodeTileId::from_idx(0), wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        }
    } else if wname.starts_with("LONG")
        || wname.starts_with("IO.OCTAL")
        || wname.starts_with("QUAD")
        || wname.starts_with("OCTAL")
        || wname.starts_with("GCLK")
        || wname.starts_with("VCLK")
    {
        let mut filter = None;
        let mut twt = NodeTileId::from_idx(0);
        let mut layer = LayerId::from_idx(0);
        if wname.starts_with("LONG") {
            if wname.contains(".H") {
                if col == edev.grid.col_lio() {
                    col += 1;
                }
                if col == acol {
                    col += 1;
                }
            } else if wname.contains(".V") {
                if row == arow {
                    row += 1;
                }
            } else {
                unreachable!()
            }
        } else if wname.starts_with("IO.OCTAL") {
            match &wname[..] {
                "IO.OCTAL.W.0" => (),
                "IO.OCTAL.E.0" => {
                    assert_ne!(row, edev.grid.row_tio());
                    row += 1;
                    wt = backend.egrid.db.get_wire("IO.OCTAL.E.1");
                    if row == edev.grid.row_tio() {
                        wt = backend.egrid.db.get_wire("IO.OCTAL.N.1");
                        col -= 1;
                    }
                }
                "IO.OCTAL.S.0" => (),
                "IO.OCTAL.N.0" => {
                    assert_ne!(col, edev.grid.col_lio());
                    col -= 1;
                    wt = backend.egrid.db.get_wire("IO.OCTAL.N.1");
                    if col == edev.grid.col_lio() {
                        wt = backend.egrid.db.get_wire("IO.OCTAL.W.1");
                        row -= 1;
                    }
                }
                _ => unreachable!(),
            }
        } else if wname.starts_with("QUAD.H") {
            if col == edev.grid.col_lio() {
                if wname.ends_with(".3") {
                    if aname.starts_with("LONG.IO") {
                        col += 1;
                        match &wname[..] {
                            "QUAD.H0.3" => {
                                filter = Some("QUAD.H0.0");
                                wt = backend.egrid.db.get_wire("QUAD.H0.4");
                            }
                            "QUAD.H1.3" => {
                                filter = Some("QUAD.H1.0");
                                wt = backend.egrid.db.get_wire("QUAD.H1.4");
                            }
                            "QUAD.H2.3" => {
                                filter = Some("QUAD.H2.0");
                                wt = backend.egrid.db.get_wire("QUAD.H2.4");
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        filter = Some("LONG.IO");
                    }
                } else if wname == "QUAD.H1.0" {
                    col += 1;
                    wt = backend.egrid.db.get_wire("QUAD.H1.1");
                }
            } else if wname == "QUAD.H2.0" {
                if col == edev.grid.col_rio() {
                    if aname.starts_with("LONG.IO") {
                        filter = Some("QUAD.H2.4");
                    } else {
                        filter = Some("LONG.IO");
                    }
                } else {
                    col += 1;
                    wt = backend.egrid.db.get_wire("QUAD.H2.1");
                }
            }
        } else if wname.starts_with("QUAD.V") {
            if row == edev.grid.row_tio() {
                if wname.ends_with(".3") {
                    if aname.starts_with("LONG.IO") {
                        row -= 1;
                        match &wname[..] {
                            "QUAD.V0.3" => {
                                filter = Some("QUAD.V0.0");
                                wt = backend.egrid.db.get_wire("QUAD.V0.4");
                            }
                            "QUAD.V1.3" => {
                                filter = Some("QUAD.V1.0");
                                wt = backend.egrid.db.get_wire("QUAD.V1.4");
                            }
                            "QUAD.V2.3" => {
                                filter = Some("QUAD.V2.0");
                                wt = backend.egrid.db.get_wire("QUAD.V2.4");
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        filter = Some("LONG.IO");
                    }
                } else if wname == "QUAD.V2.2" {
                    row -= 1;
                    wt = backend.egrid.db.get_wire("QUAD.V2.3");
                }
            } else if wname == "QUAD.V0.0" {
                if row == edev.grid.row_bio() {
                    if aname.starts_with("LONG.IO") {
                        filter = Some("QUAD.V0.4");
                    } else {
                        filter = Some("LONG.IO");
                    }
                } else {
                    row -= 1;
                    wt = backend.egrid.db.get_wire("QUAD.V0.1");
                }
            }
        } else if let Some(idx) = wname.strip_prefix("OCTAL.H.") {
            if col == edev.grid.col_lio() {
                let idx: usize = idx.parse().unwrap();
                col += 7 - idx;
                wt = backend.egrid.db.get_wire("OCTAL.H.7");
            }
        } else if let Some(idx) = wname.strip_prefix("OCTAL.V.") {
            if row == edev.grid.row_tio() {
                let idx: usize = idx.parse().unwrap();
                row -= 7 - idx;
                wt = backend.egrid.db.get_wire("OCTAL.V.7");
            }
        } else if wname.starts_with("GCLK") {
            if row == edev.grid.row_bio() {
                row = edev.grid.row_qb();
            } else {
                row = edev.grid.row_qt();
            }
            layer = backend
                .egrid
                .find_node_loc(die, (col, row), |node| {
                    backend.egrid.db.nodes.key(node.kind).starts_with("LLVQ")
                })
                .unwrap()
                .0;
        } else if wname == "VCLK" {
            if row == edev.grid.row_bio() {
                // OK
            } else if row == edev.grid.row_qb() {
                row = edev.grid.row_mid();
                layer = backend
                    .egrid
                    .find_node_loc(die, (col, row), |node| {
                        backend.egrid.db.nodes.key(node.kind).starts_with("LLVC")
                    })
                    .unwrap()
                    .0;
            } else if row == edev.grid.row_mid() {
                twt = NodeTileId::from_idx(1);
                layer = backend
                    .egrid
                    .find_node_loc(die, (col, row), |node| {
                        backend.egrid.db.nodes.key(node.kind).starts_with("LLVC")
                    })
                    .unwrap()
                    .0;
            } else if row == edev.grid.row_qt() {
                row = edev.grid.row_tio();
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
        let nloc = (die, col, row, layer);
        let node = backend.egrid.node(nloc);
        let mwt = (twt, wt);
        let res = backend
            .egrid
            .resolve_wire((die, node.tiles[twt], wt))
            .unwrap();
        assert_eq!(res, wire_target);
        let mux = &backend.egrid.db.nodes[node.kind].muxes[&mwt];
        for &mwf in &mux.ins {
            let wfname = backend.egrid.db.wires.key(mwf.1);
            if let Some(filter) = filter {
                if !wfname.starts_with(filter) {
                    continue;
                }
            } else {
                if !(wfname.starts_with("SINGLE")
                    || wfname == "GND"
                    || (wfname.starts_with("IO.DOUBLE")
                        && (wname.starts_with("OCTAL")
                            || wname.starts_with("QUAD")
                            || wname == "VCLK")))
                {
                    continue;
                }
            }
            let nwt = backend
                .egrid
                .resolve_wire((die, node.tiles[mwf.0], mwf.1))
                .unwrap();
            if nwt == wire_avoid {
                continue;
            }
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, Some((nloc, mwf)), wire_avoid);
            let (tile, wa, wb) =
                resolve_int_pip(backend, (die, col, row, layer), mwf, mwt).unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            return (fuzzer, site_name, pin_name);
        }
        panic!("umm failed at {wire_target:?} {wname}");
    } else if wname.starts_with("IO.DOUBLE") {
        let (loc, mwt) = orig_target.unwrap();
        let node = backend.egrid.node(loc);
        let mux = &backend.egrid.db.nodes[node.kind].muxes[&mwt];
        for &mwf in &mux.ins {
            let wfname = backend.egrid.db.wires.key(mwf.1);
            if !wfname.starts_with("SINGLE") {
                continue;
            }
            let nwt = backend
                .egrid
                .resolve_wire((die, node.tiles[mwf.0], mwf.1))
                .unwrap();
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wa, wb) = resolve_int_pip(backend, loc, mwf, mwt).unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wa, wb),
                Value::FromPin(site_name, pin_name.into()),
            );
            return (fuzzer, site_name, pin_name);
        }
        panic!("umm failed at {wire_target:?} {wname}");
    } else {
        panic!("how to drive {wname}");
    }
}

fn get_v4_vref_rows(
    edev: &prjcombine_virtex4::expanded::ExpandedDevice,
    die: DieId,
    col: ColId,
    row: RowId,
) -> Vec<RowId> {
    match edev.kind {
        prjcombine_virtex4::grid::GridKind::Virtex4 => {
            let row_cfg = edev.grids[die].row_reg_bot(edev.grids[die].reg_cfg);
            if Some(col) == edev.col_lio || Some(col) == edev.col_rio {
                let mut reg = edev.grids[die].row_to_reg(row);
                if reg.to_idx() % 2 == 1 {
                    reg -= 1;
                }
                let bot = edev.grids[die].row_reg_bot(reg);
                vec![bot + 4, bot + 12, bot + 20, bot + 28]
            } else if row < edev.row_dcmiob.unwrap() + 8 {
                vec![edev.row_dcmiob.unwrap() + 4]
            } else if row < row_cfg {
                let mut res = vec![];
                let mut vref_row = edev.row_dcmiob.unwrap() + 12;
                while vref_row < row_cfg - 8 {
                    res.push(vref_row);
                    vref_row += 8;
                }
                res
            } else if row < edev.row_iobdcm.unwrap() - 8 {
                let mut res = vec![];
                let mut vref_row = row_cfg + 12;
                while vref_row < edev.row_iobdcm.unwrap() - 8 {
                    res.push(vref_row);
                    vref_row += 8;
                }
                res
            } else {
                vec![edev.row_iobdcm.unwrap() - 4]
            }
        }
        prjcombine_virtex4::grid::GridKind::Virtex5 => {
            let reg = edev.grids[die].row_to_reg(row);
            let bot = edev.grids[die].row_reg_bot(reg);
            if col == edev.col_cfg
                && (reg == edev.grids[die].reg_cfg || reg == edev.grids[die].reg_cfg - 2)
            {
                vec![bot + 15]
            } else if col == edev.col_cfg
                && (reg == edev.grids[die].reg_cfg - 1 || reg == edev.grids[die].reg_cfg + 1)
            {
                vec![bot + 5]
            } else {
                vec![bot + 5, bot + 15]
            }
        }
        prjcombine_virtex4::grid::GridKind::Virtex6 => {
            let reg = edev.grids[die].row_to_reg(row);
            let bot = edev.grids[die].row_reg_bot(reg);
            vec![bot + 10, bot + 30]
        }
        prjcombine_virtex4::grid::GridKind::Virtex7 => {
            let reg = edev.grids[die].row_to_reg(row);
            let bot = edev.grids[die].row_reg_bot(reg);
            vec![bot + 11, bot + 37]
        }
    }
}

fn get_v4_vr_row(
    edev: &prjcombine_virtex4::expanded::ExpandedDevice,
    die: DieId,
    col: ColId,
    row: RowId,
) -> RowId {
    match edev.kind {
        prjcombine_virtex4::grid::GridKind::Virtex4 => {
            let row_cfg = edev.grids[die].row_reg_bot(edev.grids[die].reg_cfg);
            if Some(col) == edev.col_lio || Some(col) == edev.col_rio {
                let mut reg = edev.grids[die].row_to_reg(row);
                if reg.to_idx() % 2 == 1 {
                    reg -= 1;
                }
                let bot = edev.grids[die].row_reg_bot(reg);
                bot + 9
            } else if row < edev.row_dcmiob.unwrap() + 8 {
                edev.row_dcmiob.unwrap() + 1
            } else if row < row_cfg {
                todo!()
            } else if row < edev.row_iobdcm.unwrap() - 8 {
                todo!()
            } else {
                edev.row_iobdcm.unwrap() - 2
            }
        }
        prjcombine_virtex4::grid::GridKind::Virtex5 => {
            let row_cfg = edev.grids[die].row_reg_bot(edev.grids[die].reg_cfg);
            if Some(col) == edev.col_lio || Some(col) == edev.col_rio {
                let reg = edev.grids[die].row_to_reg(row);
                let bot = edev.grids[die].row_reg_bot(reg);
                bot + 7
            } else if row < row_cfg - 20 {
                edev.row_dcmiob.unwrap() + 2
            } else if row < row_cfg {
                todo!()
            } else if row < row_cfg + 20 {
                todo!()
            } else {
                edev.row_iobdcm.unwrap() - 3
            }
        }
        prjcombine_virtex4::grid::GridKind::Virtex6 => {
            let reg = edev.grids[die].row_to_reg(row);
            if reg == edev.grids[die].reg_cfg {
                edev.grids[die].row_reg_bot(reg) + 6
            } else if reg == edev.grids[die].reg_cfg - 1 && Some(col) == edev.col_lcio {
                edev.grids[die].row_reg_bot(reg) + 4
            } else if reg == edev.grids[die].reg_cfg - 1 && Some(col) == edev.col_rcio {
                edev.grids[die].row_reg_bot(reg) + 0
            } else {
                edev.grids[die].row_reg_bot(reg) + 14
            }
        }
        prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
    }
}

fn get_v4_center_dci_rows(
    edev: &prjcombine_virtex4::expanded::ExpandedDevice,
    bank: u32,
) -> (Option<RowId>, RowId) {
    let die = DieId::from_idx(0);
    match edev.kind {
        prjcombine_virtex4::grid::GridKind::Virtex4 => match bank {
            1 => (
                if edev.grids[die].row_bufg() == edev.row_iobdcm.unwrap() - 24 {
                    None
                } else {
                    Some(edev.row_iobdcm.unwrap() - 16 - 2)
                },
                edev.grids[die].row_bufg() + 8,
            ),
            2 => (
                if edev.grids[die].row_bufg() == edev.row_dcmiob.unwrap() + 24 {
                    None
                } else {
                    Some(edev.row_dcmiob.unwrap() + 16 + 1)
                },
                edev.grids[die].row_bufg() - 9,
            ),
            3 => (
                Some(edev.row_iobdcm.unwrap() - 2),
                edev.row_iobdcm.unwrap() - 1,
            ),
            4 => (Some(edev.row_dcmiob.unwrap() + 1), edev.row_dcmiob.unwrap()),
            _ => unreachable!(),
        },
        prjcombine_virtex4::grid::GridKind::Virtex5 => match bank {
            1 => (None, edev.grids[die].row_bufg() + 10),
            2 => (None, edev.grids[die].row_bufg() - 11),
            3 => (
                Some(edev.grids[die].row_bufg() + 30 - 3),
                edev.grids[die].row_bufg() + 30 - 1,
            ),
            4 => (
                Some(edev.grids[die].row_bufg() - 30 + 2),
                edev.grids[die].row_bufg() - 30,
            ),
            _ => unreachable!(),
        },
        prjcombine_virtex4::grid::GridKind::Virtex6 => match bank {
            24 => (
                Some(edev.grids[die].row_bufg() - 40 + 4),
                edev.grids[die].row_bufg() - 40,
            ),
            25 => (
                Some(edev.grids[die].row_bufg() + 6),
                edev.grids[die].row_bufg(),
            ),
            26 => (
                Some(edev.grids[die].row_bufg() + 40 + 16),
                edev.grids[die].row_bufg() + 40,
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

impl<'a> BelKV {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: NodeLoc,
        bel: BelId,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        let node = backend.egrid.node(loc);
        let nnode = &backend.ngrid.nodes[&loc];
        let node_data = &backend.egrid.db.nodes[node.kind];
        let bel_data = &node_data.bels[bel];
        let node_naming = &backend.ngrid.db.node_namings[nnode.naming];
        let bel_naming = &node_naming.bels[bel];
        Some(match self {
            BelKV::Nop => fuzzer,
            BelKV::Mode(mode) => {
                let site = &nnode.bels[bel];
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), "MAIN");
                }
                fuzzer.base(Key::SiteMode(site), mode)
            }
            BelKV::Unused => {
                let site = &nnode.bels[bel];
                fuzzer.base(Key::SiteMode(site), None)
            }
            BelKV::Attr(attr, val) => {
                let site = &nnode.bels[bel];
                fuzzer.base(Key::SiteAttr(site, attr.clone()), val)
            }
            BelKV::AttrAny(attr, vals) => {
                let site = &nnode.bels[bel];
                fuzzer.base_any(
                    Key::SiteAttr(site, attr.clone()),
                    vals.iter().map(Value::from),
                )
            }
            BelKV::Global(kind, opt, val) => {
                let site = &nnode.bels[bel];
                fuzzer.base(Key::GlobalOpt(kind.apply(backend, opt, site)), val)
            }
            BelKV::Pin(pin, val) => {
                let site = &nnode.bels[bel];
                fuzzer.base(Key::SitePin(site, pin.clone()), *val)
            }
            BelKV::PinFrom(pin, kind) => {
                let site = &nnode.bels[bel];
                fuzzer.base(Key::SitePinFrom(site, pin.clone()), *kind)
            }
            BelKV::PinPips(pin) => {
                let pin_naming = &bel_naming.pins[pin];
                for pip in &pin_naming.pips {
                    fuzzer = fuzzer.base(
                        Key::Pip(&nnode.names[pip.tile], &pip.wire_from, &pip.wire_to),
                        true,
                    );
                }
                fuzzer
            }
            BelKV::PinNodeMutexShared(pin) => {
                let pin_data = &bel_data.pins[pin];
                for &wire in &pin_data.wires {
                    let node = backend.egrid.node(loc);
                    let node = backend
                        .egrid
                        .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                    fuzzer = fuzzer.base(Key::NodeMutex(node), "SHARED");
                }
                fuzzer
            }
            BelKV::GlobalMutexHere(name) => fuzzer.base(
                Key::GlobalMutex(name.clone()),
                Value::Bel(loc.0, loc.1, loc.2, loc.3, bel),
            ),
            BelKV::RowMutexHere(name) => fuzzer.base(
                Key::RowMutex(name.clone(), loc.2),
                Value::Bel(loc.0, loc.1, loc.2, loc.3, bel),
            ),
            BelKV::Mutex(name, val) => fuzzer.base(
                Key::BelMutex((loc.0, loc.1, loc.2, loc.3, bel), name.clone()),
                val,
            ),
            BelKV::IsBonded => {
                let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
                    unreachable!()
                };
                match &backend.ebonds[pkg] {
                    ExpandedBond::Virtex(ebond) => {
                        let ExpandedNamedDevice::Virtex(endev) = backend.endev else {
                            unreachable!()
                        };
                        let crd = endev.grid.get_io_crd(loc.1, loc.2, bel);
                        if !ebond.ios.contains_key(&crd) {
                            return None;
                        }
                        fuzzer
                    }
                    ExpandedBond::Spartan6(ebond) => {
                        let ExpandedNamedDevice::Spartan6(endev) = backend.endev else {
                            unreachable!()
                        };
                        let crd = endev.grid.get_io_crd(loc.1, loc.2, bel);
                        if !ebond.ios.contains_key(&crd) {
                            return None;
                        }
                        fuzzer
                    }
                    ExpandedBond::Virtex4(ebond) => {
                        let node = backend.egrid.node(loc);
                        let ExpandedDevice::Virtex4(edev) = backend.edev else {
                            unreachable!()
                        };
                        let io = edev.get_io_info(prjcombine_virtex4::expanded::IoCoord {
                            die: loc.0,
                            col: loc.1,
                            row: loc.2,
                            iob: TileIobId::from_idx(
                                if edev.egrid.db.nodes.key(node.kind).ends_with("PAIR") {
                                    bel.to_idx() % 2
                                } else {
                                    0
                                },
                            ),
                        });
                        if !ebond.ios.contains_key(&(io.bank, io.biob)) {
                            return None;
                        }
                        fuzzer
                    }
                    _ => todo!(),
                }
            }
            BelKV::IsBank(bank) => match backend.edev {
                ExpandedDevice::Spartan6(edev) => {
                    let crd = edev.grid.get_io_crd(loc.1, loc.2, bel);
                    if edev.grid.get_io_bank(crd) != *bank {
                        return None;
                    }
                    fuzzer
                }
                _ => todo!(),
            },
            BelKV::IsDiff => {
                let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
                    unreachable!()
                };
                match &backend.ebonds[pkg] {
                    ExpandedBond::Virtex(ebond) => {
                        let ExpandedNamedDevice::Virtex(endev) = backend.endev else {
                            unreachable!()
                        };
                        let crd = endev.grid.get_io_crd(loc.1, loc.2, bel);
                        if !ebond.bond.diffp.contains(&crd) && !ebond.bond.diffn.contains(&crd) {
                            return None;
                        }
                        fuzzer
                    }
                    _ => todo!(),
                }
            }
            BelKV::IsVref => {
                let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
                    unreachable!()
                };
                match &backend.ebonds[pkg] {
                    ExpandedBond::Virtex(ebond) => {
                        let ExpandedNamedDevice::Virtex(endev) = backend.endev else {
                            unreachable!()
                        };
                        let crd = endev.grid.get_io_crd(loc.1, loc.2, bel);
                        if !ebond.bond.vref.contains(&crd) {
                            return None;
                        }
                        fuzzer
                    }
                    ExpandedBond::Virtex2(ebond) => {
                        let ExpandedNamedDevice::Virtex2(endev) = backend.endev else {
                            unreachable!()
                        };
                        let crd = endev.grid.get_io_crd(loc.1, loc.2, bel);
                        if !ebond.bond.vref.contains(&crd) {
                            return None;
                        }
                        fuzzer
                    }
                    ExpandedBond::Spartan6(ebond) => {
                        let ExpandedNamedDevice::Spartan6(endev) = backend.endev else {
                            unreachable!()
                        };
                        let crd = endev.grid.get_io_crd(loc.1, loc.2, bel);
                        if !ebond.bond.vref.contains(&crd) {
                            return None;
                        }
                        fuzzer
                    }
                    ExpandedBond::Virtex4(ebond) => {
                        let node = backend.egrid.node(loc);
                        let ExpandedDevice::Virtex4(edev) = backend.edev else {
                            unreachable!()
                        };
                        let io = edev.get_io_info(prjcombine_virtex4::expanded::IoCoord {
                            die: loc.0,
                            col: loc.1,
                            row: loc.2,
                            iob: TileIobId::from_idx(
                                if edev.egrid.db.nodes.key(node.kind).ends_with("PAIR") {
                                    bel.to_idx() % 2
                                } else {
                                    0
                                },
                            ),
                        });
                        if !io.is_vref || !ebond.ios.contains_key(&(io.bank, io.biob)) {
                            return None;
                        }
                        fuzzer
                    }
                    _ => todo!(),
                }
            }
            BelKV::IsVr => match backend.edev {
                ExpandedDevice::Virtex2(edev) => {
                    let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
                        unreachable!()
                    };
                    let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[pkg] else {
                        unreachable!()
                    };
                    let crd = edev.grid.get_io_crd(loc.1, loc.2, bel);
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
                    if !ebond.ios.contains_key(&crd) {
                        return None;
                    }
                    fuzzer
                }
                _ => todo!(),
            },
            BelKV::PrepVref => match backend.edev {
                ExpandedDevice::Virtex4(edev) => {
                    let is_vref_row = match edev.kind {
                        prjcombine_virtex4::grid::GridKind::Virtex4 => loc.2.to_idx() % 8 == 4,
                        prjcombine_virtex4::grid::GridKind::Virtex5 => loc.2.to_idx() % 10 == 5,
                        prjcombine_virtex4::grid::GridKind::Virtex6 => loc.2.to_idx() % 20 == 10,
                        prjcombine_virtex4::grid::GridKind::Virtex7 => {
                            matches!(loc.2.to_idx() % 50, 11 | 37)
                        }
                    };
                    if is_vref_row {
                        return None;
                    }
                    let vref_rows = get_v4_vref_rows(edev, loc.0, loc.1, loc.2);
                    if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex4 {
                        let (layer, _) = edev
                            .egrid
                            .find_node_loc(loc.0, (loc.1, vref_rows[0]), |node| {
                                edev.egrid.db.nodes.key(node.kind) == "IO"
                            })
                            .unwrap();
                        fuzzer = fuzzer.fuzz(
                            Key::TileMutex((loc.0, loc.1, vref_rows[0], layer), "VREF".to_string()),
                            None,
                            "EXCLUSIVE",
                        );
                    } else {
                        let hclk_row = edev.grids[loc.0].row_hclk(loc.2);
                        // Take exclusive mutex on VREF.
                        let (layer, _) = edev
                            .egrid
                            .find_node_loc(loc.0, (loc.1, hclk_row), |node| {
                                matches!(
                                    &edev.egrid.db.nodes.key(node.kind)[..],
                                    "HCLK_IOI"
                                        | "HCLK_IOI_HP"
                                        | "HCLK_IOI_HR"
                                        | "HCLK_IOI_CENTER"
                                        | "HCLK_IOI_TOPCEN"
                                        | "HCLK_IOI_BOTCEN"
                                        | "HCLK_IOI_CMT"
                                        | "HCLK_CMT_IOI"
                                )
                            })
                            .unwrap();
                        fuzzer = fuzzer.fuzz(
                            Key::TileMutex((loc.0, loc.1, hclk_row, layer), "VREF".to_string()),
                            None,
                            "EXCLUSIVE",
                        );
                    }
                    for vref_row in vref_rows {
                        let site = backend
                            .ngrid
                            .get_bel_name(loc.0, loc.1, vref_row, "IOB0")
                            .unwrap();
                        fuzzer = fuzzer.base(Key::SiteMode(site), None);
                    }
                    fuzzer
                }
                _ => todo!(),
            },
            BelKV::PrepVrefInternal(vref) => match backend.edev {
                ExpandedDevice::Virtex4(edev) => {
                    let hclk_row = edev.grids[loc.0].row_hclk(loc.2);
                    // Take exclusive mutex on VREF.
                    let (layer, _) = edev
                        .egrid
                        .find_node_loc(loc.0, (loc.1, hclk_row), |node| {
                            matches!(
                                &edev.egrid.db.nodes.key(node.kind)[..],
                                "HCLK_IOI"
                                    | "HCLK_IOI_HP"
                                    | "HCLK_IOI_HR"
                                    | "HCLK_IOI_CENTER"
                                    | "HCLK_IOI_TOPCEN"
                                    | "HCLK_IOI_BOTCEN"
                                    | "HCLK_IOI_CMT"
                                    | "HCLK_CMT_IOI"
                            )
                        })
                        .unwrap();
                    fuzzer = fuzzer.fuzz(
                        Key::TileMutex((loc.0, loc.1, hclk_row, layer), "VREF".to_string()),
                        None,
                        "EXCLUSIVE",
                    );
                    let io = edev.get_io_info(prjcombine_virtex4::expanded::IoCoord {
                        die: loc.0,
                        col: loc.1,
                        row: loc.2,
                        iob: TileIobId::from_idx(
                            if edev.egrid.db.nodes.key(node.kind).ends_with("PAIR") {
                                bel.to_idx() % 2
                            } else {
                                0
                            },
                        ),
                    });
                    fuzzer = fuzzer.fuzz(Key::InternalVref(io.bank), None, *vref);
                    fuzzer
                }
                _ => todo!(),
            },
            BelKV::PrepDci => match backend.edev {
                ExpandedDevice::Virtex4(edev) => {
                    match edev.kind {
                        prjcombine_virtex4::grid::GridKind::Virtex4 => {
                            if loc.1 == edev.col_cfg {
                                // Center column is more trouble than it's worth.
                                return None;
                            }
                            if loc.2.to_idx() % 32 == 9 {
                                // Not in VR tile please.
                                return None;
                            }
                            // Ensure nothing is placed in VR.
                            let vr_row = RowId::from_idx(loc.2.to_idx() / 32 * 32 + 9);
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(loc.0, loc.1, vr_row, bel)
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                            // Take exclusive mutex on bank DCI.
                            let (layer, _) = edev
                                .egrid
                                .find_node_loc(loc.0, (loc.1, vr_row - 1), |node| {
                                    edev.egrid.db.nodes.key(node.kind) == "HCLK_IOIS_DCI"
                                })
                                .unwrap();
                            fuzzer = fuzzer.fuzz(
                                Key::TileMutex(
                                    (loc.0, loc.1, vr_row - 1, layer),
                                    "BANK_DCI".to_string(),
                                ),
                                None,
                                "EXCLUSIVE",
                            );
                            // Take shared mutex on global DCI.
                            fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
                            // Anchor global DCI by putting something in bottom IOB of center column.
                            let site = backend
                                .ngrid
                                .get_bel_name(loc.0, edev.col_cfg, edev.row_dcmiob.unwrap(), "IOB0")
                                .unwrap();
                            fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                            fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                            fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
                            fuzzer =
                                fuzzer.base(Key::SiteAttr(site, "IOATTRBOX".into()), "LVDCI_33");
                            // Ensure anchor VR IOBs are free.
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(
                                        loc.0,
                                        edev.col_cfg,
                                        edev.row_dcmiob.unwrap() + 1,
                                        bel,
                                    )
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                        }
                        prjcombine_virtex4::grid::GridKind::Virtex5 => {
                            if loc.1 == edev.col_cfg {
                                // Center column is more trouble than it's worth.
                                return None;
                            }
                            if loc.2.to_idx() % 20 == 7 {
                                // Not in VR tile please.
                                return None;
                            }
                            // Ensure nothing is placed in VR.
                            let vr_row = RowId::from_idx(loc.2.to_idx() / 20 * 20 + 7);
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(loc.0, loc.1, vr_row, bel)
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                            // Take exclusive mutex on bank DCI.
                            let (layer, _) = edev
                                .egrid
                                .find_node_loc(loc.0, (loc.1, vr_row + 3), |node| {
                                    edev.egrid.db.nodes.key(node.kind) == "HCLK_IOI"
                                })
                                .unwrap();
                            fuzzer = fuzzer.fuzz(
                                Key::TileMutex(
                                    (loc.0, loc.1, vr_row + 3, layer),
                                    "BANK_DCI".to_string(),
                                ),
                                None,
                                "EXCLUSIVE",
                            );
                            // Take shared mutex on global DCI.
                            fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
                            // Anchor global DCI by putting something in bottom IOB of center column.
                            let site = backend
                                .ngrid
                                .get_bel_name(
                                    loc.0,
                                    edev.col_cfg,
                                    edev.grids[loc.0].row_bufg() - 30,
                                    "IOB0",
                                )
                                .unwrap();
                            fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                            fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                            fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
                            fuzzer =
                                fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_33");
                            // Ensure anchor VR IOBs are free.
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(
                                        loc.0,
                                        edev.col_cfg,
                                        edev.grids[loc.0].row_bufg() - 30 + 2,
                                        bel,
                                    )
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                        }
                        prjcombine_virtex4::grid::GridKind::Virtex6 => {
                            // Avoid bank 25, which is our (arbitrary) anchor.
                            if loc.1 == edev.col_lcio.unwrap()
                                && edev.grids[loc.0].row_to_reg(loc.2) == edev.grids[loc.0].reg_cfg
                            {
                                return None;
                            }
                            let vr_row = get_v4_vr_row(edev, loc.0, loc.1, loc.2);
                            if loc.2 == vr_row {
                                // Not in VR tile please.
                                return None;
                            }
                            // Ensure nothing is placed in VR.
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(loc.0, loc.1, vr_row, bel)
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                            // Take exclusive mutex on bank DCI.
                            let (layer, _) = edev
                                .egrid
                                .find_node_loc(
                                    loc.0,
                                    (loc.1, edev.grids[loc.0].row_hclk(loc.2)),
                                    |node| edev.egrid.db.nodes.key(node.kind) == "HCLK_IOI",
                                )
                                .unwrap();
                            fuzzer = fuzzer.fuzz(
                                Key::TileMutex(
                                    (loc.0, loc.1, edev.grids[loc.0].row_hclk(loc.2), layer),
                                    "BANK_DCI".to_string(),
                                ),
                                None,
                                "EXCLUSIVE",
                            );
                            // Take shared mutex on global DCI.
                            fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
                            // Anchor global DCI by putting something in bottom IOB of center column.
                            let site = backend
                                .ngrid
                                .get_bel_name(
                                    loc.0,
                                    edev.col_lcio.unwrap(),
                                    edev.grids[loc.0].row_bufg(),
                                    "IOB0",
                                )
                                .unwrap();
                            fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                            fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                            fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
                            fuzzer =
                                fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_25");
                            // Ensure anchor VR IOBs are free.
                            for bel in ["IOB0", "IOB1"] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(
                                        loc.0,
                                        edev.col_lcio.unwrap(),
                                        edev.grids[loc.0].row_bufg() + 6,
                                        bel,
                                    )
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                            // Make note of anchor VCCO.
                            let (layer, _) = edev
                                .egrid
                                .find_node_loc(
                                    loc.0,
                                    (edev.col_lcio.unwrap(), edev.grids[loc.0].row_bufg() + 20),
                                    |node| edev.egrid.db.nodes.key(node.kind) == "HCLK_IOI",
                                )
                                .unwrap();
                            fuzzer = fuzzer.base(
                                Key::TileMutex(
                                    (
                                        loc.0,
                                        edev.col_lcio.unwrap(),
                                        edev.grids[loc.0].row_bufg() + 20,
                                        layer,
                                    ),
                                    "VCCO".to_string(),
                                ),
                                "2500",
                            );
                        }
                        prjcombine_virtex4::grid::GridKind::Virtex7 => {
                            // Avoid anchor bank.
                            let anchor_reg = if edev.grids[loc.0].has_ps {
                                prjcombine_virtex4::grid::RegId::from_idx(
                                    edev.grids[loc.0].regs - 1,
                                )
                            } else {
                                prjcombine_virtex4::grid::RegId::from_idx(0)
                            };
                            if loc.1 == edev.col_rio.unwrap()
                                && edev.grids[loc.0].row_to_reg(loc.2) == anchor_reg
                            {
                                return None;
                            }
                            // Ensure nothing is placed in VR.
                            for row in [
                                edev.grids[loc.0].row_hclk(loc.2) - 25,
                                edev.grids[loc.0].row_hclk(loc.2) + 24,
                            ] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(loc.0, loc.1, row, "IOB")
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                            // Take exclusive mutex on bank DCI.
                            let (layer, _) = edev
                                .egrid
                                .find_node_loc(
                                    loc.0,
                                    (loc.1, edev.grids[loc.0].row_hclk(loc.2)),
                                    |node| edev.egrid.db.nodes.key(node.kind) == "HCLK_IOI_HP",
                                )
                                .unwrap();
                            fuzzer = fuzzer.fuzz(
                                Key::TileMutex(
                                    (loc.0, loc.1, edev.grids[loc.0].row_hclk(loc.2), layer),
                                    "BANK_DCI".to_string(),
                                ),
                                None,
                                "EXCLUSIVE",
                            );
                            // Take shared mutex on global DCI.
                            fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
                            // Anchor global DCI by putting something in arbitrary bank.
                            let site = backend
                                .ngrid
                                .get_bel_name(
                                    loc.0,
                                    edev.col_rio.unwrap(),
                                    edev.grids[loc.0].row_reg_bot(anchor_reg) + 1,
                                    "IOB0",
                                )
                                .unwrap();
                            fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                            fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
                            fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
                            fuzzer =
                                fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_18");
                            // Ensure anchor VR IOBs are free.
                            for row in [
                                edev.grids[loc.0].row_reg_hclk(anchor_reg) - 25,
                                edev.grids[loc.0].row_reg_hclk(anchor_reg) + 24,
                            ] {
                                let site = backend
                                    .ngrid
                                    .get_bel_name(loc.0, edev.col_rio.unwrap(), row, "IOB")
                                    .unwrap();
                                fuzzer = fuzzer.base(Key::SiteMode(site), None);
                            }
                            // Make note of anchor VCCO.
                            let (layer, _) = edev
                                .egrid
                                .find_node_loc(
                                    loc.0,
                                    (
                                        edev.col_rio.unwrap(),
                                        edev.grids[loc.0].row_reg_hclk(anchor_reg),
                                    ),
                                    |node| edev.egrid.db.nodes.key(node.kind) == "HCLK_IOI_HP",
                                )
                                .unwrap();
                            fuzzer = fuzzer.base(
                                Key::TileMutex(
                                    (
                                        loc.0,
                                        edev.col_rio.unwrap(),
                                        edev.grids[loc.0].row_reg_hclk(anchor_reg),
                                        layer,
                                    ),
                                    "VCCO".to_string(),
                                ),
                                "1800",
                            );
                        }
                    }
                    fuzzer
                }
                _ => todo!(),
            },
            BelKV::PrepDiffOut => match backend.edev {
                ExpandedDevice::Virtex4(edev) => {
                    match edev.kind {
                        prjcombine_virtex4::grid::GridKind::Virtex4 => {
                            // Skip non-NC pads.
                            if loc.1 == edev.col_cfg {
                                return None;
                            }
                            if matches!(loc.2.to_idx() % 16, 7 | 8) {
                                return None;
                            }
                            let lvds_row = RowId::from_idx(loc.2.to_idx() / 32 * 32 + 24);
                            // Take exclusive mutex on bank LVDS.
                            let (layer, _) = edev
                                .egrid
                                .find_node_loc(loc.0, (loc.1, lvds_row), |node| {
                                    edev.egrid.db.nodes.key(node.kind) == "HCLK_IOIS_LVDS"
                                })
                                .unwrap();
                            fuzzer = fuzzer.fuzz(
                                Key::TileMutex(
                                    (loc.0, loc.1, lvds_row, layer),
                                    "BANK_LVDS".to_string(),
                                ),
                                None,
                                "EXCLUSIVE",
                            );
                        }
                        prjcombine_virtex4::grid::GridKind::Virtex5
                        | prjcombine_virtex4::grid::GridKind::Virtex6
                        | prjcombine_virtex4::grid::GridKind::Virtex7 => {
                            let lvds_row = edev.grids[loc.0].row_hclk(loc.2);
                            // Take exclusive mutex on bank LVDS.
                            let (layer, _) = edev
                                .egrid
                                .find_node_loc(loc.0, (loc.1, lvds_row), |node| {
                                    matches!(
                                        &edev.egrid.db.nodes.key(node.kind)[..],
                                        "HCLK_IOI"
                                            | "HCLK_IOI_HP"
                                            | "HCLK_IOI_HR"
                                            | "HCLK_IOI_CENTER"
                                            | "HCLK_IOI_TOPCEN"
                                            | "HCLK_IOI_BOTCEN"
                                            | "HCLK_IOI_CMT"
                                            | "HCLK_CMT_IOI"
                                    )
                                })
                                .unwrap();
                            fuzzer = fuzzer.fuzz(
                                Key::TileMutex(
                                    (loc.0, loc.1, lvds_row, layer),
                                    "BANK_LVDS".to_string(),
                                ),
                                None,
                                "EXCLUSIVE",
                            );
                        }
                    }
                    fuzzer
                }
                _ => todo!(),
            },
            BelKV::OtherIobInput(iostd) | BelKV::OtherIobDiffOutput(iostd) => {
                let is_diff = !matches!(*self, BelKV::OtherIobInput(_));
                let is_out = matches!(*self, BelKV::OtherIobDiffOutput(_));
                match backend.edev {
                    ExpandedDevice::Virtex(edev) => {
                        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package]
                        else {
                            unreachable!()
                        };
                        let ExpandedBond::Virtex(ref ebond) = backend.ebonds[pkg] else {
                            unreachable!()
                        };
                        let ExpandedNamedDevice::Virtex(endev) = backend.endev else {
                            unreachable!()
                        };
                        let bel_key = backend.egrid.db.nodes[node.kind].bels.key(bel);
                        let (crd, orig_bank) = if bel_key.starts_with("IOB") {
                            let crd = edev.grid.get_io_crd(loc.1, loc.2, bel);
                            (Some(crd), edev.grid.get_io_bank(crd))
                        } else {
                            (
                                None,
                                if loc.2 == edev.grid.row_bio() {
                                    if bel_key == "GCLKIOB0" { 4 } else { 5 }
                                } else {
                                    if bel_key == "GCLKIOB0" { 1 } else { 0 }
                                },
                            )
                        };
                        for io in edev.grid.get_bonded_ios() {
                            let bank = edev.grid.get_io_bank(io);
                            if Some(io) != crd && bank == orig_bank && ebond.ios.contains_key(&io) {
                                let site = endev.get_io_name(io);

                                fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                                fuzzer =
                                    fuzzer.base(Key::SiteAttr(site, "IOATTRBOX".into()), iostd);
                                fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), "1");
                                fuzzer = fuzzer.base(Key::SiteAttr(site, "OUTMUX".into()), None);
                                fuzzer = fuzzer.base(Key::SiteAttr(site, "TSEL".into()), None);
                                fuzzer = fuzzer.base(Key::SitePin(site, "I".into()), true);
                                return Some(fuzzer);
                            }
                        }
                        return None;
                    }
                    ExpandedDevice::Virtex2(edev) => {
                        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package]
                        else {
                            unreachable!()
                        };
                        let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[pkg] else {
                            unreachable!()
                        };
                        let ExpandedNamedDevice::Virtex2(endev) = backend.endev else {
                            unreachable!()
                        };
                        let crd = endev.grid.get_io_crd(loc.1, loc.2, bel);
                        let orig_io_info = edev.grid.get_io_info(crd);
                        for io in edev.grid.get_bonded_ios() {
                            let io_info = edev.grid.get_io_info(io);
                            if io != crd
                                && orig_io_info.bank == io_info.bank
                                && io_info.pad_kind != Some(IobKind::Clk)
                                && (!is_diff
                                    || io_info.diff != prjcombine_virtex2::grid::IoDiffKind::None)
                                && ebond.ios.contains_key(&io)
                            {
                                let site = endev.get_io_name(io);

                                fuzzer = fuzzer.base(
                                    Key::SiteMode(site),
                                    if is_diff {
                                        match io_info.diff {
                                            prjcombine_virtex2::grid::IoDiffKind::P(_) => {
                                                if edev.grid.kind.is_spartan3a() {
                                                    "DIFFMI_NDT"
                                                } else if edev.grid.kind.is_spartan3ea() {
                                                    "DIFFMI"
                                                } else {
                                                    "DIFFM"
                                                }
                                            }
                                            prjcombine_virtex2::grid::IoDiffKind::N(_) => {
                                                if edev.grid.kind.is_spartan3a() {
                                                    "DIFFSI_NDT"
                                                } else if edev.grid.kind.is_spartan3ea() {
                                                    "DIFFSI"
                                                } else {
                                                    "DIFFS"
                                                }
                                            }
                                            prjcombine_virtex2::grid::IoDiffKind::None => {
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
                                fuzzer =
                                    fuzzer.base(Key::SiteAttr(site, "IOATTRBOX".into()), iostd);
                                if !is_out {
                                    fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), "1");
                                    fuzzer = fuzzer.base(Key::SitePin(site, "I".into()), true);
                                    if edev.grid.kind.is_spartan3a() {
                                        fuzzer = fuzzer.base(
                                            Key::SiteAttr(site, "IBUF_DELAY_VALUE".into()),
                                            "DLY0",
                                        );
                                        fuzzer = fuzzer.base(
                                            Key::SiteAttr(site, "DELAY_ADJ_ATTRBOX".into()),
                                            "FIXED",
                                        );
                                        fuzzer =
                                            fuzzer.base(Key::SiteAttr(site, "SEL_MUX".into()), "0");
                                    }
                                } else {
                                    fuzzer = fuzzer.base(Key::SiteAttr(site, "OMUX".into()), "O1");
                                    fuzzer = fuzzer.base(Key::SiteAttr(site, "O1INV".into()), "O1");
                                    fuzzer = fuzzer.base(Key::SitePin(site, "O1".into()), true);
                                }
                                return Some(fuzzer);
                            }
                        }
                        return None;
                    }
                    _ => todo!(),
                }
            }
            BelKV::BankDiffOutput(stda, stdb) => {
                match backend.edev {
                    ExpandedDevice::Virtex2(edev) => {
                        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package]
                        else {
                            unreachable!()
                        };
                        let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[pkg] else {
                            unreachable!()
                        };
                        let ExpandedNamedDevice::Virtex2(endev) = backend.endev else {
                            unreachable!()
                        };
                        let crd = edev.grid.get_io_crd(loc.1, loc.2, bel);
                        let stds = if let Some(stdb) = stdb {
                            &[stda, stdb][..]
                        } else {
                            &[stda][..]
                        };
                        let bank = edev.grid.get_io_info(crd).bank;
                        let mut done = 0;
                        let mut ios = edev.grid.get_bonded_ios();
                        if edev.grid.kind != prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                            ios.reverse();
                        }
                        for &io in &ios {
                            if io == crd {
                                if edev.grid.kind.is_spartan3ea() {
                                    // too much thinking. just pick a different loc.
                                    return None;
                                } else {
                                    continue;
                                }
                            }
                            let io_info = edev.grid.get_io_info(io);
                            if !ebond.ios.contains_key(&io)
                                || io_info.bank != bank
                                || io_info.pad_kind != Some(IobKind::Iob)
                            {
                                continue;
                            }
                            let prjcombine_virtex2::grid::IoDiffKind::P(other_iob) = io_info.diff
                            else {
                                continue;
                            };
                            // okay, got a pair.
                            let other_io = io.with_iob(other_iob);
                            let site_p = endev.get_io_name(io);
                            let site_n = endev.get_io_name(other_io);
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
                                .base(Key::SiteAttr(site_p, "IOATTRBOX".into()), std)
                                .base(Key::SiteAttr(site_n, "IOATTRBOX".into()), std)
                                .base(Key::SiteAttr(site_p, "OMUX".into()), "O1")
                                .base(Key::SiteAttr(site_p, "O1INV".into()), "O1")
                                .base(Key::SitePin(site_p, "O1".into()), true)
                                .base(Key::SitePin(site_p, "DIFFO_OUT".into()), true)
                                .base(Key::SitePin(site_n, "DIFFO_IN".into()), true)
                                .base(Key::SiteAttr(site_n, "DIFFO_IN_USED".into()), "0");
                            if edev.grid.kind.is_spartan3a() {
                                fuzzer = fuzzer
                                    .base(Key::SiteAttr(site_p, "SUSPEND".into()), "3STATE")
                                    .base(Key::SiteAttr(site_n, "SUSPEND".into()), "3STATE");
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
                    _ => todo!(),
                }
            }
            BelKV::NotIbuf => {
                if !nnode.names[NodeRawTileId::from_idx(0)].contains("IOIS") {
                    return None;
                }
                fuzzer
            }
            BelKV::VirtexIsDllIob(val) => {
                let ExpandedDevice::Virtex(edev) = backend.edev else {
                    unreachable!()
                };
                let is_dll = edev.grid.kind != prjcombine_virtex::grid::GridKind::Virtex
                    && ((loc.1 == edev.grid.col_clk() - 1 && bel.to_idx() == 1)
                        || (loc.1 == edev.grid.col_clk() && bel.to_idx() == 2));
                if *val != is_dll {
                    return None;
                }
                fuzzer
            }
            BelKV::Xc4000TbufSplitter(dir, buf) => {
                let (wire_from, wire_to, pin_from, pin_to, ex_from, ex_to) = match dir {
                    Dir::E => (
                        bel_data.pins["L"].wires.iter().copied().next().unwrap(),
                        bel_data.pins["R"].wires.iter().copied().next().unwrap(),
                        &bel_naming.pins["L"].name,
                        &bel_naming.pins["R"].name,
                        &bel_naming.pins["L.EXCL"].name,
                        &bel_naming.pins["R.EXCL"].name,
                    ),
                    Dir::W => (
                        bel_data.pins["R"].wires.iter().copied().next().unwrap(),
                        bel_data.pins["L"].wires.iter().copied().next().unwrap(),
                        &bel_naming.pins["R"].name,
                        &bel_naming.pins["L"].name,
                        &bel_naming.pins["R.EXCL"].name,
                        &bel_naming.pins["L.EXCL"].name,
                    ),
                    _ => unreachable!(),
                };
                let res_from = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire_from.0], wire_from.1))
                    .unwrap();
                let res_to = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire_to.0], wire_to.1))
                    .unwrap();
                let fuzzer = fuzzer.fuzz(Key::NodeMutex(res_to), None, "EXCLUSIVE-TGT");
                let (fuzzer, src_site, src_pin) =
                    drive_xc4000_wire(backend, fuzzer, res_from, Some((loc, wire_from)), res_to);
                let tile = &nnode.names[bel_naming.tile];
                if *buf {
                    fuzzer
                        .fuzz(
                            Key::Pip(tile, pin_from, ex_from),
                            None,
                            Value::FromPin(src_site, src_pin.into()),
                        )
                        .fuzz(
                            Key::Pip(tile, ex_from, ex_to),
                            None,
                            Value::FromPin(src_site, src_pin.into()),
                        )
                        .fuzz(
                            Key::Pip(tile, ex_to, pin_to),
                            None,
                            Value::FromPin(src_site, src_pin.into()),
                        )
                } else {
                    fuzzer.fuzz(
                        Key::Pip(tile, pin_from, pin_to),
                        None,
                        Value::FromPin(src_site, src_pin.into()),
                    )
                }
            }
            BelKV::Xc4000DriveImux(pin, drive) => {
                let wire = *bel_data.pins[*pin].wires.iter().next().unwrap();
                let res_wire = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire.0], wire.1))
                    .unwrap();
                let fuzzer = fuzzer.fuzz(Key::NodeMutex(res_wire), None, "EXCLUSIVE");
                if *drive {
                    let oloc = (res_wire.0, res_wire.1.0, res_wire.1.1, LayerId::from_idx(0));
                    let onode = backend.egrid.node(oloc);
                    let onode_data = &backend.egrid.db.nodes[onode.kind];
                    let wt = (NodeTileId::from_idx(0), res_wire.2);
                    let mux = &onode_data.muxes[&wt];
                    let wf = *mux.ins.iter().next().unwrap();
                    let res_wf = backend
                        .egrid
                        .resolve_wire((oloc.0, onode.tiles[wf.0], wf.1))
                        .unwrap();
                    let (tile, wa, wb) = resolve_int_pip(backend, oloc, wf, wt).unwrap();
                    fuzzer.base(Key::Pip(tile, wa, wb), true).fuzz(
                        Key::NodeMutex(res_wf),
                        None,
                        "EXCLUSIVE",
                    )
                } else {
                    fuzzer
                }
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum TileFuzzKV<'a> {
    Bel(BelId, BelFuzzKV),
    IobBel(usize, BelId, BelFuzzKV),
    GlobalOpt(String, String),
    GlobalOptDiff(String, String, String),
    Pip(TileWire, TileWire),
    IntPip(NodeWireId, NodeWireId),
    IntfTestPip(NodeWireId, NodeWireId),
    IntfDelay(NodeWireId, bool),
    NodeMutexExclusive(NodeWireId),
    TileMutexExclusive(String),
    RowMutexExclusive(String),
    TileRelated(TileRelation, Box<TileFuzzKV<'a>>),
    PinPair(BelId, String, BelId, String),
    VccoSenseMode(String),
    Raw(Key<'a>, Value<'a>, Value<'a>),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BelGlobalKind {
    Xy,
    Dll,
}

impl BelGlobalKind {
    pub fn apply(self, backend: &IseBackend, opt: &str, site: &str) -> String {
        match self {
            BelGlobalKind::Xy => opt.replace('*', &site[site.rfind('X').unwrap()..]),
            BelGlobalKind::Dll => {
                let ExpandedDevice::Virtex(edev) = backend.edev else {
                    unreachable!()
                };
                if opt == "TESTZD2OSC*"
                    && site.len() == 4
                    && edev.grid.kind != prjcombine_virtex::grid::GridKind::Virtex
                {
                    opt.replace('*', &format!("{}S", &site[3..]))
                } else {
                    opt.replace('*', &site[3..])
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum BelFuzzKV {
    Mode(String),
    ModeDiff(String, String),
    Attr(String, String),
    AttrDiff(String, String, String),
    Pin(String),
    PinFull(String),
    PinPips(String),
    PinFrom(String, PinFromKind, PinFromKind),
    Global(BelGlobalKind, String, String),
    AllIodelay(&'static str),
}

impl<'a> TileFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        let node = backend.egrid.node(loc);
        Some(match self {
            TileFuzzKV::Bel(bel, inner) => inner.apply(backend, loc, *bel, fuzzer)?,
            TileFuzzKV::IobBel(tile, bel, inner) => {
                let ioi_loc = find_ioi(backend, loc, *tile);
                inner.apply(backend, ioi_loc, *bel, fuzzer)?
            }
            TileFuzzKV::GlobalOpt(opt, val) => fuzzer.fuzz(Key::GlobalOpt(opt.clone()), None, val),
            TileFuzzKV::GlobalOptDiff(opt, vala, valb) => {
                fuzzer.fuzz(Key::GlobalOpt(opt.clone()), vala, valb)
            }
            TileFuzzKV::Pip(wa, wb) => {
                let (ta, wa) = resolve_tile_wire(backend, loc, wa)?;
                let (tb, wb) = resolve_tile_wire(backend, loc, wb)?;
                assert_eq!(ta, tb);
                fuzzer.fuzz(Key::Pip(ta, wa, wb), None, true)
            }
            TileFuzzKV::IntPip(wa, wb) => {
                let (tile, wa, wb) = resolve_int_pip(backend, loc, *wa, *wb)?;
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
                let (tile, wa, wb) = resolve_intf_test_pip(backend, loc, *wa, *wb)?;
                fuzzer.fuzz(Key::Pip(tile, wa, wb), None, true)
            }
            TileFuzzKV::IntfDelay(wire, state) => {
                let (tile, wa, wb, wc) = resolve_intf_delay(backend, loc, *wire)?;
                if *state {
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
                fuzzer.fuzz(Key::TileMutex(loc, name.clone()), None, "EXCLUSIVE")
            }
            TileFuzzKV::RowMutexExclusive(name) => {
                fuzzer.fuzz(Key::RowMutex(name.clone(), loc.2), None, "EXCLUSIVE")
            }
            TileFuzzKV::TileRelated(relation, chain) => {
                let loc = resolve_tile_relation(backend, loc, *relation)?;
                chain.apply(backend, loc, fuzzer)?
            }
            TileFuzzKV::PinPair(bel_a, pin_a, bel_b, pin_b) => {
                let site_a = &backend.ngrid.nodes[&loc].bels[*bel_a];
                let site_b = &backend.ngrid.nodes[&loc].bels[*bel_b];
                fuzzer.fuzz(
                    Key::SitePin(site_a, pin_a.clone()),
                    false,
                    Value::FromPin(site_b, pin_b.clone()),
                )
            }
            TileFuzzKV::VccoSenseMode(mode) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let bank = edev
                    .get_io_info(prjcombine_virtex4::expanded::IoCoord {
                        die: loc.0,
                        col: loc.1,
                        row: loc.2,
                        iob: EntityId::from_idx(0),
                    })
                    .bank;
                fuzzer.fuzz(Key::VccoSenseMode(bank), None, mode.clone())
            }
            TileFuzzKV::Raw(key, vala, valb) => {
                fuzzer.fuzz(key.clone(), vala.clone(), valb.clone())
            }
        })
    }
}

impl BelFuzzKV {
    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        loc: NodeLoc,
        bel: BelId,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        let node = backend.egrid.node(loc);
        let nnode = &backend.ngrid.nodes[&loc];
        let node_data = &backend.egrid.db.nodes[node.kind];
        let node_naming = &backend.ngrid.db.node_namings[nnode.naming];
        Some(match self {
            BelFuzzKV::Mode(val) => {
                let site = &nnode.bels[bel];
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), "MAIN");
                }
                fuzzer.fuzz(Key::SiteMode(site), None, val)
            }
            BelFuzzKV::ModeDiff(vala, valb) => {
                let site = &nnode.bels[bel];
                for &(col, row) in node.tiles.values() {
                    fuzzer = fuzzer.base(Key::IntMutex(loc.0, col, row), "MAIN");
                }
                fuzzer.fuzz(Key::SiteMode(site), vala, valb)
            }
            BelFuzzKV::Attr(attr, val) => {
                let site = &nnode.bels[bel];
                fuzzer.fuzz(Key::SiteAttr(site, attr.clone()), None, val)
            }
            BelFuzzKV::AttrDiff(attr, va, vb) => {
                let site = &nnode.bels[bel];
                fuzzer.fuzz(Key::SiteAttr(site, attr.clone()), va, vb)
            }
            BelFuzzKV::Pin(pin) => {
                let site = &nnode.bels[bel];
                fuzzer.fuzz(Key::SitePin(site, pin.clone()), false, true)
            }
            BelFuzzKV::PinFrom(pin, kind_a, kind_b) => {
                let site = &nnode.bels[bel];
                fuzzer.fuzz(Key::SitePinFrom(site, pin.clone()), *kind_a, *kind_b)
            }
            BelFuzzKV::PinFull(pin) => {
                let site = &nnode.bels[bel];
                fuzzer = fuzzer.fuzz(Key::SitePin(site, pin.clone()), false, true);
                let bel_data = &node_data.bels[bel];
                let pin_data = &bel_data.pins[pin];
                let bel_naming = &node_naming.bels[bel];
                let pin_naming = &bel_naming.pins[pin];
                assert_eq!(pin_data.wires.len(), 1);
                let wire = *pin_data.wires.first().unwrap();
                if let Some(pip) = pin_naming.int_pips.get(&wire) {
                    fuzzer = fuzzer.fuzz(
                        Key::Pip(&nnode.names[pip.tile], &pip.wire_from, &pip.wire_to),
                        false,
                        true,
                    );
                }
                fuzzer
            }
            BelFuzzKV::PinPips(pin) => {
                let bel_naming = &node_naming.bels[bel];
                let pin_naming = &bel_naming.pins[pin];
                for pip in &pin_naming.pips {
                    fuzzer = fuzzer.fuzz(
                        Key::Pip(&nnode.names[pip.tile], &pip.wire_from, &pip.wire_to),
                        false,
                        true,
                    );
                }
                fuzzer
            }
            BelFuzzKV::Global(kind, name, val) => {
                let site = &backend.ngrid.nodes[&loc].bels[bel];
                fuzzer.fuzz(Key::GlobalOpt(kind.apply(backend, name, site)), None, val)
            }
            BelFuzzKV::AllIodelay(mode) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let reg = edev.grids[loc.0].row_to_reg(loc.2);
                let bot = edev.grids[loc.0].row_reg_bot(reg);
                for i in 0..edev.grids[loc.0].rows_per_reg() {
                    let row = bot + i;
                    for bel in ["IODELAY0", "IODELAY1"] {
                        if let Some(site) = backend.ngrid.get_bel_name(loc.0, loc.1, row, bel) {
                            fuzzer = fuzzer.fuzz(Key::SiteMode(site), None, "IODELAY");
                            fuzzer =
                                fuzzer.fuzz(Key::SiteAttr(site, "IDELAY_TYPE".into()), None, *mode);
                        }
                    }
                }
                fuzzer
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum TileMultiFuzzKV<'a> {
    SiteAttr(BelId, String, MultiValue),
    IobSiteAttr(usize, BelId, String, MultiValue),
    GlobalOpt(String, MultiValue),
    BelGlobalOpt(BelId, BelGlobalKind, String, MultiValue),
    Raw(Key<'a>, MultiValue),
}

impl<'a> TileMultiFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Fuzzer<IseBackend<'a>> {
        match self {
            TileMultiFuzzKV::SiteAttr(bel, attr, val) => {
                let site = &backend.ngrid.nodes[&loc].bels[*bel];
                fuzzer.fuzz_multi(Key::SiteAttr(site, attr.clone()), *val)
            }
            TileMultiFuzzKV::IobSiteAttr(tile, bel, attr, val) => {
                let ioi_loc = find_ioi(backend, loc, *tile);
                let site = &backend.ngrid.nodes[&ioi_loc].bels[*bel];
                fuzzer.fuzz_multi(Key::SiteAttr(site, attr.clone()), *val)
            }
            TileMultiFuzzKV::GlobalOpt(attr, val) => {
                fuzzer.fuzz_multi(Key::GlobalOpt(attr.clone()), *val)
            }
            TileMultiFuzzKV::BelGlobalOpt(bel, kind, opt, val) => {
                let site = &backend.ngrid.nodes[&loc].bels[*bel];
                let name = kind.apply(backend, opt, site);
                fuzzer.fuzz_multi(Key::GlobalOpt(name), *val)
            }
            TileMultiFuzzKV::Raw(key, val) => fuzzer.fuzz_multi(key.clone(), *val),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TileBits {
    Null,
    Main(usize, usize),
    Reg(Reg),
    RegPresent(Reg),
    Raw(Vec<BitTile>),
    MainUp,
    MainDown,
    Bram,
    Spine(usize, usize),
    BTTerm,
    LRTerm,
    SpineEnd,
    Llv,
    Llh,
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
    Dcm,
    FreezeDci,
    Pcie,
    Pcie3,
    VirtexClkv,
    Cmt,
    Mgt,
    DoubleHclk,
    GtpCommonMid,
    GtpChannelMid,
    IobS6,
    MainXc4000,
}

impl TileBits {
    fn get_bits(&self, backend: &IseBackend, loc: (DieId, ColId, RowId, LayerId)) -> Vec<BitTile> {
        let (die, col, row, _) = loc;
        match *self {
            TileBits::Null => vec![],
            TileBits::Main(d, n) => match backend.edev {
                ExpandedDevice::Xc2000(edev) => (0..n)
                    .map(|idx| edev.btile_main(col, row - d + idx))
                    .collect(),
                ExpandedDevice::Virtex(edev) => (0..n)
                    .map(|idx| edev.btile_main(col, row - d + idx))
                    .collect(),
                ExpandedDevice::Virtex2(edev) => (0..n)
                    .map(|idx| edev.btile_main(col, row - d + idx))
                    .collect(),
                ExpandedDevice::Spartan6(edev) => (0..n)
                    .map(|idx| edev.btile_main(col, row - d + idx))
                    .collect(),
                ExpandedDevice::Virtex4(edev) => (0..n)
                    .map(|idx| edev.btile_main(die, col, row - d + idx))
                    .collect(),
                _ => todo!(),
            },
            TileBits::Reg(reg) => vec![BitTile::Reg(die, reg)],
            TileBits::Raw(ref raw) => raw.clone(),
            TileBits::Bram => match backend.edev {
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
                    vec![
                        edev.btile_main(col, row),
                        edev.btile_main(col, row + 1),
                        edev.btile_main(col, row + 2),
                        edev.btile_main(col, row + 3),
                        edev.btile_bram(col, row),
                        edev.btile_main(col, row).to_fixup(),
                        edev.btile_main(col, row + 1).to_fixup(),
                        edev.btile_main(col, row + 2).to_fixup(),
                        edev.btile_main(col, row + 3).to_fixup(),
                    ]
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
                _ => todo!(),
            },
            TileBits::Spine(d, n) => match backend.edev {
                ExpandedDevice::Xc2000(edev) => (0..n)
                    .map(|idx| edev.btile_llh(col, row - d + idx))
                    .collect(),
                ExpandedDevice::Virtex(edev) => {
                    (0..n).map(|idx| edev.btile_spine(row - d + idx)).collect()
                }
                ExpandedDevice::Virtex2(edev) => {
                    (0..n).map(|idx| edev.btile_spine(row - d + idx)).collect()
                }
                ExpandedDevice::Spartan6(edev) => {
                    (0..n).map(|idx| edev.btile_spine(row - d + idx)).collect()
                }
                ExpandedDevice::Virtex4(edev) => (0..n)
                    .map(|idx| edev.btile_spine(die, row - d + idx))
                    .collect(),
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
            TileBits::SpineEnd => match backend.edev {
                ExpandedDevice::Virtex(edev) => {
                    if row.to_idx() == 0 {
                        vec![edev.btile_spine(row), edev.btile_spine(row + 1)]
                    } else {
                        vec![edev.btile_spine(row), edev.btile_spine(row - 1)]
                    }
                }
                ExpandedDevice::Virtex2(edev) => {
                    vec![edev.btile_spine(row), edev.btile_btspine(row)]
                }
                ExpandedDevice::Spartan6(edev) => {
                    let dir = if row == edev.grid.row_bio_outer() {
                        Dir::S
                    } else if row == edev.grid.row_tio_outer() {
                        Dir::N
                    } else if col == edev.grid.col_lio() {
                        Dir::W
                    } else if col == edev.grid.col_rio() {
                        Dir::E
                    } else {
                        unreachable!()
                    };
                    vec![edev.btile_reg(dir)]
                }
                _ => unreachable!(),
            },
            TileBits::Llv => match backend.edev {
                ExpandedDevice::Xc2000(edev) => {
                    if col == edev.grid.col_lio() {
                        vec![edev.btile_llv(col, row), edev.btile_llv(col + 1, row)]
                    } else {
                        vec![edev.btile_llv(col, row)]
                    }
                }
                ExpandedDevice::Virtex2(edev) => {
                    if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3E {
                        vec![edev.btile_llv_b(col), edev.btile_llv_t(col)]
                    } else {
                        vec![edev.btile_llv(col)]
                    }
                }
                _ => unreachable!(),
            },
            TileBits::Llh => match backend.edev {
                ExpandedDevice::Xc2000(edev) => {
                    if row == edev.grid.row_bio() {
                        vec![edev.btile_llh(col, row), edev.btile_main(col - 1, row)]
                    } else if row == edev.grid.row_tio() {
                        vec![
                            edev.btile_llh(col, row),
                            edev.btile_llh(col, row - 1),
                            edev.btile_main(col - 1, row),
                        ]
                    } else if row == edev.grid.row_bio() + 1 {
                        vec![
                            edev.btile_llh(col, row),
                            edev.btile_llh(col, row - 1),
                            edev.btile_main(col - 1, row - 1),
                        ]
                    } else {
                        vec![edev.btile_llh(col, row), edev.btile_llh(col, row - 1)]
                    }
                }
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
                ExpandedDevice::Xc2000(edev) => {
                    vec![edev.btile_llv(col, row)]
                }
                ExpandedDevice::Virtex2(edev) => {
                    vec![edev.btile_hclk(col, row)]
                }
                ExpandedDevice::Spartan6(edev) => {
                    vec![edev.btile_hclk(col, row)]
                }
                ExpandedDevice::Virtex4(edev) => {
                    vec![edev.btile_hclk(die, col, row)]
                }
                _ => todo!(),
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
                ExpandedDevice::Xc2000(_) | ExpandedDevice::Virtex(_) => {
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
                    prjcombine_virtex4::grid::GridKind::Virtex5 => {
                        let mut res = vec![];
                        for i in 0..20 {
                            res.push(edev.btile_main(die, col, row + i));
                        }
                        for i in 0..20 {
                            res.push(edev.btile_spine(die, row + i));
                        }
                        res
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex6 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
                },
                _ => todo!(),
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
                    ExpandedDevice::Xc2000(edev) => node
                        .tiles
                        .values()
                        .map(|&(col, row)| edev.btile_main(col, row))
                        .collect(),
                    ExpandedDevice::Virtex(edev) => node
                        .tiles
                        .values()
                        .map(|&(col, row)| edev.btile_main(col, row))
                        .collect(),
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
                    _ => todo!(),
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
                            res.push(
                                edev.btile_spine(die, edev.grids[die].rows().next_back().unwrap()),
                            )
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
                    prjcombine_virtex4::grid::GridKind::Virtex5 => {
                        vec![
                            edev.btile_spine(die, row - 1),
                            edev.btile_spine(die, row),
                            edev.btile_spine_hclk(die, row),
                        ]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex6 => unreachable!(),
                    prjcombine_virtex4::grid::GridKind::Virtex7 => {
                        let mut res = vec![];
                        for i in 0..8 {
                            res.push(edev.btile_main(die, col, row - 4 + i));
                        }
                        res.push(edev.btile_hclk(die, col, row));
                        res
                    }
                }
            }
            TileBits::Dcm => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.grid.kind {
                    prjcombine_virtex2::grid::GridKind::Virtex2
                    | prjcombine_virtex2::grid::GridKind::Virtex2P
                    | prjcombine_virtex2::grid::GridKind::Virtex2PX => {
                        vec![edev.btile_main(col, row), edev.btile_btterm(col, row)]
                    }
                    prjcombine_virtex2::grid::GridKind::Spartan3 => vec![edev.btile_main(col, row)],
                    prjcombine_virtex2::grid::GridKind::FpgaCore => unreachable!(),
                    prjcombine_virtex2::grid::GridKind::Spartan3E
                    | prjcombine_virtex2::grid::GridKind::Spartan3A
                    | prjcombine_virtex2::grid::GridKind::Spartan3ADsp => {
                        let node = backend
                            .egrid
                            .find_node(die, (col, row), |n| {
                                backend.egrid.db.nodes.key(n.kind).starts_with("DCM.S3E")
                            })
                            .unwrap();
                        match &backend.egrid.db.nodes.key(node.kind)[..] {
                            "DCM.S3E.BL" | "DCM.S3E.RT" => vec![
                                edev.btile_main(col, row),
                                edev.btile_main(col, row + 1),
                                edev.btile_main(col, row + 2),
                                edev.btile_main(col, row + 3),
                                edev.btile_main(col - 3, row),
                                edev.btile_main(col - 3, row + 1),
                                edev.btile_main(col - 3, row + 2),
                                edev.btile_main(col - 3, row + 3),
                            ],
                            "DCM.S3E.BR" | "DCM.S3E.LT" => vec![
                                edev.btile_main(col, row),
                                edev.btile_main(col, row + 1),
                                edev.btile_main(col, row + 2),
                                edev.btile_main(col, row + 3),
                                edev.btile_main(col + 3, row),
                                edev.btile_main(col + 3, row + 1),
                                edev.btile_main(col + 3, row + 2),
                                edev.btile_main(col + 3, row + 3),
                            ],
                            "DCM.S3E.TL" | "DCM.S3E.RB" => vec![
                                edev.btile_main(col, row),
                                edev.btile_main(col, row - 3),
                                edev.btile_main(col, row - 2),
                                edev.btile_main(col, row - 1),
                                edev.btile_main(col - 3, row - 3),
                                edev.btile_main(col - 3, row - 2),
                                edev.btile_main(col - 3, row - 1),
                                edev.btile_main(col - 3, row),
                            ],
                            "DCM.S3E.TR" | "DCM.S3E.LB" => vec![
                                edev.btile_main(col, row),
                                edev.btile_main(col, row - 3),
                                edev.btile_main(col, row - 2),
                                edev.btile_main(col, row - 1),
                                edev.btile_main(col + 3, row - 3),
                                edev.btile_main(col + 3, row - 2),
                                edev.btile_main(col + 3, row - 1),
                                edev.btile_main(col + 3, row),
                            ],
                            _ => unreachable!(),
                        }
                    }
                }
            }
            TileBits::FreezeDci => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                vec![
                    edev.btile_lrterm(col, row),
                    edev.btile_btterm(col, row),
                    edev.btile_lrterm(col, row).to_fixup(),
                    edev.btile_btterm(col, row).to_fixup(),
                    BitTile::Reg(die, Reg::FakeFreezeDciNops),
                    BitTile::RegPresent(die, Reg::FakeFreezeDciNops),
                ]
            }
            TileBits::RegPresent(reg) => {
                vec![BitTile::Reg(die, reg), BitTile::RegPresent(die, reg)]
            }
            TileBits::Pcie => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex6 => (0..20)
                        .map(|i| edev.btile_main(die, col + 3, row + i))
                        .collect(),
                    prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
                }
            }
            TileBits::Pcie3 => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let mut res = vec![];
                for i in 0..50 {
                    res.push(edev.btile_main(die, col + 4, row + i));
                }
                res
            }
            TileBits::VirtexClkv => {
                let ExpandedDevice::Virtex(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_clkv(col, row)]
            }
            TileBits::Cmt => match backend.edev {
                ExpandedDevice::Spartan6(edev) => {
                    let mut res = vec![];
                    for i in 0..16 {
                        res.push(edev.btile_main(col, row - 8 + i));
                    }
                    for i in 0..16 {
                        res.push(edev.btile_spine(row - 8 + i));
                    }
                    res
                }
                ExpandedDevice::Virtex4(edev) => {
                    let mut res = vec![];
                    for i in 0..edev.grids[die].rows_per_reg() {
                        res.push(edev.btile_main(
                            die,
                            col,
                            edev.grids[die].row_hclk(row) - edev.grids[die].rows_per_reg() / 2 + i,
                        ));
                    }
                    res.push(edev.btile_hclk(die, col, row));
                    res
                }
                _ => unreachable!(),
            },
            TileBits::Mgt => match backend.edev {
                ExpandedDevice::Virtex4(edev) => match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex5 => {
                        let mut res = vec![];
                        for i in 0..20 {
                            res.push(edev.btile_main(die, col, row + i));
                        }
                        res.push(edev.btile_hclk(die, col, row + 10));
                        res
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex6 => {
                        let mut res = vec![];
                        for i in 0..40 {
                            res.push(edev.btile_main(die, col, row - 20 + i));
                        }
                        res.push(edev.btile_hclk(die, col, row));
                        res
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
                },
                _ => unreachable!(),
            },
            TileBits::DoubleHclk => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                assert_eq!(edev.kind, prjcombine_virtex4::grid::GridKind::Virtex7);
                assert_eq!(loc.1.to_idx() % 2, 0);
                vec![
                    edev.btile_hclk(loc.0, loc.1, loc.2),
                    edev.btile_hclk(loc.0, loc.1 + 1, loc.2),
                ]
            }
            TileBits::GtpCommonMid => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                assert_eq!(edev.kind, prjcombine_virtex4::grid::GridKind::Virtex7);
                let col = if loc.1.to_idx() % 2 == 0 {
                    loc.1 - 1
                } else {
                    loc.1 + 1
                };
                vec![
                    edev.btile_main(loc.0, col, loc.2 - 3),
                    edev.btile_main(loc.0, col, loc.2 - 2),
                    edev.btile_main(loc.0, col, loc.2 - 1),
                    edev.btile_main(loc.0, col, loc.2),
                    edev.btile_main(loc.0, col, loc.2 + 1),
                    edev.btile_main(loc.0, col, loc.2 + 2),
                    edev.btile_hclk(loc.0, col, loc.2),
                ]
            }
            TileBits::GtpChannelMid => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                assert_eq!(edev.kind, prjcombine_virtex4::grid::GridKind::Virtex7);
                let col = if loc.1.to_idx() % 2 == 0 {
                    loc.1 - 1
                } else {
                    loc.1 + 1
                };
                (0..11)
                    .map(|i| edev.btile_main(loc.0, col, loc.2 + i))
                    .collect()
            }
            TileBits::IobS6 => {
                let ExpandedDevice::Spartan6(edev) = backend.edev else {
                    unreachable!()
                };
                vec![edev.btile_iob(loc.1, loc.2)]
            }
            TileBits::MainXc4000 => {
                let ExpandedDevice::Xc2000(edev) = backend.edev else {
                    unreachable!()
                };
                if loc.1 == edev.grid.col_lio() {
                    if loc.2 == edev.grid.row_bio() {
                        // LL
                        vec![edev.btile_main(loc.1, loc.2)]
                    } else if loc.2 == edev.grid.row_tio() {
                        // UL
                        vec![edev.btile_main(loc.1, loc.2)]
                    } else {
                        // LEFT
                        vec![
                            edev.btile_main(loc.1, loc.2),
                            edev.btile_main(loc.1, loc.2 - 1),
                        ]
                    }
                } else if loc.1 == edev.grid.col_rio() {
                    if loc.2 == edev.grid.row_bio() {
                        // LR
                        vec![edev.btile_main(loc.1, loc.2)]
                    } else if loc.2 == edev.grid.row_tio() {
                        // UR
                        vec![
                            edev.btile_main(loc.1, loc.2),
                            edev.btile_main(loc.1, loc.2 - 1),
                            edev.btile_main(loc.1 - 1, loc.2),
                        ]
                    } else {
                        // RT
                        vec![
                            edev.btile_main(loc.1, loc.2),
                            edev.btile_main(loc.1, loc.2 - 1),
                            edev.btile_main(loc.1 - 1, loc.2),
                        ]
                    }
                } else {
                    if loc.2 == edev.grid.row_bio() {
                        // BOT
                        vec![
                            edev.btile_main(loc.1, loc.2),
                            edev.btile_main(loc.1 + 1, loc.2),
                        ]
                    } else if loc.2 == edev.grid.row_tio() {
                        // TOP
                        vec![
                            edev.btile_main(loc.1, loc.2),
                            edev.btile_main(loc.1, loc.2 - 1),
                            edev.btile_main(loc.1 + 1, loc.2),
                            edev.btile_main(loc.1 - 1, loc.2),
                        ]
                    } else {
                        // CLB
                        vec![
                            edev.btile_main(loc.1, loc.2),
                            edev.btile_main(loc.1, loc.2 - 1),
                            edev.btile_main(loc.1 - 1, loc.2),
                            edev.btile_main(loc.1, loc.2 + 1),
                            edev.btile_main(loc.1 + 1, loc.2),
                        ]
                    }
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ExtraFeatureKind {
    MainFixed(ColId, RowId),
    MainFixedPair(ColId, RowId),
    Corner(ColId, RowId),
    AllDcms,
    AllOtherDcms,
    AllBrams,
    AllColumnIo,
    AllIobs,
    AllGclkvm,
    AllHclk,
    Pcilogic(Dir),
    VirtexClkBt,
    DcmVreg,
    DcmLL,
    DcmUL,
    DcmLR,
    DcmUR,
    FpgaCoreIob(Dir),
    HclkDcm(Dir),
    HclkCcm(Dir),
    MgtRepeater(Dir, Option<Dir>),
    MgtRepeaterMgt(isize),
    BufpllPll(Dir, &'static str),
    Reg(Reg),
    Hclk(isize, isize),
    HclkPair(isize, isize),
    HclkIoiCenter(&'static str),
    HclkBramMgtPrev,
    PcieHclkPair,
    Pcie3HclkPair,
    PllDcm,
    Vref,
    Vr,
    VrBot,
    VrTop,
    HclkIoDci(&'static str),
    HclkIoLvds,
    AllHclkIo(&'static str),
    Cfg,
    CenterDciIo(u32),
    CenterDciVr(u32),
    CenterDciVrBot(u32),
    CenterDciVrTop(u32),
    CenterDciHclk(u32),
    CenterDciHclkCascade(u32, &'static str),
    AllBankIoi,
    ClkRebuf(Dir, NodeKindId),
    CmtDir(Dir),
    Cmt(isize),
    AllCfg,
    AllXadc,
    HclkIoiInnerSide(Dir),
    HclkIoiHere(NodeKindId),
    AllBankIo,
    AllMcbIoi,
    IoiHere,
}

impl ExtraFeatureKind {
    pub fn get_tiles(self, backend: &IseBackend, loc: NodeLoc, tile: &str) -> Vec<Vec<BitTile>> {
        match self {
            ExtraFeatureKind::MainFixed(col, row) => match backend.edev {
                ExpandedDevice::Xc2000(edev) => {
                    vec![vec![edev.btile_main(col, row)]]
                }
                ExpandedDevice::Virtex(edev) => {
                    vec![vec![edev.btile_main(col, row)]]
                }
                ExpandedDevice::Virtex2(edev) => {
                    vec![vec![edev.btile_main(col, row)]]
                }
                ExpandedDevice::Spartan6(edev) => {
                    vec![vec![edev.btile_main(col, row)]]
                }
                _ => todo!(),
            },
            ExtraFeatureKind::MainFixedPair(col, row) => match backend.edev {
                ExpandedDevice::Spartan6(edev) => {
                    vec![vec![
                        edev.btile_main(col, row),
                        edev.btile_main(col, row + 1),
                    ]]
                }
                _ => todo!(),
            },
            ExtraFeatureKind::Corner(col, row) => match backend.edev {
                ExpandedDevice::Virtex2(edev) => {
                    vec![vec![edev.btile_lrterm(col, row)]]
                }
                _ => todo!(),
            },
            ExtraFeatureKind::AllDcms => match backend.edev {
                ExpandedDevice::Virtex(edev) => {
                    let mut res = vec![];
                    for (node, name, _) in &backend.egrid.db.nodes {
                        if name.starts_with("DLL") {
                            for &loc in &backend.egrid.node_index[node] {
                                res.push(vec![edev.btile_main(loc.1, loc.2)]);
                            }
                        }
                    }
                    res
                }
                ExpandedDevice::Virtex2(edev) => {
                    let node = match edev.grid.kind {
                        prjcombine_virtex2::grid::GridKind::Virtex2 => "DCM.V2",
                        prjcombine_virtex2::grid::GridKind::Virtex2P
                        | prjcombine_virtex2::grid::GridKind::Virtex2PX => "DCM.V2P",
                        prjcombine_virtex2::grid::GridKind::Spartan3 => "DCM.S3",
                        prjcombine_virtex2::grid::GridKind::FpgaCore => unreachable!(),
                        prjcombine_virtex2::grid::GridKind::Spartan3E => unreachable!(),
                        prjcombine_virtex2::grid::GridKind::Spartan3A => unreachable!(),
                        prjcombine_virtex2::grid::GridKind::Spartan3ADsp => unreachable!(),
                    };
                    let node = backend.egrid.db.get_node(node);
                    backend.egrid.node_index[node]
                        .iter()
                        .map(|loc| vec![edev.btile_main(loc.1, loc.2)])
                        .collect()
                }
                ExpandedDevice::Spartan6(edev) => {
                    let node = backend.egrid.db.get_node("CMT_DCM");
                    backend.egrid.node_index[node]
                        .iter()
                        .copied()
                        .map(|loc| {
                            Vec::from_iter(
                                (0..16)
                                    .map(|i| edev.btile_main(loc.1, loc.2 - 8 + i))
                                    .chain((0..16).map(|i| edev.btile_spine(loc.2 - 8 + i))),
                            )
                        })
                        .collect()
                }
                _ => todo!(),
            },
            ExtraFeatureKind::AllOtherDcms => match backend.edev {
                ExpandedDevice::Spartan6(edev) => {
                    let node = backend.egrid.db.get_node("CMT_DCM");
                    backend.egrid.node_index[node]
                        .iter()
                        .copied()
                        .filter(|&other_loc| other_loc != loc)
                        .map(|loc| {
                            Vec::from_iter(
                                (0..16)
                                    .map(|i| edev.btile_main(loc.1, loc.2 - 8 + i))
                                    .chain((0..16).map(|i| edev.btile_spine(loc.2 - 8 + i))),
                            )
                        })
                        .collect()
                }
                _ => todo!(),
            },
            ExtraFeatureKind::AllBrams => match backend.edev {
                ExpandedDevice::Spartan6(edev) => {
                    let node = backend.egrid.db.get_node("BRAM");
                    backend.egrid.node_index[node]
                        .iter()
                        .map(|loc| {
                            vec![
                                edev.btile_main(loc.1, loc.2),
                                edev.btile_main(loc.1, loc.2 + 1),
                                edev.btile_main(loc.1, loc.2 + 2),
                                edev.btile_main(loc.1, loc.2 + 3),
                            ]
                        })
                        .collect()
                }
                ExpandedDevice::Virtex4(edev) => {
                    let node = backend.egrid.db.get_node("BRAM");
                    backend.egrid.node_index[node]
                        .iter()
                        .map(|loc| {
                            vec![
                                edev.btile_main(loc.0, loc.1, loc.2),
                                edev.btile_main(loc.0, loc.1, loc.2 + 1),
                                edev.btile_main(loc.0, loc.1, loc.2 + 2),
                                edev.btile_main(loc.0, loc.1, loc.2 + 3),
                                edev.btile_main(loc.0, loc.1, loc.2 + 4),
                            ]
                        })
                        .collect()
                }
                _ => unreachable!(),
            },
            ExtraFeatureKind::AllColumnIo => {
                let ExpandedDevice::Xc2000(edev) = backend.edev else {
                    unreachable!()
                };
                let mut res = vec![];
                for row in backend.egrid.die(loc.0).rows() {
                    if backend
                        .egrid
                        .find_node(loc.0, (loc.1, row), |node| {
                            backend.egrid.db.nodes.key(node.kind).starts_with("IO")
                        })
                        .is_some()
                    {
                        res.push(vec![BitTile::Null, edev.btile_main(loc.1, row)]);
                    }
                }
                res
            }
            ExtraFeatureKind::AllIobs => match backend.edev {
                ExpandedDevice::Xc2000(edev) => {
                    let node = backend.egrid.db.get_node(tile);
                    backend.egrid.node_index[node]
                        .iter()
                        .map(|loc| vec![edev.btile_main(loc.1, loc.2)])
                        .collect()
                }
                ExpandedDevice::Virtex(edev) => {
                    let node = backend.egrid.db.get_node(tile);
                    backend.egrid.node_index[node]
                        .iter()
                        .map(|loc| vec![edev.btile_main(loc.1, loc.2)])
                        .collect()
                }
                ExpandedDevice::Virtex4(edev) => {
                    let node = backend.egrid.db.get_node(tile);
                    backend.egrid.node_index[node]
                        .iter()
                        .map(|loc| {
                            backend.egrid.db.nodes[node]
                                .tiles
                                .ids()
                                .map(|ti| edev.btile_main(loc.0, loc.1, loc.2 + ti.to_idx()))
                                .collect()
                        })
                        .collect()
                }
                _ => todo!(),
            },
            ExtraFeatureKind::AllGclkvm => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                let node = backend.egrid.db.get_node("GCLKVM.S3");
                backend.egrid.node_index[node]
                    .iter()
                    .map(|loc| {
                        vec![
                            edev.btile_clkv(loc.1, loc.2 - 1),
                            edev.btile_clkv(loc.1, loc.2),
                        ]
                    })
                    .collect()
            }
            ExtraFeatureKind::AllHclk => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                let node = backend.egrid.db.get_node("GCLKH");
                backend.egrid.node_index[node]
                    .iter()
                    .map(|loc| vec![edev.btile_hclk(loc.1, loc.2)])
                    .collect()
            }
            ExtraFeatureKind::Pcilogic(dir) => {
                let ExpandedDevice::Virtex(edev) = backend.edev else {
                    unreachable!()
                };
                let col = match dir {
                    Dir::W => edev.grid.col_lio(),
                    Dir::E => edev.grid.col_rio(),
                    _ => unreachable!(),
                };
                vec![vec![edev.btile_main(col, edev.grid.row_clk())]]
            }
            ExtraFeatureKind::VirtexClkBt => {
                let ExpandedDevice::Virtex(edev) = backend.edev else {
                    unreachable!()
                };
                vec![vec![
                    edev.btile_spine(loc.2),
                    edev.btile_spine(if loc.2.to_idx() == 0 {
                        loc.2 + 1
                    } else {
                        loc.2 - 1
                    }),
                ]]
            }
            ExtraFeatureKind::DcmLL => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if loc.1.to_idx() != 3 || loc.2.to_idx() != 0 {
                    vec![]
                } else {
                    vec![vec![edev.btile_lrterm(loc.1 - 3, loc.2)]]
                }
            }
            ExtraFeatureKind::DcmUL => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if loc.1.to_idx() != 3 || loc.2.to_idx() == 0 {
                    vec![]
                } else {
                    vec![vec![edev.btile_lrterm(loc.1 - 3, loc.2)]]
                }
            }
            ExtraFeatureKind::DcmLR => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if loc.1.to_idx() == 3 || loc.2.to_idx() != 0 {
                    vec![]
                } else {
                    vec![vec![edev.btile_lrterm(loc.1 + 3, loc.2)]]
                }
            }
            ExtraFeatureKind::DcmUR => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                if loc.1.to_idx() == 3 || loc.2.to_idx() == 0 {
                    vec![]
                } else {
                    vec![vec![edev.btile_lrterm(loc.1 + 3, loc.2)]]
                }
            }
            ExtraFeatureKind::DcmVreg => {
                let node = backend
                    .egrid
                    .find_node(loc.0, (loc.1, loc.2), |n| {
                        backend.egrid.db.nodes.key(n.kind).starts_with("DCM.S3E")
                    })
                    .unwrap();
                let (col, row) = match &backend.egrid.db.nodes.key(node.kind)[..] {
                    "DCM.S3E.BL" | "DCM.S3E.TL" => (loc.1 + 1, loc.2),
                    "DCM.S3E.LB" => (loc.1, loc.2 + 1),
                    "DCM.S3E.RT" => (loc.1, loc.2 - 1),
                    _ => unreachable!(),
                };
                let (layer, _) = backend
                    .egrid
                    .find_node_loc(loc.0, (col, row), |n| {
                        backend.egrid.db.nodes.key(n.kind).starts_with("DCM.S3E")
                    })
                    .unwrap();
                vec![TileBits::Dcm.get_bits(backend, (loc.0, col, row, layer))]
            }
            ExtraFeatureKind::FpgaCoreIob(dir) => {
                let ExpandedDevice::Virtex2(edev) = backend.edev else {
                    unreachable!()
                };
                match dir {
                    Dir::W => {
                        if loc.1 != edev.grid.col_left() {
                            vec![]
                        } else {
                            vec![vec![edev.btile_lrterm(loc.1, loc.2)]]
                        }
                    }
                    Dir::E => {
                        if loc.1 != edev.grid.col_right() {
                            vec![]
                        } else {
                            vec![vec![edev.btile_lrterm(loc.1, loc.2)]]
                        }
                    }
                    Dir::S => {
                        if loc.2 != edev.grid.row_bot() {
                            vec![]
                        } else {
                            vec![vec![edev.btile_btterm(loc.1, loc.2)]]
                        }
                    }
                    Dir::N => {
                        if loc.2 != edev.grid.row_top() {
                            vec![]
                        } else {
                            vec![vec![edev.btile_btterm(loc.1, loc.2)]]
                        }
                    }
                }
            }
            ExtraFeatureKind::HclkDcm(dir) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let rows = match dir {
                    Dir::N => [loc.2, loc.2 + 4],
                    Dir::S => [loc.2 - 8, loc.2 - 4],
                    _ => unreachable!(),
                };
                let mut res = vec![];
                for row in rows {
                    if backend.egrid.find_bel(loc.0, (loc.1, row), "DCM").is_some() {
                        res.push(vec![
                            edev.btile_main(loc.0, loc.1, row),
                            edev.btile_main(loc.0, loc.1, row + 1),
                            edev.btile_main(loc.0, loc.1, row + 2),
                            edev.btile_main(loc.0, loc.1, row + 3),
                        ]);
                    }
                }
                res
            }
            ExtraFeatureKind::HclkCcm(dir) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let rows = match dir {
                    Dir::N => [loc.2, loc.2 + 4],
                    Dir::S => [loc.2 - 8, loc.2 - 4],
                    _ => unreachable!(),
                };
                let mut res = vec![];
                for row in rows {
                    if backend.egrid.find_bel(loc.0, (loc.1, row), "CCM").is_some() {
                        res.push(vec![
                            edev.btile_main(loc.0, loc.1, row),
                            edev.btile_main(loc.0, loc.1, row + 1),
                            edev.btile_main(loc.0, loc.1, row + 2),
                            edev.btile_main(loc.0, loc.1, row + 3),
                        ]);
                    }
                }
                res
            }
            ExtraFeatureKind::MgtRepeater(dir_gt, dir_row) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let row = match dir_row {
                    None => loc.2,
                    Some(Dir::S) => edev.grids[DieId::from_idx(0)].row_bufg() - 8,
                    Some(Dir::N) => edev.grids[DieId::from_idx(0)].row_bufg() + 8,
                    _ => unreachable!(),
                };
                let mut res = vec![];
                let is_l = match dir_gt {
                    Dir::W => true,
                    Dir::E => false,
                    _ => unreachable!(),
                };
                for &col in &edev.grids[DieId::from_idx(0)].cols_vbrk {
                    if (col < edev.col_cfg) == is_l {
                        res.push(vec![edev.btile_hclk(
                            DieId::from_idx(0),
                            if is_l { col } else { col - 1 },
                            row,
                        )]);
                    }
                }
                res
            }
            ExtraFeatureKind::MgtRepeaterMgt(delta) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let row = loc.2 + delta;
                let mut res = vec![];
                let is_l = loc.1 < edev.col_cfg;
                for &col in &edev.grids[DieId::from_idx(0)].cols_vbrk {
                    if (col < edev.col_cfg) == is_l {
                        res.push(vec![edev.btile_hclk(
                            DieId::from_idx(0),
                            if is_l { col } else { col - 1 },
                            row,
                        )]);
                    }
                }
                res
            }
            ExtraFeatureKind::BufpllPll(dir, kind) => {
                let ExpandedDevice::Spartan6(edev) = backend.edev else {
                    unreachable!()
                };
                let mut row = loc.2;
                loop {
                    match dir {
                        Dir::S => {
                            if row.to_idx() == 0 {
                                return vec![];
                            }
                            row -= 1;
                        }
                        Dir::N => {
                            row += 1;
                            if row == edev.grid.rows.next_id() {
                                return vec![];
                            }
                        }
                        _ => unreachable!(),
                    }
                    if let Some(node) = backend.egrid.find_node(loc.0, (loc.1, row), |node| {
                        edev.egrid.db.nodes.key(node.kind).starts_with("PLL_BUFPLL")
                    }) {
                        if edev.egrid.db.nodes.key(node.kind) != kind {
                            return vec![];
                        }
                        return vec![vec![edev.btile_spine(row - 7)]];
                    }
                }
            }
            ExtraFeatureKind::Reg(reg) => backend
                .egrid
                .die
                .ids()
                .map(|die| vec![BitTile::Reg(die, reg)])
                .collect(),
            ExtraFeatureKind::Hclk(dx, dy) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                vec![vec![edev.btile_hclk(loc.0, loc.1 + dx, loc.2 + dy)]]
            }
            ExtraFeatureKind::HclkPair(dx, dy) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let mut col = loc.1 + dx;
                if col.to_idx() % 2 == 1 {
                    col -= 1;
                }
                let row = edev.grids[loc.0].row_hclk(loc.2 + dy);
                vec![vec![
                    edev.btile_hclk(loc.0, col, row),
                    edev.btile_hclk(loc.0, col + 1, row),
                ]]
            }
            ExtraFeatureKind::HclkIoiCenter(kind) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                if loc.1 > edev.col_clk {
                    return vec![];
                }
                if backend
                    .egrid
                    .find_node(loc.0, (edev.col_clk, loc.2), |node| {
                        edev.egrid.db.nodes.key(node.kind) == kind
                    })
                    .is_some()
                {
                    vec![vec![edev.btile_hclk(loc.0, edev.col_clk, loc.2)]]
                } else {
                    vec![]
                }
            }
            ExtraFeatureKind::HclkBramMgtPrev => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let grid = edev.grids[loc.0];
                let col = if loc.1 < edev.col_clk {
                    let mut range = grid.cols_mgt_buf.range(..loc.1);
                    range.next_back()
                } else {
                    let mut range = grid.cols_mgt_buf.range((loc.1 + 1)..);
                    range.next()
                };
                match col {
                    Some(&col) => vec![vec![edev.btile_hclk(loc.0, col, loc.2)]],
                    None => vec![],
                }
            }
            ExtraFeatureKind::PcieHclkPair => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let col = if loc.1.to_idx() % 2 == 0 {
                    loc.1 - 4
                } else {
                    loc.1 - 1
                };
                let row = loc.2 + edev.grids[loc.0].rows_per_reg() / 2;
                vec![vec![
                    edev.btile_hclk(loc.0, col, row),
                    edev.btile_hclk(loc.0, col + 1, row),
                ]]
            }
            ExtraFeatureKind::Pcie3HclkPair => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                vec![
                    vec![
                        edev.btile_hclk(loc.0, loc.1 + 3, loc.2),
                        edev.btile_hclk(loc.0, loc.1 + 4, loc.2),
                    ],
                    vec![
                        edev.btile_hclk(loc.0, loc.1 + 3, loc.2 + 50),
                        edev.btile_hclk(loc.0, loc.1 + 4, loc.2 + 50),
                    ],
                ]
            }
            ExtraFeatureKind::PllDcm => {
                let ExpandedDevice::Spartan6(edev) = backend.edev else {
                    unreachable!()
                };
                vec![Vec::from_iter(
                    (0..16).map(|i| edev.btile_main(loc.1, loc.2 - 24 + i)),
                )]
            }
            ExtraFeatureKind::Vref => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4
                    | prjcombine_virtex4::grid::GridKind::Virtex5 => {
                        get_v4_vref_rows(edev, loc.0, loc.1, loc.2)
                            .into_iter()
                            .map(|vref_row| vec![edev.btile_main(loc.0, loc.1, vref_row)])
                            .collect()
                    }

                    prjcombine_virtex4::grid::GridKind::Virtex6
                    | prjcombine_virtex4::grid::GridKind::Virtex7 => {
                        get_v4_vref_rows(edev, loc.0, loc.1, loc.2)
                            .into_iter()
                            .map(|vref_row| {
                                vec![
                                    edev.btile_main(loc.0, loc.1, vref_row),
                                    edev.btile_main(loc.0, loc.1, vref_row + 1),
                                ]
                            })
                            .collect()
                    }
                }
            }
            ExtraFeatureKind::Vr => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4
                    | prjcombine_virtex4::grid::GridKind::Virtex5 => vec![vec![edev.btile_main(
                        loc.0,
                        loc.1,
                        get_v4_vr_row(edev, loc.0, loc.1, loc.2),
                    )]],

                    prjcombine_virtex4::grid::GridKind::Virtex6 => {
                        let vr_row = get_v4_vr_row(edev, loc.0, loc.1, loc.2);
                        vec![vec![
                            edev.btile_main(loc.0, loc.1, vr_row),
                            edev.btile_main(loc.0, loc.1, vr_row + 1),
                        ]]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex7 => unreachable!(),
                }
            }
            ExtraFeatureKind::VrBot => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                assert_eq!(edev.kind, prjcombine_virtex4::grid::GridKind::Virtex7);
                vec![vec![edev.btile_main(
                    loc.0,
                    loc.1,
                    edev.grids[loc.0].row_hclk(loc.2) - 25,
                )]]
            }
            ExtraFeatureKind::VrTop => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                assert_eq!(edev.kind, prjcombine_virtex4::grid::GridKind::Virtex7);
                vec![vec![edev.btile_main(
                    loc.0,
                    loc.1,
                    edev.grids[loc.0].row_hclk(loc.2) + 24,
                )]]
            }
            ExtraFeatureKind::HclkIoDci(kind) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let row = get_v4_vr_row(edev, loc.0, loc.1, loc.2);
                let reg = edev.grids[loc.0].row_to_reg(row);
                let row = edev.grids[loc.0].row_reg_hclk(reg);
                if backend
                    .egrid
                    .find_node(loc.0, (loc.1, row), |node| {
                        edev.egrid.db.nodes.key(node.kind) == kind
                    })
                    .is_some()
                {
                    vec![vec![edev.btile_hclk(loc.0, loc.1, row)]]
                } else {
                    vec![]
                }
            }
            ExtraFeatureKind::HclkIoLvds => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                assert_eq!(edev.kind, prjcombine_virtex4::grid::GridKind::Virtex4);
                assert_ne!(loc.1, edev.col_cfg);
                let row = RowId::from_idx(loc.2.to_idx() / 32 * 32 + 24);
                vec![vec![edev.btile_hclk(loc.0, loc.1, row)]]
            }
            ExtraFeatureKind::AllHclkIo(kind) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let node = backend.egrid.db.get_node(kind);
                backend.egrid.node_index[node]
                    .iter()
                    .map(|loc| vec![edev.btile_hclk(loc.0, loc.1, loc.2)])
                    .collect()
            }
            ExtraFeatureKind::Cfg => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => vec![
                        (0..16)
                            .map(|i| {
                                edev.btile_main(
                                    loc.0,
                                    edev.col_cfg,
                                    edev.grids[loc.0].row_bufg() - 8 + i,
                                )
                            })
                            .collect(),
                    ],
                    prjcombine_virtex4::grid::GridKind::Virtex5 => vec![
                        (0..20)
                            .map(|i| {
                                edev.btile_main(
                                    loc.0,
                                    edev.col_cfg,
                                    edev.grids[loc.0].row_bufg() - 10 + i,
                                )
                            })
                            .collect(),
                    ],
                    prjcombine_virtex4::grid::GridKind::Virtex6 => vec![
                        (0..80)
                            .map(|i| {
                                edev.btile_main(
                                    loc.0,
                                    edev.col_cfg,
                                    edev.grids[loc.0].row_bufg() - 40 + i,
                                )
                            })
                            .collect(),
                    ],
                    prjcombine_virtex4::grid::GridKind::Virtex7 => vec![
                        (0..50)
                            .map(|i| {
                                edev.btile_main(
                                    loc.0,
                                    edev.col_cfg,
                                    edev.grids[loc.0].row_bufg() - 50 + i,
                                )
                            })
                            .collect(),
                    ],
                }
            }
            ExtraFeatureKind::AllCfg => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                backend
                    .egrid
                    .die
                    .ids()
                    .map(|die| match edev.kind {
                        prjcombine_virtex4::grid::GridKind::Virtex4 => (0..16)
                            .map(|i| {
                                edev.btile_main(
                                    die,
                                    edev.col_cfg,
                                    edev.grids[die].row_bufg() - 8 + i,
                                )
                            })
                            .collect(),
                        prjcombine_virtex4::grid::GridKind::Virtex5 => (0..20)
                            .map(|i| {
                                edev.btile_main(
                                    die,
                                    edev.col_cfg,
                                    edev.grids[die].row_bufg() - 10 + i,
                                )
                            })
                            .collect(),
                        prjcombine_virtex4::grid::GridKind::Virtex6 => (0..80)
                            .map(|i| {
                                edev.btile_main(
                                    die,
                                    edev.col_cfg,
                                    edev.grids[die].row_bufg() - 40 + i,
                                )
                            })
                            .collect(),
                        prjcombine_virtex4::grid::GridKind::Virtex7 => (0..50)
                            .map(|i| {
                                edev.btile_main(
                                    die,
                                    edev.col_cfg,
                                    edev.grids[die].row_bufg() - 50 + i,
                                )
                            })
                            .collect(),
                    })
                    .collect()
            }
            ExtraFeatureKind::AllXadc => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                backend
                    .egrid
                    .die
                    .ids()
                    .map(|die| match edev.kind {
                        prjcombine_virtex4::grid::GridKind::Virtex4
                        | prjcombine_virtex4::grid::GridKind::Virtex5
                        | prjcombine_virtex4::grid::GridKind::Virtex6 => unreachable!(),
                        prjcombine_virtex4::grid::GridKind::Virtex7 => (0..25)
                            .map(|i| {
                                edev.btile_main(
                                    die,
                                    edev.col_cfg,
                                    edev.grids[die].row_bufg() + 25 + i,
                                )
                            })
                            .collect(),
                    })
                    .collect()
            }

            ExtraFeatureKind::CenterDciIo(bank) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4
                    | prjcombine_virtex4::grid::GridKind::Virtex5 => {
                        let (_, io_row) = get_v4_center_dci_rows(edev, bank);
                        vec![vec![edev.btile_main(loc.0, edev.col_cfg, io_row)]]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex6 => {
                        let (_, io_row) = get_v4_center_dci_rows(edev, bank);
                        vec![vec![
                            edev.btile_main(loc.0, edev.col_lcio.unwrap(), io_row),
                            edev.btile_main(loc.0, edev.col_lcio.unwrap(), io_row + 1),
                        ]]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex7 => {
                        let anchor_reg = if edev.grids[loc.0].has_ps {
                            prjcombine_virtex4::grid::RegId::from_idx(
                                edev.grids[loc.0].regs - 2 + bank as usize,
                            )
                        } else {
                            prjcombine_virtex4::grid::RegId::from_idx(bank as usize)
                        };
                        let io_row = edev.grids[loc.0].row_reg_hclk(anchor_reg) - 24;
                        vec![vec![
                            edev.btile_main(loc.0, edev.col_rio.unwrap(), io_row),
                            edev.btile_main(loc.0, edev.col_rio.unwrap(), io_row + 1),
                        ]]
                    }
                }
            }
            ExtraFeatureKind::CenterDciVr(bank) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let (vr_row, _) = get_v4_center_dci_rows(edev, bank);
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4
                    | prjcombine_virtex4::grid::GridKind::Virtex5 => {
                        vec![vec![edev.btile_main(loc.0, edev.col_cfg, vr_row.unwrap())]]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex6 => vec![vec![
                        edev.btile_main(loc.0, edev.col_lcio.unwrap(), vr_row.unwrap()),
                        edev.btile_main(loc.0, edev.col_lcio.unwrap(), vr_row.unwrap() + 1),
                    ]],
                    prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
                }
            }
            ExtraFeatureKind::CenterDciVrBot(bank) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let anchor_reg = if edev.grids[loc.0].has_ps {
                    prjcombine_virtex4::grid::RegId::from_idx(
                        edev.grids[loc.0].regs - 2 + bank as usize,
                    )
                } else {
                    prjcombine_virtex4::grid::RegId::from_idx(bank as usize)
                };
                vec![vec![edev.btile_main(
                    loc.0,
                    edev.col_rio.unwrap(),
                    edev.grids[loc.0].row_reg_hclk(anchor_reg) - 25,
                )]]
            }
            ExtraFeatureKind::CenterDciVrTop(bank) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let anchor_reg = if edev.grids[loc.0].has_ps {
                    prjcombine_virtex4::grid::RegId::from_idx(
                        edev.grids[loc.0].regs - 2 + bank as usize,
                    )
                } else {
                    prjcombine_virtex4::grid::RegId::from_idx(bank as usize)
                };
                vec![vec![edev.btile_main(
                    loc.0,
                    edev.col_rio.unwrap(),
                    edev.grids[loc.0].row_reg_hclk(anchor_reg) + 24,
                )]]
            }

            ExtraFeatureKind::CenterDciHclk(bank) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4
                    | prjcombine_virtex4::grid::GridKind::Virtex5 => {
                        let (vr_row, io_row) = get_v4_center_dci_rows(edev, bank);
                        let row = vr_row.unwrap_or(io_row);
                        let reg = edev.grids[loc.0].row_to_reg(row);
                        let row = edev.grids[loc.0].row_reg_hclk(reg);
                        vec![vec![edev.btile_hclk(loc.0, edev.col_cfg, row)]]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex6 => {
                        let (vr_row, io_row) = get_v4_center_dci_rows(edev, bank);
                        let row = vr_row.unwrap_or(io_row);
                        let reg = edev.grids[loc.0].row_to_reg(row);
                        let row = edev.grids[loc.0].row_reg_hclk(reg);
                        vec![vec![edev.btile_hclk(loc.0, edev.col_lcio.unwrap(), row)]]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex7 => {
                        let anchor_reg = if edev.grids[loc.0].has_ps {
                            prjcombine_virtex4::grid::RegId::from_idx(
                                edev.grids[loc.0].regs - bank as usize,
                            )
                        } else {
                            prjcombine_virtex4::grid::RegId::from_idx(bank as usize)
                        };
                        vec![vec![edev.btile_hclk(
                            loc.0,
                            edev.col_rio.unwrap(),
                            edev.grids[loc.0].row_reg_hclk(anchor_reg),
                        )]]
                    }
                }
            }
            ExtraFeatureKind::CenterDciHclkCascade(bank, kind) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let grid = &edev.grids[loc.0];
                let (row_a, row_b) = if bank == 1 {
                    (grid.row_bufg() + 8, edev.row_iobdcm.unwrap() - 32)
                } else {
                    (edev.row_dcmiob.unwrap() + 32, grid.row_bufg() - 8)
                };
                let rows = if row_a == row_b {
                    vec![row_a]
                } else {
                    vec![row_a, row_b]
                };
                let mut res = vec![];
                for row in rows {
                    if edev
                        .egrid
                        .find_node(loc.0, (edev.col_cfg, row), |node| {
                            edev.egrid.db.nodes.key(node.kind) == kind
                        })
                        .is_some()
                    {
                        res.push(vec![edev.btile_hclk(loc.0, edev.col_cfg, row)]);
                    }
                }
                res
            }
            ExtraFeatureKind::AllBankIoi => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let reg = edev.grids[loc.0].row_to_reg(loc.2);
                let bot = edev.grids[loc.0].row_reg_bot(reg);
                let mut res = vec![];
                for i in 0..edev.grids[loc.0].rows_per_reg() {
                    let row = bot + i;
                    if edev
                        .egrid
                        .find_bel(loc.0, (loc.1, row), "IODELAY0")
                        .is_some()
                    {
                        res.push(match edev.kind {
                            prjcombine_virtex4::grid::GridKind::Virtex4
                            | prjcombine_virtex4::grid::GridKind::Virtex5 => {
                                vec![edev.btile_main(loc.0, loc.1, row)]
                            }
                            prjcombine_virtex4::grid::GridKind::Virtex6 => vec![
                                edev.btile_main(loc.0, loc.1, row),
                                edev.btile_main(loc.0, loc.1, row + 1),
                            ],
                            prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
                        });
                    }
                }
                res
            }
            ExtraFeatureKind::ClkRebuf(dir, kind) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let mut die = loc.0;
                let mut row = loc.2;
                loop {
                    match dir {
                        Dir::S => {
                            if row.to_idx() == 0 {
                                if die.to_idx() == 0 {
                                    return vec![];
                                }
                                die -= 1;
                                row = backend.egrid.die(die).rows().next_back().unwrap();
                            } else {
                                row -= 1;
                            }
                        }
                        Dir::N => {
                            if row == backend.egrid.die(die).rows().next_back().unwrap() {
                                row = RowId::from_idx(0);
                                die += 1;
                                if die == backend.egrid.die.next_id() {
                                    return vec![];
                                }
                            } else {
                                row += 1;
                            }
                        }
                        _ => unreachable!(),
                    }
                    if let Some(node) = backend.egrid.find_node(die, (loc.1, row), |node| {
                        matches!(
                            &backend.egrid.db.nodes.key(node.kind)[..],
                            "CLK_BUFG_REBUF" | "CLK_BALI_REBUF"
                        )
                    }) {
                        if node.kind != kind {
                            return vec![];
                        }
                        let height = if backend.egrid.db.nodes.key(kind) == "CLK_BUFG_REBUF" {
                            2
                        } else {
                            16
                        };
                        return vec![
                            (0..height)
                                .map(|i| edev.btile_main(die, loc.1, row + i))
                                .collect(),
                        ];
                    }
                }
            }
            ExtraFeatureKind::CmtDir(dir) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let scol = match dir {
                    Dir::W => edev.col_lio.unwrap() + 1,
                    Dir::E => edev.col_rio.unwrap() - 1,
                    _ => unreachable!(),
                };
                if backend
                    .egrid
                    .find_node(loc.0, (scol, loc.2), |node| {
                        backend.egrid.db.nodes.key(node.kind) == "CMT"
                    })
                    .is_some()
                {
                    let mut res = vec![];
                    for i in 0..50 {
                        res.push(edev.btile_main(loc.0, scol, loc.2 - 25 + i));
                    }
                    res.push(edev.btile_hclk(loc.0, scol, loc.2));
                    vec![res]
                } else {
                    vec![]
                }
            }
            ExtraFeatureKind::Cmt(dy) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex5 => todo!(),
                    prjcombine_virtex4::grid::GridKind::Virtex6 => {
                        let row = edev.grids[loc.0].row_hclk(loc.2 + dy);
                        let mut res = vec![];
                        for i in 0..40 {
                            res.push(edev.btile_main(loc.0, loc.1, row - 20 + i));
                        }
                        res.push(edev.btile_hclk(loc.0, loc.1, row));
                        vec![res]
                    }
                    prjcombine_virtex4::grid::GridKind::Virtex7 => {
                        let row = edev.grids[loc.0].row_hclk(loc.2 + dy);
                        let mut res = vec![];
                        for i in 0..50 {
                            res.push(edev.btile_main(loc.0, loc.1, row - 25 + i));
                        }
                        res.push(edev.btile_hclk(loc.0, loc.1, row));
                        vec![res]
                    }
                }
            }
            ExtraFeatureKind::HclkIoiInnerSide(dir) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                vec![vec![edev.btile_hclk(
                    loc.0,
                    match dir {
                        Dir::W => edev.col_lcio.unwrap(),
                        Dir::E => edev.col_rcio.unwrap(),
                        _ => unreachable!(),
                    },
                    loc.2,
                )]]
            }
            ExtraFeatureKind::HclkIoiHere(kind) => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                let col = ColId::from_idx(loc.1.to_idx() ^ 1);
                if backend
                    .egrid
                    .find_node(loc.0, (col, loc.2), |node| node.kind == kind)
                    .is_some()
                {
                    vec![vec![edev.btile_hclk(loc.0, col, loc.2)]]
                } else {
                    vec![]
                }
            }
            ExtraFeatureKind::AllBankIo => {
                let ExpandedDevice::Virtex4(edev) = backend.edev else {
                    unreachable!()
                };
                assert_eq!(edev.kind, prjcombine_virtex4::grid::GridKind::Virtex7);
                (0..24)
                    .map(|i| {
                        let row = edev.grids[loc.0].row_hclk(loc.2) - 24 + i * 2;
                        vec![
                            edev.btile_main(loc.0, loc.1, row),
                            edev.btile_main(loc.0, loc.1, row + 1),
                        ]
                    })
                    .collect()
            }
            ExtraFeatureKind::AllMcbIoi => {
                let ExpandedDevice::Spartan6(edev) = backend.edev else {
                    unreachable!()
                };
                let mut res = vec![];
                for row in backend.egrid.die(loc.0).rows() {
                    if let Some(split) = edev.grid.row_mcb_split {
                        if loc.2 < split && row >= split {
                            continue;
                        }
                        if loc.2 >= split && row < split {
                            continue;
                        }
                    }
                    if backend
                        .egrid
                        .find_node(loc.0, (loc.1, row), |node| {
                            backend.egrid.db.nodes.key(node.kind) == "IOI.LR"
                        })
                        .is_some()
                    {
                        res.push(vec![edev.btile_main(loc.1, row)]);
                    }
                }
                res
            }
            ExtraFeatureKind::IoiHere => {
                let ExpandedDevice::Spartan6(edev) = backend.edev else {
                    unreachable!()
                };
                vec![vec![edev.btile_main(loc.1, loc.2)]]
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtraFeature {
    pub kind: ExtraFeatureKind,
    pub id: FeatureId,
}

impl ExtraFeature {
    pub fn new(
        kind: ExtraFeatureKind,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            id: FeatureId {
                tile: tile.into(),
                bel: bel.into(),
                attr: attr.into(),
                val: val.into(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct TileFuzzerGen<'a> {
    pub node: NodeKindId,
    pub bits: TileBits,
    pub feature: FeatureId,
    pub base: Vec<TileKV<'a>>,
    pub fuzz: Vec<TileFuzzKV<'a>>,
    pub extras: Vec<ExtraFeature>,
}

impl<'b> TileFuzzerGen<'b> {
    fn try_generate(
        &self,
        backend: &IseBackend<'b>,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
        loc: (DieId, ColId, RowId, LayerId),
    ) -> Option<(Fuzzer<IseBackend<'b>>, Vec<usize>)> {
        let tiles = self.bits.get_bits(backend, loc);
        let mut fuzzer = Fuzzer::new(FuzzerInfo {
            features: vec![FuzzerFeature {
                tiles,
                id: self.feature.clone(),
            }],
        });
        for x in &self.base {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        for x in &self.fuzz {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        if !fuzzer.is_ok(kv) {
            return None;
        }
        let mut got_extras = vec![];
        for (eidx, extra) in self.extras.iter().enumerate() {
            let extra_insts = extra.kind.get_tiles(backend, loc, &extra.id.tile);
            if !extra_insts.is_empty() {
                got_extras.push(eidx);
            }
            for tiles in extra_insts {
                fuzzer.info.features.push(FuzzerFeature {
                    id: extra.id.clone(),
                    tiles,
                });
            }
        }
        Some((fuzzer, got_extras))
    }
}

impl<'b> FuzzerGen<IseBackend<'b>> for TileFuzzerGen<'b> {
    fn generate<'a>(
        &self,
        backend: &'a IseBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
    ) -> Option<(
        Fuzzer<IseBackend<'b>>,
        Option<Box<dyn FuzzerGen<IseBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.node];
        let mut rng = rand::rng();
        let (res, extras) = 'find: {
            if locs.len() > 20 {
                for &loc in locs.choose_multiple(&mut rng, 20) {
                    if let Some(x) = self.try_generate(backend, kv, loc) {
                        break 'find x;
                    }
                }
            }
            for &loc in locs.choose_multiple(&mut rng, locs.len()) {
                if let Some(x) = self.try_generate(backend, kv, loc) {
                    break 'find x;
                }
            }
            return None;
        };
        let mut needed_extras = BTreeSet::from_iter(0..self.extras.len());
        for extra in extras {
            needed_extras.remove(&extra);
        }
        if !needed_extras.is_empty() {
            return Some((
                res,
                Some(Box::new(TileFuzzerChainGen {
                    orig: self.clone(),
                    needed_extras,
                })),
            ));
        }
        Some((res, None))
    }
}

#[derive(Debug)]
struct TileFuzzerChainGen<'a> {
    orig: TileFuzzerGen<'a>,
    needed_extras: BTreeSet<usize>,
}

impl<'b> FuzzerGen<IseBackend<'b>> for TileFuzzerChainGen<'b> {
    fn generate<'a>(
        &self,
        backend: &'a IseBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
    ) -> Option<(
        Fuzzer<IseBackend<'b>>,
        Option<Box<dyn FuzzerGen<IseBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.orig.node];
        let mut rng = rand::rng();
        let (res, extras) = 'find: {
            if locs.len() > 20 {
                for &loc in locs.choose_multiple(&mut rng, 20) {
                    if let Some((res, extras)) = self.orig.try_generate(backend, kv, loc) {
                        for &extra in &extras {
                            if self.needed_extras.contains(&extra) {
                                break 'find (res, extras);
                            }
                        }
                    }
                }
            }
            for &loc in locs.choose_multiple(&mut rng, locs.len()) {
                if let Some((res, extras)) = self.orig.try_generate(backend, kv, loc) {
                    for &extra in &extras {
                        if self.needed_extras.contains(&extra) {
                            break 'find (res, extras);
                        }
                    }
                }
            }
            return None;
        };
        let mut needed_extras = self.needed_extras.clone();
        for extra in extras {
            needed_extras.remove(&extra);
        }
        if !needed_extras.is_empty() {
            return Some((
                res,
                Some(Box::new(TileFuzzerChainGen {
                    orig: self.orig.clone(),
                    needed_extras,
                })),
            ));
        }
        Some((res, None))
    }
}

#[derive(Debug, Clone)]
pub struct TileMultiFuzzerGen<'a> {
    pub node: NodeKindId,
    pub bits: TileBits,
    pub feature: FeatureId,
    pub base: Vec<TileKV<'a>>,
    pub width: usize,
    pub fuzz: TileMultiFuzzKV<'a>,
    pub extras: Vec<ExtraFeature>,
}

impl<'b> TileMultiFuzzerGen<'b> {
    fn try_generate(
        &self,
        backend: &IseBackend<'b>,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
        loc: (DieId, ColId, RowId, LayerId),
    ) -> Option<(Fuzzer<IseBackend<'b>>, Vec<usize>)> {
        let tiles = self.bits.get_bits(backend, loc);
        let mut fuzzer = Fuzzer::new(FuzzerInfo {
            features: vec![FuzzerFeature {
                tiles,
                id: self.feature.clone(),
            }],
        });
        for x in &self.base {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        fuzzer = fuzzer.bits(self.width);
        fuzzer = self.fuzz.apply(backend, loc, fuzzer);
        if !fuzzer.is_ok(kv) {
            return None;
        }
        let mut got_extras = vec![];
        for (eidx, extra) in self.extras.iter().enumerate() {
            let extra_insts = extra.kind.get_tiles(backend, loc, &extra.id.tile);
            if !extra_insts.is_empty() {
                got_extras.push(eidx);
            }
            for tiles in extra_insts {
                fuzzer.info.features.push(FuzzerFeature {
                    id: extra.id.clone(),
                    tiles,
                });
            }
        }
        Some((fuzzer, got_extras))
    }
}

impl<'b> FuzzerGen<IseBackend<'b>> for TileMultiFuzzerGen<'b> {
    fn generate<'a>(
        &self,
        backend: &'a IseBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
    ) -> Option<(
        Fuzzer<IseBackend<'b>>,
        Option<Box<dyn FuzzerGen<IseBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.node];
        let mut rng = rand::rng();
        let (res, extras) = 'find: {
            if locs.len() > 20 {
                for &loc in locs.choose_multiple(&mut rng, 20) {
                    if let Some(x) = self.try_generate(backend, kv, loc) {
                        break 'find x;
                    }
                }
            }
            for &loc in locs.choose_multiple(&mut rng, locs.len()) {
                if let Some(x) = self.try_generate(backend, kv, loc) {
                    break 'find x;
                }
            }
            return None;
        };
        let mut needed_extras = BTreeSet::from_iter(0..self.extras.len());
        for extra in extras {
            needed_extras.remove(&extra);
        }
        if !needed_extras.is_empty() {
            return Some((
                res,
                Some(Box::new(TileMultiFuzzerChainGen {
                    orig: self.clone(),
                    needed_extras,
                })),
            ));
        }
        Some((res, None))
    }
}

#[derive(Debug)]
struct TileMultiFuzzerChainGen<'a> {
    orig: TileMultiFuzzerGen<'a>,
    needed_extras: BTreeSet<usize>,
}

impl<'b> FuzzerGen<IseBackend<'b>> for TileMultiFuzzerChainGen<'b> {
    fn generate<'a>(
        &self,
        backend: &'a IseBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
    ) -> Option<(
        Fuzzer<IseBackend<'b>>,
        Option<Box<dyn FuzzerGen<IseBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.orig.node];
        let mut rng = rand::rng();
        let (res, extras) = 'find: {
            if locs.len() > 20 {
                for &loc in locs.choose_multiple(&mut rng, 20) {
                    if let Some((res, extras)) = self.orig.try_generate(backend, kv, loc) {
                        for &extra in &extras {
                            if self.needed_extras.contains(&extra) {
                                break 'find (res, extras);
                            }
                        }
                    }
                }
            }
            for &loc in locs.choose_multiple(&mut rng, locs.len()) {
                if let Some((res, extras)) = self.orig.try_generate(backend, kv, loc) {
                    for &extra in &extras {
                        if self.needed_extras.contains(&extra) {
                            break 'find (res, extras);
                        }
                    }
                }
            }
            return None;
        };
        let mut needed_extras = self.needed_extras.clone();
        for extra in extras {
            needed_extras.remove(&extra);
        }
        if !needed_extras.is_empty() {
            return Some((
                res,
                Some(Box::new(TileMultiFuzzerChainGen {
                    orig: self.orig.clone(),
                    needed_extras,
                })),
            ));
        }
        Some((res, None))
    }
}

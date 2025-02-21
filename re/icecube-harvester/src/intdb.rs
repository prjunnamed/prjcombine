use std::collections::{BTreeSet, HashMap, hash_map};

use enum_map::EnumMap;
use prjcombine_interconnect::{
    db::{BelInfo, BelPin, Dir, IntDb, NodeKind, NodeTileId, PinDir, TermInfo, TermKind, WireKind},
    grid::{ColId, EdgeIoCoord, RowId},
};
use prjcombine_siliconblue::chip::{ChipKind, ExtraNode};
use unnamed_entity::{EntityId, EntityVec};

use crate::sites::BelPins;

fn add_input(db: &IntDb, bel: &mut BelInfo, name: &str, tile: usize, wire: &str) {
    bel.pins.insert(
        name.into(),
        BelPin {
            wires: BTreeSet::from_iter([(NodeTileId::from_idx(tile), db.get_wire(wire))]),
            dir: PinDir::Input,
            is_intf_in: false,
        },
    );
}

fn add_output(db: &IntDb, bel: &mut BelInfo, name: &str, tile: usize, wires: &[&str]) {
    bel.pins.insert(
        name.into(),
        BelPin {
            wires: BTreeSet::from_iter(
                wires
                    .iter()
                    .map(|wire| (NodeTileId::from_idx(tile), db.get_wire(wire))),
            ),
            dir: PinDir::Output,
            is_intf_in: false,
        },
    );
}

pub fn make_intdb(kind: ChipKind) -> IntDb {
    let mut db = IntDb::default();
    let mut main_terms = EnumMap::from_fn(|dir| TermKind {
        dir,
        wires: Default::default(),
    });

    for i in 0..8 {
        db.wires.insert(format!("GLOBAL.{i}"), WireKind::ClkOut(0));
    }

    for i in 0..4 {
        db.wires.insert(format!("GOUT.{i}"), WireKind::MuxOut);
    }

    for i in 0..12 {
        let mut w = db
            .wires
            .insert(format!("QUAD.H{i}.0"), WireKind::MultiOut)
            .0;
        for j in 1..5 {
            let ww = db
                .wires
                .insert(format!("QUAD.H{i}.{j}"), WireKind::MultiBranch(Dir::W))
                .0;
            main_terms[Dir::W].wires.insert(ww, TermInfo::PassFar(w));
            w = ww;
        }
    }

    for i in 0..12 {
        let mut w = db
            .wires
            .insert(format!("QUAD.V{i}.0"), WireKind::MultiOut)
            .0;
        for j in 1..5 {
            let ww = db
                .wires
                .insert(format!("QUAD.V{i}.{j}"), WireKind::MultiBranch(Dir::S))
                .0;
            main_terms[Dir::S].wires.insert(ww, TermInfo::PassFar(w));
            w = ww;
            let ww = db
                .wires
                .insert(format!("QUAD.V{i}.{j}.W"), WireKind::MultiBranch(Dir::E))
                .0;
            main_terms[Dir::E].wires.insert(ww, TermInfo::PassFar(w));
        }
    }

    for i in 0..2 {
        let mut w = db
            .wires
            .insert(format!("LONG.H{i}.0"), WireKind::MultiOut)
            .0;
        for j in 1..13 {
            let ww = db
                .wires
                .insert(format!("LONG.H{i}.{j}"), WireKind::MultiBranch(Dir::W))
                .0;
            main_terms[Dir::W].wires.insert(ww, TermInfo::PassFar(w));
            w = ww;
        }
    }
    for i in 0..2 {
        let mut w = db
            .wires
            .insert(format!("LONG.V{i}.0"), WireKind::MultiOut)
            .0;
        for j in 1..13 {
            let ww = db
                .wires
                .insert(format!("LONG.V{i}.{j}"), WireKind::MultiBranch(Dir::S))
                .0;
            main_terms[Dir::S].wires.insert(ww, TermInfo::PassFar(w));
            w = ww;
        }
    }

    for i in 0..4 {
        for j in 0..8 {
            db.wires.insert(format!("LOCAL.{i}.{j}"), WireKind::MuxOut);
        }
    }

    for i in 0..8 {
        for j in 0..4 {
            db.wires
                .insert(format!("IMUX.LC{i}.I{j}"), WireKind::MuxOut);
        }
    }

    for name in [
        "IMUX.CLK",
        "IMUX.RST",
        "IMUX.CE",
        "IMUX.IO0.DOUT0",
        "IMUX.IO0.DOUT1",
        "IMUX.IO0.OE",
        "IMUX.IO1.DOUT0",
        "IMUX.IO1.DOUT1",
        "IMUX.IO1.OE",
        "IMUX.IO.ICLK",
        "IMUX.IO.OCLK",
        "IMUX.IO.EXTRA",
    ] {
        db.wires.insert(name.into(), WireKind::MuxOut);
    }

    for i in 0..8 {
        let w = db.wires.insert(format!("OUT.LC{i}"), WireKind::LogicOut).0;
        for dir in [Dir::N, Dir::S] {
            let wo = db
                .wires
                .insert(format!("OUT.LC{i}.{dir}"), WireKind::Branch(!dir))
                .0;
            main_terms[!dir].wires.insert(wo, TermInfo::PassFar(w));
        }
        for dir in [Dir::E, Dir::W] {
            let wo = db
                .wires
                .insert(format!("OUT.LC{i}.{dir}"), WireKind::Branch(!dir))
                .0;
            main_terms[!dir].wires.insert(wo, TermInfo::PassFar(w));
            for dir2 in [Dir::N, Dir::S] {
                let woo = db
                    .wires
                    .insert(format!("OUT.LC{i}.{dir}{dir2}"), WireKind::Branch(!dir2))
                    .0;
                main_terms[!dir2].wires.insert(woo, TermInfo::PassFar(wo));
            }
        }
    }
    if !kind.has_lrio() {
        for i in 0..8 {
            db.wires
                .insert(format!("OUT.CASCADE{i}"), WireKind::LogicOut);
        }
    }

    for (dir, term) in main_terms {
        db.terms.insert(format!("MAIN.{dir}"), term);
    }

    for name in ["PLB", "INT", "INT.BRAM", "IO.L", "IO.R", "IO.B", "IO.T"] {
        if (name == "IO.L" || name == "IO.R") && !kind.has_lrio() {
            continue;
        }
        if name == "INT" && kind.has_lrio() {
            continue;
        }
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        if name == "PLB" || name == "INT" {
            for i in 0..8 {
                let mut bel = BelInfo::default();
                for j in 0..4 {
                    add_input(
                        &db,
                        &mut bel,
                        &format!("I{j}"),
                        0,
                        &format!("IMUX.LC{i}.I{j}"),
                    );
                }
                for pin in ["CLK", "RST", "CE"] {
                    add_input(&db, &mut bel, pin, 0, &format!("IMUX.{pin}"));
                }
                if name == "INT" {
                    add_input(&db, &mut bel, "CASCADE", 0, &format!("OUT.CASCADE{i}"));
                }
                add_output(&db, &mut bel, "O", 0, &[&format!("OUT.LC{i}")]);
                node.bels.insert(format!("LC{i}"), bel);
            }
        }
        if name.starts_with("IO") {
            for i in 0..2 {
                let mut bel = BelInfo::default();
                for pin in ["DOUT0", "DOUT1", "OE"] {
                    add_input(&db, &mut bel, pin, 0, &format!("IMUX.IO{i}.{pin}"));
                }
                for pin in ["ICLK", "OCLK"] {
                    add_input(&db, &mut bel, pin, 0, &format!("IMUX.IO.{pin}"));
                }
                add_input(&db, &mut bel, "CE", 0, "IMUX.CE");
                add_output(
                    &db,
                    &mut bel,
                    "DIN0",
                    0,
                    &[
                        &format!("OUT.LC{}", 2 * i)[..],
                        &format!("OUT.LC{}", 2 * i + 4)[..],
                    ],
                );
                add_output(
                    &db,
                    &mut bel,
                    "DIN1",
                    0,
                    &[
                        &format!("OUT.LC{}", 2 * i + 1)[..],
                        &format!("OUT.LC{}", 2 * i + 5)[..],
                    ],
                );
                node.bels.insert(format!("IO{i}"), bel);
            }
        }
        db.nodes.insert(name.into(), node);
    }
    db.nodes.insert(
        "CNR".into(),
        NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        },
    );

    let ice40_bramv2 = kind.has_ice40_bramv2();
    {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([(), ()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        let mut bel = BelInfo::default();
        let (tile_w, tile_r) = if ice40_bramv2 { (1, 0) } else { (0, 1) };
        add_input(&db, &mut bel, "WCLK", tile_w, "IMUX.CLK");
        add_input(&db, &mut bel, "WE", tile_w, "IMUX.RST");
        add_input(&db, &mut bel, "WCLKE", tile_w, "IMUX.CE");
        add_input(&db, &mut bel, "RCLK", tile_r, "IMUX.CLK");
        add_input(&db, &mut bel, "RE", tile_r, "IMUX.RST");
        add_input(&db, &mut bel, "RCLKE", tile_r, "IMUX.CE");
        let addr_bits = if kind.is_ice40() { 11 } else { 8 };
        for i in 0..addr_bits {
            let xi = if ice40_bramv2 { i ^ 7 } else { i };
            let lc = xi % 8;
            let ii = if xi >= 8 { 2 } else { 0 };
            add_input(
                &db,
                &mut bel,
                &format!("WADDR{i}"),
                tile_w,
                &format!("IMUX.LC{lc}.I{ii}"),
            );
            add_input(
                &db,
                &mut bel,
                &format!("RADDR{i}"),
                tile_r,
                &format!("IMUX.LC{lc}.I{ii}"),
            );
        }
        for i in 0..16 {
            let xi = if ice40_bramv2 { i ^ 15 } else { i };
            let tile = xi / 8;
            let lc = xi % 8;
            add_input(
                &db,
                &mut bel,
                &format!("WDATA{i}"),
                tile,
                &format!("IMUX.LC{lc}.I1"),
            );
            add_input(
                &db,
                &mut bel,
                &format!("MASK{i}"),
                tile,
                &format!("IMUX.LC{lc}.I3"),
            );
            add_output(
                &db,
                &mut bel,
                &format!("RDATA{i}"),
                tile,
                &[&format!("OUT.LC{lc}")],
            );
        }
        node.bels.insert("BRAM".into(), bel);
        db.nodes.insert("BRAM".into(), node);
    }

    {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        let mut bel = BelInfo::default();
        add_input(&db, &mut bel, "LATCH", 0, "IMUX.IO.EXTRA");
        node.bels.insert("IO_LATCH".into(), bel);
        db.nodes.insert("IO_LATCH".into(), node);
    }

    {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        let mut bel = BelInfo::default();
        add_input(&db, &mut bel, "I", 0, "IMUX.IO.EXTRA");
        node.bels.insert("GB_FABRIC".into(), bel);
        db.nodes.insert("GB_FABRIC".into(), node);
    }

    {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        let mut bel = BelInfo::default();
        for i in 0..8 {
            add_output(
                &db,
                &mut bel,
                &format!("O{i}"),
                0,
                &[&format!("GLOBAL.{i}")],
            );
        }
        node.bels.insert("GBOUT".into(), bel);
        db.nodes.insert("GBOUT".into(), node);
    }

    db
}

pub struct MiscNodeBuilder {
    pub node: NodeKind,
    pub io: Vec<EdgeIoCoord>,
    pub fixed_tiles: usize,
    pub tiles: EntityVec<NodeTileId, (ColId, RowId)>,
    pub tiles_map: HashMap<(ColId, RowId), NodeTileId>,
}

impl MiscNodeBuilder {
    pub fn new(fixed_tiles: &[(ColId, RowId)]) -> Self {
        let mut tiles = EntityVec::new();
        let mut tiles_map = HashMap::new();
        for &crd in fixed_tiles {
            let tile = tiles.push(crd);
            tiles_map.insert(crd, tile);
        }
        Self {
            node: NodeKind {
                tiles: EntityVec::from_iter(tiles.iter().map(|_| ())),
                muxes: Default::default(),
                iris: Default::default(),
                intfs: Default::default(),
                bels: Default::default(),
            },
            io: vec![],
            fixed_tiles: fixed_tiles.len(),
            tiles,
            tiles_map,
        }
    }

    pub fn get_tile(&mut self, crd: (ColId, RowId)) -> NodeTileId {
        match self.tiles_map.entry(crd) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
            hash_map::Entry::Vacant(entry) => {
                let tile = self.node.tiles.push(());
                self.tiles.push(crd);
                entry.insert(tile);
                tile
            }
        }
    }

    pub fn add_bel(&mut self, name: &str, pins: &BelPins) {
        let mut bel = BelInfo::default();
        for (pin, &(_, crd, wire)) in &pins.ins {
            let tile = self.get_tile(crd);
            bel.pins.insert(
                pin.clone(),
                BelPin {
                    wires: BTreeSet::from_iter([(tile, wire)]),
                    dir: PinDir::Input,
                    is_intf_in: false,
                },
            );
        }
        for (pin, iwires) in &pins.outs {
            let mut wires = BTreeSet::new();
            for &(_, crd, wire) in iwires {
                let tile = self.get_tile(crd);
                wires.insert((tile, wire));
            }
            bel.pins.insert(
                pin.clone(),
                BelPin {
                    wires,
                    dir: PinDir::Output,
                    is_intf_in: false,
                },
            );
        }
        self.node.bels.insert(name.into(), bel);
    }

    pub fn finish(mut self) -> (NodeKind, ExtraNode) {
        let mut tiles_sorted = Vec::from_iter(self.tiles.values().copied());
        let mut new_tiles: EntityVec<NodeTileId, _> =
            EntityVec::from_iter(tiles_sorted[..self.fixed_tiles].iter().copied());
        let mut new_tiles_map: HashMap<_, _> =
            HashMap::from_iter(new_tiles.iter().map(|(k, &v)| (v, k)));
        tiles_sorted.sort();
        for crd in tiles_sorted {
            match new_tiles_map.entry(crd) {
                hash_map::Entry::Occupied(_) => (),
                hash_map::Entry::Vacant(entry) => {
                    let tile = new_tiles.push(crd);
                    entry.insert(tile);
                }
            }
        }
        for bel in self.node.bels.values_mut() {
            for pin in bel.pins.values_mut() {
                pin.wires = pin
                    .wires
                    .iter()
                    .map(|&(tile, wire)| {
                        let new_tile = new_tiles_map[&self.tiles[tile]];
                        (new_tile, wire)
                    })
                    .collect();
            }
        }
        (
            self.node,
            ExtraNode {
                io: self.io,
                tiles: new_tiles,
            },
        )
    }
}

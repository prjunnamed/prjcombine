use std::collections::{BTreeMap, BTreeSet, HashMap, hash_map};

use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, BelSlotId, CellSlotId, ConnectorClass, ConnectorWire, IntDb,
        TileClass, TileSlotId, TileWireCoord, WireKind,
    },
    dir::{Dir, DirMap},
    grid::{CellCoord, EdgeIoCoord},
};
use prjcombine_siliconblue::{
    bels,
    chip::{Chip, ChipKind, SpecialIoKey, SpecialTile},
    cslots, regions, tslots,
};
use prjcombine_entity::EntityVec;

use crate::sites::BelPins;

fn add_input(db: &IntDb, bel: &mut Bel, name: &str, cell: usize, wire: &str) {
    bel.pins.insert(
        name.into(),
        BelPin::new_in(TileWireCoord::new_idx(cell, db.get_wire(wire))),
    );
}

fn add_output(db: &IntDb, bel: &mut Bel, name: &str, cell: usize, wires: &[&str]) {
    bel.pins.insert(
        name.into(),
        BelPin::new_out_multi(
            wires
                .iter()
                .map(|wire| TileWireCoord::new_idx(cell, db.get_wire(wire))),
        ),
    );
}

pub fn make_intdb(kind: ChipKind) -> IntDb {
    let mut db = IntDb::new(tslots::SLOTS, bels::SLOTS, regions::SLOTS, cslots::SLOTS);

    let term_slots = DirMap::from_fn(|dir| match dir {
        Dir::W => cslots::W,
        Dir::E => cslots::E,
        Dir::S => cslots::S,
        Dir::N => cslots::N,
    });

    let mut passes = DirMap::from_fn(|dir| ConnectorClass::new(term_slots[dir]));

    for i in 0..8 {
        db.wires
            .insert(format!("GLOBAL.{i}"), WireKind::Regional(regions::GLOBAL));
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
                .insert(format!("QUAD.H{i}.{j}"), WireKind::MultiBranch(cslots::W))
                .0;
            passes[Dir::W].wires.insert(ww, ConnectorWire::Pass(w));
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
                .insert(format!("QUAD.V{i}.{j}"), WireKind::MultiBranch(cslots::S))
                .0;
            passes[Dir::S].wires.insert(ww, ConnectorWire::Pass(w));
            w = ww;
            let ww = db
                .wires
                .insert(format!("QUAD.V{i}.{j}.W"), WireKind::MultiBranch(cslots::E))
                .0;
            passes[Dir::E].wires.insert(ww, ConnectorWire::Pass(w));
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
                .insert(format!("LONG.H{i}.{j}"), WireKind::MultiBranch(cslots::W))
                .0;
            passes[Dir::W].wires.insert(ww, ConnectorWire::Pass(w));
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
                .insert(format!("LONG.V{i}.{j}"), WireKind::MultiBranch(cslots::S))
                .0;
            passes[Dir::S].wires.insert(ww, ConnectorWire::Pass(w));
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
        "IMUX.CLK.OPTINV",
        "IMUX.RST",
        "IMUX.CE",
        "IMUX.IO0.DOUT0",
        "IMUX.IO0.DOUT1",
        "IMUX.IO0.OE",
        "IMUX.IO1.DOUT0",
        "IMUX.IO1.DOUT1",
        "IMUX.IO1.OE",
        "IMUX.IO.ICLK",
        "IMUX.IO.ICLK.OPTINV",
        "IMUX.IO.OCLK",
        "IMUX.IO.OCLK.OPTINV",
        "IMUX.IO.EXTRA",
    ] {
        db.wires.insert(name.into(), WireKind::MuxOut);
    }

    for i in 0..8 {
        let w = db.wires.insert(format!("OUT.LC{i}"), WireKind::LogicOut).0;
        for dir in [Dir::N, Dir::S] {
            let wo = db
                .wires
                .insert(
                    format!("OUT.LC{i}.{dir}"),
                    WireKind::Branch(term_slots[!dir]),
                )
                .0;
            passes[!dir].wires.insert(wo, ConnectorWire::Pass(w));
        }
        for dir in [Dir::E, Dir::W] {
            let wo = db
                .wires
                .insert(
                    format!("OUT.LC{i}.{dir}"),
                    WireKind::Branch(term_slots[!dir]),
                )
                .0;
            passes[!dir].wires.insert(wo, ConnectorWire::Pass(w));
            for dir2 in [Dir::N, Dir::S] {
                let woo = db
                    .wires
                    .insert(
                        format!("OUT.LC{i}.{dir}{dir2}"),
                        WireKind::Branch(term_slots[!dir2]),
                    )
                    .0;
                passes[!dir2].wires.insert(woo, ConnectorWire::Pass(wo));
            }
        }
    }

    for (dir, pass) in passes {
        db.conn_classes.insert(format!("PASS_{dir}"), pass);
    }

    {
        let mut tcls = TileClass::new(tslots::MAIN, 1);
        for i in 0..8 {
            let mut bel = Bel::default();
            for j in 0..4 {
                add_input(
                    &db,
                    &mut bel,
                    &format!("I{j}"),
                    0,
                    &format!("IMUX.LC{i}.I{j}"),
                );
            }
            add_input(&db, &mut bel, "CLK", 0, "IMUX.CLK.OPTINV");
            add_input(&db, &mut bel, "RST", 0, "IMUX.RST");
            add_input(&db, &mut bel, "CE", 0, "IMUX.CE");
            add_output(&db, &mut bel, "O", 0, &[&format!("OUT.LC{i}")]);
            tcls.bels.insert(bels::LC[i], BelInfo::Bel(bel));
        }
        db.tile_classes.insert(kind.tile_class_plb().into(), tcls);
    }
    if kind != ChipKind::Ice40P03 {
        let tcls = TileClass::new(tslots::MAIN, 1);
        db.tile_classes.insert("INT_BRAM".into(), tcls);
    }
    for dir in Dir::DIRS {
        let Some(tile) = kind.tile_class_ioi(dir) else {
            continue;
        };
        let mut tcls = TileClass::new(tslots::MAIN, 1);
        for i in 0..2 {
            let mut bel = Bel::default();
            for pin in ["DOUT0", "DOUT1", "OE"] {
                add_input(&db, &mut bel, pin, 0, &format!("IMUX.IO{i}.{pin}"));
            }
            for pin in ["ICLK", "OCLK"] {
                add_input(&db, &mut bel, pin, 0, &format!("IMUX.IO.{pin}.OPTINV"));
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
            tcls.bels.insert(bels::IO[i], BelInfo::Bel(bel));
        }
        db.tile_classes.insert(tile.into(), tcls);
        let Some(tile) = kind.tile_class_iob(dir) else {
            continue;
        };
        let tcls = TileClass::new(tslots::IOB, 1);
        db.tile_classes.insert(tile.into(), tcls);
    }

    if kind != ChipKind::Ice40P03 {
        let ice40_bramv2 = kind.has_ice40_bramv2();
        let mut tcls = TileClass::new(tslots::BEL, 2);
        let mut bel = Bel::default();
        let (tile_w, tile_r) = if ice40_bramv2 { (1, 0) } else { (0, 1) };
        add_input(&db, &mut bel, "WCLK", tile_w, "IMUX.CLK.OPTINV");
        add_input(&db, &mut bel, "WE", tile_w, "IMUX.RST");
        add_input(&db, &mut bel, "WCLKE", tile_w, "IMUX.CE");
        add_input(&db, &mut bel, "RCLK", tile_r, "IMUX.CLK.OPTINV");
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
        tcls.bels.insert(bels::BRAM, BelInfo::Bel(bel));
        db.tile_classes.insert(kind.tile_class_bram().into(), tcls);
    }

    if let Some(tcname) = kind.tile_class_colbuf() {
        for tcname in [tcname, "COLBUF_IO_W", "COLBUF_IO_E"] {
            let tcls = TileClass::new(tslots::COLBUF, 1);
            db.tile_classes.insert(tcname.into(), tcls);
        }
    }

    {
        let mut tcls = TileClass::new(tslots::BEL, 1);
        let mut bel = Bel::default();
        add_input(&db, &mut bel, "LATCH", 0, "IMUX.IO.EXTRA");
        tcls.bels.insert(bels::IO_LATCH, BelInfo::Bel(bel));
        db.tile_classes.insert("IO_LATCH".into(), tcls);
    }

    {
        let mut tcls = TileClass::new(tslots::BEL, 1);
        let mut bel = Bel::default();
        add_input(&db, &mut bel, "I", 0, "IMUX.IO.EXTRA");
        tcls.bels.insert(bels::GB_FABRIC, BelInfo::Bel(bel));
        db.tile_classes.insert("GB_FABRIC".into(), tcls);
    }

    {
        let mut tcls = TileClass::new(tslots::GB_ROOT, 1);
        let mut bel = Bel::default();
        for i in 0..8 {
            add_output(
                &db,
                &mut bel,
                &format!("O{i}"),
                0,
                &[&format!("GLOBAL.{i}")],
            );
        }
        tcls.bels.insert(bels::GB_ROOT, BelInfo::Bel(bel));
        db.tile_classes
            .insert(kind.tile_class_gb_root().into(), tcls);
    }

    db
}

pub struct MiscTileBuilder<'a> {
    pub chip: &'a Chip,
    pub tcls: TileClass,
    pub io: BTreeMap<SpecialIoKey, EdgeIoCoord>,
    pub fixed_cells: usize,
    pub cells: EntityVec<CellSlotId, CellCoord>,
    pub cells_map: HashMap<CellCoord, CellSlotId>,
}

impl<'a> MiscTileBuilder<'a> {
    pub fn new(chip: &'a Chip, slot: TileSlotId, fixed_cells: &[CellCoord]) -> Self {
        let mut cells = EntityVec::new();
        let mut cells_map = HashMap::new();
        for &crd in fixed_cells {
            let cell = cells.push(crd);
            cells_map.insert(crd, cell);
        }
        Self {
            chip,
            tcls: TileClass::new(slot, cells.len()),
            io: Default::default(),
            fixed_cells: fixed_cells.len(),
            cells,
            cells_map,
        }
    }

    pub fn get_cell(&mut self, crd: CellCoord) -> CellSlotId {
        match self.cells_map.entry(crd) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
            hash_map::Entry::Vacant(entry) => {
                let cell = self.tcls.cells.push(());
                self.cells.push(crd);
                entry.insert(cell);
                cell
            }
        }
    }

    pub fn add_bel(&mut self, slot: BelSlotId, pins: &BelPins) {
        let mut bel = Bel::default();
        for (pin, &wire) in &pins.ins {
            let cell = self.get_cell(wire.cell);
            bel.pins.insert(
                pin.clone(),
                BelPin::new_in(TileWireCoord {
                    cell,
                    wire: wire.slot,
                }),
            );
        }
        for (pin, iwires) in &pins.outs {
            let mut wires = BTreeSet::new();
            for &wire in iwires {
                let cell = self.get_cell(wire.cell);
                wires.insert(TileWireCoord {
                    cell,
                    wire: wire.slot,
                });
            }
            bel.pins.insert(pin.clone(), BelPin::new_out_multi(wires));
        }
        self.tcls.bels.insert(slot, BelInfo::Bel(bel));
    }

    pub fn finish(mut self) -> (TileClass, SpecialTile) {
        let mut cells_sorted = Vec::from_iter(self.cells.values().copied());
        let mut new_cells: EntityVec<CellSlotId, _> =
            EntityVec::from_iter(cells_sorted[..self.fixed_cells].iter().copied());
        let mut new_cells_map: HashMap<_, _> =
            HashMap::from_iter(new_cells.iter().map(|(k, &v)| (v, k)));
        cells_sorted.sort_by_key(|&crd| {
            // corners, then west/east edge, then south/north edge
            (
                if crd.col == self.chip.col_w() || crd.col == self.chip.col_e() {
                    if crd.row == self.chip.row_s() || crd.row == self.chip.row_n() {
                        0
                    } else {
                        1
                    }
                } else {
                    2
                },
                crd,
            )
        });
        for crd in cells_sorted {
            match new_cells_map.entry(crd) {
                hash_map::Entry::Occupied(_) => (),
                hash_map::Entry::Vacant(entry) => {
                    let cell = new_cells.push(crd);
                    entry.insert(cell);
                }
            }
        }
        for bel in self.tcls.bels.values_mut() {
            let BelInfo::Bel(bel) = bel else {
                unreachable!()
            };
            for pin in bel.pins.values_mut() {
                pin.wires = pin
                    .wires
                    .iter()
                    .map(|&twc| {
                        let new_cell = new_cells_map[&self.cells[twc.cell]];
                        TileWireCoord {
                            cell: new_cell,
                            wire: twc.wire,
                        }
                    })
                    .collect();
            }
        }
        (
            self.tcls,
            SpecialTile {
                io: self.io,
                cells: new_cells,
            },
        )
    }
}

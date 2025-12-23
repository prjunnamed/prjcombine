use std::collections::{BTreeMap, BTreeSet, HashMap, hash_map};

use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{
        BelInfo, BelPin, BelSlotId, CellSlotId, IntDb, LegacyBel, TileClass, TileSlotId,
        TileWireCoord, WireSlotId,
    },
    dir::Dir,
    grid::{CellCoord, EdgeIoCoord},
};
use prjcombine_siliconblue::{
    chip::{Chip, ChipKind, SpecialIoKey, SpecialTile},
    defs::{self, bslots as bels, tslots},
};

use crate::sites::BelPins;

fn add_input(bel: &mut LegacyBel, name: &str, cell: usize, wire: WireSlotId) {
    bel.pins.insert(
        name.into(),
        BelPin::new_in(TileWireCoord::new_idx(cell, wire)),
    );
}

fn add_output(bel: &mut LegacyBel, name: &str, cell: usize, wires: &[WireSlotId]) {
    bel.pins.insert(
        name.into(),
        BelPin::new_out_multi(wires.iter().map(|&wire| TileWireCoord::new_idx(cell, wire))),
    );
}

pub fn make_intdb(kind: ChipKind) -> IntDb {
    let mut db: IntDb = bincode::decode_from_slice(
        prjcombine_siliconblue::defs::INIT,
        bincode::config::standard(),
    )
    .unwrap()
    .0;

    {
        let mut tcls = TileClass::new(tslots::MAIN, 1);
        for i in 0..8 {
            let mut bel = LegacyBel::default();
            for (pin, wire) in [
                ("I0", defs::wires::IMUX_LC_I0[i]),
                ("I1", defs::wires::IMUX_LC_I1[i]),
                ("I2", defs::wires::IMUX_LC_I2[i]),
                ("I3", defs::wires::IMUX_LC_I3[i]),
                ("CLK", defs::wires::IMUX_CLK_OPTINV),
                ("RST", defs::wires::IMUX_RST),
                ("CE", defs::wires::IMUX_CE),
            ] {
                add_input(&mut bel, pin, 0, wire);
            }
            add_output(&mut bel, "O", 0, &[defs::wires::OUT_LC[i]]);
            tcls.bels.insert(bels::LC[i], BelInfo::Legacy(bel));
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
            let mut bel = LegacyBel::default();
            for (pin, wire) in [
                ("DOUT0", defs::wires::IMUX_IO_DOUT0[i]),
                ("DOUT1", defs::wires::IMUX_IO_DOUT1[i]),
                ("OE", defs::wires::IMUX_IO_OE[i]),
                ("ICLK", defs::wires::IMUX_IO_ICLK_OPTINV),
                ("OCLK", defs::wires::IMUX_IO_OCLK_OPTINV),
                ("CE", defs::wires::IMUX_CE),
            ] {
                add_input(&mut bel, pin, 0, wire);
            }
            add_output(
                &mut bel,
                "DIN0",
                0,
                &[defs::wires::OUT_LC[i * 2], defs::wires::OUT_LC[i * 2 + 4]],
            );
            add_output(
                &mut bel,
                "DIN1",
                0,
                &[
                    defs::wires::OUT_LC[i * 2 + 1],
                    defs::wires::OUT_LC[i * 2 + 5],
                ],
            );
            tcls.bels.insert(bels::IO[i], BelInfo::Legacy(bel));
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
        let mut bel = LegacyBel::default();
        let (tile_w, tile_r) = if ice40_bramv2 { (1, 0) } else { (0, 1) };
        add_input(&mut bel, "WCLK", tile_w, defs::wires::IMUX_CLK_OPTINV);
        add_input(&mut bel, "WE", tile_w, defs::wires::IMUX_RST);
        add_input(&mut bel, "WCLKE", tile_w, defs::wires::IMUX_CE);
        add_input(&mut bel, "RCLK", tile_r, defs::wires::IMUX_CLK_OPTINV);
        add_input(&mut bel, "RE", tile_r, defs::wires::IMUX_RST);
        add_input(&mut bel, "RCLKE", tile_r, defs::wires::IMUX_CE);
        let addr_bits = if kind.is_ice40() { 11 } else { 8 };
        for i in 0..addr_bits {
            let xi = if ice40_bramv2 { i ^ 7 } else { i };
            let lc = xi % 8;
            let wires = if xi >= 8 {
                &defs::wires::IMUX_LC_I2
            } else {
                &defs::wires::IMUX_LC_I0
            };
            add_input(&mut bel, &format!("WADDR{i}"), tile_w, wires[lc]);
            add_input(&mut bel, &format!("RADDR{i}"), tile_r, wires[lc]);
        }
        for i in 0..16 {
            let xi = if ice40_bramv2 { i ^ 15 } else { i };
            let tile = xi / 8;
            let lc = xi % 8;
            add_input(
                &mut bel,
                &format!("WDATA{i}"),
                tile,
                defs::wires::IMUX_LC_I1[lc],
            );
            add_input(
                &mut bel,
                &format!("MASK{i}"),
                tile,
                defs::wires::IMUX_LC_I3[lc],
            );
            add_output(
                &mut bel,
                &format!("RDATA{i}"),
                tile,
                &[defs::wires::OUT_LC[lc]],
            );
        }
        tcls.bels.insert(bels::BRAM, BelInfo::Legacy(bel));
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
        let mut bel = LegacyBel::default();
        add_input(&mut bel, "LATCH", 0, defs::wires::IMUX_IO_EXTRA);
        tcls.bels.insert(bels::IO_LATCH, BelInfo::Legacy(bel));
        db.tile_classes.insert("IO_LATCH".into(), tcls);
    }

    {
        let mut tcls = TileClass::new(tslots::BEL, 1);
        let mut bel = LegacyBel::default();
        add_input(&mut bel, "I", 0, defs::wires::IMUX_IO_EXTRA);
        tcls.bels.insert(bels::GB_FABRIC, BelInfo::Legacy(bel));
        db.tile_classes.insert("GB_FABRIC".into(), tcls);
    }

    {
        let mut tcls = TileClass::new(tslots::GB_ROOT, 1);
        let mut bel = LegacyBel::default();
        for i in 0..8 {
            add_output(&mut bel, &format!("O{i}"), 0, &[defs::wires::GLOBAL[i]]);
        }
        tcls.bels.insert(bels::GB_ROOT, BelInfo::Legacy(bel));
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
                let cell = self.tcls.cells.push(self.tcls.cells.next_id().to_string());
                self.cells.push(crd);
                entry.insert(cell);
                cell
            }
        }
    }

    pub fn add_bel(&mut self, slot: BelSlotId, pins: &BelPins) {
        let mut bel = LegacyBel::default();
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
        self.tcls.bels.insert(slot, BelInfo::Legacy(bel));
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
            let BelInfo::Legacy(bel) = bel else {
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

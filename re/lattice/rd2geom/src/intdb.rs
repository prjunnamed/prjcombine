use prjcombine_ecp::{
    bels,
    chip::ChipKind,
    expanded::{REGION_PCLK, REGION_SCLK, REGION_VSDCLK},
    tslots,
};
use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, BelSlotId, Buf, CellSlotId, ConnectorClass, ConnectorSlot,
        ConnectorSlotId, ConnectorWire, IntDb, SwitchBox, SwitchBoxItem, TileClass, TileSlotId,
        TileWireCoord, WireKind,
    },
    dir::{Dir, DirMap},
};
use unnamed_entity::EntityId;

struct TileClassBuilder<'db> {
    db: &'db mut IntDbBuilder,
    name: String,
    class: TileClass,
}

impl<'db> TileClassBuilder<'db> {
    fn bel<'tcls>(&'tcls mut self, slot: BelSlotId) -> BelBuilder<'db, 'tcls> {
        BelBuilder {
            tcls: self,
            slot,
            bel: Bel::default(),
        }
    }
}

impl Drop for TileClassBuilder<'_> {
    fn drop(&mut self) {
        self.db.db.tile_classes.insert(
            std::mem::take(&mut self.name),
            std::mem::replace(&mut self.class, TileClass::new(tslots::INT, 1)),
        );
    }
}

struct BelBuilder<'db, 'tcls> {
    tcls: &'tcls mut TileClassBuilder<'db>,
    slot: BelSlotId,
    bel: Bel,
}

impl BelBuilder<'_, '_> {
    fn add_input(&mut self, name: &str, cell: usize, wire: &str) {
        self.bel.pins.insert(
            name.into(),
            BelPin::new_in(TileWireCoord {
                cell: CellSlotId::from_idx(cell),
                wire: self.tcls.db.db.get_wire(wire),
            }),
        );
    }

    fn add_output(&mut self, name: &str, cell: usize, wire: &str) {
        self.bel.pins.insert(
            name.into(),
            BelPin::new_out(TileWireCoord {
                cell: CellSlotId::from_idx(cell),
                wire: self.tcls.db.db.get_wire(wire),
            }),
        );
    }
}

impl Drop for BelBuilder<'_, '_> {
    fn drop(&mut self) {
        self.tcls
            .class
            .bels
            .insert(self.slot, BelInfo::Bel(std::mem::take(&mut self.bel)));
    }
}

struct IntDbBuilder {
    conn_slots: DirMap<ConnectorSlotId>,
    passes: DirMap<ConnectorClass>,
    terms: DirMap<ConnectorClass>,
    conn_slot_sw: ConnectorSlotId,
    conn_slot_se: ConnectorSlotId,
    pass_sw: ConnectorClass,
    pass_se: ConnectorClass,
    kind: ChipKind,
    db: IntDb,
}

impl IntDbBuilder {
    fn tile_class(
        &mut self,
        name: impl Into<String>,
        slot: TileSlotId,
        num_cells: usize,
    ) -> TileClassBuilder<'_> {
        TileClassBuilder {
            db: self,
            name: name.into(),
            class: TileClass::new(slot, num_cells),
        }
    }

    fn fill_slots(&mut self) {
        assert_eq!(
            self.db.region_slots.insert("PCLK0".into()).0,
            REGION_PCLK[0]
        );
        assert_eq!(
            self.db.region_slots.insert("PCLK1".into()).0,
            REGION_PCLK[1]
        );
        assert_eq!(
            self.db.region_slots.insert("PCLK2".into()).0,
            REGION_PCLK[2]
        );
        assert_eq!(
            self.db.region_slots.insert("PCLK3".into()).0,
            REGION_PCLK[3]
        );
        assert_eq!(
            self.db.region_slots.insert("SCLK0".into()).0,
            REGION_SCLK[0]
        );
        assert_eq!(
            self.db.region_slots.insert("SCLK1".into()).0,
            REGION_SCLK[1]
        );
        assert_eq!(
            self.db.region_slots.insert("SCLK2".into()).0,
            REGION_SCLK[2]
        );
        assert_eq!(
            self.db.region_slots.insert("SCLK3".into()).0,
            REGION_SCLK[3]
        );
        assert_eq!(
            self.db.region_slots.insert("VSDCLK".into()).0,
            REGION_VSDCLK
        );
        self.db.init_slots(tslots::SLOTS, bels::SLOTS);

        let slot_w = self
            .db
            .conn_slots
            .insert(
                "W".into(),
                ConnectorSlot {
                    opposite: ConnectorSlotId::from_idx(0),
                },
            )
            .0;
        let slot_e = self
            .db
            .conn_slots
            .insert("E".into(), ConnectorSlot { opposite: slot_w })
            .0;
        let slot_s = self
            .db
            .conn_slots
            .insert(
                "S".into(),
                ConnectorSlot {
                    opposite: ConnectorSlotId::from_idx(0),
                },
            )
            .0;
        let slot_n = self
            .db
            .conn_slots
            .insert("N".into(), ConnectorSlot { opposite: slot_s })
            .0;
        self.db.conn_slots[slot_w].opposite = slot_e;
        self.db.conn_slots[slot_s].opposite = slot_n;

        self.conn_slot_sw = self
            .db
            .conn_slots
            .insert(
                "SW".into(),
                ConnectorSlot {
                    opposite: ConnectorSlotId::from_idx(0),
                },
            )
            .0;
        self.conn_slot_se = self
            .db
            .conn_slots
            .insert(
                "SE".into(),
                ConnectorSlot {
                    opposite: self.conn_slot_sw,
                },
            )
            .0;
        self.db.conn_slots[self.conn_slot_sw].opposite = self.conn_slot_se;

        self.pass_sw.slot = self.conn_slot_sw;
        self.pass_se.slot = self.conn_slot_se;

        self.conn_slots = DirMap::from_fn(|dir| match dir {
            Dir::W => slot_w,
            Dir::E => slot_e,
            Dir::S => slot_s,
            Dir::N => slot_n,
        });

        self.passes = DirMap::from_fn(|dir| ConnectorClass {
            slot: self.conn_slots[dir],
            wires: Default::default(),
        });

        self.terms = DirMap::from_fn(|dir| ConnectorClass {
            slot: self.conn_slots[dir],
            wires: Default::default(),
        });
    }

    fn fill_x0_wires(&mut self) {
        let num_x0 = if self.kind.has_x0_branch() { 4 } else { 2 };
        for dir in Dir::DIRS {
            for i in 0..num_x0 {
                if self.kind.has_x0_branch() {
                    let w0 = self
                        .db
                        .wires
                        .insert(format!("X0_{dir}{i}_0"), WireKind::MuxOut)
                        .0;
                    let w1 = self
                        .db
                        .wires
                        .insert(
                            format!("X0_{dir}{i}_1"),
                            WireKind::Branch(self.conn_slots[!dir]),
                        )
                        .0;
                    self.passes[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
                } else {
                    self.db
                        .wires
                        .insert(format!("X0_{dir}{i}"), WireKind::MuxOut);
                }
            }
        }
    }

    fn fill_x1_wires(&mut self) {
        if self.kind.has_x1_bi() {
            for i in [1, 4] {
                self.db.wires.insert(format!("X1_H{i}"), WireKind::MuxOut);
            }
            for i in [1, 4] {
                self.db.wires.insert(format!("X1_V{i}"), WireKind::MuxOut);
            }
        }

        let nums_x1 = if self.kind.has_x1_bi() {
            [0, 1, 4, 5].as_slice()
        } else {
            [0, 1].as_slice()
        };

        for dir in Dir::DIRS {
            for &i in nums_x1 {
                let w0 = self
                    .db
                    .wires
                    .insert(format!("X1_{dir}{i}_0"), WireKind::MuxOut)
                    .0;
                let w1 = self
                    .db
                    .wires
                    .insert(
                        format!("X1_{dir}{i}_1"),
                        WireKind::Branch(self.conn_slots[!dir]),
                    )
                    .0;
                self.passes[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
            }
        }
    }

    fn fill_x2_wires(&mut self) {
        for dir in Dir::DIRS {
            for i in 0..8 {
                let w0 = self
                    .db
                    .wires
                    .insert(format!("X2_{dir}{i}_0"), WireKind::MuxOut)
                    .0;
                let w1 = self
                    .db
                    .wires
                    .insert(
                        format!("X2_{dir}{i}_1"),
                        WireKind::Branch(self.conn_slots[!dir]),
                    )
                    .0;
                let w2 = self
                    .db
                    .wires
                    .insert(
                        format!("X2_{dir}{i}_2"),
                        WireKind::Branch(self.conn_slots[!dir]),
                    )
                    .0;
                self.passes[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
                self.passes[!dir].wires.insert(w2, ConnectorWire::Pass(w1));
            }
        }

        for dir in Dir::DIRS {
            for i in 0..8 {
                for seg in 0..2 {
                    let w0 = self.db.get_wire(&format!("X2_{dir}{i}_{seg}"));
                    let w1 = self.db.get_wire(&format!(
                        "X2_{ndir}{i}_{nseg}",
                        ndir = !dir,
                        nseg = seg + 1
                    ));
                    self.terms[dir].wires.insert(w1, ConnectorWire::Reflect(w0));
                }
            }
        }
    }

    fn fill_x6_wires(&mut self) {
        for dir in Dir::DIRS {
            for i in 0..4 {
                let mut w = self
                    .db
                    .wires
                    .insert(format!("X6_{dir}{i}_0"), WireKind::MuxOut)
                    .0;
                for j in 1..=6 {
                    let nw = self
                        .db
                        .wires
                        .insert(
                            format!("X6_{dir}{i}_{j}"),
                            WireKind::Branch(self.conn_slots[!dir]),
                        )
                        .0;
                    self.passes[!dir].wires.insert(nw, ConnectorWire::Pass(w));
                    w = nw;
                }
            }
        }

        for dir in Dir::DIRS {
            for i in 0..4 {
                for seg in 0..6 {
                    let w0 = self.db.get_wire(&format!("X6_{dir}{i}_{seg}"));
                    let w1 = self.db.get_wire(&format!(
                        "X6_{ndir}{i}_{nseg}",
                        ndir = !dir,
                        nseg = seg + 1
                    ));
                    self.terms[dir].wires.insert(w1, ConnectorWire::Reflect(w0));
                }
            }
        }
    }

    fn fill_pclk_wires(&mut self) {
        let num_clk = match self.kind {
            ChipKind::Ecp | ChipKind::Xp => 4,
            ChipKind::MachXo => 4,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => 8,
            ChipKind::Ecp3 | ChipKind::Ecp3A => 8,
        };
        for i in 0..num_clk {
            let region = if self.kind.has_distributed_sclk_ecp3() {
                REGION_PCLK[i % 4]
            } else {
                REGION_PCLK[0]
            };
            self.db
                .wires
                .insert(format!("PCLK{i}"), WireKind::Regional(region));
        }
    }

    fn fill_sclk_wires(&mut self) {
        let num_clk = match self.kind {
            ChipKind::Ecp | ChipKind::Xp => 4,
            ChipKind::MachXo => 4,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => 8,
            ChipKind::Ecp3 | ChipKind::Ecp3A => 8,
        };

        for i in 0..num_clk {
            let region = if self.kind.has_distributed_sclk() {
                REGION_SCLK[i % 4]
            } else {
                REGION_PCLK[0]
            };
            self.db
                .wires
                .insert(format!("SCLK{i}"), WireKind::Regional(region));
        }

        if self.kind.has_distributed_sclk() {
            let mut hsdclk = vec![];
            for i in 0..8 {
                let w = self
                    .db
                    .wires
                    .insert(format!("HSDCLK{i}"), WireKind::Branch(self.conn_slot_sw))
                    .0;
                hsdclk.push(w);
            }
            for i in 0..8 {
                let ni = if self.kind.has_distributed_sclk_ecp3() {
                    (i + 1) % 4 + i / 4 * 4
                } else {
                    (i + 3) % 4 + i / 4 * 4
                };
                self.pass_sw
                    .wires
                    .insert(hsdclk[ni], ConnectorWire::Pass(hsdclk[i]));
            }
            let num_vsdclk = if self.kind.has_distributed_sclk_ecp3() {
                8
            } else {
                2
            };
            for i in 0..num_vsdclk {
                let w = self
                    .db
                    .wires
                    .insert(format!("VSDCLK{i}"), WireKind::Regional(REGION_VSDCLK))
                    .0;
                let w_n = self
                    .db
                    .wires
                    .insert(
                        format!("VSDCLK{i}_N"),
                        WireKind::Branch(self.conn_slots[Dir::S]),
                    )
                    .0;
                self.passes[Dir::S]
                    .wires
                    .insert(w_n, ConnectorWire::Pass(w));
            }
        }
    }

    fn fill_imux_wires(&mut self) {
        for l in ['A', 'B', 'C', 'D', 'M'] {
            for i in 0..8 {
                self.db
                    .wires
                    .insert(format!("IMUX_{l}{i}"), WireKind::MuxOut);
            }
        }
        for l in ["CLK", "LSR", "CE"] {
            for i in 0..4 {
                self.db
                    .wires
                    .insert(format!("IMUX_{l}{i}"), WireKind::MuxOut);
            }
        }
    }

    fn fill_out_wires(&mut self) {
        for l in ["F", "Q", "OFX"] {
            for i in 0..8 {
                let w = self
                    .db
                    .wires
                    .insert(format!("OUT_{l}{i}"), WireKind::LogicOut)
                    .0;
                if self.kind.has_x0_branch() {
                    if (l == "OFX" && i == 3) || (l == "F" && matches!(i, 0..3)) {
                        let w_w = self
                            .db
                            .wires
                            .insert(
                                format!("OUT_{l}{i}_W"),
                                WireKind::Branch(self.conn_slots[Dir::E]),
                            )
                            .0;
                        self.passes[Dir::E]
                            .wires
                            .insert(w_w, ConnectorWire::Pass(w));
                    }
                    if l == "F" && matches!(i, 4..8) {
                        let w_e = self
                            .db
                            .wires
                            .insert(
                                format!("OUT_{l}{i}_E"),
                                WireKind::Branch(self.conn_slots[Dir::W]),
                            )
                            .0;
                        self.passes[Dir::W]
                            .wires
                            .insert(w_e, ConnectorWire::Pass(w));
                    }
                }
            }
        }
        for i in 0..12 {
            self.db
                .wires
                .insert(format!("OUT_TI{i}"), WireKind::LogicOut);
        }
    }

    fn fill_wires(&mut self) {
        self.db.wires.insert("TIE0".into(), WireKind::Tie0);
        self.db.wires.insert("TIE1".into(), WireKind::Tie1);

        self.fill_x0_wires();
        self.fill_x1_wires();
        self.fill_x2_wires();
        self.fill_x6_wires();
        self.fill_pclk_wires();
        self.fill_sclk_wires();
        self.fill_imux_wires();
        self.fill_out_wires();
    }

    fn fill_int_tiles(&mut self) {
        let int_tiles = match self.kind {
            ChipKind::MachXo => [
                "INT_PLC",
                "INT_SIO_W",
                "INT_SIO_W_CLK",
                "INT_SIO_E",
                "INT_SIO_E_CFG",
                "INT_SIO_S4",
                "INT_SIO_S6",
                "INT_SIO_N4",
                "INT_SIO_N6",
                "INT_SIO_XW",
            ]
            .as_slice(),
            ChipKind::Ecp | ChipKind::Xp => {
                ["INT_PLC", "INT_IO_WE", "INT_IO_SN", "INT_EBR", "INT_PLL"].as_slice()
            }
            ChipKind::Ecp2 | ChipKind::Xp2 => [
                "INT_PLC",
                "INT_IO_WE",
                "INT_IO_S",
                "INT_IO_N",
                "INT_EBR",
                "INT_EBR_IO",
            ]
            .as_slice(),
            ChipKind::Ecp2M => [
                "INT_PLC",
                "INT_IO_WE",
                "INT_IO_S",
                "INT_IO_N",
                "INT_EBR",
                "INT_EBR_IO",
                "INT_SERDES_N",
            ]
            .as_slice(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => [
                "INT_PLC",
                "INT_IO_WE",
                "INT_IO_S",
                "INT_IO_N",
                "INT_EBR",
                "INT_EBR_IO",
                "INT_EBR_SCLK",
                "INT_EBR_IO_SCLK",
            ]
            .as_slice(),
        };
        for &name in int_tiles {
            let mut tcls = self.tile_class(name, tslots::INT, 1);
            tcls.class
                .bels
                .insert(bels::INT, BelInfo::SwitchBox(SwitchBox::default()));
        }

        if self.kind == ChipKind::MachXo {
            self.tile_class("CIBTEST_SEL", tslots::BEL, 1)
                .bel(bels::CIBTEST_SEL);
        }
    }

    fn fill_pclk_tiles_ecp3(&mut self) {
        for i in 0..4 {
            {
                let mut tcls = self.tile_class(format!("PCLK{i}_SOURCE"), tslots::PCLK_SOURCE, 2);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            if i != 2 {
                let mut tcls = self.tile_class(format!("PCLK{i}_SOURCE_W"), tslots::PCLK_SOURCE, 2);
                tcls.bel(bels::PCLK_SOURCE_W);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            if i != 1 {
                let mut tcls = self.tile_class(format!("PCLK{i}_SOURCE_E"), tslots::PCLK_SOURCE, 2);
                tcls.bel(bels::PCLK_SOURCE_E);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }
        }
    }

    fn fill_pclk_tiles(&mut self) {
        match self.kind {
            ChipKind::Ecp
            | ChipKind::Xp
            | ChipKind::MachXo
            | ChipKind::Ecp2
            | ChipKind::Ecp2M
            | ChipKind::Xp2 => (),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_pclk_tiles_ecp3(),
        }
    }

    fn fill_sclk_tiles(&mut self) {
        if !self.kind.has_distributed_sclk() {
            return;
        }

        for i in 0..4 {
            let mut tiles = vec![(format!("SCLK{i}_SOURCE"), vec![(i, 0), (i + 4, 1)])];
            if self.kind.has_distributed_sclk_ecp3() {
                if i != 2 {
                    tiles.push((
                        format!("SCLK{i}_SOURCE_W"),
                        vec![
                            (i, 0),
                            (i + 4, 1),
                            ((i + 3) % 4, 6),
                            ((i + 3) % 4 + 4, 7),
                            ((i + 2) % 4, 4),
                            ((i + 2) % 4 + 4, 5),
                        ],
                    ));
                }
                if i != 1 {
                    tiles.push((
                        format!("SCLK{i}_SOURCE_E"),
                        vec![(i, 0), (i + 4, 1), ((i + 1) % 4, 2), ((i + 1) % 4 + 4, 3)],
                    ));
                }
            }

            for (name, clocks) in tiles {
                let mut sb = SwitchBox::default();
                for (si, vsdi) in clocks {
                    sb.items.push(SwitchBoxItem::PermaBuf(Buf {
                        dst: TileWireCoord {
                            cell: CellSlotId::from_idx(0),
                            wire: self.db.get_wire(&format!("SCLK{si}")),
                        },
                        src: TileWireCoord {
                            cell: CellSlotId::from_idx(0),
                            wire: self.db.get_wire(&format!("VSDCLK{vsdi}")),
                        }
                        .pos(),
                    }));
                }
                let mut tcls = self.tile_class(name, tslots::SCLK_SOURCE, 1);
                tcls.class
                    .bels
                    .insert(bels::SCLK_SOURCE, BelInfo::SwitchBox(sb));
            }
        }

        for name in ["HSDCLK_SPLITTER", "HSDCLK_ROOT"] {
            let mut sb = SwitchBox::default();
            for i in 0..2 {
                let wire = self.db.get_wire(&format!("HSDCLK{ii}", ii = i * 4));
                for j in 0..4 {
                    let cell_w = CellSlotId::from_idx(j);
                    let cell_e = CellSlotId::from_idx(4 + j);
                    sb.items.push(SwitchBoxItem::ProgBuf(Buf {
                        dst: TileWireCoord { cell: cell_w, wire },
                        src: TileWireCoord { cell: cell_e, wire }.pos(),
                    }));
                    sb.items.push(SwitchBoxItem::ProgBuf(Buf {
                        dst: TileWireCoord { cell: cell_e, wire },
                        src: TileWireCoord { cell: cell_w, wire }.pos(),
                    }));
                }
            }
            let mut tcls = self.tile_class(name, tslots::HSDCLK_SPLITTER, 8);
            tcls.class
                .bels
                .insert(bels::HSDCLK_SPLITTER, BelInfo::SwitchBox(sb));
            if name == "HSDCLK_ROOT" {
                let mut bel = tcls.bel(bels::HSDCLK_ROOT);
                for i in 0..8 {
                    bel.add_output(
                        &format!("OUT_W{i}"),
                        i % 4,
                        &format!("HSDCLK{ii}", ii = i / 4 * 4),
                    );
                    bel.add_output(
                        &format!("OUT_E{i}"),
                        4 + i % 4,
                        &format!("HSDCLK{ii}", ii = i / 4 * 4),
                    );
                }
            }
        }
    }

    fn fill_clk_tiles_machxo(&mut self) {
        for name in ["CLK_ROOT_0PLL", "CLK_ROOT_1PLL", "CLK_ROOT_2PLL"] {
            self.tile_class(name, tslots::CLK, 6).bel(bels::CLK_ROOT);
        }
    }

    fn fill_clk_tiles_ecp(&mut self) {
        let tile_classes = match self.kind {
            ChipKind::Ecp => [("CLK_ROOT_2PLL", 22), ("CLK_ROOT_4PLL", 32)].as_slice(),
            ChipKind::Xp => [
                ("CLK_ROOT_2PLL_A", 28),
                ("CLK_ROOT_2PLL_B", 28),
                ("CLK_ROOT_4PLL", 32),
            ]
            .as_slice(),
            ChipKind::Ecp2 => [
                ("CLK_ROOT_2PLL", 30),
                ("CLK_ROOT_4PLL", 30),
                ("CLK_ROOT_6PLL", 30),
            ]
            .as_slice(),
            ChipKind::Ecp2M => [("CLK_ROOT_8PLL", 30)].as_slice(),
            ChipKind::Xp2 => [("CLK_ROOT_2PLL", 30), ("CLK_ROOT_4PLL", 30)].as_slice(),
            _ => unreachable!(),
        };
        for &(name, num_cells) in tile_classes {
            let mut tcls = self.tile_class(name, tslots::CLK, num_cells);
            tcls.bel(bels::CLK_ROOT);
            for i in 0..2 {
                tcls.bel(bels::DCS_SW[i]);
                tcls.bel(bels::DCS_SE[i]);
                tcls.bel(bels::DCS_NW[i]);
                tcls.bel(bels::DCS_NE[i]);
            }
        }
    }

    fn fill_clk_tiles_ecp3(&mut self) {
        let mut tcls = self.tile_class("CLK_ROOT", tslots::CLK, 30);
        tcls.bel(bels::CLK_ROOT);
        for i in 0..6 {
            tcls.bel(bels::DCC_SW[i]);
            tcls.bel(bels::DCC_SE[i]);
            tcls.bel(bels::DCC_NW[i]);
            tcls.bel(bels::DCC_NE[i]);
        }
        for i in 0..2 {
            tcls.bel(bels::DCS_SW[i]);
            tcls.bel(bels::DCS_SE[i]);
            tcls.bel(bels::DCS_NW[i]);
            tcls.bel(bels::DCS_NE[i]);
        }
    }

    fn fill_clk_tiles(&mut self) {
        match self.kind {
            ChipKind::Ecp | ChipKind::Xp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                self.fill_clk_tiles_ecp()
            }
            ChipKind::MachXo => self.fill_clk_tiles_machxo(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_clk_tiles_ecp3(),
        }
    }

    fn fill_plc_tiles(&mut self) {
        let kind = self.kind;
        for name in ["PLC", "FPLC"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 1);
            for i in 0..4 {
                let mut bel = tcls.bel(bels::SLICE[i]);
                let i0 = 2 * i;
                let i1 = 2 * i + 1;
                for l in ['A', 'B', 'C', 'D', 'M'] {
                    bel.add_input(&format!("{l}0"), 0, &format!("IMUX_{l}{i0}"));
                    bel.add_input(&format!("{l}1"), 0, &format!("IMUX_{l}{i1}"));
                }
                if i < 3 || !kind.has_ecp2_plc() {
                    bel.add_input("CLK", 0, &format!("IMUX_CLK{i}"));
                    bel.add_input("LSR", 0, &format!("IMUX_LSR{i}"));
                    bel.add_input("CE", 0, &format!("IMUX_CE{i}"));
                    bel.add_output("Q0", 0, &format!("OUT_Q{i0}"));
                    bel.add_output("Q1", 0, &format!("OUT_Q{i1}"));
                }
                for l in ["F", "OFX"] {
                    bel.add_output(&format!("{l}0"), 0, &format!("OUT_{l}{i0}"));
                    bel.add_output(&format!("{l}1"), 0, &format!("OUT_{l}{i1}"));
                }
                if i == 3 && kind.has_x0_branch() {
                    bel.add_input("FXB", 0, "OUT_OFX3_W");
                }
            }
        }
    }

    fn fill_ebr_tiles(&mut self) {
        let num_cells = match self.kind {
            ChipKind::Ecp | ChipKind::Xp => 2,
            ChipKind::MachXo => 4,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                3
            }
        };
        self.tile_class("EBR", tslots::BEL, num_cells)
            .bel(bels::EBR0);
    }

    fn fill_dsp_tiles(&mut self) {
        let (num_cells, num_bels) = match self.kind {
            ChipKind::Ecp => (8, 1),
            ChipKind::Xp | ChipKind::MachXo => return,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => (9, 1),
            ChipKind::Ecp3 | ChipKind::Ecp3A => (9, 2),
        };
        let mut tcls = self.tile_class("DSP", tslots::BEL, num_cells);
        for i in 0..num_bels {
            tcls.bel(bels::DSP[i]);
        }
    }

    fn fill_config_tiles_ecp(&mut self) {
        for (name, num_cells) in [("CONFIG_S", 4), ("CONFIG_L", 5)] {
            let mut tcls = self.tile_class(name, tslots::BEL, num_cells);
            tcls.bel(bels::START);
            tcls.bel(bels::OSC);
            tcls.bel(bels::JTAG);
            // RDBK exists as stub?
            tcls.bel(bels::GSR);
        }
    }

    fn fill_config_tiles_xp(&mut self) {
        let mut tcls = self.tile_class("CONFIG", tslots::BEL, 1);
        tcls.bel(bels::START);
        tcls.bel(bels::JTAG);
        tcls.bel(bels::GSR);
        // OSC and RDBK exist as stubs?
    }

    fn fill_config_tiles_machxo(&mut self) {
        for name in ["OSC", "OSC_X"] {
            self.tile_class(name, tslots::BEL, 1).bel(bels::OSC);
        }

        let mut tcls = self.tile_class("CONFIG", tslots::BEL, 5);
        tcls.bel(bels::GSR);
        tcls.bel(bels::JTAG);
    }

    fn fill_config_tiles_ecp2(&mut self) {
        let mut tcls = self.tile_class("CONFIG", tslots::BEL, 3);
        tcls.bel(bels::START);
        tcls.bel(bels::OSC);
        tcls.bel(bels::JTAG);
        tcls.bel(bels::GSR);
        tcls.bel(bels::SED);
        tcls.bel(bels::SPIM);
    }

    fn fill_config_tiles_xp2(&mut self) {
        {
            let mut tcls = self.tile_class("CONFIG", tslots::BEL, 1);
            tcls.bel(bels::JTAG);
            tcls.bel(bels::SED);
        }

        self.tile_class("OSC", tslots::BEL, 1).bel(bels::OSC);
    }

    fn fill_config_tiles_ecp3(&mut self) {
        {
            let mut tcls = self.tile_class("CONFIG", tslots::BEL, 14);
            tcls.bel(bels::START);
            tcls.bel(bels::JTAG);
            tcls.bel(bels::OSC);
            tcls.bel(bels::GSR);
            tcls.bel(bels::SED);
            tcls.bel(bels::AMBOOT);
            tcls.bel(bels::PERREG);
        }

        for (name, num_cells) in [
            ("TEST_SW", 3),
            ("TEST_SE", 3),
            ("TEST_NW", 2),
            ("TEST_NE", 2),
        ] {
            let mut tcls = self.tile_class(name, tslots::BEL, num_cells);
            tcls.bel(bels::TESTIN);
            tcls.bel(bels::TESTOUT);
            if name == "TEST_SE" {
                tcls.bel(bels::DTS);
            }
        }
    }

    fn fill_config_tiles(&mut self) {
        match self.kind {
            ChipKind::Ecp => self.fill_config_tiles_ecp(),
            ChipKind::Xp => self.fill_config_tiles_xp(),
            ChipKind::MachXo => self.fill_config_tiles_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.fill_config_tiles_ecp2(),
            ChipKind::Xp2 => self.fill_config_tiles_xp2(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_config_tiles_ecp3(),
        }
    }

    fn fill_io_tiles_ecp(&mut self) {
        for name in ["IO_W", "IO_E", "IO_S", "IO_N"] {
            let mut tcls = self.tile_class(name, tslots::IO, 1);
            tcls.bel(bels::IO0);
            tcls.bel(bels::IO1);
        }
        for name in ["DQS_W", "DQS_E", "DQS_S", "DQS_N"] {
            if matches!(self.kind, ChipKind::Ecp2 | ChipKind::Ecp2M) && name == "DQS_N" {
                continue;
            }
            self.tile_class(name, tslots::BEL, 1).bel(bels::DQS);
        }
        if matches!(self.kind, ChipKind::Ecp | ChipKind::Xp) {
            for name in ["DQSDLL_S", "DQSDLL_N"] {
                self.tile_class(name, tslots::BEL, 1).bel(bels::DQSDLL);
            }
        }
        if self.kind == ChipKind::Xp2 {
            for name in ["CLK_W", "CLK_E"] {
                let mut tcls =
                    self.tile_class(name, tslots::BEL, if name == "CLK_E" { 2 } else { 1 });
                tcls.bel(bels::DQSDLL);
                tcls.bel(bels::CLKDIV);
                if name == "CLK_E" {
                    tcls.bel(bels::SSPI);
                    tcls.bel(bels::STF);
                    tcls.bel(bels::WAKEUP);
                    tcls.bel(bels::START);
                    tcls.bel(bels::GSR);
                }
            }
        }
        if matches!(self.kind, ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2) {
            for (name, num_cells) in [
                ("ECLK_ROOT_W", 1),
                ("ECLK_ROOT_E", 1),
                ("ECLK_ROOT_S", 2),
                ("ECLK_ROOT_N", 2),
            ] {
                self.tile_class(name, tslots::CLK, num_cells)
                    .bel(bels::ECLK_ROOT);
            }
            {
                let mut tcls = self.tile_class("ECLK_TAP", tslots::ECLK_TAP, 1);
                let mut bel = tcls.bel(bels::ECLK_TAP);
                bel.add_output("ECLK0", 0, "OUT_F6");
                bel.add_output("ECLK1", 0, "OUT_F7");
            }
        }
    }

    fn fill_io_tiles_machxo(&mut self) {
        for (name, num) in [
            ("SIO_W2", 4),
            ("SIO_W4", 4),
            ("SIO_XW2", 4),
            ("SIO_XW4", 4),
            ("SIO_E2", 4),
            ("SIO_E4", 4),
            ("SIO_S4", 4),
            ("SIO_S6", 6),
            ("SIO_N4", 4),
            ("SIO_N6", 6),
        ] {
            let mut tcls = self.tile_class(name, tslots::IO, 1);
            for i in 0..num {
                tcls.bel(bels::IO[i]);
            }
        }
    }

    fn fill_io_tiles_ecp3(&mut self) {
        for name in [
            "IO_W",
            "IO_DQS_W",
            "IO_DQS_DUMMY_W",
            "IO_E",
            "IO_DQS_E",
            "XSIO_E",
            "XSIO_S",
            "SIO_N",
            "SIO_A_N",
            "SIO_DQS_N",
            "XSIO_N",
        ] {
            let mut tcls = self.tile_class(name, tslots::IO, 3);
            for i in 0..4 {
                tcls.bel(bels::IO[i]);
            }
            if name.contains("DQS") {}
        }
        for name in ["DQS_N", "DQS_W", "DQS_E", "DQS_A_W", "DQS_A_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, if name == "DQS_N" { 6 } else { 3 });
            tcls.bel(bels::DQS);
            if name.starts_with("DQS_A") {
                tcls.bel(bels::DQSTEST);
            }
        }
        for name in ["IO_PLL_W", "IO_PLL_E"] {
            let mut tcls = self.tile_class(name, tslots::IO, 2);
            for i in 0..4 {
                tcls.bel(bels::IO[i]);
            }
        }
        for name in ["ECLK_ROOT_W", "ECLK_ROOT_E", "ECLK_ROOT_N"] {
            let mut tcls = self.tile_class(name, tslots::CLK, 1);
            tcls.bel(bels::ECLKSYNC0);
            tcls.bel(bels::ECLKSYNC1);
        }
        {
            let mut tcls = self.tile_class("ECLK_TAP", tslots::ECLK_TAP, 1);
            let mut bel = tcls.bel(bels::ECLK_TAP);
            bel.add_output("ECLK0", 0, "OUT_F6");
            bel.add_output("ECLK1", 0, "OUT_F7");
        }
    }

    fn fill_io_tiles(&mut self) {
        match self.kind {
            ChipKind::Ecp | ChipKind::Xp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                self.fill_io_tiles_ecp()
            }
            ChipKind::MachXo => self.fill_io_tiles_machxo(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_io_tiles_ecp3(),
        }
    }

    fn fill_serdes_tiles_ecp2(&mut self) {
        for name in ["SERDES_S", "SERDES_N"] {
            self.tile_class(name, tslots::BEL, 27).bel(bels::SERDES);
        }
    }

    fn fill_serdes_tiles_ecp3(&mut self) {
        self.tile_class("SERDES", tslots::BEL, 36).bel(bels::SERDES);
    }

    fn fill_serdes_tiles(&mut self) {
        match self.kind {
            ChipKind::Ecp | ChipKind::Xp | ChipKind::Ecp2 | ChipKind::Xp2 | ChipKind::MachXo => (),
            ChipKind::Ecp2M => self.fill_serdes_tiles_ecp2(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_serdes_tiles_ecp3(),
        }
    }

    fn fill_pll_tiles_ecp(&mut self) {
        for name in ["PLL_W", "PLL_E"] {
            self.tile_class(name, tslots::BEL, 1).bel(bels::PLL);
        }
    }

    fn fill_pll_tiles_machxo(&mut self) {
        for name in ["PLL_S", "PLL_N"] {
            self.tile_class(name, tslots::BEL, 1).bel(bels::PLL);
        }
    }

    fn fill_pll_tiles_ecp2(&mut self) {
        for name in ["SPLL_W", "SPLL_E"] {
            self.tile_class(name, tslots::BEL, 2).bel(bels::SPLL);
        }
        for name in ["PLL_W", "PLL_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 4);
            tcls.bel(bels::PLL);
            tcls.bel(bels::DLL);
            tcls.bel(bels::DLLDEL);
            tcls.bel(bels::CLKDIV);
            tcls.bel(bels::ECLK_ALT_ROOT);
            tcls.bel(bels::DQSDLL);
        }
    }

    fn fill_pll_tiles_xp2(&mut self) {
        for name in ["PLL_S", "PLL_N"] {
            self.tile_class(name, tslots::BEL, 2).bel(bels::PLL);
        }
    }

    fn fill_pll_tiles_ecp3(&mut self) {
        for name in ["PLL_DLL_W", "PLL_DLL_E", "PLL_DLL_A_W", "PLL_DLL_A_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 18);
            tcls.bel(bels::PLL);
            tcls.bel(bels::DLL);
            tcls.bel(bels::DLLDEL);
            tcls.bel(bels::DQSDLL);
            tcls.bel(bels::DQSDLLTEST);
            tcls.bel(bels::ECLK_ALT_ROOT);
            tcls.bel(bels::CLKDIV);
        }
        for name in ["PLL_W", "PLL_E", "PLL_A_W", "PLL_A_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 13);
            tcls.bel(bels::PLL);
        }
    }

    fn fill_pll_tiles(&mut self) {
        match self.kind {
            ChipKind::Ecp | ChipKind::Xp => self.fill_pll_tiles_ecp(),
            ChipKind::MachXo => self.fill_pll_tiles_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.fill_pll_tiles_ecp2(),
            ChipKind::Xp2 => self.fill_pll_tiles_xp2(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_pll_tiles_ecp3(),
        }
    }

    fn finish(mut self) -> IntDb {
        for (dir, pass) in self.passes {
            self.db.conn_classes.insert(format!("PASS_{dir}"), pass);
        }

        for (dir, pass) in self.terms {
            self.db.conn_classes.insert(format!("TERM_{dir}"), pass);
        }

        self.db.conn_classes.insert("PASS_SW".into(), self.pass_sw);
        self.db.conn_classes.insert("PASS_SE".into(), self.pass_se);

        self.db
    }

    fn build(mut self) -> IntDb {
        self.fill_slots();
        self.fill_wires();
        self.fill_int_tiles();
        self.fill_pclk_tiles();
        self.fill_sclk_tiles();
        self.fill_clk_tiles();
        self.fill_plc_tiles();
        self.fill_ebr_tiles();
        self.fill_dsp_tiles();
        self.fill_config_tiles();
        self.fill_io_tiles();
        self.fill_serdes_tiles();
        self.fill_pll_tiles();
        self.finish()
    }
}

pub fn init_intdb(kind: ChipKind) -> IntDb {
    let builder = IntDbBuilder {
        kind,
        db: IntDb::default(),
        // placeholders.
        conn_slot_sw: ConnectorSlotId::from_idx(0),
        conn_slot_se: ConnectorSlotId::from_idx(0),
        conn_slots: DirMap::from_fn(|_| ConnectorSlotId::from_idx(0)),
        passes: DirMap::from_fn(|_| ConnectorClass {
            slot: ConnectorSlotId::from_idx(0),
            wires: Default::default(),
        }),
        pass_sw: ConnectorClass {
            slot: ConnectorSlotId::from_idx(0),
            wires: Default::default(),
        },
        pass_se: ConnectorClass {
            slot: ConnectorSlotId::from_idx(0),
            wires: Default::default(),
        },
        terms: DirMap::from_fn(|_| ConnectorClass {
            slot: ConnectorSlotId::from_idx(0),
            wires: Default::default(),
        }),
    };
    builder.build()
}

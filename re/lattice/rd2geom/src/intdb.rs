use prjcombine_ecp::{bels, chip::ChipKind, cslots, regions, tslots};
use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, BelSlotId, Buf, ConnectorClass, ConnectorSlotId, ConnectorWire,
        IntDb, SwitchBox, SwitchBoxItem, TileClass, TileSlotId, TileWireCoord, WireKind,
    },
    dir::{Dir, DirH, DirMap},
};

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
            BelPin::new_in(TileWireCoord::new_idx(cell, self.tcls.db.db.get_wire(wire))),
        );
    }

    fn add_output(&mut self, name: &str, cell: usize, wire: &str) {
        self.bel.pins.insert(
            name.into(),
            BelPin::new_out(TileWireCoord::new_idx(cell, self.tcls.db.db.get_wire(wire))),
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

    fn fill_x0_wires(&mut self) {
        let num_x0 = match self.kind {
            ChipKind::Ecp | ChipKind::Xp | ChipKind::MachXo | ChipKind::MachXo2(_) => 4,
            _ => 2,
        };
        for dir in Dir::DIRS {
            for i in 0..num_x0 {
                self.db
                    .wires
                    .insert(format!("X0_{dir}{i}"), WireKind::MuxOut);
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

        let nums_x1 = if matches!(self.kind, ChipKind::Scm | ChipKind::Ecp4) {
            [0, 1, 4, 5, 6, 7].as_slice()
        } else if self.kind.has_x1_bi() {
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

        if matches!(self.kind, ChipKind::Ecp5 | ChipKind::Crosslink) {
            for dir in Dir::DIRS {
                for i in nums_x1 {
                    let w0 = self.db.get_wire(&format!("X1_{dir}{i}_0"));
                    let w1 = self.db.get_wire(&format!("X1_{ndir}{i}_1", ndir = !dir));
                    self.terms[dir].wires.insert(w1, ConnectorWire::Reflect(w0));
                }
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
        let num_x6 = match self.kind {
            ChipKind::Scm | ChipKind::Ecp4 => 8,
            _ => 4,
        };
        for dir in Dir::DIRS {
            for i in 0..num_x6 {
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
            for i in 0..num_x6 {
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
            ChipKind::Scm => 12,
            ChipKind::Ecp | ChipKind::Xp => 4,
            ChipKind::MachXo => 4,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => 8,
            ChipKind::Ecp3 | ChipKind::Ecp3A => 8,
            ChipKind::MachXo2(_) => 8,
            ChipKind::Ecp4 => 20,
            ChipKind::Ecp5 => 16,
            ChipKind::Crosslink => 8,
        };
        for i in 0..num_clk {
            let region = if self.kind.has_distributed_sclk_ecp3() {
                regions::PCLK[i % 4]
            } else {
                regions::PCLK0
            };
            self.db
                .wires
                .insert(format!("PCLK{i}"), WireKind::Regional(region));
        }
    }

    fn fill_sclk_wires(&mut self) {
        let num_clk = match self.kind {
            ChipKind::Scm => 0,
            ChipKind::Ecp | ChipKind::Xp => 4,
            ChipKind::MachXo => 4,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => 8,
            ChipKind::Ecp3 | ChipKind::Ecp3A => 8,
            ChipKind::MachXo2(_) => 8,
            ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink => 0,
        };

        for i in 0..num_clk {
            let region = if self.kind.has_distributed_sclk() {
                regions::SCLK[i % 4]
            } else {
                regions::PCLK0
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
                    .insert(format!("HSDCLK{i}"), WireKind::Branch(cslots::SW))
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
                    .insert(format!("VSDCLK{i}"), WireKind::Regional(regions::VSDCLK))
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

    fn fill_sclk_wires_scm(&mut self) {
        if self.kind != ChipKind::Scm {
            return;
        }
        for i in 0..2 {
            let w = self.db.wires.insert(format!("SCLK{i}"), WireKind::MuxOut).0;
            for dir in [Dir::W, Dir::E] {
                let w1 = self
                    .db
                    .wires
                    .insert(
                        format!("SCLK{i}_{dir}"),
                        WireKind::Branch(self.conn_slots[!dir]),
                    )
                    .0;
                self.passes[!dir].wires.insert(w1, ConnectorWire::Pass(w));
            }
        }
        let mut w = self.db.wires.insert("VSDCLK0".into(), WireKind::MultiOut).0;
        for seg in 1..=6 {
            let nw = self
                .db
                .wires
                .insert(format!("VSDCLK{seg}"), WireKind::MultiBranch(cslots::S))
                .0;
            self.passes[Dir::S].wires.insert(nw, ConnectorWire::Pass(w));
            w = nw;
        }
        let mut hsdclk = vec![];
        for i in 0..=6 {
            hsdclk.push(
                self.db
                    .wires
                    .insert(
                        format!("HSDCLK{i}"),
                        match i.cmp(&3) {
                            std::cmp::Ordering::Less => WireKind::MultiBranch(cslots::E),
                            std::cmp::Ordering::Equal => WireKind::MultiOut,
                            std::cmp::Ordering::Greater => WireKind::MultiBranch(cslots::W),
                        },
                    )
                    .0,
            );
        }
        for i in 0..=2 {
            self.passes[Dir::E]
                .wires
                .insert(hsdclk[i], ConnectorWire::Pass(hsdclk[i + 1]));
            self.terms[Dir::E]
                .wires
                .insert(hsdclk[i], ConnectorWire::Reflect(hsdclk[6 - (i + 1)]));
        }
        for i in 4..=6 {
            self.passes[Dir::W]
                .wires
                .insert(hsdclk[i], ConnectorWire::Pass(hsdclk[i - 1]));
            self.terms[Dir::W]
                .wires
                .insert(hsdclk[i], ConnectorWire::Reflect(hsdclk[6 - (i - 1)]));
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
        let num_clk = if matches!(
            self.kind,
            ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink
        ) {
            2
        } else {
            4
        };
        for l in ["CLK", "LSR"] {
            for i in 0..num_clk {
                self.db
                    .wires
                    .insert(format!("IMUX_{l}{i}"), WireKind::MuxOut);
            }
        }
        if self.kind == ChipKind::Ecp4 {
            for i in 0..2 {
                self.db
                    .wires
                    .insert(format!("IMUX_CLK{i}_DELAY"), WireKind::MuxOut);
            }
        }
        if matches!(
            self.kind,
            ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink
        ) {
            for i in 0..4 {
                self.db
                    .wires
                    .insert(format!("IMUX_MUXCLK{i}"), WireKind::MuxOut);
            }
            for i in 0..4 {
                self.db
                    .wires
                    .insert(format!("IMUX_MUXLSR{i}"), WireKind::MuxOut);
            }
        }
        for i in 0..4 {
            self.db
                .wires
                .insert(format!("IMUX_CE{i}"), WireKind::MuxOut);
        }
    }

    fn fill_out_wires(&mut self) {
        for l in ["F", "Q", "OFX"] {
            if matches!(self.kind, ChipKind::Ecp5 | ChipKind::Crosslink) && l == "OFX" {
                continue;
            }
            for i in 0..8 {
                let w = self
                    .db
                    .wires
                    .insert(format!("OUT_{l}{i}"), WireKind::LogicOut)
                    .0;
                if (l == "OFX" && i == 3 && self.kind.has_out_ofx_branch())
                    || (l == "F"
                        && i == 3
                        && matches!(self.kind, ChipKind::Ecp5 | ChipKind::Crosslink))
                    || (l == "F" && matches!(i, 0..3) && self.kind.has_out_f_branch())
                {
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
                if l == "F" && matches!(i, 4..8) && self.kind.has_out_f_branch() {
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
        for i in 0..12 {
            self.db
                .wires
                .insert(format!("OUT_TI{i}"), WireKind::LogicOut);
        }
    }

    fn fill_ebr_wires_scm(&mut self) {
        if self.kind != ChipKind::Scm {
            return;
        }
        for pin in ["DIA", "DIB"] {
            for i in 0..18 {
                self.db
                    .wires
                    .insert(format!("IMUX_EBR_{pin}{i}"), WireKind::MuxOut);
            }
        }
        for pin in ["ADA", "ADB"] {
            for i in 3..14 {
                self.db
                    .wires
                    .insert(format!("IMUX_EBR_{pin}{i}"), WireKind::MuxOut);
            }
        }
        for pin in ["CLKA", "CLKB", "RSTA", "RSTB", "CEA", "CEB", "WEA", "WEB"] {
            self.db
                .wires
                .insert(format!("IMUX_EBR_{pin}"), WireKind::MuxOut);
        }
        for pin in ["DOA", "DOB"] {
            for i in 0..18 {
                self.db
                    .wires
                    .insert(format!("OUT_EBR_{pin}{i}"), WireKind::LogicOut);
            }
        }
        for pin in ["AE", "FF", "AF", "EF"] {
            self.db
                .wires
                .insert(format!("OUT_EBR_{pin}"), WireKind::LogicOut);
        }
        for i in 0..64 {
            self.db
                .wires
                .insert(format!("EBR_W{i}_0"), WireKind::MuxOut);
            self.db
                .wires
                .insert(format!("EBR_W{i}_1"), WireKind::Branch(cslots::EBR_E));
        }
        for i in 0..64 {
            self.db
                .wires
                .insert(format!("EBR_E{i}_0"), WireKind::MuxOut);
            self.db
                .wires
                .insert(format!("EBR_E{i}_1"), WireKind::Branch(cslots::EBR_W));
        }
        let mut conn_w_w = ConnectorClass::new(cslots::EBR_W);
        let mut conn_m_w = ConnectorClass::new(cslots::EBR_W);
        let mut conn_e_w = ConnectorClass::new(cslots::EBR_W);
        let mut conn_w_e = ConnectorClass::new(cslots::EBR_E);
        let mut conn_m_e = ConnectorClass::new(cslots::EBR_E);
        let mut conn_e_e = ConnectorClass::new(cslots::EBR_E);
        for i in 0..64 {
            let w1 = self.db.get_wire(&format!("EBR_E{i}_1"));
            let w0 = self.db.get_wire(&format!("EBR_E{i}_0"));
            conn_w_w.wires.insert(w1, ConnectorWire::Pass(w0));
            if i < 48 {
                conn_e_w.wires.insert(w1, ConnectorWire::Pass(w0));
            }
            let w1 = self.db.get_wire(&format!("EBR_W{i}_1"));
            let w0 = self.db.get_wire(&format!("EBR_W{i}_0"));
            conn_e_e.wires.insert(w1, ConnectorWire::Pass(w0));
            if i < 48 {
                conn_w_e.wires.insert(w1, ConnectorWire::Pass(w0));
            }
        }
        for i in 0..48 {
            let ii = if i < 24 { i } else { i + 16 };
            let w1 = self.db.get_wire(&format!("EBR_E{i}_1"));
            let w0 = self.db.get_wire(&format!("EBR_E{ii}_0"));
            conn_m_w.wires.insert(w1, ConnectorWire::Pass(w0));
            let w1 = self.db.get_wire(&format!("EBR_W{i}_1"));
            let w0 = self.db.get_wire(&format!("EBR_W{ii}_0"));
            conn_m_e.wires.insert(w1, ConnectorWire::Pass(w0));
        }
        self.db.conn_classes.insert("PASS_EBR_W_W".into(), conn_w_w);
        self.db.conn_classes.insert("PASS_EBR_M_W".into(), conn_m_w);
        self.db.conn_classes.insert("PASS_EBR_E_W".into(), conn_e_w);
        self.db.conn_classes.insert("PASS_EBR_W_E".into(), conn_w_e);
        self.db.conn_classes.insert("PASS_EBR_M_E".into(), conn_m_e);
        self.db.conn_classes.insert("PASS_EBR_E_E".into(), conn_e_e);
    }

    fn fill_io_wires_scm(&mut self) {
        if self.kind != ChipKind::Scm {
            return;
        }
        for i in 0..32 {
            self.db
                .wires
                .insert(format!("IMUX_IO{i}"), WireKind::MuxOut);
        }
        for i in 0..24 {
            self.db
                .wires
                .insert(format!("OUT_IO{i}"), WireKind::LogicOut);
        }
        for dir in [Dir::S, Dir::N] {
            for i in 0..8 {
                for j in 0..32 {
                    self.db
                        .wires
                        .insert(format!("OUT_MACO_IO_{dir}{i}_{j}"), WireKind::LogicOut);
                }
            }
        }
        let mut conn_w = ConnectorClass::new(cslots::IO_W);
        let mut conn_e = ConnectorClass::new(cslots::IO_E);
        for i in 0..32 {
            let mut w = self
                .db
                .wires
                .insert(format!("IO_W{i}_0"), WireKind::MuxOut)
                .0;
            for seg in 1..=8 {
                let nw = self
                    .db
                    .wires
                    .insert(format!("IO_W{i}_{seg}"), WireKind::Branch(cslots::IO_E))
                    .0;
                conn_e.wires.insert(nw, ConnectorWire::Pass(w));
                w = nw;
            }
        }
        for i in 0..32 {
            let mut w = self
                .db
                .wires
                .insert(format!("IO_E{i}_0"), WireKind::MuxOut)
                .0;
            for seg in 1..=8 {
                let nw = self
                    .db
                    .wires
                    .insert(format!("IO_E{i}_{seg}"), WireKind::Branch(cslots::IO_W))
                    .0;
                conn_w.wires.insert(nw, ConnectorWire::Pass(w));
                w = nw;
            }
        }
        for (dir, slot, conn) in [
            (DirH::W, cslots::IO_E, &mut conn_e),
            (DirH::E, cslots::IO_W, &mut conn_w),
        ] {
            let w = self
                .db
                .wires
                .insert(format!("IO_T_{dir}"), WireKind::Branch(slot))
                .0;
            conn.wires.insert(w, ConnectorWire::Pass(w));
        }
        self.db.conn_classes.insert("PASS_IO_W".into(), conn_w);
        self.db.conn_classes.insert("PASS_IO_E".into(), conn_e);
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
        self.fill_sclk_wires_scm();
        self.fill_imux_wires();
        self.fill_out_wires();
        self.fill_ebr_wires_scm();
        self.fill_io_wires_scm();
    }

    fn fill_int_tiles(&mut self) {
        let int_tiles = match self.kind {
            ChipKind::Scm => ["INT_PLC", "INT_EBR", "INT_IO"].as_slice(),
            ChipKind::MachXo => [
                "INT_PLC",
                "INT_SIO_S_W",
                "INT_SIO_S_W_CLK",
                "INT_SIO_S_E",
                "INT_SIO_S_E_CFG",
                "INT_SIO_S_S4",
                "INT_SIO_S_S6",
                "INT_SIO_S_N4",
                "INT_SIO_S_N6",
                "INT_SIO_L_W",
                "INT_SIO_L_E",
                "INT_SIO_L_E_CFG",
                "INT_SIO_L_S4",
                "INT_SIO_L_S6",
                "INT_SIO_L_N4",
                "INT_SIO_L_N6",
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
            ChipKind::MachXo2(_) => {
                ["INT_PLC", "INT_IO_WE", "INT_IO_S", "INT_IO_N", "INT_EBR"].as_slice()
            }
            ChipKind::Ecp4 => {
                ["INT_PLC", "INT_IO_WE", "INT_IO_S", "INT_IO_N", "INT_EBR"].as_slice()
            }
            ChipKind::Ecp5 => {
                ["INT_PLC", "INT_IO_WE", "INT_IO_S", "INT_IO_N", "INT_EBR"].as_slice()
            }
            ChipKind::Crosslink => ["INT_PLC", "INT_IO_S", "INT_IO_N", "INT_EBR"].as_slice(),
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

            if !(i == 2 && matches!(self.kind, ChipKind::Ecp3 | ChipKind::Ecp3A)) {
                let mut tcls = self.tile_class(format!("PCLK{i}_SOURCE_W"), tslots::PCLK_SOURCE, 2);
                tcls.bel(bels::PCLK_SOURCE_W);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            if !(i == 1 && matches!(self.kind, ChipKind::Ecp3 | ChipKind::Ecp3A)) {
                let mut tcls = self.tile_class(format!("PCLK{i}_SOURCE_E"), tslots::PCLK_SOURCE, 2);
                tcls.bel(bels::PCLK_SOURCE_E);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }
        }
    }

    fn fill_pclk_tiles_machxo2(&mut self) {
        for i in 0..4 {
            for name in [format!("PCLK{i}_SOURCE_IO_N"), format!("PCLK{i}_SOURCE_N")] {
                let mut tcls = self.tile_class(name, tslots::PCLK_SOURCE, 1);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            if matches!(i, 0 | 3) {
                let mut tcls =
                    self.tile_class(format!("PCLK{i}_SOURCE_IO_N_W"), tslots::PCLK_SOURCE, 1);
                tcls.bel(bels::PCLK_SOURCE_W);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            if i == 1 {
                let mut tcls =
                    self.tile_class(format!("PCLK{i}_SOURCE_N_W"), tslots::PCLK_SOURCE, 1);
                tcls.bel(bels::PCLK_SOURCE_W);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            if matches!(i, 0 | 1) {
                let mut tcls =
                    self.tile_class(format!("PCLK{i}_SOURCE_IO_N_E"), tslots::PCLK_SOURCE, 1);
                tcls.bel(bels::PCLK_SOURCE_E);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            if i == 1 {
                let mut tcls =
                    self.tile_class(format!("PCLK{i}_SOURCE_N_E"), tslots::PCLK_SOURCE, 1);
                tcls.bel(bels::PCLK_SOURCE_E);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            {
                let mut tcls = self.tile_class(format!("PCLK{i}_SOURCE"), tslots::PCLK_SOURCE, 2);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            {
                let mut tcls = self.tile_class(format!("PCLK{i}_SOURCE_W"), tslots::PCLK_SOURCE, 2);
                tcls.bel(bels::PCLK_SOURCE_W);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }

            if i != 2 {
                let mut tcls = self.tile_class(format!("PCLK{i}_SOURCE_E"), tslots::PCLK_SOURCE, 2);
                tcls.bel(bels::PCLK_SOURCE_E);
                tcls.bel(bels::PCLK_DCC0);
                tcls.bel(bels::PCLK_DCC1);
            }
        }
    }

    fn fill_pclk_tiles(&mut self) {
        match self.kind {
            ChipKind::Scm
            | ChipKind::Ecp
            | ChipKind::Xp
            | ChipKind::MachXo
            | ChipKind::Ecp2
            | ChipKind::Ecp2M
            | ChipKind::Xp2
            | ChipKind::Ecp4
            | ChipKind::Ecp5
            | ChipKind::Crosslink => (),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_pclk_tiles_ecp3(),
            ChipKind::MachXo2(_) => self.fill_pclk_tiles_machxo2(),
        }
    }

    fn fill_sclk_tiles(&mut self) {
        if !self.kind.has_distributed_sclk() {
            return;
        }

        for i in 0..4 {
            let mut tiles = vec![(format!("SCLK{i}_SOURCE"), vec![(i, 0), (i + 4, 1)])];
            if self.kind.has_distributed_sclk_ecp3() {
                if !(i == 2 && matches!(self.kind, ChipKind::Ecp3 | ChipKind::Ecp3A)) {
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
                if !(i == 1 && matches!(self.kind, ChipKind::Ecp3 | ChipKind::Ecp3A)) {
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
                        dst: TileWireCoord::new_idx(0, self.db.get_wire(&format!("SCLK{si}"))),
                        src: TileWireCoord::new_idx(0, self.db.get_wire(&format!("VSDCLK{vsdi}")))
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
            if name == "HSDCLK_SPLITTER" && matches!(self.kind, ChipKind::MachXo2(_)) {
                continue;
            }
            let mut sb = SwitchBox::default();
            for i in 0..2 {
                let wire = self.db.get_wire(&format!("HSDCLK{ii}", ii = i * 4));
                for j in 0..4 {
                    sb.items.push(SwitchBoxItem::ProgBuf(Buf {
                        dst: TileWireCoord::new_idx(j, wire),
                        src: TileWireCoord::new_idx(4 + j, wire).pos(),
                    }));
                    sb.items.push(SwitchBoxItem::ProgBuf(Buf {
                        dst: TileWireCoord::new_idx(4 + j, wire),
                        src: TileWireCoord::new_idx(j, wire).pos(),
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

    fn fill_clk_tiles_scm(&mut self) {
        for slot in bels::CLKDIV {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for name in ["CLK_W", "CLK_E", "CLK_N"] {
            let mut tcls = self.tile_class(name, tslots::CLK, 2);
            tcls.bel(bels::ECLK_ROOT);
            for i in 0..4 {
                tcls.bel(bels::CLKDIV[i]);
            }
            for i in 0..2 {
                tcls.bel(bels::DCS[i]);
            }
            tcls.bel(bels::CLK_EDGE);
        }
        {
            let mut tcls = self.tile_class("CLK_S", tslots::CLK, 2);
            for i in 0..2 {
                tcls.bel(bels::DCS[i]);
            }
            tcls.bel(bels::CLK_EDGE);
        }
        for name in ["CLK_SW", "CLK_SE"] {
            let mut tcls = self.tile_class(name, tslots::CLK, 2);
            tcls.bel(bels::ECLK_ROOT);
            for i in 0..4 {
                tcls.bel(bels::CLKDIV[i]);
            }
        }
        {
            let mut tcls = self.tile_class("CLK_ROOT", tslots::CLK, 4);
            tcls.bel(bels::CLK_ROOT);
            tcls.bel(bels::CLKTEST);
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

    fn fill_clk_tiles_machxo2(&mut self) {
        for slot in bels::CLKDIV {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for slot in bels::DLLDEL {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for slot in bels::DQS {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for (name, num_cells) in [
            ("CLK_ROOT_0EBR", 8),
            ("CLK_ROOT_1EBR", 8),
            ("CLK_ROOT_2EBR", 13),
            ("CLK_ROOT_3EBR", 14),
        ] {
            let mut tcls = self.tile_class(name, tslots::CLK, num_cells);
            for i in 0..8 {
                tcls.bel(bels::DCC[i]);
            }
            for bel in bels::DCM {
                tcls.bel(bel);
            }
            if name != "CLK_ROOT_0EBR" {
                for bel in bels::ECLKBRIDGECS {
                    tcls.bel(bel);
                }
            }
            tcls.bel(bels::CLKTEST);
            tcls.bel(bels::CLK_ROOT);
        }
        for name in ["CLK_S", "CLK_N"] {
            let mut tcls = self.tile_class(name, tslots::CLK, 1);
            tcls.bel(bels::ECLKSYNC0);
            tcls.bel(bels::ECLKSYNC1);
            tcls.bel(bels::DLLDEL0);
            tcls.bel(bels::DLLDEL1);
            tcls.bel(bels::CLKDIV0);
            tcls.bel(bels::CLKDIV1);
            if name == "CLK_S" {
                tcls.bel(bels::CLKFBBUF0);
                tcls.bel(bels::CLKFBBUF1);
            }
        }
        for name in ["DQSDLL_S", "DQSDLL_N"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 1);
            tcls.bel(bels::DQSDLL);
            tcls.bel(bels::DQSDLLTEST);
        }
        {
            let mut tcls = self.tile_class("CLK_W", tslots::CLK, 1);
            tcls.bel(bels::DLLDEL0);
            tcls.bel(bels::DLLDEL1);
            tcls.bel(bels::DLLDEL2);
        }
        {
            let mut tcls = self.tile_class("CLK_E", tslots::CLK, 1);
            tcls.bel(bels::DLLDEL0);
        }
        {
            let mut tcls = self.tile_class("CLK_E_DQS", tslots::CLK, 5);
            tcls.bel(bels::DLLDEL0);
            tcls.bel(bels::DQS0);
            tcls.bel(bels::DQS1);
        }
    }

    fn fill_clk_tiles_ecp4(&mut self) {
        for slot in bels::CLKDIV {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for slot in bels::DLLDEL {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for (name, num_cells, bank_range, num_dcc) in [
            ("CLK_N_S", 8, 1..3, 16),
            ("CLK_N_M", 10, 1..3, 16),
            ("CLK_N_L", 22, 0..4, 16),
            ("CLK_W_S", 8, 0..2, 14),
            ("CLK_W_M", 10, 0..2, 14),
            ("CLK_W_L", 12, 0..2, 14),
            ("CLK_E_S", 8, 0..2, 14),
            ("CLK_E_M", 10, 0..2, 14),
            ("CLK_E_L", 12, 0..2, 14),
        ] {
            let mut tcls = self.tile_class(name, tslots::CLK, num_cells);
            for i in 0..num_dcc {
                tcls.bel(bels::DCC[i]);
            }
            tcls.bel(bels::CLKTEST);
            tcls.bel(bels::CLK_EDGE);
            for i in bank_range.clone() {
                for j in 0..4 {
                    tcls.bel(bels::ECLKSYNC[i * 4 + j]);
                }
            }
            tcls.bel(bels::CLKTEST_ECLK);
            for i in bank_range.clone() {
                for j in 0..2 {
                    tcls.bel(bels::DLLDEL[i * 2 + j]);
                }
            }
            for i in 0..4 {
                tcls.bel(bels::CLKDIV[i]);
            }
            if !name.starts_with("CLK_N") {
                for i in 0..2 {
                    tcls.bel(bels::BRGECLKSYNC[i]);
                    tcls.bel(bels::ECLKBRIDGECS[i]);
                }
            }
        }
        for (name, num_cells, num_quads) in
            [("CLK_S_S", 2, 1), ("CLK_S_M", 4, 2), ("CLK_S_L", 6, 3)]
        {
            let mut tcls = self.tile_class(name, tslots::CLK, num_cells);
            for i in 0..16 {
                tcls.bel(bels::DCC[i]);
            }
            tcls.bel(bels::CLKTEST);
            tcls.bel(bels::CLK_EDGE);
            for i in 0..num_quads {
                tcls.bel(bels::PCSCLKDIV[i]);
            }
        }

        {
            let mut tcls = self.tile_class("CLK_ROOT", tslots::CLK, 4);
            tcls.bel(bels::DCC_SW0);
            tcls.bel(bels::DCC_SE0);
            tcls.bel(bels::DCC_NW0);
            tcls.bel(bels::DCC_NE0);
            tcls.bel(bels::DCS0);
            tcls.bel(bels::DCS1);
            tcls.bel(bels::CLK_ROOT);
            tcls.bel(bels::CLKTEST);
        }
    }

    fn fill_clk_tiles_ecp5(&mut self) {
        for slot in bels::CLKDIV {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for slot in bels::DLLDEL {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for (name, num_cells) in [
            ("CLK_W_S", 12),
            ("CLK_W_L", 14),
            ("CLK_E_S", 12),
            ("CLK_E_L", 14),
        ] {
            let mut tcls = self.tile_class(name, tslots::CLK, num_cells);
            for i in 0..14 {
                tcls.bel(bels::DCC[i]);
            }
            tcls.bel(bels::CLKTEST);
            tcls.bel(bels::CLK_EDGE);
            for i in 0..2 {
                for j in 0..2 {
                    tcls.bel(bels::ECLKSYNC[i * 2 + j]);
                }
            }
            tcls.bel(bels::CLKTEST_ECLK);
            for i in 0..2 {
                for j in 0..2 {
                    tcls.bel(bels::DLLDEL[i * 2 + j]);
                }
            }
            for i in 0..2 {
                tcls.bel(bels::CLKDIV[i]);
            }
            tcls.bel(bels::BRGECLKSYNC0);
            tcls.bel(bels::ECLKBRIDGECS0);
        }
        for (name, num_cells) in [("CLK_N_S", 10), ("CLK_N_L", 12)] {
            let mut tcls = self.tile_class(name, tslots::CLK, num_cells);
            for i in 0..12 {
                tcls.bel(bels::DCC[i]);
            }
            tcls.bel(bels::CLKTEST);
            tcls.bel(bels::CLK_EDGE);
            for i in 0..2 {
                for j in 0..2 {
                    tcls.bel(bels::DLLDEL[i * 2 + j]);
                }
            }
        }
        for (name, num_cells) in [("CLK_S_S", 8), ("CLK_S_L", 10)] {
            let mut tcls = self.tile_class(name, tslots::CLK, num_cells);
            for i in 0..16 {
                tcls.bel(bels::DCC[i]);
            }
            tcls.bel(bels::CLKTEST);
            tcls.bel(bels::CLK_EDGE);
            for i in 0..2 {
                tcls.bel(bels::PCSCLKDIV[i]);
            }
        }

        for (name, num_cells) in [("CLK_ROOT_S", 4), ("CLK_ROOT_L", 8)] {
            let mut tcls = self.tile_class(name, tslots::CLK, num_cells);
            tcls.bel(bels::DCC_SW0);
            tcls.bel(bels::DCC_SE0);
            tcls.bel(bels::DCC_NW0);
            tcls.bel(bels::DCC_NE0);
            tcls.bel(bels::DCS0);
            tcls.bel(bels::DCS1);
            tcls.bel(bels::CLK_ROOT);
            tcls.bel(bels::CLKTEST);
        }
    }

    fn fill_clk_tiles_crosslink(&mut self) {
        for slot in bels::CLKDIV {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        for slot in bels::DLLDEL {
            self.db.bel_slots[slot].tile_slot = tslots::CLK;
        }
        {
            let mut tcls = self.tile_class("CLK_S", tslots::CLK, 6);
            for i in 0..8 {
                tcls.bel(bels::DCC[i]);
            }
            tcls.bel(bels::CLKTEST);
            tcls.bel(bels::CLK_EDGE);
            for i in 0..2 {
                for j in 0..2 {
                    tcls.bel(bels::ECLKSYNC[i * 2 + j]);
                }
            }
            tcls.bel(bels::CLKTEST_ECLK);
            for i in 0..2 {
                for j in 0..2 {
                    tcls.bel(bels::DLLDEL[i * 2 + j]);
                }
            }
            for i in 0..4 {
                tcls.bel(bels::CLKDIV[i]);
            }
        }
        {
            let mut tcls = self.tile_class("CLK_N", tslots::CLK, 2);
            for i in 0..6 {
                tcls.bel(bels::DCC[i]);
            }
            tcls.bel(bels::CLKTEST);
            tcls.bel(bels::CLK_EDGE);
        }
        {
            let mut tcls = self.tile_class("CLK_ROOT", tslots::CLK, 1);
            tcls.bel(bels::DCS0);
            tcls.bel(bels::CLK_ROOT);
            tcls.bel(bels::CLKTEST);
        }
    }

    fn fill_clk_tiles(&mut self) {
        match self.kind {
            ChipKind::Scm => self.fill_clk_tiles_scm(),
            ChipKind::Ecp | ChipKind::Xp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                self.fill_clk_tiles_ecp()
            }
            ChipKind::MachXo => self.fill_clk_tiles_machxo(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_clk_tiles_ecp3(),
            ChipKind::MachXo2(_) => self.fill_clk_tiles_machxo2(),
            ChipKind::Ecp4 => self.fill_clk_tiles_ecp4(),
            ChipKind::Ecp5 => self.fill_clk_tiles_ecp5(),
            ChipKind::Crosslink => self.fill_clk_tiles_crosslink(),
        }
    }

    fn fill_plc_tiles(&mut self) {
        let kind = self.kind;
        for name in ["PLC", "FPLC"] {
            if name == "FPLC"
                && matches!(
                    self.kind,
                    ChipKind::Scm
                        | ChipKind::MachXo2(_)
                        | ChipKind::Ecp4
                        | ChipKind::Ecp5
                        | ChipKind::Crosslink
                )
            {
                continue;
            }
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
                    if matches!(kind, ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink) {
                        bel.add_input("CLK", 0, &format!("IMUX_MUXCLK{i}"));
                        bel.add_input("LSR", 0, &format!("IMUX_MUXLSR{i}"));
                    } else {
                        bel.add_input("CLK", 0, &format!("IMUX_CLK{i}"));
                        bel.add_input("LSR", 0, &format!("IMUX_LSR{i}"));
                    }
                    bel.add_input("CE", 0, &format!("IMUX_CE{i}"));
                    bel.add_output("Q0", 0, &format!("OUT_Q{i0}"));
                    bel.add_output("Q1", 0, &format!("OUT_Q{i1}"));
                }
                for l in ["F", "OFX"] {
                    if matches!(kind, ChipKind::Ecp5 | ChipKind::Crosslink) && l == "OFX" {
                        continue;
                    }
                    bel.add_output(&format!("{l}0"), 0, &format!("OUT_{l}{i0}"));
                    bel.add_output(&format!("{l}1"), 0, &format!("OUT_{l}{i1}"));
                }
                if i == 3 && kind.has_ecp_plc() {
                    bel.add_input("FXB", 0, "OUT_OFX3_W");
                }
                if i == 3 && matches!(kind, ChipKind::MachXo2(_) | ChipKind::Ecp4) {
                    bel.add_input("FXA", 0, "OUT_OFX3_W");
                }
                if i == 3 && matches!(kind, ChipKind::Ecp5 | ChipKind::Crosslink) {
                    bel.add_input("FXA", 0, "OUT_F3_W");
                }
                if i == 2 && matches!(kind, ChipKind::Ecp5 | ChipKind::Crosslink) {
                    bel.add_input("WCK", 0, "IMUX_CLK1");
                    bel.add_input("WRE", 0, "IMUX_LSR1");
                }
            }
        }
    }

    fn fill_ebr_tiles(&mut self) {
        let (num_cells, num_bels) = match self.kind {
            ChipKind::Scm | ChipKind::Ecp | ChipKind::Xp => (2, 1),
            ChipKind::MachXo => (4, 1),
            ChipKind::Ecp2
            | ChipKind::Ecp2M
            | ChipKind::Xp2
            | ChipKind::Ecp3
            | ChipKind::Ecp3A
            | ChipKind::MachXo2(_) => (3, 1),
            ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink => (9, 4),
        };
        let tiles = match self.kind {
            ChipKind::Scm => ["EBR_W", "EBR_E"].as_slice(),
            ChipKind::MachXo2(_) => ["EBR", "EBR_N"].as_slice(),
            _ => ["EBR"].as_slice(),
        };
        let kind = self.kind;
        for &tcname in tiles {
            let mut tcls = self.tile_class(tcname, tslots::BEL, num_cells);
            for i in 0..num_bels {
                tcls.bel(bels::EBR[i]);
            }
            if kind == ChipKind::Scm {
                tcls.bel(bels::EBR_INT);
            }
        }
    }

    fn fill_dsp_tiles(&mut self) {
        let (num_cells, num_bels) = match self.kind {
            ChipKind::Scm
            | ChipKind::Xp
            | ChipKind::MachXo
            | ChipKind::MachXo2(_)
            | ChipKind::Crosslink => return,
            ChipKind::Ecp => (8, 1),
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => (9, 1),
            ChipKind::Ecp3 | ChipKind::Ecp3A | ChipKind::Ecp4 | ChipKind::Ecp5 => (9, 2),
        };
        let mut tcls = self.tile_class("DSP", tslots::BEL, num_cells);
        for i in 0..num_bels {
            tcls.bel(bels::DSP[i]);
        }
    }

    fn fill_config_tiles_scm(&mut self) {
        let mut tcls = self.tile_class("CONFIG", tslots::BEL, 13);
        tcls.bel(bels::JTAG);
        tcls.bel(bels::RDBK);
        tcls.bel(bels::GSR);
        tcls.bel(bels::OSC);
        tcls.bel(bels::START);
        tcls.bel(bels::SYSBUS);
        tcls.bel(bels::SERDES_CENTER);
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

    fn fill_config_tiles_machxo2(&mut self) {
        for (name, num_cells) in [("CONFIG", 4), ("CONFIG_XO3D", 10)] {
            let mut tcls = self.tile_class(name, tslots::BEL, num_cells);
            tcls.bel(bels::START);
            tcls.bel(bels::JTAG);
            tcls.bel(bels::OSC);
            tcls.bel(bels::GSR);
            tcls.bel(bels::SED);
            tcls.bel(bels::PCNTR);
            tcls.bel(bels::TSALL);
            tcls.bel(bels::EFB);
            if name == "CONFIG_XO3D" {
                tcls.bel(bels::ESB);
            }
        }
    }

    fn fill_config_tiles_ecp4(&mut self) {
        let mut tcls = self.tile_class("CONFIG", tslots::BEL, 11);
        tcls.bel(bels::START);
        tcls.bel(bels::JTAG);
        tcls.bel(bels::OSC);
        tcls.bel(bels::GSR);
        tcls.bel(bels::SED);
        tcls.bel(bels::PCNTR);
        tcls.bel(bels::EFB);
    }

    fn fill_config_tiles_ecp5(&mut self) {
        let mut tcls = self.tile_class("CONFIG", tslots::BEL, 4);
        tcls.bel(bels::START);
        tcls.bel(bels::JTAG);
        tcls.bel(bels::OSC);
        tcls.bel(bels::GSR);
        tcls.bel(bels::SED);
        tcls.bel(bels::CCLK);
    }

    fn fill_config_tiles_crosslink(&mut self) {
        for name in ["I2C_W", "I2C_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 2);
            tcls.bel(bels::I2C);
            if name == "I2C_E" {
                tcls.bel(bels::NVCMTEST);
            }
        }
        self.tile_class("OSC", tslots::BEL, 1).bel(bels::OSC);
        {
            let mut tcls = self.tile_class("CONFIG", tslots::BEL, 4);
            tcls.bel(bels::GSR);
            tcls.bel(bels::CFGTEST);
        }
        {
            let mut tcls = self.tile_class("PMU", tslots::BEL, 1);
            tcls.bel(bels::PMU);
            tcls.bel(bels::PMUTEST);
        }
    }

    fn fill_config_tiles(&mut self) {
        match self.kind {
            ChipKind::Scm => self.fill_config_tiles_scm(),
            ChipKind::Ecp => self.fill_config_tiles_ecp(),
            ChipKind::Xp => self.fill_config_tiles_xp(),
            ChipKind::MachXo => self.fill_config_tiles_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.fill_config_tiles_ecp2(),
            ChipKind::Xp2 => self.fill_config_tiles_xp2(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_config_tiles_ecp3(),
            ChipKind::MachXo2(_) => self.fill_config_tiles_machxo2(),
            ChipKind::Ecp4 => self.fill_config_tiles_ecp4(),
            ChipKind::Ecp5 => self.fill_config_tiles_ecp5(),
            ChipKind::Crosslink => self.fill_config_tiles_crosslink(),
        }
    }

    fn fill_io_tiles_scm(&mut self) {
        for (name, num_io, num_cells) in [
            ("IO_W4", 4, 2),
            ("IO_W12", 12, 4),
            ("IO_E4", 4, 2),
            ("IO_E12", 12, 4),
            ("IO_S4", 4, 2),
            ("IO_S12", 12, 4),
            ("IO_N4", 4, 2),
            ("IO_N8", 8, 3),
            ("IO_N12", 12, 4),
        ] {
            let mut tcls = self.tile_class(name, tslots::IO, num_cells);
            for i in 0..num_io {
                tcls.bel(bels::IO[i]);
            }
            for i in 0..num_io / 4 {
                tcls.bel(bels::PICTEST[i]);
            }
        }
        for name in ["IO_INT_W", "IO_INT_E", "IO_INT_S"] {
            self.tile_class(name, tslots::BEL, 1).bel(bels::IO_INT);
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
            self.tile_class(name, tslots::BEL, 1).bel(bels::DQS0);
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
                tcls.bel(bels::CLKDIV0);
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
            ("SIO_S_W2", 4),
            ("SIO_S_W4", 4),
            ("SIO_S_E2", 4),
            ("SIO_S_E4", 4),
            ("SIO_S_S4", 4),
            ("SIO_S_S6", 6),
            ("SIO_S_N4", 4),
            ("SIO_S_N6", 6),
            ("SIO_L_W2", 4),
            ("SIO_L_W4", 4),
            ("SIO_L_E2", 4),
            ("SIO_L_E4", 4),
            ("SIO_L_S4", 4),
            ("SIO_L_S6", 6),
            ("SIO_L_N4", 4),
            ("SIO_L_N6", 6),
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
            tcls.bel(bels::DQS0);
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

    fn fill_io_tiles_machxo2(&mut self) {
        for (name, num_cells) in [
            ("BC", 1),
            ("BC_N", 1),
            ("BCSR_W", 1),
            ("BCSR_E", 2),
            ("BCSR_S", 1),
            ("BCSR_N", 1),
        ] {
            let mut tcls = self.tile_class(name, tslots::BC, num_cells);
            tcls.bel(bels::BCINRD);
            tcls.bel(bels::BCPG);
            if name.ends_with("_N") {
                tcls.bel(bels::BCLVDSO);
            }
            if name.starts_with("BCSR") {
                tcls.bel(bels::BCSLEWRATE);
            }
        }
        for (name, num) in [
            ("IO_W2", 2),
            ("IO_W4", 4),
            ("IO_W4_I3C", 4),
            ("IO_E2", 2),
            ("IO_E4", 4),
            ("IO_S4", 4),
            ("SIO_S2", 2),
            ("SIO_S4", 4),
            ("IO_N4", 4),
            ("SIO_N2", 2),
            ("SIO_N4", 4),
        ] {
            let mut tcls = self.tile_class(name, tslots::IO, 1);
            for i in 0..num {
                tcls.bel(bels::IO[i]);
            }
        }
    }

    fn fill_io_tiles_ecp4(&mut self) {
        for name in ["BC_W", "BC_E", "BC_N"] {
            let mut tcls = self.tile_class(name, tslots::BC, 1);
            tcls.bel(bels::BCINRD);
            tcls.bel(bels::BCPG);
            tcls.bel(bels::BCLVDSO);
            tcls.bel(bels::BCPUSL);
            tcls.bel(bels::BREFTEST);
        }
        {
            let mut tcls = self.tile_class("PVTTEST", tslots::BEL, 1);
            tcls.bel(bels::PVTTEST);
            tcls.bel(bels::PVTCAL);
        }
        for name in ["DDRDLL_S", "DDRDLL_N"] {
            self.tile_class(name, tslots::BEL, 1).bel(bels::DDRDLL);
        }
        for name in ["DTR_S", "DTR_N"] {
            self.tile_class(name, tslots::BEL, 1).bel(bels::DTR);
        }
        for name in [
            "IO_W",
            "IO_W_EBR_S",
            "IO_W_EBR_N",
            "IO_W_DSP_S",
            "IO_W_DSP_N",
            "IO_E",
            "IO_E_EBR_S",
            "IO_E_EBR_N",
            "IO_E_DSP_S",
            "IO_E_DSP_N",
            "IO_N",
        ] {
            let mut tcls = self.tile_class(name, tslots::IO, 4);
            for i in 0..4 {
                tcls.bel(bels::IO[i]);
            }
        }
        for name in [
            "DQS_W",
            "DQS_W_BELOW_DSP_N",
            "DQS_W_BELOW_EBR_N",
            "DQS_W_BELOW_EBR_S",
            "DQS_W_EBR_S",
            "DQS_W_EBR_N",
            "DQS_W_DSP_N",
            "DQS_E",
            "DQS_E_BELOW_DSP_N",
            "DQS_E_BELOW_EBR_N",
            "DQS_E_BELOW_EBR_S",
            "DQS_E_EBR_S",
            "DQS_E_EBR_N",
            "DQS_E_DSP_N",
            "DQS_N",
        ] {
            self.tile_class(name, tslots::BEL, 5).bel(bels::DQS0);
        }
    }

    fn fill_io_tiles_ecp5(&mut self) {
        for (name, is_we) in [
            ("BC0", false),
            ("BC1", false),
            ("BC2", true),
            ("BC3", true),
            ("BC4", false),
            ("BC8", false),
            ("BC6", true),
            ("BC7", true),
        ] {
            let mut tcls = self.tile_class(name, tslots::BC, 1);
            if is_we {
                tcls.bel(bels::BCINRD);
                tcls.bel(bels::BCLVDSO);
            }
            tcls.bel(bels::BREFTEST);
        }
        {
            self.tile_class("DTR", tslots::BEL, 1).bel(bels::DTR);
        }
        self.tile_class("DDRDLL", tslots::BEL, 1).bel(bels::DDRDLL);
        for (name, num_io, num_cells) in [
            ("IO_W4", 4, 3),
            ("IO_E4", 4, 3),
            ("IO_N2", 2, 2),
            ("IO_S1", 1, 1),
            ("IO_S2", 2, 2),
        ] {
            let mut tcls = self.tile_class(name, tslots::IO, num_cells);
            for i in 0..num_io {
                tcls.bel(bels::IO[i]);
            }
        }
        for name in ["DQS_W", "DQS_E"] {
            self.tile_class(name, tslots::BEL, 6).bel(bels::DQS0);
        }
    }

    fn fill_io_tiles_crosslink(&mut self) {
        for slot in [bels::BCINRD, bels::BCLVDSO] {
            self.db.bel_slots[slot].tile_slot = tslots::BEL;
        }
        {
            let mut tcls = self.tile_class("BC", tslots::BEL, 1);
            tcls.bel(bels::BCINRD);
            tcls.bel(bels::BCLVDSO);
            tcls.bel(bels::DDRDLL);
        }
        for (name, num_cells) in [("IO_S4", 4), ("IO_S1A", 1), ("IO_S1B", 1)] {
            let mut tcls = self.tile_class(name, tslots::IO, num_cells);
            for i in 0..num_cells {
                tcls.bel(bels::IO[i]);
            }
        }
    }

    fn fill_io_tiles(&mut self) {
        match self.kind {
            ChipKind::Scm => self.fill_io_tiles_scm(),
            ChipKind::Ecp | ChipKind::Xp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                self.fill_io_tiles_ecp()
            }
            ChipKind::MachXo => self.fill_io_tiles_machxo(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_io_tiles_ecp3(),
            ChipKind::MachXo2(_) => self.fill_io_tiles_machxo2(),
            ChipKind::Ecp4 => self.fill_io_tiles_ecp4(),
            ChipKind::Ecp5 => self.fill_io_tiles_ecp5(),
            ChipKind::Crosslink => self.fill_io_tiles_crosslink(),
        }
    }

    fn fill_serdes_tiles_scm(&mut self) {
        for name in ["SERDES_W", "SERDES_E"] {
            self.tile_class(name, tslots::BEL, 7).bel(bels::SERDES);
        }
        for name in ["MACO_W", "MACO_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 16);
            tcls.bel(bels::MACO);
            tcls.bel(bels::MACO_INT);
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

    fn fill_serdes_tiles_ecp4(&mut self) {
        for (name, num_cells) in [("SERDES1", 62), ("SERDES2", 120), ("SERDES3", 177)] {
            self.tile_class(name, tslots::BEL, num_cells)
                .bel(bels::SERDES);
        }
    }

    fn fill_serdes_tiles_ecp5(&mut self) {
        self.tile_class("SERDES", tslots::BEL, 12).bel(bels::SERDES);
    }

    fn fill_mipi_tiles_crosslink(&mut self) {
        for name in ["MIPI_W", "MIPI_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 24);
            tcls.bel(bels::MIPI);
            tcls.bel(bels::CLKTEST_MIPI);
        }
    }

    fn fill_serdes_tiles(&mut self) {
        match self.kind {
            ChipKind::Scm => self.fill_serdes_tiles_scm(),
            ChipKind::Ecp
            | ChipKind::Xp
            | ChipKind::Ecp2
            | ChipKind::Xp2
            | ChipKind::MachXo
            | ChipKind::MachXo2(_) => (),
            ChipKind::Ecp2M => self.fill_serdes_tiles_ecp2(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_serdes_tiles_ecp3(),
            ChipKind::Ecp4 => self.fill_serdes_tiles_ecp4(),
            ChipKind::Ecp5 => self.fill_serdes_tiles_ecp5(),
            ChipKind::Crosslink => self.fill_mipi_tiles_crosslink(),
        }
    }

    fn fill_pll_tiles_scm(&mut self) {
        for name in ["PLL_SW", "PLL_SE"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 6);
            for i in 0..2 {
                tcls.bel(bels::PLL[i]);
            }
            for i in 0..4 {
                tcls.bel(bels::DLL[i]);
            }
            for i in 0..2 {
                tcls.bel(bels::DLL_DCNTL[i]);
            }
            tcls.bel(bels::PLL_SMI);
            tcls.bel(bels::PROMON);
            if name == "PLL_SW" {
                tcls.bel(bels::RNET);
            }
        }
        for name in ["PLL_NW", "PLL_NE"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 3);
            for i in 0..2 {
                tcls.bel(bels::PLL[i]);
            }
            for i in 0..2 {
                tcls.bel(bels::DLL[i]);
            }
            for i in 0..2 {
                tcls.bel(bels::DLL_DCNTL[i]);
            }
            tcls.bel(bels::PLL_SMI);
            tcls.bel(bels::SERDES_CORNER);
            if name == "PLL_NW" {
                tcls.bel(bels::M0);
                tcls.bel(bels::M1);
                tcls.bel(bels::M2);
                tcls.bel(bels::M3);
                tcls.bel(bels::RESETN);
                tcls.bel(bels::RDCFGN);
            }
            if name == "PLL_NE" {
                tcls.bel(bels::CCLK);
                tcls.bel(bels::TCK);
                tcls.bel(bels::TMS);
                tcls.bel(bels::TDI);
            }
        }
    }

    fn fill_pll_tiles_ecp(&mut self) {
        for name in ["PLL_W", "PLL_E"] {
            self.tile_class(name, tslots::BEL, 1).bel(bels::PLL0);
        }
    }

    fn fill_pll_tiles_machxo(&mut self) {
        for name in ["PLL_S", "PLL_N"] {
            self.tile_class(name, tslots::BEL, 1).bel(bels::PLL0);
        }
    }

    fn fill_pll_tiles_ecp2(&mut self) {
        for name in ["SPLL_W", "SPLL_E"] {
            self.tile_class(name, tslots::BEL, 2).bel(bels::SPLL);
        }
        for name in ["PLL_W", "PLL_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 4);
            tcls.bel(bels::PLL0);
            tcls.bel(bels::DLL0);
            tcls.bel(bels::DLLDEL0);
            tcls.bel(bels::CLKDIV0);
            tcls.bel(bels::ECLK_ALT_ROOT);
            tcls.bel(bels::DQSDLL);
        }
    }

    fn fill_pll_tiles_xp2(&mut self) {
        for name in ["PLL_S", "PLL_N"] {
            self.tile_class(name, tslots::BEL, 2).bel(bels::PLL0);
        }
    }

    fn fill_pll_tiles_ecp3(&mut self) {
        for name in ["PLL_DLL_W", "PLL_DLL_E", "PLL_DLL_A_W", "PLL_DLL_A_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 18);
            tcls.bel(bels::PLL0);
            tcls.bel(bels::DLL0);
            tcls.bel(bels::DLLDEL0);
            tcls.bel(bels::DQSDLL);
            tcls.bel(bels::DQSDLLTEST);
            tcls.bel(bels::ECLK_ALT_ROOT);
            tcls.bel(bels::CLKDIV0);
        }
        for name in ["PLL_W", "PLL_E", "PLL_A_W", "PLL_A_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 13);
            tcls.bel(bels::PLL0);
        }
    }

    fn fill_pll_tiles_machxo2(&mut self) {
        for name in ["PLL_W", "PLL_E"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 2);
            tcls.bel(bels::PLL0);
            tcls.bel(bels::PLLREFCS0);
        }
    }

    fn fill_pll_tiles_ecp4(&mut self) {
        for name in ["PLL_SW", "PLL_SE", "PLL_NW", "PLL_NE"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 4);
            for i in 0..2 {
                tcls.bel(bels::PLL[i]);
                tcls.bel(bels::PLLREFCS[i]);
            }
        }
    }

    fn fill_pll_tiles_ecp5(&mut self) {
        for name in ["PLL_SW", "PLL_SE", "PLL_NW", "PLL_NE"] {
            let mut tcls = self.tile_class(name, tslots::BEL, 2);
            tcls.bel(bels::PLL0);
            tcls.bel(bels::PLLREFCS0);
        }
    }

    fn fill_pll_tiles_crosslink(&mut self) {
        let mut tcls = self.tile_class("PLL", tslots::BEL, 2);
        tcls.bel(bels::PLL0);
        tcls.bel(bels::PLLREFCS0);
    }

    fn fill_pll_tiles(&mut self) {
        match self.kind {
            ChipKind::Scm => self.fill_pll_tiles_scm(),
            ChipKind::Ecp | ChipKind::Xp => self.fill_pll_tiles_ecp(),
            ChipKind::MachXo => self.fill_pll_tiles_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.fill_pll_tiles_ecp2(),
            ChipKind::Xp2 => self.fill_pll_tiles_xp2(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.fill_pll_tiles_ecp3(),
            ChipKind::MachXo2(_) => self.fill_pll_tiles_machxo2(),
            ChipKind::Ecp4 => self.fill_pll_tiles_ecp4(),
            ChipKind::Ecp5 => self.fill_pll_tiles_ecp5(),
            ChipKind::Crosslink => self.fill_pll_tiles_crosslink(),
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
    let conn_slots = DirMap::from_fn(|dir| match dir {
        Dir::W => cslots::W,
        Dir::E => cslots::E,
        Dir::S => cslots::S,
        Dir::N => cslots::N,
    });
    let builder = IntDbBuilder {
        kind,
        db: IntDb::new(tslots::SLOTS, bels::SLOTS, regions::SLOTS, cslots::SLOTS),
        conn_slots,
        passes: DirMap::from_fn(|dir| ConnectorClass::new(conn_slots[dir])),
        terms: DirMap::from_fn(|dir| ConnectorClass::new(conn_slots[dir])),
        pass_sw: ConnectorClass::new(cslots::SW),
        pass_se: ConnectorClass::new(cslots::SE),
    };
    builder.build()
}

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, IoGroupKind, MachXo2Kind, PllLoc, RowKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, CellSlotId, TileWireCoord},
    dir::{Dir, DirHV, DirV},
    grid::{BelCoord, CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    fn get_nominal_bc_cell(&self, bank: u32) -> CellCoord {
        let has_bank4 = self.chip.special_loc.contains_key(&SpecialLocKey::Bc(4));
        let (col, row) = match bank {
            0 => (self.chip.col_clk - 1, self.chip.row_n()),
            1 => (self.chip.col_e(), self.chip.row_clk),
            2 => (self.chip.col_clk - 1, self.chip.row_s()),
            3 if !has_bank4 => (self.chip.col_w(), self.chip.row_clk),
            3 if has_bank4 => (self.chip.col_w(), self.chip.row_s()),
            4 if has_bank4 => (self.chip.col_w(), self.chip.row_clk),
            5 if has_bank4 => (self.chip.col_w(), self.chip.row_n()),
            _ => unreachable!(),
        };
        CellCoord::new(DieId::from_idx(0), col, row)
    }

    pub(super) fn process_bc_machxo2(&mut self) {
        for (&key, &cell_loc) in &self.chip.special_loc {
            let SpecialLocKey::Bc(bank) = key else {
                continue;
            };
            let cell = self.get_nominal_bc_cell(bank);
            for (bel, suffix, pin_out, pin_in) in [
                (bels::BCPG, "BCPG", "PGENO", "PGENI"),
                (bels::BCINRD, "BCINRD", "INRDENO", "INRDENI"),
                (bels::BCLVDSO, "BCLVDSO", "LVDSENO", "LVDSENI"),
                (bels::BCSLEWRATE, "BCSLEWRATE", "SLEWRATEENO", "SLEWRATEENI"),
            ] {
                let bcrd = cell_loc.bel(bel);
                if self.edev.egrid.has_bel(bcrd) {
                    if bel == bels::BCLVDSO {
                        self.name_bel(bcrd, ["BCLVDSO"]);
                    } else {
                        self.name_bel(bcrd, [format!("{suffix}{bank}")]);
                    }
                    self.insert_simple_bel(bcrd, cell, suffix);
                    let wire_out = if bel == bels::BCSLEWRATE {
                        self.rc_io_wire(cell, &format!("J{pin_out}_{suffix}"))
                    } else {
                        let wire_in = self.rc_io_wire(cell, &format!("J{pin_in}_{suffix}"));
                        let wire_out = self.rc_io_wire(cell, &format!("{pin_out}_{suffix}"));
                        self.claim_pip(wire_out, wire_in);
                        wire_out
                    };
                    self.add_bel_wire(bcrd, pin_out, wire_out);
                    let wire_out_out = self.claim_single_out(wire_out);
                    self.add_bel_wire(bcrd, format!("{pin_out}_OUT"), wire_out_out);
                }
            }
        }
    }

    pub(super) fn process_eclk_machxo2(&mut self) {
        let is_smol = self.chip.rows[self.chip.row_clk].kind != RowKind::Ebr;
        if is_smol {
            return;
        }
        for edge in [DirV::S, DirV::N] {
            for idx in 0..2 {
                let bcrd = self.chip.bel_eclksync(Dir::V(edge), idx);
                let cell = bcrd.cell;
                self.name_bel(
                    bcrd,
                    [format!(
                        "{bt}ECLKSYNC{idx}",
                        bt = match edge {
                            DirV::S => 'B',
                            DirV::N => 'T',
                        }
                    )],
                );
                let mut bel = Bel::default();

                let eclki = self.rc_wire(cell, &format!("ECLKI{idx}_ECLKSYNC"));
                self.add_bel_wire(bcrd, "ECLKI", eclki);

                let eclki_in = self.rc_wire(cell, &format!("ECLKI{idx}"));
                self.add_bel_wire(bcrd, "ECLKI_IN", eclki_in);
                self.claim_pip(eclki, eclki_in);

                let stop = self.rc_wire(cell, &format!("JSTOP{idx}_ECLKSYNC"));
                self.add_bel_wire(bcrd, "STOP", stop);
                bel.pins
                    .insert("STOP".into(), self.xlat_int_wire(bcrd, stop));

                let eclko = self.rc_wire(cell, &format!("JECLKO{idx}_ECLKSYNC"));
                self.add_bel_wire(bcrd, "ECLKO", eclko);
                self.claim_pip(eclko, eclki);
                bel.pins
                    .insert("ECLKO".into(), self.xlat_int_wire(bcrd, eclko));

                let eclko_out = self.pips_fwd[&eclko]
                    .iter()
                    .copied()
                    .find(|&wn| {
                        let suffix = self.naming.strings[wn.suffix].as_str();
                        suffix.starts_with("TECLK") || suffix.starts_with("BECLK")
                    })
                    .unwrap();
                self.add_bel_wire(bcrd, "ECLKO_OUT", eclko_out);
                self.claim_pip(eclko_out, eclko);

                let eclki_int = self.rc_wire(cell, &format!("JECLKCIB{idx}"));
                self.add_bel_wire(bcrd, "ECLKI_INT", eclki_int);
                self.claim_pip(eclki_in, eclki_int);
                bel.pins
                    .insert("ECLKI".into(), self.xlat_int_wire(bcrd, eclki_int));

                let eclki_brg = self.rc_wire(cell, &format!("JECLKBRG{idx}"));
                self.add_bel_wire(bcrd, "ECLKI_BRG", eclki_brg);
                self.claim_pip(eclki_in, eclki_brg);
                let wire_brg = self.rc_wire(
                    self.chip.bel_clk_root().delta(-1, 0),
                    &format!("JECSOUT{idx}_ECLKBRIDGECS"),
                );
                self.claim_pip(eclki_brg, wire_brg);

                let eclki_io = self.rc_wire(cell, &format!("JINECK{idx}"));
                self.add_bel_wire(bcrd, "ECLKI_IO", eclki_io);
                self.claim_pip(eclki_in, eclki_io);
                let wire_io = self.rc_io_wire(cell, &format!("JINCK{idx}"));
                self.claim_pip(eclki_io, wire_io);

                for (plli, hv) in [(0, DirHV::NW), (1, DirHV::NE)] {
                    let Some(&cell_pll) = self
                        .chip
                        .special_loc
                        .get(&SpecialLocKey::Pll(PllLoc::new(hv, 0)))
                    else {
                        continue;
                    };
                    for pin in ["CLKOP", "CLKOS"] {
                        let eclki_pll = self.rc_wire(cell, &format!("JPLL{pin}{plli}"));
                        self.add_bel_wire_no_claim(bcrd, format!("ECLK_PLL_{hv}_{pin}"), eclki_pll);
                        self.claim_pip(eclki_in, eclki_pll);
                        if idx == 0 {
                            self.claim_node(eclki_pll);
                            let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                            self.claim_pip(eclki_pll, wire_pll);
                        }
                    }
                }

                self.insert_bel(bcrd, bel);

                if edge == DirV::S {
                    let bcrd = bcrd.bel(bels::CLKFBBUF[idx]);
                    self.name_bel(bcrd, [format!("CLKFBBUF{idx}")]);
                    self.insert_bel(bcrd, Bel::default());

                    let a = self.rc_wire(cell, &format!("JA{idx}_CLKFBBUF"));
                    self.add_bel_wire(bcrd, "A", a);

                    let hv = [DirHV::NW, DirHV::NE][idx];
                    if let Some(&cell_pll) = self
                        .chip
                        .special_loc
                        .get(&SpecialLocKey::Pll(PllLoc::new(hv, 0)))
                    {
                        let wire_pll = self.rc_wire(cell_pll, "JCLKOP_PLL");
                        self.claim_pip(a, wire_pll);
                    }

                    let z = self.rc_wire(cell, &format!("Z{idx}_CLKFBBUF"));
                    self.add_bel_wire(bcrd, "Z", z);

                    let z_out = self.rc_wire(cell, &format!("CLKFBBUF{idx}"));
                    self.add_bel_wire(bcrd, "Z_OUT", z_out);
                    self.claim_pip(z_out, z);

                    let clkfb = self.rc_wire(cell, &format!("JPLLCLKFB{idx}"));
                    self.add_bel_wire(bcrd, "CLKFB", clkfb);
                    self.claim_pip(clkfb, z_out);
                    self.claim_pip(clkfb, eclko_out);
                }
            }
        }
    }

    pub(super) fn process_dqsdll_machxo2(&mut self) {
        for edge in [DirV::S, DirV::N] {
            let Some(&cell) = self
                .chip
                .special_loc
                .get(&SpecialLocKey::DqsDll(Dir::V(edge)))
            else {
                continue;
            };
            let bt = match edge {
                DirV::S => 'B',
                DirV::N => 'T',
            };

            let bcrd = cell.bel(bels::DQSDLL);
            self.name_bel(bcrd, [format!("{bt}DQSDLL",)]);
            let mut bel = self.extract_simple_bel(bcrd, cell, "DQSDLL");

            let dqsdel = self.rc_wire(cell, "JDQSDEL_DQSDLL");
            self.add_bel_wire(bcrd, "DQSDEL", dqsdel);

            let clk = self.rc_wire(cell, "CLK_DQSDLL");
            self.add_bel_wire(bcrd, "CLK", clk);
            let clk_in = self.rc_wire(cell, "DQSDLLCLK");
            self.add_bel_wire(bcrd, "CLK_IN", clk_in);
            self.claim_pip(clk, clk_in);
            let clk_int = self.rc_wire(cell, "JDQSDLLSCLK");
            self.add_bel_wire(bcrd, "CLK_INT", clk_int);
            bel.pins
                .insert("CLK".into(), self.xlat_int_wire(bcrd, clk_int));
            self.claim_pip(clk_in, clk_int);

            for i in 0..2 {
                let bcrd_eclksync = self.chip.bel_eclksync(Dir::V(edge), i);
                let wire_eclk = self.naming.bel_wire(bcrd_eclksync, "ECLKO_OUT");
                self.claim_pip(clk_in, wire_eclk);
            }

            self.insert_bel(bcrd, bel);

            let bcrd_test = bcrd.bel(bels::DQSDLLTEST);
            self.name_bel(bcrd_test, [format!("{bt}DQSDLLTEST",)]);
            self.insert_simple_bel(bcrd_test, cell, "DQSDLLTEST");
        }
    }

    fn is_machnx_io(&self, io: EdgeIoCoord) -> bool {
        if self.chip.kind != ChipKind::MachXo2(MachXo2Kind::MachNx) {
            return false;
        }
        let EdgeIoCoord::W(row, iob) = io else {
            return false;
        };
        matches!(
            (row.to_idx(), iob.to_idx()),
            (2 | 3 | 4 | 5 | 6 | 9 | 10, _)
                | (11, 1 | 2)
                | (12, 1)
                | (13, 0 | 1)
                | (14, 0)
                | (16, 1)
        )
    }

    pub(super) fn process_dqs_machxo2(&mut self) {
        let is_smol = self.chip.rows[self.chip.row_clk].kind != RowKind::Ebr;
        if is_smol {
            return;
        }
        if self.chip.kind == ChipKind::MachXo2(MachXo2Kind::MachXo3L) {
            return;
        }
        let has_dqs = self.chip.kind == ChipKind::MachXo2(MachXo2Kind::MachXo2);
        let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for i in 0..2 {
            let bcrd = cell.bel(bels::DQS[i]);
            if has_dqs {
                self.name_bel(bcrd, [format!("DQS{i}")]);
            } else {
                self.name_bel_null(bcrd);
            }
            let mut bel = Bel::default();
            for pin in ["DDRCLKPOL", "DQSR90", "DQSW90"] {
                let cell_io = [cell.delta(0, 2), cell.delta(0, -2)][i];
                let wire_io = self.rc_io_wire(cell_io, &format!("{pin}A_RIOLOGIC"));
                let wire_out = self.find_single_in(wire_io);
                self.add_bel_wire(bcrd, format!("{pin}_OUT"), wire_out);

                if has_dqs {
                    let wire = self.rc_io_wire(
                        cell,
                        &if pin.ends_with('0') {
                            format!("J{pin}_{i}_DQS")
                        } else {
                            format!("J{pin}{i}_DQS")
                        },
                    );
                    self.add_bel_wire(bcrd, pin, wire);
                    self.claim_pip(wire_out, wire);
                    bel.pins
                        .insert(pin.into(), self.xlat_int_wire(bcrd, wire_out));
                }
            }
            if has_dqs {
                for pin in [
                    "DATAVALID",
                    "BURSTDET",
                    "RST",
                    "SCLK",
                    "READ",
                    "READCLKSEL0",
                    "READCLKSEL1",
                ] {
                    let wire = self.rc_io_wire(
                        cell,
                        &if pin.ends_with(['0', '1']) {
                            format!("J{pin}_{i}_DQS")
                        } else {
                            format!("J{pin}{i}_DQS")
                        },
                    );
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }

                let cell_dqsdll = self.chip.special_loc[&SpecialLocKey::DqsDll(Dir::N)];
                let dqsdel = self.rc_io_wire(cell, &format!("JDQSDEL{i}_DQS"));
                self.add_bel_wire(bcrd, "DQSDEL", dqsdel);
                let wire_dqsdll = self.rc_wire(cell_dqsdll, "JDQSDEL_DQSDLL");
                self.claim_pip(dqsdel, wire_dqsdll);

                let dqsi = self.rc_io_wire(cell, &format!("JDQSI{i}_DQS"));
                self.add_bel_wire(bcrd, "DQSI", dqsi);
                let io = self.chip.special_io[&SpecialIoKey::DqsE(i as u8)];
                let cell_io = self.chip.get_io_loc(io).cell;
                let wire_io = self.rc_io_wire(
                    cell_io,
                    &format!("JDI{abcd}", abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx()]),
                );
                self.claim_pip(dqsi, wire_io);

                self.insert_bel(bcrd, bel);
            }
        }
    }

    fn process_single_io_machxo2(&mut self, bcrd: BelCoord) {
        let is_smol = self.chip.rows[self.chip.row_clk].kind != RowKind::Ebr;
        let io = self.chip.get_io_crd(bcrd);
        let idx = io.iob().to_idx();
        let abcd = ['A', 'B', 'C', 'D'][idx];
        let cell = bcrd.cell;
        let (r, c) = self.rc(cell);
        let mut names = match io.edge() {
            Dir::W => [format!("PL{r}{abcd}"), format!("IOL_L{r}{abcd}")],
            Dir::E => [format!("PR{r}{abcd}"), format!("IOL_R{r}{abcd}")],
            Dir::S => [format!("PB{c}{abcd}"), format!("IOL_B{c}{abcd}")],
            Dir::N => [format!("PT{c}{abcd}"), format!("IOL_T{c}{abcd}")],
        };
        if self.chip.kind == ChipKind::MachXo2(MachXo2Kind::MachNx)
            && let EdgeIoCoord::W(row, iob) = io
            && let Some(name) = match (row.to_idx(), iob.to_idx()) {
                (16, 1) => Some("NXBOOT_MCSN"),
                (11, 1) => Some("NX_PROGRAMN"),
                (11, 2) => Some("NX_JTAGEN"),
                (10, 0) => Some("ICC00"),
                (10, 1) => Some("ICC01"),
                (10, 2) => Some("ICC02"),
                (10, 3) => Some("ICC03"),
                (9, 0) => Some("ICC04"),
                (9, 1) => Some("ICC05"),
                (9, 2) => Some("ICC06"),
                (9, 3) => Some("ICC07"),
                (6, 0) => Some("ICC08"),
                (6, 1) => Some("ICC09"),
                (6, 2) => Some("ICC10"),
                (6, 3) => Some("ICC11"),
                (5, 0) => Some("ICC12"),
                (5, 1) => Some("ICC13"),
                (5, 2) => Some("ICC14"),
                (5, 3) => Some("ICC15"),
                (4, 0) => Some("ICC16"),
                (4, 1) => Some("ICC17"),
                (4, 2) => Some("ICC18"),
                (4, 3) => Some("ICC19"),
                (3, 0) => Some("ICC20"),
                (3, 1) => Some("ICC21"),
                (3, 2) => Some("ICC22"),
                (3, 3) => Some("ICC23"),
                (2, 0) => Some("ICC24"),
                (2, 1) => Some("ICC25"),
                (2, 2) => Some("ICC26"),
                (2, 3) => Some("ICC27"),
                _ => None,
            }
        {
            names[0] = name.to_string();
        }
        if self.chip.kind == ChipKind::MachXo2(MachXo2Kind::MachNx) && io == EdgeIoCoord::N(ColId::from_idx(14), TileIobId::from_idx(2)) {
            // what. this is actually the XO3 TDO -> XO5 TDI pad btw.
            names[0] = "Unused".to_string();
        }
        self.name_bel(bcrd, names);
        let bank = self.chip.get_io_bank(io);
        let iol = if is_smol {
            "IOLOGIC"
        } else {
            match (bank, idx) {
                (0, 0) => "TIOLOGIC",
                (0, 2) => "TSIOLOGIC",
                (2, 0) => "BIOLOGIC",
                (2, 2) => "BSIOLOGIC",
                (1, _) if self.chip.kind != ChipKind::MachXo2(MachXo2Kind::MachXo3L) => "RIOLOGIC",
                _ => "IOLOGIC",
            }
        };
        let mut bel = Bel::default();

        let mut pins = vec!["CLK", "LSR", "CE", "TS", "OPOS", "ONEG", "IP", "IN"];
        match iol {
            "TIOLOGIC" => {
                pins.extend([
                    "TXD0", "TXD1", "TXD2", "TXD3", "TXD4", "TXD5", "TXD6", "TXD7",
                ]);
            }
            "TSIOLOGIC" => {
                pins.extend(["TXD0", "TXD1", "TXD2", "TXD3"]);
            }
            "BIOLOGIC" | "BSIOLOGIC" => {
                pins.extend([
                    "RXD0", "RXD1", "RXD2", "RXD3", "DEL0", "DEL1", "DEL2", "DEL3", "DEL4", "SLIP",
                ]);
            }
            _ => (),
        }
        for pin in pins {
            let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_{iol}"));
            self.add_bel_wire(bcrd, pin, wire);
            bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
        }

        if iol == "BIOLOGIC" {
            for i in 0..8 {
                let wire = self.rc_io_wire(cell, &format!("JRXDA{i}_{iol}"));
                self.add_bel_wire(bcrd, format!("RXDA{i}"), wire);
                bel.pins
                    .insert(format!("RXDA{i}"), self.xlat_int_wire(bcrd, wire));
            }
        }

        let paddi_pio = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDI_PIO", paddi_pio);

        let di = self.rc_io_wire(cell, &format!("JDI{abcd}"));
        self.add_bel_wire(bcrd, "DI", di);
        let di_iol = self.rc_io_wire(cell, &format!("DI{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "DI_IOLOGIC", di_iol);
        let paddi_iol = self.rc_io_wire(cell, &format!("PADDI{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "PADDI_IOLOGIC", paddi_iol);
        let indd = self.rc_io_wire(cell, &format!("INDD{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "INDD", indd);
        self.claim_pip(paddi_iol, paddi_pio);
        self.claim_pip(di_iol, di);
        self.claim_pip(di, indd);
        self.claim_pip(di, paddi_pio);
        if self.is_machnx_io(io) {
            let wire = self.intdb.get_wire(&format!("OUT_Q{idx}"));
            let wire = TileWireCoord {
                cell: CellSlotId::from_idx(0),
                wire,
            };
            bel.pins.insert("DI".into(), BelPin::new_out(wire));
        } else {
            bel.pins.insert("DI".into(), self.xlat_int_wire(bcrd, di));
        }

        let paddo_pio = self.rc_io_wire(cell, &format!("PADDO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDO_PIO", paddo_pio);
        let paddt_pio = self.rc_io_wire(cell, &format!("PADDT{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDT_PIO", paddt_pio);

        if !self.is_machnx_io(io) {
            let paddo = self.rc_io_wire(cell, &format!("JPADDO{abcd}"));
            self.add_bel_wire(bcrd, "PADDO", paddo);
            let paddt = self.rc_io_wire(cell, &format!("JPADDT{abcd}"));
            self.add_bel_wire(bcrd, "PADDT", paddt);
            self.claim_pip(paddo_pio, paddo);
            self.claim_pip(paddt_pio, paddt);
            let bpin = self.xlat_int_wire(bcrd, paddo);
            assert_eq!(bpin, bel.pins["OPOS"]);
            let bpin = self.xlat_int_wire(bcrd, paddt);
            assert_eq!(bpin, bel.pins["TS"]);
        }

        let ioldo_pio = self.rc_io_wire(cell, &format!("IOLDO{abcd}_PIO"));
        let ioldo_iol = self.rc_io_wire(cell, &format!("IOLDO{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLDO_PIO", ioldo_pio);
        self.add_bel_wire(bcrd, "IOLDO_IOLOGIC", ioldo_iol);
        self.claim_pip(ioldo_pio, ioldo_iol);

        let iolto_pio = self.rc_io_wire(cell, &format!("IOLTO{abcd}_PIO"));
        let iolto_iol = self.rc_io_wire(cell, &format!("IOLTO{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLTO_PIO", iolto_pio);
        self.add_bel_wire(bcrd, "IOLTO_IOLOGIC", iolto_iol);
        self.claim_pip(iolto_pio, iolto_iol);

        for (pin, bslot_rc, pin_src) in [
            ("PG", bels::BCPG, "PGENO_OUT"),
            ("INRD", bels::BCINRD, "INRDENO_OUT"),
            ("LVDS", bels::BCLVDSO, "LVDSENO_OUT"),
            ("SLEWRATE", bels::BCSLEWRATE, "SLEWRATEENO_OUT"),
        ] {
            let cell_rc = self.chip.special_loc[&SpecialLocKey::Bc(bank)];
            let bcrd_rc = cell_rc.bel(bslot_rc);
            if !self.edev.egrid.has_bel(bcrd_rc) {
                continue;
            }
            let wire_src = self.naming.bel_wire(bcrd_rc, pin_src);
            let wire = self.rc_io_wire(
                cell,
                &if pin == "SLEWRATE" {
                    format!("J{pin}{abcd}_PIO")
                } else {
                    format!("{pin}{abcd}_PIO")
                },
            );
            self.add_bel_wire(bcrd, pin, wire);
            self.claim_pip(wire, wire_src);
        }

        if !is_smol && matches!(bank, 0 | 2) && matches!(idx, 0 | 2) {
            let eclk_iol = self.rc_io_wire(cell, &format!("ECLK{abcd}_{iol}"));
            let eclk = self.rc_io_wire(cell, &format!("ECLK{abcd}"));
            self.add_bel_wire(bcrd, "ECLK_IOLOGIC", eclk_iol);
            self.add_bel_wire(bcrd, "ECLK", eclk);
            self.claim_pip(eclk_iol, eclk);

            for i in 0..2 {
                let bcrd_eclksync = self.chip.bel_eclksync(io.edge(), i);
                let wire_eclk = self.naming.bel_wire(bcrd_eclksync, "ECLKO_OUT");
                self.claim_pip(eclk, wire_eclk);
            }
        }

        if !is_smol && bank != 0 {
            // dummy
            let wire = self.rc_io_wire(cell, &format!("LVDS{abcd}_PIO"));
            self.add_bel_wire(bcrd, "LVDS", wire);
        }

        if iol == "RIOLOGIC" {
            for pin in ["DQSW90", "DQSR90", "DDRCLKPOL"] {
                let wire = self.rc_io_wire(cell, &format!("{pin}{abcd}_{iol}"));
                self.add_bel_wire(bcrd, pin, wire);
                let bel_dqs =
                    CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk).bel(
                        if cell.row < self.chip.row_clk {
                            bels::DQS1
                        } else {
                            bels::DQS0
                        },
                    );
                let wire_dqs = self.naming.bel_wire(bel_dqs, &format!("{pin}_OUT"));
                self.claim_pip(wire, wire_dqs);
            }
        }

        if matches!(
            self.chip.kind,
            ChipKind::MachXo2(
                MachXo2Kind::MachXo3Lfp | MachXo2Kind::MachXo3D | MachXo2Kind::MachNx
            )
        ) {
            let is_i3c = io.edge() == Dir::W
                && self.chip.rows[cell.row].io_w == IoGroupKind::QuadI3c
                && idx < 2;

            for pin in ["RESEN", "PULLUPEN"] {
                let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_PIO"));
                self.add_bel_wire(bcrd, pin, wire);
                if is_i3c {
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }
            }
        }

        self.insert_bel(bcrd, bel);
    }

    pub(super) fn process_io_machxo2(&mut self) {
        for (row, rd) in &self.chip.rows {
            let num_io = match rd.io_w {
                IoGroupKind::Quad | IoGroupKind::QuadI3c => 4,
                IoGroupKind::Double => 2,
                _ => 0,
            };
            for iob in 0..num_io {
                let io = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                let bcrd = self.chip.get_io_loc(io);
                self.process_single_io_machxo2(bcrd);
            }
            let num_io = match rd.io_e {
                IoGroupKind::Quad => 4,
                IoGroupKind::Double => 2,
                _ => 0,
            };
            for iob in 0..num_io {
                let io = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                let bcrd = self.chip.get_io_loc(io);
                self.process_single_io_machxo2(bcrd);
            }
        }
        for (col, cd) in &self.chip.columns {
            let num_io = match cd.io_s {
                IoGroupKind::Quad | IoGroupKind::QuadReverse => 4,
                IoGroupKind::Double => 2,
                _ => 0,
            };
            for iob in 0..num_io {
                let io = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                let bcrd = self.chip.get_io_loc(io);
                self.process_single_io_machxo2(bcrd);
            }
            let num_io = match cd.io_n {
                IoGroupKind::Quad | IoGroupKind::QuadReverse => 4,
                IoGroupKind::Double => 2,
                _ => 0,
            };
            for iob in 0..num_io {
                let io = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                let bcrd = self.chip.get_io_loc(io);
                self.process_single_io_machxo2(bcrd);
            }
        }
    }

    pub(super) fn process_icc_machxo2(&mut self) {
        if self.chip.kind != ChipKind::MachXo2(MachXo2Kind::MachNx) {
            return;
        }
        // fake loc, fake bel.
        let bcrd = self.chip.special_loc[&SpecialLocKey::Config].bel(bels::IO0);
        self.name_bel(bcrd, ["ICC_R20", "ICCREG_R21", "ICC1_R14"]);
        let cell_icc1 = CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(16));
        for (pin, row, iob) in [
            ("PFRMCLK", 14, 0),
            ("PFRMISO", 13, 1),
            ("PFRMOSI", 13, 0),
            ("PFRCSN", 12, 1),
        ] {
            let row = RowId::from_idx(row);
            let cell_io = cell_icc1.with_row(row);

            let wire_i = self.rc_io_wire(cell_icc1, &format!("J{pin}I_ICC1"));
            self.add_bel_wire(bcrd, format!("{pin}I"), wire_i);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("OUT_Q{iob}")));
            self.claim_pip_int_out(wire_int, wire_i);

            let wire_o = self.rc_io_wire(cell_icc1, &format!("J{pin}O_ICC1"));
            self.add_bel_wire(bcrd, format!("{pin}O"), wire_o);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_A{iob}")));
            self.claim_pip_int_in(wire_o, wire_int);

            let wire_oe = self.rc_io_wire(cell_icc1, &format!("J{pin}OE_ICC1"));
            self.add_bel_wire(bcrd, format!("{pin}OE"), wire_oe);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_C{iob}")));
            self.claim_pip_int_in(wire_oe, wire_int);
        }
        {
            let (pin, row, iob) = ("PFRMCSN", 16, 1);
            let row = RowId::from_idx(row);
            let cell_io = cell_icc1.with_row(row);
            let wire = self.rc_io_wire(cell_icc1, &format!("J{pin}_ICC1"));
            self.add_bel_wire(bcrd, pin, wire);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("OUT_Q{iob}")));
            self.claim_pip_int_out(wire_int, wire);
        }
        for (pin, row, iob) in [("PFRPRMN", 11, 1), ("PFRJTAGEN", 11, 2)] {
            let row = RowId::from_idx(row);
            let cell_io = cell_icc1.with_row(row);
            let wire = self.rc_io_wire(cell_icc1, &format!("J{pin}_ICC1"));
            self.add_bel_wire(bcrd, pin, wire);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_A{iob}")));
            self.claim_pip_int_in(wire, wire_int);
        }
        let cell_icc = CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(10));
        let cell_iccreg =
            CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(9));
        for (pin_icc, pin_iccreg, row, iob) in [
            ("TXVIO0", "TXV0", 3, 2),
            ("TXVIO1", "TXV1", 3, 3),
            ("AUX", "TXAUX", 2, 3),
            ("INT", "TXINT", 2, 2),
            ("TXDATA0", "TXD0", 5, 2),
            ("TXDATA1", "TXD1", 5, 3),
            ("TXDATA2", "TXD2", 4, 0),
            ("TXDATA3", "TXD3", 4, 1),
            ("TXDATA4", "TXD4", 4, 2),
            ("TXDATA5", "TXD5", 4, 3),
            ("TXDATA6", "TXD6", 3, 1),
            ("TXDATA7", "TXD7", 10, 0),
            ("TXCLK", "TXCLK", 10, 2),
            ("TXVALID", "TXVLD", 5, 1),
            ("TXREADY", "TXRDY", 5, 0),
        ] {
            let wire_icc = self.rc_io_wire(cell_icc, &format!("J{pin_icc}_ICC"));
            self.add_bel_wire(bcrd, pin_icc, wire_icc);
            let wire_q = self.rc_io_wire(cell_iccreg, &format!("JQ{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("Q_{pin_iccreg}"), wire_q);
            let wire_d = self.rc_io_wire(cell_iccreg, &format!("JD{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("D_{pin_iccreg}"), wire_d);
            let row = RowId::from_idx(row);
            let cell_io = cell_icc1.with_row(row);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_A{iob}")));
            self.claim_pip_int_in(wire_icc, wire_int);
            self.claim_pip_int_in(wire_d, wire_int);
            self.claim_pip(wire_icc, wire_q);

            let wire_clk = self.rc_io_wire(cell_iccreg, &format!("JCK{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("CLK_{pin_iccreg}"), wire_clk);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_CLK{iob}")));
            self.claim_pip_int_in(wire_clk, wire_int);

            let wire_rst = self.rc_io_wire(cell_iccreg, &format!("JRS{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("RST_{pin_iccreg}"), wire_rst);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_LSR{iob}")));
            self.claim_pip_int_in(wire_rst, wire_int);

            let wire_ce = self.rc_io_wire(cell_iccreg, &format!("JCE{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("CE_{pin_iccreg}"), wire_ce);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_CE{iob}")));
            self.claim_pip_int_in(wire_ce, wire_int);
            if pin_icc == "TXCLK" {
                // ???
                let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_B{iob}")));
                self.claim_pip_int_in(wire_ce, wire_int);
            }
        }
        for (pin_icc, pin_iccreg, row, iob) in [
            ("RXVIO0", "RXV0", 2, 0),
            ("RXVIO1", "RXV1", 2, 1),
            ("RXDATA0", "RXD0", 10, 3),
            ("RXDATA1", "RXD1", 9, 0),
            ("RXDATA2", "RXD2", 9, 1),
            ("RXDATA3", "RXD3", 9, 2),
            ("RXDATA4", "RXD4", 9, 3),
            ("RXDATA5", "RXD5", 6, 0),
            ("RXDATA6", "RXD6", 6, 1),
            ("RXDATA7", "RXD7", 6, 2),
            ("RXVALID", "RXVLD", 6, 3),
            ("RXREADY", "RXRDY", 10, 1),
        ] {
            let wire_icc = self.rc_io_wire(cell_icc, &format!("J{pin_icc}_ICC"));
            self.add_bel_wire(bcrd, pin_icc, wire_icc);
            let wire_q = self.rc_io_wire(cell_iccreg, &format!("JQ{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("Q_{pin_iccreg}"), wire_q);
            let wire_d = self.rc_io_wire(cell_iccreg, &format!("JD{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("D_{pin_iccreg}"), wire_d);
            let row = RowId::from_idx(row);
            let cell_io = cell_icc1.with_row(row);
            self.claim_pip(wire_d, wire_icc);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("OUT_F{iob}")));
            self.claim_pip_int_out(wire_int, wire_q);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("OUT_Q{iob}")));
            self.claim_pip_int_out(wire_int, wire_icc);

            let wire_clk = self.rc_io_wire(cell_iccreg, &format!("JCK{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("CLK_{pin_iccreg}"), wire_clk);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_CLK{iob}")));
            self.claim_pip_int_in(wire_clk, wire_int);

            let wire_rst = self.rc_io_wire(cell_iccreg, &format!("JRS{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("RST_{pin_iccreg}"), wire_rst);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_LSR{iob}")));
            self.claim_pip_int_in(wire_rst, wire_int);

            let wire_ce = self.rc_io_wire(cell_iccreg, &format!("JCE{pin_iccreg}_ICCREG"));
            self.add_bel_wire(bcrd, format!("CE_{pin_iccreg}"), wire_ce);
            let wire_int = cell_io.wire(self.intdb.get_wire(&format!("IMUX_CE{iob}")));
            self.claim_pip_int_in(wire_ce, wire_int);
        }

        let wire_icc = self.rc_io_wire(cell_icc, "JRXCLK_ICC");
        self.add_bel_wire(bcrd, "RXCLK", wire_icc);
        let row = RowId::from_idx(3);
        let cell_io = cell_icc1.with_row(row);
        let wire_int = cell_io.wire(self.intdb.get_wire("OUT_Q0"));
        self.claim_pip_int_out(wire_int, wire_icc);
        let cell_dlldel = cell_icc1.with_row(self.chip.row_clk);
        let wire_clki = self.rc_io_wire(cell_dlldel, "JCLKI0_DLLDEL");
        let wire_paddi = self.rc_io_wire(cell_dlldel, "JPADDI0");
        self.claim_pip(wire_clki, wire_icc);
        self.claim_pip(wire_paddi, wire_icc);
    }
}

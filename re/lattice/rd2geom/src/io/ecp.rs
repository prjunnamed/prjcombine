use std::collections::BTreeMap;

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, IoGroupKind, PllLoc, PllPad, RowKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, TileWireCoord},
    dir::{Dir, DirH, DirV},
    grid::{BelCoord, CellCoord, DieId},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_single_io_ecp(&mut self, bcrd: BelCoord, mut dummy: bool, is_dqs: bool) {
        let is_ecp2 = !matches!(self.chip.kind, ChipKind::Ecp | ChipKind::Xp);
        let idx = bels::IO.iter().position(|&slot| slot == bcrd.slot).unwrap();
        let cell = bcrd.cell;
        let ab = ['A', 'B'][idx];
        let io = self.chip.get_io_crd(bcrd);
        let (r, c) = self.rc(cell);
        let names = match io.edge() {
            Dir::W => [format!("PL{r}{ab}"), format!("IOL_L{r}{ab}")],
            Dir::E => [format!("PR{r}{ab}"), format!("IOL_R{r}{ab}")],
            Dir::S => [format!("PB{c}{ab}"), format!("IOL_B{c}{ab}")],
            Dir::N => [format!("PT{c}{ab}"), format!("IOL_T{c}{ab}")],
        };
        self.name_bel(bcrd, names);

        if self.chip.kind == ChipKind::Xp
            && self.chip.rows.len() == 36
            && cell.col == self.chip.col_e() - 1
        {
            dummy = false;
        }

        if self.chip.kind == ChipKind::Xp
            && self.chip.rows.len() == 48
            && (cell.col == self.chip.col_w() + 1
                || cell.col == self.chip.col_w() + 2
                || cell.col == self.chip.col_e() - 1
                || cell.col == self.chip.col_e() - 2)
        {
            dummy = false;
        }

        let very_dummy = dummy
            && self.chip.rows.len() == 44
            && (cell.col == self.chip.col_w() + 1 || cell.col == self.chip.col_e() - 1);

        let mut bel = Bel::default();
        let td_iologic = self.rc_io_wire(cell, &format!("JTD{ab}_IOLOGIC"));
        let opos0_iologic = self.rc_io_wire(cell, &format!("JOPOS0{ab}_IOLOGIC"));
        let opos1_iologic = self.rc_io_wire(cell, &format!("JOPOS1{ab}_IOLOGIC"));
        let oneg0_iologic = self.rc_io_wire(cell, &format!("JONEG0{ab}_IOLOGIC"));
        let oneg1_iologic = self.rc_io_wire(cell, &format!("JONEG1{ab}_IOLOGIC"));
        let ipos0_iologic = self.rc_io_wire(cell, &format!("JIPOS0{ab}_IOLOGIC"));
        let ipos1_iologic = self.rc_io_wire(cell, &format!("JIPOS1{ab}_IOLOGIC"));
        let inff_iologic = self.rc_io_wire(cell, &format!("JINFF{ab}_IOLOGIC"));
        let clk_iologic = self.rc_io_wire(cell, &format!("JCLK{ab}_IOLOGIC"));
        let lsr_iologic = self.rc_io_wire(cell, &format!("JLSR{ab}_IOLOGIC"));
        let ce_iologic = self.rc_io_wire(cell, &format!("JCE{ab}_IOLOGIC"));
        let di_iologic = self.rc_io_wire(cell, &format!("DI{ab}_IOLOGIC"));
        let indd_iologic = self.rc_io_wire(cell, &format!("JINDD{ab}_IOLOGIC"));
        let dqs_iologic = self.rc_io_wire(cell, &format!("JDQS{ab}_IOLOGIC"));
        let ddrclkpol_iologic = self.rc_io_wire(cell, &format!("JDDRCLKPOL{ab}_IOLOGIC"));
        let paddo_pio = self.rc_io_wire(cell, &format!("JPADDO{ab}_PIO"));
        let paddt_pio = self.rc_io_wire(cell, &format!("JPADDT{ab}_PIO"));
        let paddi_pio = self.rc_io_wire(cell, &format!("JPADDI{ab}_PIO"));
        let iolto_pio = self.rc_io_wire(cell, &format!("IOLTO{ab}_PIO"));
        let ioldo_pio = self.rc_io_wire(cell, &format!("IOLDO{ab}_PIO"));
        let iolto_iologic = self.rc_io_wire(cell, &format!("IOLTO{ab}_IOLOGIC"));
        let ioldo_iologic = self.rc_io_wire(cell, &format!("IOLDO{ab}_IOLOGIC"));

        self.add_bel_wire(bcrd, "TD_IOLOGIC", td_iologic);
        self.add_bel_wire(bcrd, "OPOS0", opos0_iologic);
        self.add_bel_wire(bcrd, "OPOS1", opos1_iologic);
        self.add_bel_wire(bcrd, "ONEG0", oneg0_iologic);
        self.add_bel_wire(bcrd, "ONEG1", oneg1_iologic);
        self.add_bel_wire(bcrd, "IPOS0", ipos0_iologic);
        self.add_bel_wire(bcrd, "IPOS1", ipos1_iologic);
        self.add_bel_wire(bcrd, "INFF", inff_iologic);
        self.add_bel_wire(bcrd, "CLK", clk_iologic);
        self.add_bel_wire(bcrd, "LSR", lsr_iologic);
        self.add_bel_wire(bcrd, "CE", ce_iologic);
        self.add_bel_wire(bcrd, "DI_IOLOGIC", di_iologic);
        self.add_bel_wire(bcrd, "INDD_IOLOGIC", indd_iologic);
        self.add_bel_wire(bcrd, "DQS_IOLOGIC", dqs_iologic);
        self.add_bel_wire(bcrd, "DDRCLKPOLA_IOLOGIC", ddrclkpol_iologic);
        self.add_bel_wire(bcrd, "PADDO_PIO", paddo_pio);
        self.add_bel_wire(bcrd, "PADDT_PIO", paddt_pio);
        self.add_bel_wire(bcrd, "PADDI_PIO", paddi_pio);
        self.add_bel_wire(bcrd, "IOLTO_PIO", iolto_pio);
        self.add_bel_wire(bcrd, "IOLDO_PIO", ioldo_pio);
        self.add_bel_wire(bcrd, "IOLTO_IOLOGIC", iolto_iologic);
        self.add_bel_wire(bcrd, "IOLDO_IOLOGIC", ioldo_iologic);

        if is_ecp2 {
            let dqsxfer_iologic = self.rc_io_wire(cell, &format!("JDQSXFER{ab}_IOLOGIC"));
            self.add_bel_wire(bcrd, "DQSXFER_IOLOGIC", dqsxfer_iologic);

            let xclk = self.rc_io_wire(cell, &format!("JXCLK{ab}_IOLOGIC"));
            self.add_bel_wire(bcrd, "XCLK", xclk);

            let eclki_iologic = self.rc_io_wire(cell, &format!("JECLKI{ab}_IOLOGIC"));
            let eclko_iologic = self.rc_io_wire(cell, &format!("JECLKO{ab}_IOLOGIC"));
            let eclki = self.rc_io_wire(cell, &format!("JECLKI{ab}"));
            let eclko = self.rc_io_wire(cell, &format!("JECLKO{ab}"));
            self.add_bel_wire(bcrd, "ECLKI_IOLOGIC", eclki_iologic);
            self.add_bel_wire(bcrd, "ECLKO_IOLOGIC", eclko_iologic);
            self.add_bel_wire(bcrd, "ECLKI", eclki);
            self.add_bel_wire(bcrd, "ECLKO", eclko);
            self.claim_pip(eclki_iologic, eclki);
            self.claim_pip(eclko_iologic, eclko);

            let bel_eclk = self.chip.bel_eclk_root(io.edge());
            let eclk0 = self.naming.bel_wire(bel_eclk, "ECLK0");
            let eclk1 = self.naming.bel_wire(bel_eclk, "ECLK1");
            self.claim_pip(eclki, eclk0);
            self.claim_pip(eclki, eclk1);
            self.claim_pip(eclko, eclk0);
            self.claim_pip(eclko, eclk1);
        };

        if !very_dummy {
            let di = self.rc_io_wire(cell, &format!("JDI{ab}"));
            self.add_bel_wire(bcrd, "DI", di);

            let bpin_opos0 = self.xlat_int_wire(bcrd, opos0_iologic);
            let bpin_opos1 = self.xlat_int_wire(bcrd, opos1_iologic);
            let bpin_oneg0 = self.xlat_int_wire(bcrd, oneg0_iologic);
            let bpin_oneg1 = self.xlat_int_wire(bcrd, oneg1_iologic);
            let bpin_oneg1_alt = self.xlat_int_wire(bcrd, td_iologic);
            assert_eq!(bpin_oneg1, bpin_oneg1_alt);
            let bpin_oneg0_alt = self.xlat_int_wire(bcrd, paddo_pio);
            assert_eq!(bpin_oneg0, bpin_oneg0_alt);
            let bpin_ipos0 = self.xlat_int_wire(bcrd, ipos0_iologic);
            let bpin_ipos1 = self.xlat_int_wire(bcrd, ipos1_iologic);
            let bpin_clk = self.xlat_int_wire(bcrd, clk_iologic);
            let bpin_lsr = self.xlat_int_wire(bcrd, lsr_iologic);
            let bpin_ce = self.xlat_int_wire(bcrd, ce_iologic);
            let bpin_td = self.xlat_int_wire(bcrd, paddt_pio);
            let bpin_inff = self.xlat_int_wire(bcrd, inff_iologic);
            let bpin_di = self.xlat_int_wire(bcrd, di);
            assert_eq!(bpin_ipos0, bpin_inff);
            bel.pins.insert("OPOS0".into(), bpin_opos0);
            bel.pins.insert("OPOS1".into(), bpin_opos1);
            bel.pins.insert("ONEG0".into(), bpin_oneg0);
            bel.pins.insert("ONEG1".into(), bpin_oneg1);
            bel.pins.insert("IPOS0".into(), bpin_ipos0);
            bel.pins.insert("IPOS1".into(), bpin_ipos1);
            bel.pins.insert("DI".into(), bpin_di);
            bel.pins.insert("TD".into(), bpin_td);
            bel.pins.insert("CLK".into(), bpin_clk);
            bel.pins.insert("LSR".into(), bpin_lsr);
            bel.pins.insert("CE".into(), bpin_ce);

            if is_ecp2 {
                let pins = if idx == 0 {
                    [
                        "DEL0", "DEL1", "DEL2", "DEL3", "OPOS2", "ONEG2", "QPOS0", "QPOS1",
                        "QNEG0", "QNEG1",
                    ]
                    .as_slice()
                } else {
                    ["DEL0", "DEL1", "DEL2", "DEL3", "QPOS0", "QPOS1"].as_slice()
                };
                for &pin in pins {
                    let wire = self.rc_io_wire(cell, &format!("J{pin}{ab}_IOLOGIC"));
                    self.add_bel_wire(bcrd, pin, wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);
                    bel.pins.insert(pin.into(), bpin);
                }
                if idx == 1 {
                    for pin in ["QNEG0", "QNEG1", "OPOS2", "ONEG2"] {
                        let wire = self.rc_io_wire(cell, &format!("J{pin}{ab}_IOLOGIC"));
                        self.add_bel_wire(bcrd, pin, wire);
                    }
                }
            }

            self.insert_bel(bcrd, bel);

            if !dummy {
                let indd = self.rc_io_wire(cell, &format!("JINDD{ab}"));
                let paddi = self.rc_io_wire(cell, &format!("JPADDI{ab}"));
                self.add_bel_wire(bcrd, "INDD", indd);
                self.add_bel_wire(bcrd, "PADDI", paddi);
                self.claim_pip(indd, indd_iologic);
                self.claim_pip(paddi, paddi_pio);
                self.claim_pip(di, indd);
                self.claim_pip(di, paddi);
            }

            if self.edev.dqs.contains_key(&cell)
                || self.chip.kind == ChipKind::Ecp
                || self.chip.kind == ChipKind::Xp2
                || (matches!(self.chip.kind, ChipKind::Ecp2 | ChipKind::Ecp2M)
                    && io.edge() != Dir::N)
                || (self.chip.kind == ChipKind::Xp
                    && self.chip.rows.len() == 48
                    && matches!(cell.row.to_idx(), 22..26))
            {
                let dqs = self.rc_io_wire(cell, "JDQS");
                let ddrclkpol = self.rc_io_wire(cell, "JDDRCLKPOL");
                if idx == 0 {
                    self.add_bel_wire(bcrd, "DQS", dqs);
                    self.add_bel_wire(bcrd, "DDRCLKPOL", ddrclkpol);
                    if let Some(&cell_dqs) = self.edev.dqs.get(&cell) {
                        let bel_dqs = cell_dqs.bel(bels::DQS0);
                        let dqs_tree = self.naming.bel_wire(bel_dqs, "DQSO_TREE");
                        let ddrclkpol_tree = self.naming.bel_wire(bel_dqs, "DDRCLKPOL_TREE");
                        self.claim_pip(dqs, dqs_tree);
                        self.claim_pip(ddrclkpol, ddrclkpol_tree);
                    }
                } else {
                    self.add_bel_wire_no_claim(bcrd, "DQS", dqs);
                    self.add_bel_wire_no_claim(bcrd, "DDRCLKPOL", ddrclkpol);
                }
                if !dummy {
                    let skip_self_dqs = match self.chip.kind {
                        ChipKind::Ecp2 | ChipKind::Xp2 => true,
                        ChipKind::Ecp2M => self.chip.rows.len() != 55,
                        _ => false,
                    };
                    if !(skip_self_dqs && is_dqs) {
                        self.claim_pip(dqs_iologic, dqs);
                    }
                    self.claim_pip(ddrclkpol_iologic, ddrclkpol);
                }
                if is_ecp2 {
                    let dqsxfer_iologic = self.rc_io_wire(cell, &format!("JDQSXFER{ab}_IOLOGIC"));
                    let dqsxfer = self.rc_io_wire(cell, "JDQSXFER");
                    if idx == 0 {
                        self.add_bel_wire(bcrd, "DQSXFER", dqsxfer);
                        if let Some(&cell_dqs) = self.edev.dqs.get(&cell) {
                            let bel_dqs = cell_dqs.bel(bels::DQS0);
                            let dqsxfer_tree = self.naming.bel_wire(bel_dqs, "DQSXFER_TREE");
                            self.claim_pip(dqsxfer, dqsxfer_tree);
                        }
                    } else {
                        self.add_bel_wire_no_claim(bcrd, "DQSXFER", dqsxfer);
                    }
                    self.claim_pip(dqsxfer_iologic, dqsxfer);
                }
            }
        }

        self.claim_pip(di_iologic, paddi_pio);
        self.claim_pip(iolto_pio, iolto_iologic);
        self.claim_pip(ioldo_pio, ioldo_iologic);
    }

    fn process_dqs_ecp(&mut self, bcrd: BelCoord) {
        let is_ecp2 = !matches!(self.chip.kind, ChipKind::Ecp | ChipKind::Xp);
        let cell = bcrd.cell;
        let io = self.chip.get_io_crd(bcrd.bel(bels::IO0));
        let (r, c) = self.rc(cell);
        let name = match io.edge() {
            Dir::W => format!("LDQS{r}"),
            Dir::E => format!("RDQS{r}"),
            Dir::S => format!("BDQS{c}"),
            Dir::N => format!("TDQS{c}"),
        };
        self.name_bel(bcrd, [name]);
        let mut bel = self.extract_simple_bel(bcrd, cell, "DQS");
        let ddrclkpol = self.rc_io_wire(cell, "JDDRCLKPOL_DQS");
        let ddrclkpol_out = self.rc_io_wire(cell, if is_ecp2 { "CLKPOL2PIC" } else { "DDRCLKPOL" });
        let dqsdel = self.rc_io_wire(cell, "JDQSDEL_DQS");
        let dqsc = self.rc_io_wire(cell, "JDQSC_DQS");
        let dqsi = self.rc_io_wire(cell, "JDQSI_DQS");
        let dqso = self.rc_io_wire(cell, "DQSO_DQS");
        let dqso_out = self.rc_io_wire(cell, if is_ecp2 { "DQSO2PIC" } else { "DQSO" });
        let indqsa = self.rc_io_wire(cell, "JINDQSA");
        let io0_di = self.rc_io_wire(cell, "JDIA");
        let dqso_tree = self.find_single_out(dqso_out);
        let ddrclkpol_tree = self.pips_fwd[&ddrclkpol_out]
            .iter()
            .copied()
            .find(|w| !self.int_wires.contains_key(w))
            .unwrap();
        if io.edge() == Dir::S || (is_ecp2 && io.edge() == Dir::W) {
            // fuck this shit.
            self.add_bel_wire(bcrd, "DDRCLKPOL", ddrclkpol);
            let bpin = self.xlat_int_wire(bcrd, ddrclkpol_out);
            bel.pins.insert("DDRCLKPOL".into(), bpin);
        }
        self.add_bel_wire(bcrd, "DDRCLKPOL_OUT", ddrclkpol_out);
        self.add_bel_wire(bcrd, "DQSDEL", dqsdel);
        self.add_bel_wire(bcrd, "DQSC", dqsc);
        self.add_bel_wire(bcrd, "DQSI", dqsi);
        self.add_bel_wire(bcrd, "DQSO", dqso);
        self.add_bel_wire(bcrd, "DQSO_OUT", dqso_out);
        self.add_bel_wire(bcrd, "INDQSA", indqsa);
        if self.chip.kind == ChipKind::Xp
            && self.chip.rows.len() == 27
            && (matches!(cell.col.to_idx(), 13 | 29))
        {
            // ??? idk I guess the database is fucked
            self.add_bel_wire_no_claim(bcrd, "DQSO_TREE", dqso_tree);
            self.add_bel_wire_no_claim(bcrd, "DDRCLKPOL_TREE", ddrclkpol_tree);
        } else {
            self.add_bel_wire(bcrd, "DQSO_TREE", dqso_tree);
            self.add_bel_wire(bcrd, "DDRCLKPOL_TREE", ddrclkpol_tree);
        }
        let wire_io = self.get_io_wire_in(io);
        self.claim_pip(dqsi, wire_io);
        self.claim_pip(indqsa, dqsc);
        self.claim_pip(io0_di, indqsa);
        self.claim_pip(dqso_out, dqso);
        self.claim_pip(dqso_tree, dqso_out);
        self.claim_pip(ddrclkpol_out, ddrclkpol);
        self.claim_pip(ddrclkpol_tree, ddrclkpol_out);

        if is_ecp2 {
            let dqsxfer = self.rc_io_wire(cell, "DQSXFER_DQS");
            let dqsxfer_out = self.rc_io_wire(cell, "DQSXFER2PIC");
            let dqsxfer_tree = self.pips_fwd[&dqsxfer_out]
                .iter()
                .copied()
                .find(|w| !self.int_wires.contains_key(w))
                .unwrap();
            self.add_bel_wire(bcrd, "DQSXFER", dqsxfer);
            self.add_bel_wire(bcrd, "DQSXFER_OUT", dqsxfer_out);
            self.add_bel_wire(bcrd, "DQSXFER_TREE", dqsxfer_tree);
            self.claim_pip(dqsxfer_out, dqsxfer);
            self.claim_pip(dqsxfer_tree, dqsxfer_out);

            let xclk = self.rc_io_wire(cell, "JXCLK_DQS");
            let xclk_iologic = self.rc_io_wire(cell, "JXCLKA_IOLOGIC");
            self.add_bel_wire(bcrd, "XCLK", xclk);
            self.claim_pip(xclk, xclk_iologic);
        }

        let bcrd_dqsdll = self.chip.bel_dqsdll(cell);
        let dqsdel_dqsdll = self.naming.bel_wire(bcrd_dqsdll, "DQSDEL");
        self.claim_pip(dqsdel, dqsdel_dqsdll);

        self.insert_bel(bcrd, bel);
    }

    pub(super) fn process_dqsdll_ecp(&mut self) {
        for edge in [DirV::S, DirV::N] {
            let bcrd = self.chip.bel_dqsdll_ecp(edge);
            self.name_bel(
                bcrd,
                [match edge {
                    DirV::S => "BDLL",
                    DirV::N => "TDLL",
                }],
            );
            let cell = bcrd.cell.with_col(self.chip.col_clk - 1);
            self.insert_simple_bel(bcrd, cell, "DQSDLL");
            let dqsdel = self.rc_io_wire(cell, "JDQSDEL_DQSDLL");
            self.add_bel_wire(bcrd, "DQSDEL", dqsdel);
        }
    }

    pub(super) fn process_dqsdll_ecp2(&mut self) {
        for edge in [DirH::W, DirH::E] {
            let bcrd = self.chip.bel_dqsdll_ecp2(edge);
            let cell = bcrd.cell;
            self.name_bel(
                bcrd,
                [match edge {
                    DirH::W => "LDQSDLL",
                    DirH::E => "RDQSDLL",
                }],
            );
            self.insert_simple_bel(bcrd, cell, "DQSDLL");
            let dqsdel = self.rc_io_wire(cell, "JDQSDEL_DQSDLL");
            self.add_bel_wire(bcrd, "DQSDEL", dqsdel);
        }
    }

    fn process_io_cell_ecp(&mut self, cell: CellCoord, kind: IoGroupKind) {
        let is_ecp2 = !matches!(self.chip.kind, ChipKind::Ecp | ChipKind::Xp);
        if is_ecp2 && matches!(kind, IoGroupKind::None | IoGroupKind::Serdes) {
            return;
        }
        self.process_single_io_ecp(
            cell.bel(bels::IO0),
            matches!(kind, IoGroupKind::DoubleB | IoGroupKind::None),
            kind == IoGroupKind::DoubleDqs,
        );
        self.process_single_io_ecp(
            cell.bel(bels::IO1),
            matches!(kind, IoGroupKind::DoubleA | IoGroupKind::None),
            false,
        );
    }

    pub(super) fn process_io_ecp(&mut self) {
        let die = DieId::from_idx(0);
        for cell in self.edev.egrid.column(die, self.chip.col_w()) {
            if self.chip.rows[cell.row].io_w == IoGroupKind::DoubleDqs {
                self.process_dqs_ecp(cell.bel(bels::DQS0));
            }
        }
        for cell in self.edev.egrid.column(die, self.chip.col_e()) {
            if self.chip.rows[cell.row].io_e == IoGroupKind::DoubleDqs {
                self.process_dqs_ecp(cell.bel(bels::DQS0));
            }
        }
        for cell in self.edev.egrid.row(die, self.chip.row_s()) {
            if self.chip.columns[cell.col].io_s == IoGroupKind::DoubleDqs {
                self.process_dqs_ecp(cell.bel(bels::DQS0));
            }
        }
        for cell in self.edev.egrid.row(die, self.chip.row_n()) {
            if self.chip.columns[cell.col].io_n == IoGroupKind::DoubleDqs {
                self.process_dqs_ecp(cell.bel(bels::DQS0));
            }
        }
        for cell in self.edev.egrid.column(die, self.chip.col_w()) {
            let rd = &self.chip.rows[cell.row];
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            self.process_io_cell_ecp(cell, rd.io_w);
        }
        for cell in self.edev.egrid.column(die, self.chip.col_e()) {
            let rd = &self.chip.rows[cell.row];
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            self.process_io_cell_ecp(cell, rd.io_e);
        }
        for cell in self.edev.egrid.row(die, self.chip.row_s()) {
            if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                continue;
            }
            let cd = &self.chip.columns[cell.col];
            self.process_io_cell_ecp(cell, cd.io_s);
        }
        for cell in self.edev.egrid.row(die, self.chip.row_n()) {
            if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                continue;
            }
            let cd = &self.chip.columns[cell.col];
            self.process_io_cell_ecp(cell, cd.io_n);
        }
    }

    pub(super) fn process_eclk_ecp2(&mut self) {
        let die = DieId::from_idx(0);
        for (edge, io) in [
            (
                Dir::W,
                self.edev
                    .egrid
                    .column(die, self.chip.col_w())
                    .find(|cell| self.chip.rows[cell.row].io_w == IoGroupKind::Double)
                    .unwrap(),
            ),
            (
                Dir::E,
                self.edev
                    .egrid
                    .column(die, self.chip.col_e())
                    .find(|cell| self.chip.rows[cell.row].io_e == IoGroupKind::Double)
                    .unwrap(),
            ),
            (
                Dir::S,
                self.edev
                    .egrid
                    .row(die, self.chip.row_s())
                    .find(|cell| self.chip.columns[cell.col].io_s == IoGroupKind::Double)
                    .unwrap(),
            ),
            (
                Dir::N,
                self.edev
                    .egrid
                    .row(die, self.chip.row_n())
                    .find(|cell| self.chip.columns[cell.col].io_n == IoGroupKind::Double)
                    .unwrap(),
            ),
        ] {
            let eclki = self.rc_io_wire(io, "JECLKIA");
            let eclks = self.pips_bwd[&eclki].clone();
            let bcrd = self.chip.bel_eclk_root(edge);
            let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
            let mut bel = Bel::default();
            self.name_bel_null(bcrd);
            for i in 0..2 {
                let eclk = eclks
                    .iter()
                    .copied()
                    .find(|wn| self.naming.strings[wn.suffix].ends_with(['0', '1'][i]))
                    .unwrap();
                self.add_bel_wire(bcrd, format!("ECLK{i}"), eclk);

                let eclk_in = if matches!(edge, Dir::H(_)) {
                    let eclk_in = self.claim_single_in(eclk);
                    self.add_bel_wire(bcrd, format!("ECLK{i}_IN"), eclk_in);
                    eclk_in
                } else {
                    eclk
                };

                let mut inps = BTreeMap::new();
                for &wn in &self.pips_bwd[&eclk_in] {
                    inps.insert(self.naming.strings[wn.suffix].clone(), wn);
                }

                let eclk_io = inps.remove(&format!("JPIO{i}")).unwrap();
                self.add_bel_wire(bcrd, format!("ECLK{i}_IO"), eclk_io);
                self.claim_pip(eclk_in, eclk_io);
                let wire_io = self.get_special_io_wire_in(SpecialIoKey::Clock(edge, i as u8));
                self.claim_pip(eclk_io, wire_io);

                let eclk_int = inps.remove(&format!("JCIBCLK{i}")).unwrap();
                self.add_bel_wire(bcrd, format!("ECLK{i}_INT"), eclk_int);
                self.claim_pip(eclk_in, eclk_int);
                let bpin = self.xlat_int_wire(bcrd, eclk_int);
                bel.pins.insert(format!("ECLK{i}_IN"), bpin);

                if let Dir::H(h) = edge {
                    let eclk_pll = inps.remove(&format!("JFRC{i}")).unwrap();
                    self.add_bel_wire(bcrd, format!("ECLK{i}_PLL"), eclk_pll);
                    self.claim_pip(eclk_in, eclk_pll);
                    let cell_pll =
                        self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new_hv(h, DirV::S, 0))];
                    let wire_pll = self.rc_wire(cell_pll, &format!("JFRC{i}"));
                    self.claim_pip(eclk_pll, wire_pll);

                    let wire = self.intdb.get_wire(["OUT_F6", "OUT_F7"][i]);
                    let wire = TileWireCoord::new_idx(0, wire);
                    bel.pins
                        .insert(format!("PAD{i}_OUT"), BelPin::new_out(wire));
                    let wire = self.edev.egrid.tile_wire(tcrd, wire);
                    self.claim_pip_int_out(wire, wire_io);
                } else {
                    for ci in 0..2 {
                        let wire = self.intdb.get_wire(["OUT_F6", "OUT_F7"][i]);
                        let wire = TileWireCoord::new_idx(ci, wire);
                        bel.pins
                            .insert(format!("PAD{i}_OUT{ci}"), BelPin::new_out(wire));
                        let wire = self.edev.egrid.tile_wire(tcrd, wire);
                        self.claim_pip_int_out(wire, wire_io);
                    }
                }

                assert!(inps.is_empty());
            }
            self.insert_bel(bcrd, bel);
        }
    }

    pub(super) fn process_eclk_xp2(&mut self) {
        for (lrbt, edge) in [('L', Dir::W), ('R', Dir::E), ('B', Dir::S), ('T', Dir::N)] {
            let bcrd = self.chip.bel_eclk_root(edge);
            let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
            let cell = match edge {
                Dir::H(_) => bcrd.cell,
                Dir::V(_) => bcrd.cell.delta(-1, 0),
            };
            let mut bel = Bel::default();
            self.name_bel_null(bcrd);

            let mut wires_pllpio = vec![];
            let mut wires_pllclkop = vec![];
            let mut wires_pllclkos = vec![];

            if let Dir::H(h) = edge {
                for v in [DirV::S, DirV::N] {
                    let pll_loc = PllLoc::new_hv(h, v, 0);
                    if let Some(&cell_pll) = self.chip.special_loc.get(&SpecialLocKey::Pll(pll_loc))
                    {
                        let idx = match (h, v) {
                            (DirH::W, DirV::S) => 1,
                            (DirH::W, DirV::N) => 0,
                            (DirH::E, DirV::S) => 0,
                            (DirH::E, DirV::N) => 1,
                        };

                        let wire_io_pll =
                            self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn0, pll_loc));
                        let wire_io = self.rc_wire(cell, &format!("JPLLPIO{idx}"));
                        self.add_bel_wire(bcrd, format!("PLL_{v}{h}_IO"), wire_io);
                        self.claim_pip(wire_io, wire_io_pll);
                        wires_pllpio.push(wire_io);

                        let wire_clkop_pll = self.rc_wire(cell_pll, "JCLKOP_PLL");
                        let wire_clkop = self.rc_wire(cell, &format!("JPLLCLKOP{idx}"));
                        self.add_bel_wire(bcrd, format!("PLL_{v}{h}_CLKOP"), wire_clkop);
                        self.claim_pip(wire_clkop, wire_clkop_pll);
                        wires_pllclkop.push(wire_clkop);

                        let wire_clkos_pll = self.rc_wire(cell_pll, "JCLKOS_PLL");
                        let wire_clkos = self.rc_wire(cell, &format!("JPLLCLKOS{idx}"));
                        self.add_bel_wire(bcrd, format!("PLL_{v}{h}_CLKOS"), wire_clkos);
                        self.claim_pip(wire_clkos, wire_clkos_pll);
                        wires_pllclkos.push(wire_clkos);
                    }
                }
            }

            for i in 0..2 {
                let eclk_in = self.rc_wire(cell, &format!("J{lrbt}FRC{i}"));
                self.add_bel_wire(bcrd, format!("ECLK{i}_IN"), eclk_in);

                let eclk = self.pips_fwd[&eclk_in]
                    .iter()
                    .copied()
                    .find(|wn| {
                        self.naming.strings[wn.suffix].contains("FRC")
                            && !self.naming.strings[wn.suffix].contains("JFRC")
                    })
                    .unwrap();
                self.add_bel_wire(bcrd, format!("ECLK{i}"), eclk);
                self.claim_pip(eclk, eclk_in);

                let eclk_io = self.rc_wire(cell, &format!("JPIO{i}"));
                self.add_bel_wire(bcrd, format!("ECLK{i}_IO"), eclk_io);
                self.claim_pip(eclk_in, eclk_io);
                let wire_io = self.get_special_io_wire_in(SpecialIoKey::Clock(edge, i as u8));
                self.claim_pip(eclk_io, wire_io);

                let eclk_int = self.rc_wire(cell, &format!("JCIBCLK{i}"));
                self.add_bel_wire(bcrd, format!("ECLK{i}_INT"), eclk_int);
                self.claim_pip(eclk_in, eclk_int);
                let bpin = self.xlat_int_wire(bcrd, eclk_int);
                bel.pins.insert(format!("ECLK{i}_IN"), bpin);

                if let Dir::H(_) = edge {
                    for &wire in &wires_pllpio {
                        self.claim_pip(eclk_in, wire);
                    }
                    if i == 0 {
                        for &wire in &wires_pllclkop {
                            self.claim_pip(eclk_in, wire);
                        }
                    } else {
                        for &wire in &wires_pllclkos {
                            self.claim_pip(eclk_in, wire);
                        }
                    }

                    let wire = self.intdb.get_wire(["OUT_F6", "OUT_F7"][i]);
                    let wire = TileWireCoord::new_idx(0, wire);
                    bel.pins
                        .insert(format!("PAD{i}_OUT"), BelPin::new_out(wire));
                    let wire = self.edev.egrid.tile_wire(tcrd, wire);
                    self.claim_pip_int_out(wire, wire_io);
                } else {
                    for ci in 0..2 {
                        let wire = self.intdb.get_wire(["OUT_F6", "OUT_F7"][i]);
                        let wire = TileWireCoord::new_idx(ci, wire);
                        bel.pins
                            .insert(format!("PAD{i}_OUT{ci}"), BelPin::new_out(wire));
                        let wire = self.edev.egrid.tile_wire(tcrd, wire);
                        self.claim_pip_int_out(wire, wire_io);
                    }
                }
            }
            self.insert_bel(bcrd, bel);
        }
    }

    pub(super) fn process_eclk_tap_ecp2(&mut self) {
        let tcid = self.intdb.get_tile_class("ECLK_TAP");
        for &tcrd in &self.edev.egrid.tile_index[tcid] {
            let bcrd = tcrd.bel(bels::ECLK_TAP);
            let edge = if tcrd.row == self.chip.row_s() {
                Dir::S
            } else if tcrd.row == self.chip.row_n() {
                Dir::N
            } else if tcrd.col == self.chip.col_w() {
                Dir::W
            } else if tcrd.col == self.chip.col_e() {
                Dir::E
            } else {
                unreachable!()
            };
            let bel_eclk = self.chip.bel_eclk_root(edge);
            let eclk0 = self.naming.bel_wire(bel_eclk, "ECLK0");
            let eclk1 = self.naming.bel_wire(bel_eclk, "ECLK1");
            let eclk0_out = self.edev.egrid.get_bel_pin(bcrd, "ECLK0")[0];
            let eclk1_out = self.edev.egrid.get_bel_pin(bcrd, "ECLK1")[0];
            self.claim_pip_int_out(eclk0_out, eclk0);
            self.claim_pip_int_out(eclk1_out, eclk1);
        }
    }
}

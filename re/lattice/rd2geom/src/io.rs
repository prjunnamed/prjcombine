use prjcombine_ecp::{
    bels,
    chip::{ChipKind, IoKind, RowKind, SpecialIoKey},
    tslots,
};
use prjcombine_interconnect::{
    db::Bel,
    dir::{Dir, DirV},
    grid::{BelCoord, CellCoord, DieId, EdgeIoCoord},
};
use prjcombine_re_lattice_naming::WireName;
use unnamed_entity::EntityId;

use crate::{ChipContext, chip::ChipExt};

impl ChipContext<'_> {
    fn process_single_io_ecp(&mut self, bcrd: BelCoord, mut dummy: bool) {
        let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
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
        self.add_bel_wire(bcrd, "OPOS0_IOLOGIC", opos0_iologic);
        self.add_bel_wire(bcrd, "OPOS1_IOLOGIC", opos1_iologic);
        self.add_bel_wire(bcrd, "ONEG0_IOLOGIC", oneg0_iologic);
        self.add_bel_wire(bcrd, "ONEG1_IOLOGIC", oneg1_iologic);
        self.add_bel_wire(bcrd, "IPOS0_IOLOGIC", ipos0_iologic);
        self.add_bel_wire(bcrd, "IPOS1_IOLOGIC", ipos1_iologic);
        self.add_bel_wire(bcrd, "INFF_IOLOGIC", inff_iologic);
        self.add_bel_wire(bcrd, "CLK_IOLOGIC", clk_iologic);
        self.add_bel_wire(bcrd, "LSR_IOLOGIC", lsr_iologic);
        self.add_bel_wire(bcrd, "CE_IOLOGIC", ce_iologic);
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

        if !very_dummy {
            let di = self.rc_io_wire(cell, &format!("JDI{ab}"));
            self.add_bel_wire(bcrd, "DI", di);

            let bpin_opos0 = self.xlat_int_wire(tcrd, opos0_iologic).unwrap();
            let bpin_opos1 = self.xlat_int_wire(tcrd, opos1_iologic).unwrap();
            let bpin_oneg0 = self.xlat_int_wire(tcrd, oneg0_iologic).unwrap();
            let bpin_oneg1 = self.xlat_int_wire(tcrd, oneg1_iologic).unwrap();
            let bpin_oneg1_alt = self.xlat_int_wire(tcrd, td_iologic).unwrap();
            assert_eq!(bpin_oneg1, bpin_oneg1_alt);
            let bpin_oneg0_alt = self.xlat_int_wire(tcrd, paddo_pio).unwrap();
            assert_eq!(bpin_oneg0, bpin_oneg0_alt);
            let bpin_ipos0 = self.xlat_int_wire(tcrd, ipos0_iologic).unwrap();
            let bpin_ipos1 = self.xlat_int_wire(tcrd, ipos1_iologic).unwrap();
            let bpin_clk = self.xlat_int_wire(tcrd, clk_iologic).unwrap();
            let bpin_lsr = self.xlat_int_wire(tcrd, lsr_iologic).unwrap();
            let bpin_ce = self.xlat_int_wire(tcrd, ce_iologic).unwrap();
            let bpin_td = self.xlat_int_wire(tcrd, paddt_pio).unwrap();
            let bpin_inff = self.xlat_int_wire(tcrd, inff_iologic).unwrap();
            let bpin_di = self.xlat_int_wire(tcrd, di).unwrap();
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
                        let bel_dqs = cell_dqs.bel(bels::DQS);
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
                    self.claim_pip(dqs_iologic, dqs);
                    self.claim_pip(ddrclkpol_iologic, ddrclkpol);
                }
            }
        }

        self.claim_pip(di_iologic, paddi_pio);
        self.claim_pip(iolto_pio, iolto_iologic);
        self.claim_pip(ioldo_pio, ioldo_iologic);
    }

    fn process_dqs_ecp(&mut self, bcrd: BelCoord) {
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
        let ddrclkpol_out = self.rc_io_wire(cell, "DDRCLKPOL");
        let dqsdel = self.rc_io_wire(cell, "JDQSDEL_DQS");
        let dqsc = self.rc_io_wire(cell, "JDQSC_DQS");
        let dqsi = self.rc_io_wire(cell, "JDQSI_DQS");
        let dqso = self.rc_io_wire(cell, "DQSO_DQS");
        let dqso_out = self.rc_io_wire(cell, "DQSO");
        let indqsa = self.rc_io_wire(cell, "JINDQSA");
        let io0_di = self.rc_io_wire(cell, "JDIA");
        let dqso_tree = self.pips_fwd[&dqso_out].iter().copied().next().unwrap();
        let ddrclkpol_tree = self.pips_fwd[&ddrclkpol_out]
            .iter()
            .copied()
            .find(|w| !self.int_wires.contains_key(w))
            .unwrap();
        if io.edge() == Dir::S {
            // fuck this shit.
            self.add_bel_wire(bcrd, "DDRCLKPOL", ddrclkpol);
            let bpin = self
                .xlat_int_wire(self.edev.egrid.get_tile_by_bel(bcrd), ddrclkpol_out)
                .unwrap();
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
        let wire_io = self.get_io_wire(io);
        self.claim_pip(dqsi, wire_io);
        self.claim_pip(indqsa, dqsc);
        self.claim_pip(io0_di, indqsa);
        self.claim_pip(dqso_out, dqso);
        self.claim_pip(dqso_tree, dqso_out);
        self.claim_pip(ddrclkpol_out, ddrclkpol);
        self.claim_pip(ddrclkpol_tree, ddrclkpol_out);
        let bcrd_dqsdll = self.chip.bel_dqsdll(if cell.row < self.chip.row_clk {
            DirV::S
        } else {
            DirV::N
        });
        let dqsdel_dqsdll = self.naming.bel_wire(bcrd_dqsdll, "DQSDEL");
        self.claim_pip(dqsdel, dqsdel_dqsdll);
        self.insert_bel(bcrd, bel);
    }

    fn process_dqsdll_ecp(&mut self) {
        for edge in [DirV::S, DirV::N] {
            let bcrd = self.chip.bel_dqsdll(edge);
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

    fn process_io_ecp(&mut self) {
        self.process_dqsdll_ecp();
        for (row, rd) in &self.chip.rows {
            if rd.io_w == IoKind::DoubleDqs {
                let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), row);
                self.process_dqs_ecp(cell.bel(bels::DQS));
            }
            if rd.io_e == IoKind::DoubleDqs {
                let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), row);
                self.process_dqs_ecp(cell.bel(bels::DQS));
            }
        }
        for (col, cd) in &self.chip.columns {
            if cd.io_s == IoKind::DoubleDqs {
                let cell = CellCoord::new(DieId::from_idx(0), col, self.chip.row_s());
                self.process_dqs_ecp(cell.bel(bels::DQS));
            }
            if cd.io_n == IoKind::DoubleDqs {
                let cell = CellCoord::new(DieId::from_idx(0), col, self.chip.row_n());
                self.process_dqs_ecp(cell.bel(bels::DQS));
            }
        }
        for (row, rd) in &self.chip.rows {
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), row);
            self.process_single_io_ecp(
                cell.bel(bels::IO0),
                matches!(rd.io_w, IoKind::DoubleB | IoKind::None),
            );
            self.process_single_io_ecp(
                cell.bel(bels::IO1),
                matches!(rd.io_w, IoKind::DoubleA | IoKind::None),
            );
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), row);
            self.process_single_io_ecp(
                cell.bel(bels::IO0),
                matches!(rd.io_e, IoKind::DoubleB | IoKind::None),
            );
            self.process_single_io_ecp(
                cell.bel(bels::IO1),
                matches!(rd.io_e, IoKind::DoubleA | IoKind::None),
            );
        }
        for (col, cd) in &self.chip.columns {
            if col == self.chip.col_w() || col == self.chip.col_e() {
                continue;
            }
            let cell = CellCoord::new(DieId::from_idx(0), col, self.chip.row_s());
            self.process_single_io_ecp(
                cell.bel(bels::IO0),
                matches!(cd.io_s, IoKind::DoubleB | IoKind::None),
            );
            self.process_single_io_ecp(
                cell.bel(bels::IO1),
                matches!(cd.io_s, IoKind::DoubleA | IoKind::None),
            );
            let cell = CellCoord::new(DieId::from_idx(0), col, self.chip.row_n());
            self.process_single_io_ecp(
                cell.bel(bels::IO0),
                matches!(cd.io_n, IoKind::DoubleB | IoKind::None),
            );
            self.process_single_io_ecp(
                cell.bel(bels::IO1),
                matches!(cd.io_n, IoKind::DoubleA | IoKind::None),
            );
        }
    }

    fn process_single_io_machxo(&mut self, bcrd: BelCoord) {
        let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
        let idx = bels::IO.iter().position(|&slot| slot == bcrd.slot).unwrap();
        let cell = bcrd.cell;
        let abcd = ['A', 'B', 'C', 'D', 'E', 'F'][idx];
        let io = self.chip.get_io_crd(bcrd);
        let (r, c) = self.rc(cell);
        let name = match io.edge() {
            Dir::W => format!("PL{r}{abcd}"),
            Dir::E => format!("PR{r}{abcd}"),
            Dir::S => format!("PB{c}{abcd}"),
            Dir::N => format!("PT{c}{abcd}"),
        };
        self.name_bel(bcrd, [name]);

        let mut bel = Bel::default();

        let ddtd0 = self.rc_io_wire(cell, &format!("JDDTD0{abcd}"));
        let ddtd1 = self.rc_io_wire(cell, &format!("JDDTD1{abcd}"));
        self.add_bel_wire(bcrd, "DDTD0", ddtd0);
        self.add_bel_wire(bcrd, "DDTD1", ddtd1);
        bel.pins
            .insert("DDTD0".into(), self.xlat_int_wire(tcrd, ddtd0).unwrap());
        bel.pins
            .insert("DDTD1".into(), self.xlat_int_wire(tcrd, ddtd1).unwrap());

        let dd2 = self.rc_io_wire(cell, &format!("JDD2{abcd}"));
        self.add_bel_wire(bcrd, "DD2", dd2);

        let (plc_cell, plc_idx) = self.chip.io_direct_plc[&io];
        let slice = plc_cell.bel(bels::SLICE[usize::from(plc_idx / 2)]);
        let plcf = self.naming.bel_wire(slice, &format!("F{}_IO", plc_idx % 2));
        let plcq = self.naming.bel_wire(slice, &format!("Q{}_IO", plc_idx % 2));

        let dd = self.rc_wire(self.chip.xlat_rc_wire(plcf), &format!("JDD{plc_idx}"));
        self.add_bel_wire(bcrd, "DD", dd);
        self.claim_pip(dd, plcf);
        self.claim_pip(dd, plcq);
        self.claim_pip(dd2, dd);

        let paddo = self.rc_io_wire(cell, &format!("JPADDO{abcd}"));
        let paddt = self.rc_io_wire(cell, &format!("JPADDT{abcd}"));
        self.add_bel_wire(bcrd, "PADDO", paddo);
        self.add_bel_wire(bcrd, "PADDT", paddt);
        self.claim_pip(paddo, dd2);
        self.claim_pip(paddo, ddtd0);
        self.claim_pip(paddo, ddtd1);
        self.claim_pip(paddt, ddtd0);
        self.claim_pip(paddt, ddtd1);

        let paddo_pio = self.rc_io_wire(cell, &format!("JPADDO{abcd}_PIO"));
        let paddt_pio = self.rc_io_wire(cell, &format!("JPADDT{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDO_PIO", paddo_pio);
        self.add_bel_wire(bcrd, "PADDT_PIO", paddt_pio);
        self.claim_pip(paddo_pio, paddo);
        self.claim_pip(paddt_pio, paddt);

        let paddi_pio = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDI_PIO", paddi_pio);
        bel.pins
            .insert("PADDI".into(), self.xlat_int_wire(tcrd, paddi_pio).unwrap());

        self.insert_bel(bcrd, bel);
    }

    fn process_pictest(&mut self, cell: CellCoord) {
        let (r, c) = self.rc(cell);
        let lr = if c == 1 { 'L' } else { 'R' };
        let tcrd = cell.tile(tslots::IO);
        let bcrd2 = cell.bel(bels::IO2);
        let bcrd3 = cell.bel(bels::IO3);
        self.name_bel(bcrd2, [format!("{lr}PICTEST{r}")]);
        self.name_bel_null(bcrd3);

        let mut bel = Bel::default();
        let ddtd0 = self.rc_io_wire(cell, "JC0_PICTEST");
        let ddtd1 = self.rc_io_wire(cell, "JC1_PICTEST");
        self.add_bel_wire(bcrd2, "DDTD0", ddtd0);
        self.add_bel_wire(bcrd2, "DDTD1", ddtd1);
        bel.pins
            .insert("DDTD0".into(), self.xlat_int_wire(tcrd, ddtd0).unwrap());
        bel.pins
            .insert("DDTD1".into(), self.xlat_int_wire(tcrd, ddtd1).unwrap());
        let paddi_pio = self.rc_io_wire(cell, "JQ0_PICTEST");
        self.add_bel_wire(bcrd2, "PADDI_PIO", paddi_pio);
        bel.pins
            .insert("PADDI".into(), self.xlat_int_wire(tcrd, paddi_pio).unwrap());
        self.insert_bel(bcrd2, bel);

        let mut bel = Bel::default();
        let ddtd0 = self.rc_io_wire(cell, "JD0_PICTEST");
        let ddtd1 = self.rc_io_wire(cell, "JD1_PICTEST");
        self.add_bel_wire(bcrd3, "DDTD0", ddtd0);
        self.add_bel_wire(bcrd3, "DDTD1", ddtd1);
        bel.pins
            .insert("DDTD0".into(), self.xlat_int_wire(tcrd, ddtd0).unwrap());
        bel.pins
            .insert("DDTD1".into(), self.xlat_int_wire(tcrd, ddtd1).unwrap());
        let paddi_pio = self.rc_io_wire(cell, "JQ1_PICTEST");
        self.add_bel_wire(bcrd3, "PADDI_PIO", paddi_pio);
        bel.pins
            .insert("PADDI".into(), self.xlat_int_wire(tcrd, paddi_pio).unwrap());
        self.insert_bel(bcrd3, bel);
    }

    fn process_io_machxo(&mut self) {
        for (row, rd) in &self.chip.rows {
            let num_io = match rd.io_w {
                IoKind::None => 0,
                IoKind::Double => 2,
                IoKind::Quad | IoKind::QuadReverse => 4,
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), row);
            for i in 0..num_io {
                self.process_single_io_machxo(cell.bel(bels::IO[i]));
            }
            if num_io == 2 {
                self.process_pictest(cell);
            }
            let num_io = match rd.io_e {
                IoKind::None => 0,
                IoKind::Double => 2,
                IoKind::Quad | IoKind::QuadReverse => 4,
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), row);
            for i in 0..num_io {
                self.process_single_io_machxo(cell.bel(bels::IO[i]));
            }
            if num_io == 2 {
                self.process_pictest(cell);
            }
        }
        for (col, cd) in &self.chip.columns {
            let num_io = match cd.io_s {
                IoKind::None => 0,
                IoKind::Quad | IoKind::QuadReverse => 4,
                IoKind::Hex | IoKind::HexReverse => 6,
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), col, self.chip.row_s());
            for i in 0..num_io {
                self.process_single_io_machxo(cell.bel(bels::IO[i]));
            }
            let num_io = match cd.io_n {
                IoKind::None => 0,
                IoKind::Quad | IoKind::QuadReverse => 4,
                IoKind::Hex | IoKind::HexReverse => 6,
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), col, self.chip.row_n());
            for i in 0..num_io {
                self.process_single_io_machxo(cell.bel(bels::IO[i]));
            }
        }
    }

    pub fn process_io(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => self.process_io_ecp(),
            ChipKind::MachXo => self.process_io_machxo(),
        }
    }

    pub fn get_io_wire(&self, io: EdgeIoCoord) -> WireName {
        let bel = self.chip.get_io_loc(io);
        let abcd = ['A', 'B', 'C', 'D', 'E', 'F'][io.iob().to_idx()];
        self.rc_io_wire(bel.cell, &format!("JPADDI{abcd}_PIO"))
    }

    pub fn get_special_io_wire(&self, key: SpecialIoKey) -> WireName {
        self.get_io_wire(self.chip.special_io[&key])
    }
}

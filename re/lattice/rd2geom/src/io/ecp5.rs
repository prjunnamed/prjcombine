use prjcombine_ecp::{
    bels,
    chip::{IoGroupKind, IoKind, PllLoc, SpecialIoKey, SpecialLocKey},
};
use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::Bel,
    dir::{Dir, DirH, DirHV, DirV},
    grid::{BelCoord, CellCoord, DieId, EdgeIoCoord},
};

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_bc_ecp5(&mut self) {
        for (&key, &cell_loc) in &self.chip.special_loc {
            let SpecialLocKey::Bc(bank) = key else {
                continue;
            };
            let (col, row, suffix) = match bank {
                1 | 2 => (self.chip.col_e(), self.chip.row_n(), "R"),
                3 | 4 => (self.chip.col_e(), self.chip.row_s(), "R"),
                8 | 6 => (self.chip.col_w(), self.chip.row_s(), "L"),
                7 | 0 => (self.chip.col_w(), self.chip.row_n(), "L"),
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), col, row);
            for (bel, name, pin_out, pin_in) in [
                (bels::BCINRD, "BCINRD", "INRDENO", "INRDENI"),
                (bels::BCLVDSO, "BCLVDSO", "LVDSENO", "LVDSENI"),
            ] {
                let bcrd = cell_loc.bel(bel);
                if !self.edev.has_bel(bcrd) {
                    continue;
                }
                self.name_bel(bcrd, [format!("{name}{bank}")]);

                let wire_in = self.rc_corner_wire(cell, &format!("J{pin_in}_{name}_{suffix}"));
                self.add_bel_wire(bcrd, pin_in, wire_in);
                let mut bel = Bel::default();
                bel.pins
                    .insert(pin_in.into(), self.xlat_int_wire(bcrd, wire_in));
                self.insert_bel(bcrd, bel);

                let wire_out = self.rc_corner_wire(cell, &format!("{pin_out}_{name}_{suffix}"));
                self.add_bel_wire(bcrd, pin_out, wire_out);
                self.claim_pip(wire_out, wire_in);

                let wire_out_out = self.claim_single_out(wire_out);
                self.add_bel_wire(bcrd, format!("{pin_out}_OUT"), wire_out_out);
            }
            {
                let bcrd = cell_loc.bel(bels::BREFTEST);
                self.name_bel(bcrd, [format!("BREFTEST{bank}")]);
                let mut bel = Bel::default();

                for pin in ["PVT_SRC_IN", "PVT_SNK_IN", "PVT_SRC_OUT", "PVT_SNK_OUT"] {
                    for i in 0..6 {
                        let wire = self.rc_corner_wire(cell, &format!("J{pin}{i}_BREFTEST{bank}"));
                        self.add_bel_wire(bcrd, format!("{pin}{i}"), wire);
                        bel.pins
                            .insert(format!("{pin}{i}"), self.xlat_int_wire(bcrd, wire));
                    }
                }
                self.insert_bel(bcrd, bel);
            }
        }
        if !self.chip.special_loc.contains_key(&SpecialLocKey::Bc(4)) {
            self.dummy_sites.insert("BREFTEST4".into());
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_s());
            for pin in ["PVT_SRC_IN", "PVT_SNK_IN", "PVT_SRC_OUT", "PVT_SNK_OUT"] {
                for i in 0..6 {
                    let wire = self.rc_corner_wire(cell, &format!("J{pin}{i}_BREFTEST4"));
                    self.claim_node(wire);
                }
            }
        }
    }

    pub(super) fn process_eclk_ecp5(&mut self) {
        let cell_w = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk);
        let cell_e = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for (bank, edge, bank_idx, cell_tile) in [
            (6, DirH::W, 0, cell_w),
            (7, DirH::W, 1, cell_w),
            (3, DirH::E, 0, cell_e),
            (2, DirH::E, 1, cell_e),
        ] {
            for eclk_idx in 0..2 {
                let cell = cell_tile.delta(0, -1 + (bank_idx as i32));
                let bcrd = cell_tile.bel(bels::ECLKSYNC[bank_idx * 2 + eclk_idx]);

                self.name_bel(bcrd, [format!("ECLKSYNC{eclk_idx}_BK{bank}")]);
                let mut bel = Bel::default();

                let stop = self.rc_io_wire(cell, &format!("JSTOP_ECLKSYNC{eclk_idx}"));
                self.add_bel_wire(bcrd, "STOP", stop);
                bel.pins
                    .insert("STOP".into(), self.xlat_int_wire(bcrd, stop));

                let eclki = self.rc_io_wire(cell, &format!("ECLKI_ECLKSYNC{eclk_idx}"));
                self.add_bel_wire(bcrd, "ECLKI", eclki);
                let eclki_in = self.claim_single_in(eclki);
                self.add_bel_wire(bcrd, "ECLKI_IN", eclki_in);

                let lr = match edge {
                    DirH::W => 'L',
                    DirH::E => 'R',
                };

                for (ul, v) in [('L', DirV::S), ('U', DirV::N)] {
                    let eclki_int = self.rc_io_wire(cell, &format!("J{ul}{lr}QECLKCIB{eclk_idx}"));
                    self.add_bel_wire(bcrd, format!("ECLKI_INT_{v}"), eclki_int);
                    self.claim_pip(eclki_in, eclki_int);
                    bel.pins
                        .insert(format!("ECLKI_{v}"), self.xlat_int_wire(bcrd, eclki_int));
                }

                for i in 0..4 {
                    let name = match edge {
                        DirH::W => ["60", "61", "70", "71"][i],
                        DirH::E => ["30", "31", "20", "21"][i],
                    };
                    let pclk = self.rc_io_wire(cell, &format!("JPCLKT{name}"));
                    self.add_bel_wire_no_claim(bcrd, format!("IO_IN_{edge}{i}"), pclk);
                    self.claim_pip(eclki_in, pclk);
                    if eclk_idx == 0 {
                        self.claim_node(pclk);
                        let cell_dlldel = bcrd.cell.delta(0, -2 + ((i ^ 1) as i32));
                        let wire_dlldel = self.rc_io_wire(cell_dlldel, "JINCK");
                        self.claim_pip(pclk, wire_dlldel);
                    }
                }

                let plls = match edge {
                    DirH::W => [DirHV::SW, DirHV::NW],
                    DirH::E => [DirHV::SE, DirHV::NE],
                };
                for hv in plls {
                    let Some(&cell_pll) = self
                        .chip
                        .special_loc
                        .get(&SpecialLocKey::Pll(PllLoc::new(hv, 0)))
                    else {
                        continue;
                    };
                    let corner = match hv {
                        DirHV::SW => "LL",
                        DirHV::SE => "LR",
                        DirHV::NW => "UL",
                        DirHV::NE => "UR",
                    };
                    for pin in ["CLKOP", "CLKOS"] {
                        let wire = self.rc_io_wire(cell, &format!("J{corner}CPLL0{pin}"));
                        self.add_bel_wire_no_claim(bcrd, format!("PLL_IN_{hv}_{pin}"), wire);
                        self.claim_pip(eclki_in, wire);
                        if eclk_idx == 0 {
                            self.claim_node(wire);
                            let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                            self.claim_pip(wire, wire_pll);
                        }
                    }
                }
                let eclko = self.rc_io_wire(cell, &format!("JECLKO_ECLKSYNC{eclk_idx}"));
                self.add_bel_wire(bcrd, "ELCKO", eclko);
                self.claim_pip(eclko, eclki);

                let eclko_out = self.rc_io_wire(cell, &format!("SYNCECLK{eclk_idx}"));
                self.add_bel_wire(bcrd, "ELCKO_OUT", eclko_out);
                self.claim_pip(eclko_out, eclko);

                let eclk_brg = self.rc_io_wire(cell, &format!("JBRGECLK{eclk_idx}"));
                self.add_bel_wire(bcrd, "ECLK_BRG", eclk_brg);
                let cell_clk = self.chip.bel_clk_root().cell.delta(-1, 0);
                let wire_brg = self.rc_wire(cell_clk, &format!("JBRGECLK{eclk_idx}"));
                self.claim_pip(eclk_brg, wire_brg);

                let eclk_mux = self.rc_io_wire(cell, &format!("JECLK{eclk_idx}"));
                self.add_bel_wire(bcrd, "ECLK_MUX", eclk_mux);
                self.claim_pip(eclk_mux, eclko_out);
                self.claim_pip(eclk_mux, eclk_brg);
                bel.pins
                    .insert("ECLK".into(), self.xlat_int_wire(bcrd, eclk_mux));

                let eclk = self.pips_fwd[&eclk_mux]
                    .iter()
                    .copied()
                    .find(|wn| self.naming.strings[wn.suffix].starts_with("BANK"))
                    .unwrap();
                self.add_bel_wire(bcrd, "ECLK", eclk);
                self.claim_pip(eclk, eclk_mux);

                let cell_neigh = if bank_idx == 0 {
                    cell.delta(0, 1)
                } else {
                    cell.delta(0, -1)
                };
                let eclk_neigh = self.rc_io_wire(cell, &format!("JNEIGHBORECLK{eclk_idx}"));
                self.add_bel_wire(bcrd, "ECLK_NEIGHBOR", eclk_neigh);
                self.claim_pip(eclk_mux, eclk_neigh);
                let wire_neigh = self.rc_io_wire(cell_neigh, &format!("JECLKO_ECLKSYNC{eclk_idx}"));
                self.claim_pip(eclk_neigh, wire_neigh);

                self.insert_bel(bcrd, bel);
            }
        }

        for (lr, cell) in [('L', cell_w), ('R', cell_e)] {
            let bcrd = cell.bel(bels::CLKTEST_ECLK);
            self.name_bel(bcrd, [format!("CLKTEST_{lr}ECLK")]);
            let mut bel = Bel::default();

            for i in 0..24 {
                let wire = self.rc_io_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                if i < 6 {
                    bel.pins
                        .insert(format!("TESTIN{i}"), self.xlat_int_wire(bcrd, wire));
                }
            }

            self.insert_bel(bcrd, bel);
        }

        for (cell, eclk_idx) in [(cell_w, 1), (cell_e, 0)] {
            let bcrd_cs = cell.bel(bels::ECLKBRIDGECS0);
            self.name_bel(bcrd_cs, [format!("ECLKBRIDGECS{eclk_idx}")]);
            let mut bel = Bel::default();

            let sel = self.rc_io_wire(cell, &format!("JSEL_ECLKBRIDGECS{eclk_idx}"));
            self.add_bel_wire(bcrd_cs, "SEL", sel);
            bel.pins
                .insert("SEL".into(), self.xlat_int_wire(bcrd_cs, sel));

            let clk0 = self.rc_io_wire(cell, &format!("JCLK0_ECLKBRIDGECS{eclk_idx}"));
            self.add_bel_wire(bcrd_cs, "CLK0", clk0);
            let bcrd_eclk = cell.bel(bels::ECLKSYNC[2 + eclk_idx]);
            let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKI_IN");
            self.claim_pip(clk0, wire_eclk);
            let clk1 = self.rc_io_wire(cell, &format!("JCLK1_ECLKBRIDGECS{eclk_idx}"));
            self.add_bel_wire(bcrd_cs, "CLK1", clk1);
            let bcrd_eclk = cell.bel(bels::ECLKSYNC[eclk_idx]);
            let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKI_IN");
            self.claim_pip(clk1, wire_eclk);

            let ecsout = self.rc_io_wire(cell, &format!("ECSOUT_ECLKBRIDGECS{eclk_idx}"));
            self.add_bel_wire(bcrd_cs, "ECSOUT", ecsout);

            self.insert_bel(bcrd_cs, bel);

            let bcrd_brg = cell.bel(bels::BRGECLKSYNC0);
            self.name_bel(bcrd_brg, [format!("BRGECLKSYNC{eclk_idx}")]);
            let mut bel = Bel::default();

            let stop = self.rc_io_wire(cell, &format!("JSTOP_BRGECLKSYNC{eclk_idx}"));
            self.add_bel_wire(bcrd_brg, "STOP", stop);
            bel.pins
                .insert("STOP".into(), self.xlat_int_wire(bcrd_brg, stop));

            let eclki = self.rc_io_wire(cell, &format!("ECLKI_BRGECLKSYNC{eclk_idx}"));
            self.add_bel_wire(bcrd_brg, "ECLKI", eclki);
            self.claim_pip(eclki, ecsout);

            let eclko = self.rc_io_wire(cell, &format!("JECLKO_BRGECLKSYNC{eclk_idx}"));
            self.add_bel_wire(bcrd_brg, "ECLKO", eclko);
            self.claim_pip(eclko, eclki);

            self.insert_bel(bcrd_brg, bel);
        }
    }

    pub(super) fn process_ddrdll_ecp5(&mut self) {
        for hv in DirHV::DIRS {
            let cell = self.chip.special_loc[&SpecialLocKey::DdrDll(hv)];
            let bcrd = cell.bel(bels::DDRDLL);
            let corner = match hv {
                DirHV::SW => "BL",
                DirHV::SE => "BR",
                DirHV::NW => "TL",
                DirHV::NE => "TR",
            };
            let cell = cell.with_cr(self.chip.col_edge(hv.h), self.chip.row_edge(hv.v));
            self.name_bel(bcrd, [format!("DDRDLL_{corner}")]);
            let mut bel = self.extract_simple_bel(bcrd, cell, "DDRDLL");

            let clk = self.rc_corner_wire(cell, "JCLK_DDRDLL");
            self.add_bel_wire(bcrd, "CLK", clk);
            let clk_in = self.claim_single_in(clk);
            self.add_bel_wire(bcrd, "CLK_IN", clk_in);

            let clk_int = self.rc_corner_wire(cell, "JCIBCLK0");
            self.add_bel_wire(bcrd, "CLK_INT", clk_int);
            self.claim_pip(clk_in, clk_int);
            bel.pins
                .insert("CLK".into(), self.xlat_int_wire(bcrd, clk_int));

            let bank = match hv {
                DirHV::SW => 6,
                DirHV::SE => 3,
                DirHV::NW => 7,
                DirHV::NE => 2,
            };
            for i in 0..2 {
                let bcrd_eclk = self.chip.bel_eclksync_bank(bank, i);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
                self.claim_pip(clk_in, wire_eclk);
            }

            let ddrdel = self.rc_corner_wire(cell, "DDRDEL_DDRDLL");
            self.add_bel_wire(bcrd, "DDRDEL", ddrdel);
            let ddrdel_out = self.claim_single_out(ddrdel);
            self.add_bel_wire(bcrd, "DDRDEL_OUT", ddrdel_out);

            self.insert_bel(bcrd, bel);

            if hv == DirHV::NW {
                let cell = bcrd.cell.with_col(self.chip.col_w());
                let wire = self.rc_wire(cell, "TESTIN_PVTTEST");
                self.claim_node(wire);
                self.dummy_sites.insert("PVTTEST".to_string());
            }
        }
    }

    pub(super) fn process_dlldel_ecp5(&mut self) {
        let cell_n = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_n());
        let cell_w = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk);
        let cell_e = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for (name, edge, idx) in [
            ("00", Dir::N, 0),
            ("01", Dir::N, 1),
            ("10", Dir::N, 2),
            ("11", Dir::N, 3),
            ("60", Dir::W, 0),
            ("61", Dir::W, 1),
            ("70", Dir::W, 2),
            ("71", Dir::W, 3),
            ("30", Dir::E, 0),
            ("31", Dir::E, 1),
            ("20", Dir::E, 2),
            ("21", Dir::E, 3),
        ] {
            let Some(&io) = self
                .chip
                .special_io
                .get(&SpecialIoKey::Clock(edge, idx as u8))
            else {
                continue;
            };
            let (cell, cell_tile) = match edge {
                Dir::N => (cell_n.delta(-2 + (idx as i32), 0), cell_n),
                Dir::W => (cell_w.delta(0, -2 + ((idx ^ 1) as i32)), cell_w),
                Dir::E => (cell_e.delta(0, -2 + ((idx ^ 1) as i32)), cell_e),
                Dir::S => unreachable!(),
            };
            let bcrd = cell_tile.bel(bels::DLLDEL[idx]);
            self.name_bel(bcrd, [format!("DLLDEL_{name}")]);

            let (cell_io, abcd) = self.xlat_io_loc_ecp5(io);
            let paddi_pio = self.rc_io_wire(cell_io, &format!("JPADDI{abcd}_PIO"));

            let paddi = self.rc_io_wire(cell, "JPADDI");
            self.add_bel_wire(bcrd, "PADDI", paddi);
            self.claim_pip(paddi, paddi_pio);

            let a = self.rc_io_wire(cell, "JA_DLLDEL");
            self.add_bel_wire(bcrd, "A", a);
            self.claim_pip(a, paddi_pio);

            let z = self.rc_io_wire(cell, "Z_DLLDEL");
            self.add_bel_wire(bcrd, "Z", z);

            let z_out = self.rc_io_wire(cell, "DLLDEL");
            self.add_bel_wire(bcrd, "Z_OUT", z_out);
            self.claim_pip(z_out, z);

            let inck = self.rc_io_wire(cell, "JINCK");
            self.add_bel_wire(bcrd, "INCK", inck);
            self.claim_pip(inck, paddi);
            self.claim_pip(inck, z_out);

            let ddrdel = self.rc_io_wire(cell, "DDRDEL_DLLDEL");
            self.add_bel_wire(bcrd, "DDRDEL", ddrdel);
            let ddrdel_in = self.rc_io_wire(cell, "DDRDEL");
            self.add_bel_wire(bcrd, "DDRDEL_IN", ddrdel_in);
            self.claim_pip(ddrdel, ddrdel_in);

            let ddrdll_sources = match io.edge() {
                Dir::W => [DirHV::SW, DirHV::NW],
                Dir::E => [DirHV::SE, DirHV::NE],
                Dir::N => [DirHV::NW, DirHV::NE],
                _ => unreachable!(),
            };
            for hv in ddrdll_sources {
                let bcrd_ddrdll =
                    self.chip.special_loc[&SpecialLocKey::DdrDll(hv)].bel(bels::DDRDLL);
                let wire_ddrdll = self.naming.bel_wire(bcrd_ddrdll, "DDRDEL_OUT");
                self.claim_pip(ddrdel_in, wire_ddrdll);
            }

            self.insert_simple_bel(bcrd, cell, "DLLDEL");
        }
    }

    pub(super) fn process_dtr_ecp5(&mut self) {
        let tcid = self.intdb.get_tile_class("DTR");
        for &tcrd in &self.edev.tile_index[tcid] {
            let bcrd = tcrd.bel(bels::DTR);
            self.name_bel(bcrd, ["DTR"]);
            self.insert_simple_bel(bcrd, bcrd.cell, "DTR");
        }
    }

    pub fn xlat_io_loc_ecp5(&self, io: EdgeIoCoord) -> (CellCoord, &'static str) {
        let bcrd = self.chip.get_io_loc(io);
        let abcd = ["A", "B", "C", "D"][io.iob().to_idx()];
        match io.edge() {
            Dir::H(_) => (bcrd.cell.delta(0, 2), abcd),
            Dir::V(_) => (bcrd.cell, abcd),
        }
    }

    fn process_dqs_ecp5(&mut self, bcrd: BelCoord) {
        let io = self.chip.get_io_crd(bcrd.bel(bels::IO0));
        let bank = self.chip.get_io_bank(io);
        let cell = bcrd.cell.delta(0, 2);
        let (r, _c) = self.rc(cell);
        match io.edge() {
            Dir::W => {
                self.name_bel(bcrd, [format!("LDQS{r}")]);
            }
            Dir::E => {
                self.name_bel(bcrd, [format!("RDQS{r}")]);
            }
            _ => unreachable!(),
        }

        self.insert_simple_bel(bcrd, cell, "DQS");

        let dqsi = self.rc_io_wire(cell, "JDQSI_DQS");
        self.add_bel_wire(bcrd, "DQSI", dqsi);
        let paddi = self.rc_io_wire(cell, "JPADDIA_PIO");
        self.claim_pip(dqsi, paddi);

        let ddrdel = self.rc_io_wire(cell, "DDRDEL_DQS");
        self.add_bel_wire(bcrd, "DDRDEL", ddrdel);
        let ddrdel_in = self.rc_io_wire(cell, "DDRDEL");
        self.add_bel_wire(bcrd, "DDRDEL_IN", ddrdel_in);
        self.claim_pip(ddrdel, ddrdel_in);

        let ddrdll_sources = match io.edge() {
            Dir::W => [DirHV::SW, DirHV::NW],
            Dir::E => [DirHV::SE, DirHV::NE],
            _ => unreachable!(),
        };
        for hv in ddrdll_sources {
            let bcrd_ddrdll = self.chip.special_loc[&SpecialLocKey::DdrDll(hv)].bel(bels::DDRDLL);
            let wire_ddrdll = self.naming.bel_wire(bcrd_ddrdll, "DDRDEL_OUT");
            self.claim_pip(ddrdel_in, wire_ddrdll);
        }

        let eclk = self.rc_io_wire(cell, "ECLK_DQS");
        self.add_bel_wire(bcrd, "ECLK", eclk);
        let eclk_in = self.rc_io_wire(cell, "DQSECLK");
        self.add_bel_wire(bcrd, "ECLK_IN", eclk_in);
        self.claim_pip(eclk, eclk_in);

        for i in 0..2 {
            let bcrd_eclk = self.chip.bel_eclksync_bank(bank, i);
            let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
            self.claim_pip(eclk_in, wire_eclk);
        }

        for pin in [
            "RDPNTR0", "RDPNTR1", "RDPNTR2", "WRPNTR0", "WRPNTR1", "WRPNTR2",
        ] {
            let wire = self.rc_io_wire(cell, &format!("{pin}_DQS"));
            self.add_bel_wire(bcrd, pin, wire);
            let wire_out = self.claim_single_out(wire);
            self.add_bel_wire(bcrd, format!("{pin}_OUT"), wire_out);
        }
        for pin in ["DQSR90", "DQSW270", "DQSW"] {
            let wire = self.rc_io_wire(cell, &format!("J{pin}_DQS"));
            let wire_out = self.pips_fwd[&wire]
                .iter()
                .copied()
                .find(|w| !self.int_wires.contains_key(w))
                .unwrap();
            self.add_bel_wire(bcrd, format!("{pin}_OUT"), wire_out);
            self.claim_pip(wire_out, wire);
        }
    }

    fn process_single_io_ecp5(&mut self, bcrd: BelCoord) {
        let io = self.chip.get_io_crd(bcrd);
        let bank = self.chip.get_io_bank(io);
        let idx = io.iob().to_idx();
        let cell = match io.edge() {
            Dir::H(_) => bcrd.cell.delta(0, 2),
            Dir::V(_) => bcrd.cell,
        };
        let abcd = ["A", "B", "C", "D"][idx];
        let (r, c) = self.rc(cell);
        match io.edge() {
            Dir::W => {
                self.name_bel(bcrd, [format!("PL{r}{abcd}"), format!("IOL_L{r}{abcd}")]);
            }
            Dir::E => {
                self.name_bel(bcrd, [format!("PR{r}{abcd}"), format!("IOL_R{r}{abcd}")]);
            }
            Dir::S => {
                self.name_bel(bcrd, [format!("PB{c}{abcd}"), format!("IOL_B{c}{abcd}")]);
            }
            Dir::N => {
                self.name_bel(bcrd, [format!("PT{c}{abcd}"), format!("IOL_T{c}{abcd}")]);
            }
        }
        let kind = self.chip.get_io_kind(io);
        let iol = match kind {
            IoKind::Io => "IOLOGIC",
            IoKind::Sio => "SIOLOGIC",
            _ => unreachable!(),
        };
        let mut bel = Bel::default();

        let mut pins = vec![
            "LSR",
            "CE",
            "CLK",
            "CFLAG",
            "RXDATA0",
            "RXDATA1",
            "TXDATA0",
            "TXDATA1",
            "TSDATA0",
            "DIRECTION",
            "MOVE",
            "LOADN",
            "INFF",
        ];
        if kind == IoKind::Io {
            pins.extend([
                "RXDATA2", "RXDATA3", "TXDATA2", "TXDATA3", "TSDATA1", "SLIP",
            ]);
            if matches!(idx, 0 | 2) {
                pins.extend([
                    "RXDATA4", "RXDATA5", "RXDATA6", "TXDATA4", "TXDATA5", "TXDATA6",
                ]);
            }
        }

        for pin in pins {
            let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_{iol}"));
            self.add_bel_wire(bcrd, pin, wire);
            bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
        }

        if kind == IoKind::Io && matches!(idx, 1 | 3) {
            for pin in [
                "RXDATA4", "RXDATA5", "RXDATA6", "TXDATA4", "TXDATA5", "TXDATA6",
            ] {
                let wire = self.rc_io_wire(cell, &format!("{pin}{abcd}_{iol}"));
                self.add_bel_wire(bcrd, pin, wire);
            }
        }

        let paddi_pio = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDI_PIO", paddi_pio);
        let paddi_iol = self.rc_io_wire(cell, &format!("PADDI{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "PADDI_IOLOGIC", paddi_iol);
        self.claim_pip(paddi_iol, paddi_pio);
        let indd_iol = self.rc_io_wire(cell, &format!("INDD{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "INDD_IOLOGIC", indd_iol);
        let di = self.rc_io_wire(cell, &format!("JDI{abcd}"));
        self.add_bel_wire(bcrd, "DI", di);
        self.claim_pip(di, paddi_pio);
        self.claim_pip(di, indd_iol);
        bel.pins.insert("DI".into(), self.xlat_int_wire(bcrd, di));
        let di_iol = self.rc_io_wire(cell, &format!("DI{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "DI_IOLOGIC", di_iol);
        self.claim_pip(di_iol, di);

        let paddo_pio = self.rc_io_wire(cell, &format!("PADDO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDO_PIO", paddo_pio);
        let paddo = self.rc_io_wire(cell, &format!("JPADDO{abcd}"));
        self.add_bel_wire(bcrd, "PADDO", paddo);
        self.claim_pip(paddo_pio, paddo);
        assert_eq!(self.xlat_int_wire(bcrd, paddo), bel.pins["TXDATA0"]);

        let paddt_pio = self.rc_io_wire(cell, &format!("PADDT{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDT_PIO", paddt_pio);
        let paddt = self.rc_io_wire(cell, &format!("JPADDT{abcd}"));
        self.add_bel_wire(bcrd, "PADDT", paddt);
        self.claim_pip(paddt_pio, paddt);
        assert_eq!(self.xlat_int_wire(bcrd, paddt), bel.pins["TSDATA0"]);

        let ioldo_iol = self.rc_io_wire(cell, &format!("IOLDO{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLDO_IOLOGIC", ioldo_iol);
        let ioldoi_iol = self.rc_io_wire(cell, &format!("IOLDOI{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLDOI_IOLOGIC", ioldoi_iol);
        self.claim_pip(ioldoi_iol, ioldo_iol);
        let ioldod_iol = self.rc_io_wire(cell, &format!("IOLDOD{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLDOD_IOLOGIC", ioldod_iol);
        let ioldo = self.rc_io_wire(cell, &format!("IOLDO{abcd}"));
        self.add_bel_wire(bcrd, "IOLDO", ioldo);
        self.claim_pip(ioldo, ioldo_iol);
        self.claim_pip(ioldo, ioldod_iol);
        let ioldo_pio = self.rc_io_wire(cell, &format!("IOLDO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "IOLDO_PIO", ioldo_pio);
        self.claim_pip(ioldo_pio, ioldo);

        let iolto_iol = self.rc_io_wire(cell, &format!("IOLTO{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLTO_IOLOGIC", iolto_iol);
        let iolto_pio = self.rc_io_wire(cell, &format!("IOLTO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "IOLTO_PIO", iolto_pio);
        self.claim_pip(iolto_pio, iolto_iol);

        for (pin, bslot_rc, pin_src) in [
            ("INRD", bels::BCINRD, "INRDENO_OUT"),
            ("LVDS", bels::BCLVDSO, "LVDSENO_OUT"),
        ] {
            let wire = self.rc_io_wire(cell, &format!("{pin}{abcd}_PIO"));
            self.add_bel_wire(bcrd, pin, wire);
            let cell_rc = self.chip.special_loc[&SpecialLocKey::Bc(bank)];
            let bcrd_rc = cell_rc.bel(bslot_rc);
            if self.edev.has_bel(bcrd_rc) {
                let wire_src = self.naming.bel_wire(bcrd_rc, pin_src);
                self.claim_pip(wire, wire_src);
            }
        }

        if kind == IoKind::Io {
            for pin in [
                "DQSW", "DQSR90", "DQSW270", "RDPNTR0", "RDPNTR1", "RDPNTR2", "WRPNTR0", "WRPNTR1",
                "WRPNTR2",
            ] {
                let wire = self.rc_io_wire(cell, &format!("{pin}{abcd}_IOLOGIC"));
                self.add_bel_wire(bcrd, pin, wire);
                if let Some(&cell_dqs) = self.edev.dqs.get(&bcrd.cell) {
                    let bcrd_dqs = cell_dqs.bel(bels::DQS0);
                    let wire_dqs = self.naming.bel_wire(bcrd_dqs, &format!("{pin}_OUT"));
                    self.claim_pip(wire, wire_dqs);
                }
            }

            let eclk = self.rc_io_wire(cell, &format!("ECLK{abcd}_IOLOGIC"));
            self.add_bel_wire(bcrd, "ECLK", eclk);
            let eclk_in = self.rc_io_wire(cell, &format!("ECLK{abcd}"));
            self.add_bel_wire(bcrd, "ECLK_IN", eclk_in);
            self.claim_pip(eclk, eclk_in);

            for i in 0..2 {
                let bcrd_eclk = self.chip.bel_eclksync_bank(bank, i);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
                self.claim_pip(eclk_in, wire_eclk);
            }
        }

        self.insert_bel(bcrd, bel);
    }

    pub(super) fn process_io_ecp5(&mut self) {
        for tcname in ["DQS_W", "DQS_E"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                self.process_dqs_ecp5(tcrd.bel(bels::DQS0));
            }
        }
        for (tcname, num_io) in [
            ("IO_W4", 4),
            ("IO_E4", 4),
            ("IO_N2", 2),
            ("IO_S2", 2),
            ("IO_S1", 1),
        ] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                for i in 0..num_io {
                    let bcrd = tcrd.bel(bels::IO[i]);
                    self.process_single_io_ecp5(bcrd);
                }
            }
        }
        if self.naming.strings.get("JCH0_FF_TX_D_0_DCU").is_none() {
            self.skip_serdes = true;
        }
        if self.skip_serdes && self.chip.rows.len() == 49 {
            // what. have you been drinking.
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_clk - 1, self.chip.row_n());
            for wn in [
                "JCFLAGA_SIOLOGIC",
                "JRXDATA1A_SIOLOGIC",
                "JRXDATA0A_SIOLOGIC",
                "JINFFA_SIOLOGIC",
                "JCLKA_SIOLOGIC",
                "JLSRA_SIOLOGIC",
                "JCEA_SIOLOGIC",
                "JTSDATA0A_SIOLOGIC",
                "JTXDATA1A_SIOLOGIC",
                "JTXDATA0A_SIOLOGIC",
                "JDIRECTIONA_SIOLOGIC",
                "JMOVEA_SIOLOGIC",
                "JLOADNA_SIOLOGIC",
                "DIA_SIOLOGIC",
                "INDDA_SIOLOGIC",
                "IOLTOA_SIOLOGIC",
                "IOLDODA_SIOLOGIC",
                "IOLDOA_SIOLOGIC",
                "IOLDOIA_SIOLOGIC",
                "PADDIA_SIOLOGIC",
                "JDIA",
                "IOLDOA",
                "PADDTA_PIO",
                "PADDOA_PIO",
                "IOLTOA_PIO",
                "IOLDOA_PIO",
                "JPADDIA_PIO",
                "LVDSA_PIO",
                "INRDA_PIO",
                "JCFLAGB_SIOLOGIC",
                "JRXDATA1B_SIOLOGIC",
                "JRXDATA0B_SIOLOGIC",
                "JINFFB_SIOLOGIC",
                "JCLKB_SIOLOGIC",
                "JLSRB_SIOLOGIC",
                "JCEB_SIOLOGIC",
                "JTSDATA0B_SIOLOGIC",
                "JTXDATA1B_SIOLOGIC",
                "JTXDATA0B_SIOLOGIC",
                "JDIRECTIONB_SIOLOGIC",
                "JMOVEB_SIOLOGIC",
                "JLOADNB_SIOLOGIC",
                "DIB_SIOLOGIC",
                "INDDB_SIOLOGIC",
                "IOLTOB_SIOLOGIC",
                "IOLDODB_SIOLOGIC",
                "IOLDOB_SIOLOGIC",
                "IOLDOIB_SIOLOGIC",
                "PADDIB_SIOLOGIC",
                "JDIB",
                "IOLDOB",
                "PADDTB_PIO",
                "PADDOB_PIO",
                "IOLTOB_PIO",
                "IOLDOB_PIO",
                "JPADDIB_PIO",
                "LVDSB_PIO",
                "INRDB_PIO",
            ] {
                let wire = self.rc_io_wire(cell, wn);
                self.claim_node(wire);
            }
            for (wt, wf) in [
                ("DIA_SIOLOGIC", "JDIA"),
                ("IOLDOIA_SIOLOGIC", "IOLDOA_SIOLOGIC"),
                ("PADDIA_SIOLOGIC", "JPADDIA_PIO"),
                ("JDIA", "INDDA_SIOLOGIC"),
                ("JDIA", "JPADDIA_PIO"),
                ("IOLDOA", "IOLDODA_SIOLOGIC"),
                ("IOLDOA", "IOLDOA_SIOLOGIC"),
                ("IOLTOA_PIO", "IOLTOA_SIOLOGIC"),
                ("IOLDOA_PIO", "IOLDOA"),
                ("DIB_SIOLOGIC", "JDIB"),
                ("IOLDOIB_SIOLOGIC", "IOLDOB_SIOLOGIC"),
                ("PADDIB_SIOLOGIC", "JPADDIB_PIO"),
                ("JDIB", "INDDB_SIOLOGIC"),
                ("JDIB", "JPADDIB_PIO"),
                ("IOLDOB", "IOLDODB_SIOLOGIC"),
                ("IOLDOB", "IOLDOB_SIOLOGIC"),
                ("IOLTOB_PIO", "IOLTOB_SIOLOGIC"),
                ("IOLDOB_PIO", "IOLDOB"),
            ] {
                let wt = self.rc_io_wire(cell, wt);
                let wf = self.rc_io_wire(cell, wf);
                self.claim_pip(wt, wf);
            }
            for name in ["PT31A", "IOL_T31A", "PT31B", "IOL_T31B"] {
                self.dummy_sites.insert(name.to_string());
            }
        }
    }

    pub(super) fn process_clkdiv_ecp5(&mut self) {
        let cell_w = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk);
        let cell_e = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for (lr, cell) in [('L', cell_w), ('R', cell_e)] {
            for i in 0..2 {
                let bcrd = cell.bel(bels::CLKDIV[i]);
                self.name_bel(bcrd, [format!("CLKDIV_{lr}{i}")]);
                let mut bel = Bel::default();

                for pin in ["ALIGNWD", "RST", "CDIVX"] {
                    let wire = self.rc_io_wire(cell, &format!("J{pin}_CLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }

                let clki = self.rc_io_wire(cell, &format!("CLKI_CLKDIV{i}"));
                self.add_bel_wire(bcrd, "CLKI", clki);
                let clki_in = self.claim_single_in(clki);
                self.add_bel_wire(bcrd, "CLKI_IN", clki_in);

                for bank_idx in 0..2 {
                    let bcrd_eclk = bcrd.bel(bels::ECLKSYNC[bank_idx * 2 + i]);
                    let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
                    self.claim_pip(clki_in, wire_eclk);
                }

                self.insert_bel(bcrd, bel);
            }
        }
        if self.skip_serdes {
            return;
        }

        let serdes_cols = Vec::from_iter(
            self.chip
                .columns
                .ids()
                .filter(|&col| self.chip.columns[col].io_s == IoGroupKind::Serdes),
        );

        let cell_tile = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_s());
        let cell = cell_tile.delta(-1, 0);
        for i in 0..2 {
            let bcrd = cell_tile.bel(bels::PCSCLKDIV[i]);
            self.name_bel(bcrd, [format!("PCSCLKDIV{i}")]);

            let clki = self.rc_wire(cell, &format!("CLKI_PCSCLKDIV{i}"));
            self.add_bel_wire(bcrd, "CLKI", clki);
            let clki_in = self.claim_single_in(clki);
            self.add_bel_wire(bcrd, "CLKI_IN", clki_in);

            if let Some(&col_pcs) = serdes_cols.get(i) {
                let cell_pcs = cell.with_col(col_pcs);
                let mut bel = Bel::default();
                for pin in ["RST", "SEL0", "SEL1", "SEL2"] {
                    let wire = self.rc_wire(cell, &format!("J{pin}_PCSCLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }
                let clki_int = self.rc_wire(cell, &format!("JPCSCDIVCIB{i}"));
                self.add_bel_wire(bcrd, "CLKI_INT", clki_int);
                self.claim_pip(clki_in, clki_int);
                bel.pins
                    .insert("CLKI".into(), self.xlat_int_wire(bcrd, clki_int));

                let abcd = ['A', 'B'][i];
                for ch in 0..2 {
                    let clki_rxclk = self.rc_wire(cell, &format!("JPCS{abcd}RXCLK{ch}"));
                    self.add_bel_wire(bcrd, format!("CLKI_RXCLK{ch}"), clki_rxclk);
                    self.claim_pip(clki_in, clki_rxclk);
                    let wire_pcs = self.rc_io_wire(cell_pcs, &format!("JCH{ch}_FF_RX_PCLK_DCU"));
                    self.claim_pip(clki_rxclk, wire_pcs);

                    let clki_txclk = self.rc_wire(cell, &format!("JPCS{abcd}TXCLK{ch}"));
                    self.add_bel_wire(bcrd, format!("CLKI_TXCLK{ch}"), clki_txclk);
                    self.claim_pip(clki_in, clki_txclk);
                    let wire_pcs = self.rc_io_wire(cell_pcs, &format!("JCH{ch}_FF_TX_PCLK_DCU"));
                    self.claim_pip(clki_txclk, wire_pcs);
                }

                self.insert_bel(bcrd, bel);
            } else {
                for pin in ["RST", "SEL0", "SEL1", "SEL2"] {
                    let wire = self.rc_wire(cell, &format!("J{pin}_PCSCLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                }
            }

            for pin in ["CDIV1", "CDIVX"] {
                let wire = self.rc_wire(cell, &format!("{pin}_PCSCLKDIV{i}"));
                self.add_bel_wire(bcrd, pin, wire);
            }
        }
    }
}

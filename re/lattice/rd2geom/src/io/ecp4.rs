use prjcombine_ecp::{
    bels,
    chip::{RowKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{CellSlotId, LegacyBel},
    dir::{Dir, DirHV, DirV},
    grid::{BelCoord, CellCoord, DieId, EdgeIoCoord},
};

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_bc_ecp4(&mut self) {
        let has_bank0 = self.chip.special_loc.contains_key(&SpecialLocKey::Bc(0));
        for (&key, &cell_loc) in &self.chip.special_loc {
            let SpecialLocKey::Bc(bank) = key else {
                continue;
            };
            let (col, row, suffix, is_corner) = match bank {
                0 => (self.chip.col_w(), self.chip.row_n(), "T", true),
                1 if !has_bank0 => (self.chip.col_w(), self.chip.row_n(), "T", true),
                1 if has_bank0 => (self.chip.col_clk - 1, self.chip.row_n(), "TL", false),
                2 if has_bank0 => (self.chip.col_clk - 1, self.chip.row_n(), "TR", false),
                2 if !has_bank0 => (self.chip.col_e(), self.chip.row_n(), "T", true),
                3 => (self.chip.col_e(), self.chip.row_n(), "T", true),
                4 => (self.chip.col_e(), self.chip.row_n(), "R", true),
                5 => (self.chip.col_e(), self.chip.row_s(), "R", false),
                6 => (self.chip.col_w(), self.chip.row_s(), "L", false),
                7 => (self.chip.col_w(), self.chip.row_n(), "L", true),
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), col, row);
            for (bel, name, pin_out, pin_in) in [
                (bels::BCPG, "BCPG", "PGENO", "PGENI"),
                (bels::BCINRD, "BCINRD", "INRDENO", "INRDENI"),
                (bels::BCLVDSO, "BCLVDSO", "LVDSENO", "LVDSENI"),
                (bels::BCPUSL, "BCPUSL", "PUSLENO", "PUSLENI"),
            ] {
                let bcrd = cell_loc.bel(bel);
                self.name_bel(bcrd, [format!("{name}{bank}")]);

                let wire_in = if is_corner {
                    self.rc_corner_wire(cell, &format!("J{pin_in}_{name}_{suffix}"))
                } else {
                    self.rc_io_wire(cell, &format!("J{pin_in}_{name}_{suffix}"))
                };
                self.add_bel_wire(bcrd, pin_in, wire_in);
                let mut bel = LegacyBel::default();
                bel.pins
                    .insert(pin_in.into(), self.xlat_int_wire(bcrd, wire_in));
                self.insert_bel(bcrd, bel);

                let wire_out = if is_corner {
                    self.rc_corner_wire(cell, &format!("{pin_out}_{name}_{suffix}"))
                } else {
                    self.rc_io_wire(cell, &format!("{pin_out}_{name}_{suffix}"))
                };
                self.add_bel_wire(bcrd, pin_out, wire_out);
                self.claim_pip(wire_out, wire_in);

                let wire_out_out = self.claim_single_out(wire_out);
                self.add_bel_wire(bcrd, format!("{pin_out}_OUT"), wire_out_out);
            }
            {
                let bcrd = cell_loc.bel(bels::BREFTEST);
                self.name_bel(bcrd, [format!("BREFTEST{bank}")]);
                let mut bel = LegacyBel::default();

                for pin in [
                    "TESTIN0", "TESTIN1", "TESTIN2", "TESTIN3", "TESTIN4", "TESTIN5", "TESTOUT0",
                    "TESTOUT1", "TESTOUT2", "TESTOUT3", "TESTOUT4", "TESTOUT5",
                ] {
                    let wire = if is_corner {
                        self.rc_corner_wire(cell, &format!("J{pin}_BREFTEST_{suffix}"))
                    } else {
                        self.rc_io_wire(cell, &format!("J{pin}_BREFTEST_{suffix}"))
                    };
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }
                self.insert_bel(bcrd, bel);
            }
        }
        {
            let cell = self.chip.special_loc[&SpecialLocKey::Bc(if has_bank0 { 3 } else { 2 })];
            let bcrd = cell.bel(bels::PVTTEST);
            self.name_bel(bcrd, ["PVTTEST"]);
            self.insert_simple_bel(bcrd, cell, "PVTTEST");
            let bcrd = cell.bel(bels::PVTCAL);
            self.name_bel(bcrd, ["PVTCAL"]);
            self.insert_simple_bel(bcrd, cell, "PVTCAL");
        }
    }

    pub(super) fn process_eclk_ecp4(&mut self) {
        let cell_n = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_n());
        let cell_w = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk);
        let cell_e = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for (edge, bank_idx) in [
            (Dir::N, 0),
            (Dir::N, 1),
            (Dir::N, 2),
            (Dir::N, 3),
            (Dir::W, 0),
            (Dir::W, 1),
            (Dir::E, 0),
            (Dir::E, 1),
        ] {
            for eclk_idx in 0..4 {
                let (bank, cell, cell_tile) = match edge {
                    Dir::N => (bank_idx, cell_n.delta(-2 + (bank_idx as i32), 0), cell_n),
                    Dir::W => (
                        5 - bank_idx,
                        cell_w.delta(0, -1 + (bank_idx as i32)),
                        cell_w,
                    ),
                    Dir::E => (
                        6 + bank_idx,
                        cell_e.delta(0, -1 + (bank_idx as i32)),
                        cell_e,
                    ),
                    Dir::S => unreachable!(),
                };
                let bcrd = cell_tile.bel(bels::ECLKSYNC[bank_idx * 4 + eclk_idx]);
                if !self.edev.has_bel(bcrd) {
                    continue;
                }
                self.name_bel(bcrd, [format!("ECLKSYNC{eclk_idx}_BK{bank}")]);
                let mut bel = LegacyBel::default();

                let stop = self.rc_io_wire(cell, &format!("JSTOP_ECLKSYNC{eclk_idx}"));
                self.add_bel_wire(bcrd, "STOP", stop);
                bel.pins
                    .insert("STOP".into(), self.xlat_int_wire(bcrd, stop));

                let eclki = self.rc_io_wire(cell, &format!("ECLKI_ECLKSYNC{eclk_idx}"));
                self.add_bel_wire(bcrd, "ECLKI", eclki);
                let eclki_in = self.claim_single_in(eclki);
                self.add_bel_wire(bcrd, "ECLKI_IN", eclki_in);

                let eclki_int = self.rc_io_wire(cell, &format!("JECLKCIB{eclk_idx}"));
                self.add_bel_wire(bcrd, "ECLKI_INT", eclki_int);
                self.claim_pip(eclki_in, eclki_int);
                bel.pins
                    .insert("ECLKI".into(), self.xlat_int_wire(bcrd, eclki_int));

                for i in 0..8 {
                    if !self.edev.has_bel(bcrd.bel(bels::DLLDEL[i])) {
                        continue;
                    }
                    let name = match edge {
                        Dir::N => ["00", "01", "10", "11", "20", "21", "30", "31"][i],
                        Dir::W => ["60", "61", "70", "71"][i],
                        Dir::E => ["50", "51", "40", "41"][i],
                        Dir::S => unreachable!(),
                    };
                    let pclk = self.rc_io_wire(cell, &format!("JPCLKT{name}"));
                    self.add_bel_wire_no_claim(bcrd, format!("IO_IN_{edge}{i}"), pclk);
                    self.claim_pip(eclki_in, pclk);
                    if eclk_idx == 0 {
                        self.claim_node(pclk);
                        let cell_dlldel = match edge {
                            Dir::N => bcrd.cell.delta(-4 + (i as i32), 0),
                            Dir::W => bcrd.cell.delta(0, -2 + (i as i32)),
                            Dir::E => bcrd.cell.delta(0, -2 + ((i ^ 1) as i32)),
                            Dir::S => unreachable!(),
                        };
                        let wire_dlldel = self.rc_io_wire(cell_dlldel, "JINCK");
                        self.claim_pip(pclk, wire_dlldel);
                    }
                }

                let plls = match edge {
                    Dir::W => [DirHV::SW, DirHV::NW],
                    Dir::E => [DirHV::SE, DirHV::NE],
                    Dir::N => [DirHV::NW, DirHV::NE],
                    Dir::S => unreachable!(),
                };
                for hv in plls {
                    let corner = match hv {
                        DirHV::SW => "LL",
                        DirHV::SE => "LR",
                        DirHV::NW => "UL",
                        DirHV::NE => "UR",
                    };
                    for i in 0..2 {
                        for pin in ["CLKOP", "CLKOS"] {
                            let wire = self.rc_io_wire(cell, &format!("J{corner}CPLL{i}{pin}"));
                            self.add_bel_wire_no_claim(bcrd, format!("PLL_IN_{hv}{i}_{pin}"), wire);
                            self.claim_pip(eclki_in, wire);
                            if eclk_idx == 0 {
                                self.claim_node(wire);
                                let cell_pll = cell.with_cr(
                                    self.chip.col_edge(hv.h),
                                    match hv.v {
                                        DirV::S => self.chip.row_s() + (2 - i),
                                        DirV::N => self.chip.row_n() - (1 + i),
                                    },
                                );
                                let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                                self.claim_pip(wire, wire_pll);
                            }
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

                if edge == Dir::N {
                    if bank_idx != 0
                        && self
                            .edev
                            .has_bel(bcrd.bel(bels::ECLKSYNC[(bank_idx - 1) * 4]))
                    {
                        let cell_prev = cell.delta(-1, 0);
                        let eclk_prev = self.rc_io_wire(cell, &format!("JPREVECLK{eclk_idx}"));
                        self.add_bel_wire(bcrd, "ECLK_PREV", eclk_prev);
                        self.claim_pip(eclk_mux, eclk_prev);
                        let wire_prev =
                            self.rc_io_wire(cell_prev, &format!("JECLKO_ECLKSYNC{eclk_idx}"));
                        self.claim_pip(eclk_prev, wire_prev);
                    }
                    if bank_idx != 3
                        && self
                            .edev
                            .has_bel(bcrd.bel(bels::ECLKSYNC[(bank_idx + 1) * 4]))
                    {
                        let cell_next = cell.delta(1, 0);
                        let eclk_next = self.rc_io_wire(cell, &format!("JNEXTECLK{eclk_idx}"));
                        self.add_bel_wire(bcrd, "ECLK_NEXT", eclk_next);
                        self.claim_pip(eclk_mux, eclk_next);
                        let wire_next =
                            self.rc_io_wire(cell_next, &format!("JECLKO_ECLKSYNC{eclk_idx}"));
                        self.claim_pip(eclk_next, wire_next);
                    }
                } else {
                    let cell_neigh = if bank_idx == 0 {
                        cell.delta(0, 1)
                    } else {
                        cell.delta(0, -1)
                    };
                    let eclk_neigh = self.rc_io_wire(cell, &format!("JNEIGHBORECLK{eclk_idx}"));
                    self.add_bel_wire(bcrd, "ECLK_NEIGHBOR", eclk_neigh);
                    self.claim_pip(eclk_mux, eclk_neigh);
                    let wire_neigh =
                        self.rc_io_wire(cell_neigh, &format!("JECLKO_ECLKSYNC{eclk_idx}"));
                    self.claim_pip(eclk_neigh, wire_neigh);
                }

                self.insert_bel(bcrd, bel);
            }
        }
        for (lrt, cell_tile, cell, n) in [
            ('T', cell_n, cell_n.delta(-1, 0), 7),
            ('L', cell_w, cell_w, 12),
            ('R', cell_e, cell_e, 12),
        ] {
            let bcrd = cell_tile.bel(bels::CLKTEST_ECLK);
            self.name_bel(bcrd, [format!("CLKTEST_{lrt}ECLK")]);
            let mut bel = LegacyBel::default();

            for i in 0..24 {
                let wire = self.rc_io_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                if i < n {
                    bel.pins
                        .insert(format!("TESTIN{i}"), self.xlat_int_wire(bcrd, wire));
                }
            }

            self.insert_bel(bcrd, bel);
        }

        for (cell, base) in [(cell_w, 2), (cell_e, 0)] {
            for i in 0..2 {
                let eclk_idx = base + i;
                let bcrd_cs = cell.bel(bels::ECLKBRIDGECS[i]);
                self.name_bel(bcrd_cs, [format!("ECLKBRIDGECS{eclk_idx}")]);
                let mut bel = LegacyBel::default();

                let sel = self.rc_io_wire(cell, &format!("JSEL_ECLKBRIDGECS{eclk_idx}"));
                self.add_bel_wire(bcrd_cs, "SEL", sel);
                bel.pins
                    .insert("SEL".into(), self.xlat_int_wire(bcrd_cs, sel));

                let clk0 = self.rc_io_wire(cell, &format!("JCLK0_ECLKBRIDGECS{eclk_idx}"));
                self.add_bel_wire(bcrd_cs, "CLK0", clk0);
                let bcrd_eclk = cell.bel(bels::ECLKSYNC[4 + eclk_idx]);
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

                let bcrd_brg = cell.bel(bels::BRGECLKSYNC[i]);
                self.name_bel(bcrd_brg, [format!("BRGECLKSYNC{eclk_idx}")]);
                let mut bel = LegacyBel::default();

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
    }

    pub(super) fn process_ddrdll_ecp4(&mut self) {
        for (col, row, ncol, idx, is_corner, name, eclk_banks) in [
            (
                self.chip.col_w() + 2,
                self.chip.row_s(),
                self.chip.col_w(),
                0,
                false,
                "DDRDLL_BL0",
                [6].as_slice(),
            ),
            (
                self.chip.col_w() + 3,
                self.chip.row_s(),
                self.chip.col_w(),
                1,
                false,
                "DDRDLL_BL1",
                [6].as_slice(),
            ),
            (
                self.chip.col_e() - 3,
                self.chip.row_s(),
                self.chip.col_e(),
                0,
                false,
                "DDRDLL_BR0",
                [5].as_slice(),
            ),
            (
                self.chip.col_e() - 2,
                self.chip.row_s(),
                self.chip.col_e(),
                1,
                false,
                "DDRDLL_BR1",
                [5].as_slice(),
            ),
            (
                self.chip.col_w() + 2,
                self.chip.row_n(),
                self.chip.col_w(),
                0,
                true,
                "DDRDLL_TL0",
                [7, 1].as_slice(),
            ),
            (
                self.chip.col_w() + 3,
                self.chip.row_n(),
                self.chip.col_w(),
                1,
                true,
                "DDRDLL_TL1",
                [7, 1].as_slice(),
            ),
            (
                self.chip.col_e() - 3,
                self.chip.row_n(),
                self.chip.col_e(),
                0,
                true,
                "DDRDLL_TR0",
                [2, 4].as_slice(),
            ),
            (
                self.chip.col_e() - 2,
                self.chip.row_n(),
                self.chip.col_e(),
                1,
                true,
                "DDRDLL_TR1",
                [2, 4].as_slice(),
            ),
        ] {
            let bcrd = CellCoord::new(DieId::from_idx(0), col, row).bel(bels::DDRDLL);
            let cell = CellCoord::new(DieId::from_idx(0), ncol, row);
            self.name_bel(bcrd, [name]);
            let mut bel = LegacyBel::default();
            for pin in [
                "RST", "FREEZE", "UDDCNTLN", "DIVOSC", "DCNTL0", "DCNTL1", "DCNTL2", "DCNTL3",
                "DCNTL4", "DCNTL5", "DCNTL6", "DCNTL7", "LOCK",
            ] {
                let wire = if is_corner {
                    self.rc_corner_wire(cell, &format!("J{pin}_DDRDLL{idx}"))
                } else {
                    self.rc_io_wire(cell, &format!("J{pin}_DDRDLL{idx}"))
                };
                self.add_bel_wire(bcrd, pin, wire);
                bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
            }

            let clk = if is_corner {
                self.rc_corner_wire(cell, &format!("JCLK_DDRDLL{idx}"))
            } else {
                self.rc_io_wire(cell, &format!("JCLK_DDRDLL{idx}"))
            };
            self.add_bel_wire(bcrd, "CLK", clk);
            let clk_in = self.claim_single_in(clk);
            self.add_bel_wire(bcrd, "CLK_IN", clk_in);

            let clk_int = if is_corner {
                self.rc_corner_wire(cell, &format!("JCIBCLK{idx}"))
            } else {
                self.rc_io_wire(cell, &format!("JCIBCLK{idx}"))
            };
            self.add_bel_wire(bcrd, "CLK_INT", clk_int);
            self.claim_pip(clk_in, clk_int);
            bel.pins
                .insert("CLK".into(), self.xlat_int_wire(bcrd, clk_int));

            for &bank in eclk_banks {
                for i in 0..4 {
                    let bcrd_eclk = self.chip.bel_eclksync_bank(bank, i);
                    let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
                    self.claim_pip(clk_in, wire_eclk);
                }
            }

            let ddrdel = if is_corner {
                self.rc_corner_wire(cell, &format!("DDRDEL_DDRDLL{idx}"))
            } else {
                self.rc_io_wire(cell, &format!("DDRDEL_DDRDLL{idx}"))
            };
            self.add_bel_wire(bcrd, "DDRDEL", ddrdel);
            let ddrdel_out = self.claim_single_out(ddrdel);
            self.add_bel_wire(bcrd, "DDRDEL_OUT", ddrdel_out);

            self.insert_bel(bcrd, bel);
        }
    }

    pub(super) fn process_dlldel_ecp4(&mut self) {
        let cell_n = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_n());
        let cell_w = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk);
        let cell_e = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for (name, edge, idx) in [
            ("00", Dir::N, 0),
            ("01", Dir::N, 1),
            ("10", Dir::N, 2),
            ("11", Dir::N, 3),
            ("20", Dir::N, 4),
            ("21", Dir::N, 5),
            ("30", Dir::N, 6),
            ("31", Dir::N, 7),
            ("60", Dir::W, 0),
            ("61", Dir::W, 1),
            ("70", Dir::W, 2),
            ("71", Dir::W, 3),
            ("50", Dir::E, 0),
            ("51", Dir::E, 1),
            ("40", Dir::E, 2),
            ("41", Dir::E, 3),
        ] {
            let Some(&io) = self
                .chip
                .special_io
                .get(&SpecialIoKey::Clock(edge, idx as u8))
            else {
                continue;
            };
            let (cell, cell_tile) = match edge {
                Dir::N => (cell_n.delta(-4 + (idx as i32), 0), cell_n),
                Dir::W => (cell_w.delta(0, -2 + (idx as i32)), cell_w),
                Dir::E => (cell_e.delta(0, -2 + ((idx ^ 1) as i32)), cell_e),
                Dir::S => unreachable!(),
            };
            let bcrd = cell_tile.bel(bels::DLLDEL[idx]);
            self.name_bel(bcrd, [format!("DLLDEL_{name}")]);

            let (cell_io, abcd) = self.xlat_io_loc_ecp4(io);
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

            let ddrdll_sources = match edge {
                Dir::W => [
                    (self.chip.col_w() + 2, self.chip.row_s()),
                    (self.chip.col_w() + 3, self.chip.row_s()),
                    (self.chip.col_w() + 2, self.chip.row_n()),
                    (self.chip.col_w() + 3, self.chip.row_n()),
                ],
                Dir::E => [
                    (self.chip.col_e() - 3, self.chip.row_s()),
                    (self.chip.col_e() - 2, self.chip.row_s()),
                    (self.chip.col_e() - 3, self.chip.row_n()),
                    (self.chip.col_e() - 2, self.chip.row_n()),
                ],
                Dir::N => [
                    (self.chip.col_w() + 2, self.chip.row_n()),
                    (self.chip.col_w() + 3, self.chip.row_n()),
                    (self.chip.col_e() - 3, self.chip.row_n()),
                    (self.chip.col_e() - 2, self.chip.row_n()),
                ],
                Dir::S => unreachable!(),
            };
            for (col, row) in ddrdll_sources {
                let bcrd_ddrdll = cell.with_cr(col, row).bel(bels::DDRDLL);
                let wire_ddrdll = self.naming.bel_wire(bcrd_ddrdll, "DDRDEL_OUT");
                self.claim_pip(ddrdel_in, wire_ddrdll);
            }

            self.insert_simple_bel(bcrd, cell, "DLLDEL");
        }
    }

    pub(super) fn process_dtr_ecp4(&mut self) {
        for (tcname, name) in [("DTR_S", "DTR_BR"), ("DTR_N", "DTR_TL")] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::DTR);
                self.name_bel(bcrd, [name]);
                self.insert_simple_bel(bcrd, bcrd.cell, "DTR");
            }
        }
    }

    pub fn xlat_io_loc_ecp4(&self, io: EdgeIoCoord) -> (CellCoord, &'static str) {
        let bcrd = self.chip.get_io_loc(io);
        let tcrd = self.edev.bel_tile(bcrd);
        let mut cell = self.edev[tcrd].cells[CellSlotId::from_idx(io.iob().to_idx() % 4)];
        let mut abcd = "";
        if let Dir::H(edge) = io.edge()
            && cell.col != self.chip.col_edge(edge)
        {
            abcd = match self.chip.rows[cell.row].kind {
                RowKind::Ebr => ["EA", "EB", "EC", "ED"][io.iob().to_idx() % 4],
                RowKind::Dsp => ["EA", "EB"][io.iob().to_idx() % 2],
                _ => unreachable!(),
            };
            cell = cell.with_col(self.chip.col_edge(edge))
        }
        (cell, abcd)
    }

    fn process_dqs_ecp4(&mut self, bcrd: BelCoord) {
        let io = self.chip.get_io_crd(bcrd.bel(bels::IO0));
        let bank = self.chip.get_io_bank(io);
        let (cell, abcd) = self.xlat_io_loc_ecp4(io);

        let (r, c) = self.rc(cell);
        match io.edge() {
            Dir::W => {
                self.name_bel(bcrd, [format!("LDQS{r}")]);
            }
            Dir::E => {
                self.name_bel(bcrd, [format!("RDQS{r}")]);
            }
            Dir::N => {
                self.name_bel(bcrd, [format!("TDQS{c}")]);
            }
            Dir::S => unreachable!(),
        }
        self.insert_simple_bel(bcrd, cell, "DQS");

        let dqsi = self.rc_io_wire(cell, "JDQSI_DQS");
        self.add_bel_wire(bcrd, "DQSI", dqsi);
        let paddi = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.claim_pip(dqsi, paddi);

        let ddrdel = self.rc_io_wire(cell, "DDRDEL_DQS");
        self.add_bel_wire(bcrd, "DDRDEL", ddrdel);
        let ddrdel_in = self.rc_io_wire(cell, "DDRDEL");
        self.add_bel_wire(bcrd, "DDRDEL_IN", ddrdel_in);
        self.claim_pip(ddrdel, ddrdel_in);

        let ddrdll_sources = match io.edge() {
            Dir::W => [
                (self.chip.col_w() + 2, self.chip.row_s()),
                (self.chip.col_w() + 3, self.chip.row_s()),
                (self.chip.col_w() + 2, self.chip.row_n()),
                (self.chip.col_w() + 3, self.chip.row_n()),
            ],
            Dir::E => [
                (self.chip.col_e() - 3, self.chip.row_s()),
                (self.chip.col_e() - 2, self.chip.row_s()),
                (self.chip.col_e() - 3, self.chip.row_n()),
                (self.chip.col_e() - 2, self.chip.row_n()),
            ],
            Dir::N => [
                (self.chip.col_w() + 2, self.chip.row_n()),
                (self.chip.col_w() + 3, self.chip.row_n()),
                (self.chip.col_e() - 3, self.chip.row_n()),
                (self.chip.col_e() - 2, self.chip.row_n()),
            ],
            Dir::S => unreachable!(),
        };
        for (col, row) in ddrdll_sources {
            let bcrd_ddrdll = cell.with_cr(col, row).bel(bels::DDRDLL);
            let wire_ddrdll = self.naming.bel_wire(bcrd_ddrdll, "DDRDEL_OUT");
            self.claim_pip(ddrdel_in, wire_ddrdll);
        }

        let eclk = self.rc_io_wire(cell, "ECLK_DQS");
        self.add_bel_wire(bcrd, "ECLK", eclk);
        let eclk_in = self.rc_io_wire(cell, "DQSECLK");
        self.add_bel_wire(bcrd, "ECLK_IN", eclk_in);
        self.claim_pip(eclk, eclk_in);

        let eclk90 = self.rc_io_wire(cell, "ECLK90_DQS");
        self.add_bel_wire(bcrd, "ECLK90", eclk90);
        let eclk90_in = self.rc_io_wire(cell, "ECLK90");
        self.add_bel_wire(bcrd, "ECLK90_IN", eclk90_in);
        self.claim_pip(eclk90, eclk90_in);

        for i in 0..4 {
            let bcrd_eclk = self.chip.bel_eclksync_bank(bank, i);
            let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
            self.claim_pip(eclk_in, wire_eclk);
            self.claim_pip(eclk90_in, wire_eclk);
        }

        for pin in [
            "NEXTCLK", "RDPNTR0", "RDPNTR1", "RDPNTR2", "WRPNTR0", "WRPNTR1", "WRPNTR2",
        ] {
            let wire = self.rc_io_wire(cell, &format!("{pin}_DQS"));
            self.add_bel_wire(bcrd, pin, wire);
            let wire_out = self.claim_single_out(wire);
            self.add_bel_wire(bcrd, format!("{pin}_OUT"), wire_out);
        }
        for pin in ["CRUCLK", "DQSR90", "DQSW270", "DQSW"] {
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

    fn process_single_io_ecp4(&mut self, bcrd: BelCoord) {
        let io = self.chip.get_io_crd(bcrd);
        let idx = io.iob().to_idx() % 4;
        let (cell, abcd) = self.xlat_io_loc_ecp4(io);

        let (r, c) = self.rc(cell);
        match io.edge() {
            Dir::W => {
                self.name_bel(bcrd, [format!("PL{r}{abcd}"), format!("IOL_L{r}{abcd}")]);
            }
            Dir::E => {
                self.name_bel(bcrd, [format!("PR{r}{abcd}"), format!("IOL_R{r}{abcd}")]);
            }
            Dir::N => {
                self.name_bel(bcrd, [format!("PT{c}"), format!("IOL_T{c}")]);
            }
            Dir::S => unreachable!(),
        }
        let mut bel = LegacyBel::default();

        for pin in [
            "CLK",
            "LSR",
            "CE",
            "LOADN",
            "MOVE",
            "DIRECTION",
            "WINDOWSIZE0",
            "WINDOWSIZE1",
            "ACK",
            "TXDATA0",
            "TXDATA1",
            "TXDATA2",
            "TXDATA3",
            "TXDATA4",
            "TXDATA5",
            "TXDATA6",
            "TXDATA7",
            "TXDATA8",
            "TXDATA9",
            "TSDATA0",
            "TSDATA1",
            "TSDATA2",
            "TSDATA3",
            "SLIP",
            "RXDATA0",
            "RXDATA1",
            "RXDATA2",
            "RXDATA3",
            "RXDATA4",
            "RXDATA5",
            "RXDATA6",
            "RXDATA7",
            "RXDATA8",
            "RXDATA9",
            "MINUS",
            "PLUS",
            "CFLAG",
        ] {
            let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_IOLOGIC"));
            self.add_bel_wire(bcrd, pin, wire);
            bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
        }

        let paddi_pio = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDI_PIO", paddi_pio);
        let paddi_iol = self.rc_io_wire(cell, &format!("JPADDI{abcd}_IOLOGIC"));
        self.add_bel_wire(bcrd, "PADDI_IOLOGIC", paddi_iol);
        self.claim_pip(paddi_iol, paddi_pio);
        let indd_iol = self.rc_io_wire(cell, &format!("INDD{abcd}_IOLOGIC"));
        self.add_bel_wire(bcrd, "INDD_IOLOGIC", indd_iol);
        let di = self.rc_io_wire(cell, &format!("JDI{abcd}"));
        self.add_bel_wire(bcrd, "DI", di);
        self.claim_pip(di, paddi_pio);
        self.claim_pip(di, indd_iol);
        bel.pins.insert("DI".into(), self.xlat_int_wire(bcrd, di));
        let di_iol = self.rc_io_wire(cell, &format!("DI{abcd}_IOLOGIC"));
        self.add_bel_wire(bcrd, "DI_IOLOGIC", di_iol);
        self.claim_pip(di_iol, di);
        if idx == 3 {
            let bcrd2 = bcrd.bel(bels::IO2);
            let io2 = self.chip.get_io_crd(bcrd2);
            let (cell2, abcd2) = self.xlat_io_loc_ecp4(io2);
            let paddi_pio2 = self.rc_io_wire(cell2, &format!("JPADDI{abcd2}_PIO"));
            self.claim_pip(paddi_iol, paddi_pio2);
            self.claim_pip(di, paddi_pio2);
        }

        let inff = self.rc_io_wire(cell, &format!("JINFF{abcd}_IOLOGIC"));
        self.add_bel_wire(bcrd, "INFF", inff);
        assert_eq!(self.xlat_int_wire(bcrd, inff), bel.pins["RXDATA0"]);

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

        let ioldo_iol = self.rc_io_wire(cell, &format!("IOLDO{abcd}_IOLOGIC"));
        self.add_bel_wire(bcrd, "IOLDO_IOLOGIC", ioldo_iol);
        let ioldoi_iol = self.rc_io_wire(cell, &format!("IOLDOI{abcd}_IOLOGIC"));
        self.add_bel_wire(bcrd, "IOLDOI_IOLOGIC", ioldoi_iol);
        self.claim_pip(ioldoi_iol, ioldo_iol);
        let ioldod_iol = self.rc_io_wire(cell, &format!("IOLDOD{abcd}_IOLOGIC"));
        self.add_bel_wire(bcrd, "IOLDOD_IOLOGIC", ioldod_iol);
        let ioldo = self.rc_io_wire(cell, &format!("IOLDO{abcd}"));
        self.add_bel_wire(bcrd, "IOLDO", ioldo);
        self.claim_pip(ioldo, ioldo_iol);
        self.claim_pip(ioldo, ioldod_iol);
        let ioldo_pio = self.rc_io_wire(cell, &format!("IOLDO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "IOLDO_PIO", ioldo_pio);
        self.claim_pip(ioldo_pio, ioldo);

        let iolto_iol = self.rc_io_wire(cell, &format!("IOLTO{abcd}_IOLOGIC"));
        self.add_bel_wire(bcrd, "IOLTO_IOLOGIC", iolto_iol);
        let iolto_pio = self.rc_io_wire(cell, &format!("IOLTO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "IOLTO_PIO", iolto_pio);
        self.claim_pip(iolto_pio, iolto_iol);

        let bank = self.chip.get_io_bank(io);
        let has_fucked_pusl = io.edge() == Dir::W
            && abcd.is_empty()
            && if cell.row < self.chip.row_clk {
                self.chip.rows[cell.row].kind == RowKind::Ebr
            } else {
                self.chip.rows[cell.row].kind == RowKind::Dsp
            };
        for (pin, bslot_rc, pin_src) in [
            ("PG", bels::BCPG, "PGENO_OUT"),
            ("INRD", bels::BCINRD, "INRDENO_OUT"),
            ("LVDS", bels::BCLVDSO, "LVDSENO_OUT"),
            ("PUSL", bels::BCPUSL, "PUSLENO_OUT"),
        ] {
            let wire = if has_fucked_pusl && pin == "PUSL" {
                // what in *fuck's* name
                self.rc_io_wire(cell, &format!("LVDS{abcd}_PIO"))
            } else {
                let wire = self.rc_io_wire(cell, &format!("{pin}{abcd}_PIO"));
                self.add_bel_wire(bcrd, pin, wire);
                wire
            };
            let cell_rc = self.chip.special_loc[&SpecialLocKey::Bc(bank)];
            let bcrd_rc = cell_rc.bel(bslot_rc);
            let wire_src = self.naming.bel_wire(bcrd_rc, pin_src);
            self.claim_pip(wire, wire_src);
        }

        for pin in [
            "DQSW", "DQSR90", "DQSW270", "RDPNTR0", "RDPNTR1", "RDPNTR2", "WRPNTR0", "WRPNTR1",
            "WRPNTR2",
        ] {
            let wire = if bcrd.col == self.chip.col_e() - 1
                && let Some(idx) = pin.strip_prefix("RDPNTR")
            {
                // fuck's sake fuck's sake fuck's sake
                self.rc_io_wire(cell, &format!("REPNTR{idx}{abcd}_IOLOGIC"))
            } else {
                self.rc_io_wire(cell, &format!("{pin}{abcd}_IOLOGIC"))
            };
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

        for i in 0..4 {
            let bcrd_eclk = self.chip.bel_eclksync_bank(bank, i);
            let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
            self.claim_pip(eclk_in, wire_eclk);
        }

        if let Some(&cell_dqs) = self.edev.dqs.get(&bcrd.cell) {
            let bcrd_dqs = cell_dqs.bel(bels::DQS0);
            let nextclk = self.naming.bel_wire(bcrd_dqs, "NEXTCLK_OUT");
            self.claim_pip(eclk_in, nextclk);
            let cruclk = self.naming.bel_wire(bcrd_dqs, "CRUCLK_OUT");
            self.claim_pip(eclk_in, cruclk);
        }

        self.insert_bel(bcrd, bel);
    }

    pub(super) fn process_io_ecp4(&mut self) {
        for tcname in [
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
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::DQS0);
                self.process_dqs_ecp4(bcrd);
            }
        }
        for tcname in [
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
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                for i in 0..4 {
                    let bcrd = tcrd.bel(bels::IO[i]);
                    self.process_single_io_ecp4(bcrd);
                }
            }
        }
    }

    pub(super) fn process_clkdiv_ecp4(&mut self) {
        let cell_n = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_n());
        let cell_w = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk);
        let cell_e = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for (lrt, cell_tile, cell) in [
            ('T', cell_n, cell_n.delta(-1, 0)),
            ('L', cell_w, cell_w),
            ('R', cell_e, cell_e),
        ] {
            for i in 0..4 {
                let bcrd = cell_tile.bel(bels::CLKDIV[i]);
                self.name_bel(bcrd, [format!("CLKDIV_{lrt}{i}")]);
                let mut bel = LegacyBel::default();

                for pin in ["ALIGNWD", "RST", "CDIVX"] {
                    let wire = self.rc_io_wire(cell, &format!("J{pin}_CLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }

                let clki = self.rc_io_wire(cell, &format!("CLKI_CLKDIV{i}"));
                self.add_bel_wire(bcrd, "CLKI", clki);
                let clki_in = self.claim_single_in(clki);
                self.add_bel_wire(bcrd, "CLKI_IN", clki_in);

                for bank_idx in 0..4 {
                    let bcrd_eclk = bcrd.bel(bels::ECLKSYNC[bank_idx * 4 + i]);
                    if !self.edev.has_bel(bcrd_eclk) {
                        continue;
                    }
                    let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
                    self.claim_pip(clki_in, wire_eclk);
                }

                self.insert_bel(bcrd, bel);
            }
        }

        let num_quads = if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesSingle)
        {
            1
        } else if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesDouble)
        {
            2
        } else if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesTriple)
        {
            3
        } else {
            unreachable!()
        };
        let cell_pcs = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_s());

        let cell_tile = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_s());
        let cell = cell_tile.delta(-1, 0);
        for i in 0..4 {
            let bcrd = cell_tile.bel(bels::PCSCLKDIV[i]);
            self.name_bel(bcrd, [format!("PCSCLKDIV{i}")]);

            let clki = self.rc_wire(cell, &format!("CLKI_PCSCLKDIV{i}"));
            self.add_bel_wire(bcrd, "CLKI", clki);
            let clki_in = self.claim_single_in(clki);
            self.add_bel_wire(bcrd, "CLKI_IN", clki_in);

            if i < num_quads {
                let mut bel = LegacyBel::default();
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

                let abcd = ['A', 'B', 'C', 'D'][i];
                for j in 0..4 {
                    let clki_rxclk = self.rc_wire(cell, &format!("JPCS{abcd}RXCLK{j}"));
                    self.add_bel_wire(bcrd, format!("CLKI_RXCLK{j}"), clki_rxclk);
                    self.claim_pip(clki_in, clki_rxclk);
                    let wire_pcs = self.rc_io_sn_wire(cell_pcs, &format!("JQ{i}CH{j}_FOPCLKB_ASB"));
                    self.claim_pip(clki_rxclk, wire_pcs);

                    let clki_txclk = self.rc_wire(cell, &format!("JPCS{abcd}TXCLK{j}"));
                    self.add_bel_wire(bcrd, format!("CLKI_TXCLK{j}"), clki_txclk);
                    self.claim_pip(clki_in, clki_txclk);
                    let wire_pcs = self.rc_io_sn_wire(cell_pcs, &format!("JQ{i}CH{j}_FOPCLKA_ASB"));
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

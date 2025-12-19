use std::collections::{BTreeMap, btree_map};

use prjcombine_ecp::{
    bels,
    chip::{IoGroupKind, SpecialIoKey},
};
use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{Bel, BelPin, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, DieId},
};
use prjcombine_re_lattice_naming::WireName;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pclk_scm(&mut self) -> BTreeMap<(DirHV, usize), WireName> {
        let mut hpcx = BTreeMap::new();
        let mut hpbx = BTreeMap::new();
        let mut vpsx = BTreeMap::new();
        for i in 0..12 {
            let pclk = self.intdb.get_wire(&format!("PCLK{i}"));
            for cell in self.edev.die_cells(DieId::from_idx(0)) {
                if !self.edev.has_bel(cell.bel(bels::INT)) {
                    continue;
                }
                let pclk = self.naming.interconnect[&cell.wire(pclk)];
                let cur_hpbx = self.claim_single_in(pclk);
                let col_tag = self.pclk_cols[cell.col].0;
                let cell_tag = cell.with_col(col_tag);
                match hpbx.entry((cell_tag, i)) {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(cur_hpbx);
                    }
                    btree_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), cur_hpbx);
                    }
                }
            }
            for cell in self.edev.die_cells(DieId::from_idx(0)) {
                if !self.chip.columns[cell.col].pclk_drive {
                    continue;
                }
                let col_tag = self.pclk_cols[cell.col].0;
                let cell_tag = cell.with_col(col_tag);
                let Some(&cur_hpbx) = hpbx.get(&(cell_tag, i)) else {
                    continue;
                };
                let bcrd = cell.bel(bels::INT);
                if !self.edev.has_bel(bcrd) && i == 0 {
                    self.name_bel_null(bcrd);
                }
                self.add_bel_wire(bcrd, format!("PCLK{i}"), cur_hpbx);
                let cur_vpsx = self.claim_single_in(cur_hpbx);
                self.add_bel_wire_no_claim(bcrd, format!("PCLK{i}_IN"), cur_vpsx);
                let v = if cell.row < self.chip.row_clk {
                    DirV::S
                } else {
                    DirV::N
                };
                match vpsx.entry((cell.col, v, i)) {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(cur_vpsx);
                        self.claim_node(cur_vpsx);
                        let h = if cell.col < self.chip.col_clk {
                            DirH::W
                        } else {
                            DirH::E
                        };
                        let hv = DirHV { h, v };
                        let cur_hpcx = self.claim_single_in(cur_vpsx);
                        match hpcx.entry((hv, i)) {
                            btree_map::Entry::Vacant(e) => {
                                e.insert(cur_hpcx);
                            }
                            btree_map::Entry::Occupied(e) => {
                                assert_eq!(*e.get(), cur_hpcx);
                            }
                        }
                    }
                    btree_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), cur_vpsx);
                    }
                }
            }
        }
        hpcx
    }

    pub(super) fn process_clk_edge_scm(&mut self) {
        for edge in Dir::DIRS {
            let lrbt = match edge {
                Dir::W => 'L',
                Dir::E => 'R',
                Dir::S => 'B',
                Dir::N => 'T',
            };
            let (col, row) = match edge {
                Dir::H(edge) => (self.chip.col_edge(edge), self.chip.row_clk),
                Dir::V(edge) => (self.chip.col_clk, self.chip.row_edge(edge)),
            };
            let cell_tile = CellCoord::new(DieId::from_idx(0), col, row);
            let cell = match edge {
                Dir::H(_) => cell_tile,
                Dir::V(_) => cell_tile.delta(-1, 0),
            };
            for i in 0..2 {
                let bcrd = cell_tile.bel(bels::DCS[i]);
                let ab = ['A', 'B'][i];
                self.name_bel(bcrd, [format!("DCS{lrbt}{ab}")]);
                let mut bel = Bel::default();

                let dcs = self.rc_io_wire(cell, &format!("DCS{ab}_DCS"));
                self.add_bel_wire(bcrd, "DCS", dcs);

                for pin in ["CLK0", "CLK1", "SEL"] {
                    let wire = self.rc_io_wire(cell, &format!("J{pin}{ab}_DCS"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                    if pin == "CLK0" {
                        self.claim_pip(dcs, wire);
                    }
                }

                self.insert_bel(bcrd, bel);
            }
            {
                let bcrd = cell_tile.bel(bels::CLK_EDGE);
                self.name_bel_null(bcrd);
                self.insert_bel(bcrd, Bel::default());

                for i in 0..2 {
                    let ab = ['A', 'B'][i];
                    let wire = self.rc_io_wire(cell, &format!("DCS{ab}"));
                    self.add_bel_wire(bcrd, format!("IN_DCS{i}"), wire);
                    let wire_dcs = self.rc_io_wire(cell, &format!("DCS{ab}_DCS"));
                    self.claim_pip(wire, wire_dcs);
                }

                let banks: &[_] = match edge {
                    Dir::W => &[(0, 7), (4, 6)],
                    Dir::E => &[(0, 2), (4, 3)],
                    Dir::S => &[(0, 5), (8, 4)],
                    Dir::N => &[(0, 1)],
                };
                for &(bank_base, bank) in banks {
                    if !matches!(bank, 3 | 6) {
                        for i in 0..4 {
                            let abcd = ['A', 'B', 'C', 'D'][i];
                            let wire = self.rc_io_wire(cell, &format!("JCLKD{bank}{abcd}"));
                            self.add_bel_wire(bcrd, format!("IN_CLKDIV_{bank}_{i}"), wire);
                            let bcrd_cdiv = self.chip.bel_eclk_root_bank(bank).bel(bels::CLKDIV[i]);
                            let wire_clkdiv = self.naming.bel_wire(bcrd_cdiv, "CLKO");
                            self.claim_pip(wire, wire_clkdiv);
                        }
                    }
                    for i in 0..4 {
                        let idx = bank_base + i;
                        let wire = self.rc_io_wire(cell, &format!("JPCK{bank}{i}"));
                        self.add_bel_wire(bcrd, format!("IN_IO_{edge}{idx}"), wire);
                        let io = self.chip.special_io[&SpecialIoKey::Clock(edge, idx as u8)];
                        let (cell_io, abcd) = self.xlat_io_loc_scm(io);
                        let wire_io = self.rc_io_wire(cell_io, &format!("JINDDCK{abcd}"));
                        self.claim_pip(wire, wire_io);
                    }
                }

                if let Dir::H(edge) = edge {
                    let mut cells_serdes = vec![];
                    match edge {
                        DirH::W => {
                            for (col, cd) in &self.chip.columns {
                                if cd.io_n == IoGroupKind::Serdes && col < self.chip.col_clk {
                                    cells_serdes
                                        .push(bcrd.with_cr(col + 6, self.chip.row_n() - 11));
                                }
                            }
                        }
                        DirH::E => {
                            for (col, cd) in self.chip.columns.iter().rev() {
                                if cd.io_n == IoGroupKind::Serdes && col >= self.chip.col_clk {
                                    cells_serdes.push(bcrd.with_cr(col, self.chip.row_n() - 11));
                                }
                            }
                        }
                    };
                    for (i, cell_serdes) in cells_serdes.into_iter().enumerate() {
                        for (j, pin) in ["FF_SYSCLK_P1", "FF_RXCLK_P1", "FF_RXCLK_P2"]
                            .into_iter()
                            .enumerate()
                        {
                            let wire = self
                                .rc_io_wire(cell, &format!("J{lrbt}SED{ii:02}01", ii = i * 3 + j));
                            self.add_bel_wire(bcrd, format!("IN_PCS_{edge}{i}_{pin}"), wire);
                            let wire_serdes = self.rc_wire(cell_serdes, &format!("J{pin}_PCS"));
                            self.claim_pip(wire, wire_serdes);
                        }
                    }

                    for v in [DirV::S, DirV::N] {
                        let cell_pll = bcrd.with_row(self.chip.row_edge(v));
                        let hv = DirHV { h: edge, v };
                        let ul = match v {
                            DirV::S => 'L',
                            DirV::N => 'U',
                        };
                        for i in 0..2 {
                            let abcd = ['A', 'B'][i];
                            for (pin, wn) in [("CLKOP", "NCLK"), ("CLKOS", "MCLK")] {
                                let wire =
                                    self.rc_io_wire(cell, &format!("J{ul}{lrbt}C{wn}{abcd}"));
                                self.add_bel_wire(bcrd, format!("IN_PLL_{hv}{i}_{pin}"), wire);
                                let wire_pll =
                                    self.rc_corner_wire(cell_pll, &format!("J{pin}{abcd}_PLL"));
                                self.claim_pip(wire, wire_pll);
                            }
                        }
                        for i in 0..4 {
                            let abcd = ['C', 'D', 'E', 'F'][i];
                            if i >= 2 && v == DirV::N {
                                continue;
                            }
                            for (pin, wn) in [("CLKOP", "MCLK"), ("CLKOS", "NCLK")] {
                                let wire =
                                    self.rc_io_wire(cell, &format!("J{ul}{lrbt}C{wn}{abcd}"));
                                self.add_bel_wire(bcrd, format!("IN_DLL_{hv}{i}_{pin}"), wire);
                                let wire_pll =
                                    self.rc_corner_wire(cell_pll, &format!("J{pin}{abcd}_DLL"));
                                self.claim_pip(wire, wire_pll);
                            }
                        }
                    }
                }

                let inps: &[&[_]] = match edge {
                    Dir::W => &[
                        &[
                            "IN_PCS_W0_FF_SYSCLK_P1",
                            "IN_CLKDIV_7_0",
                            "IN_PLL_NW0_CLKOS",
                        ],
                        &["IN_PCS_W0_FF_RXCLK_P1", "IN_CLKDIV_7_1", "IN_PLL_NW0_CLKOP"],
                        &["IN_PCS_W0_FF_RXCLK_P2", "IN_CLKDIV_7_2", "IN_PLL_NW1_CLKOS"],
                        &[
                            "IN_PCS_W1_FF_SYSCLK_P1",
                            "IN_CLKDIV_7_3",
                            "IN_PLL_NW1_CLKOP",
                        ],
                        &["IN_PCS_W1_FF_RXCLK_P1", "IN_IO_W7", "IN_DLL_NW0_CLKOP"],
                        &["IN_PCS_W1_FF_RXCLK_P2", "IN_IO_W3", "IN_DLL_NW0_CLKOS"],
                        &["IN_PCS_W2_FF_SYSCLK_P1", "IN_IO_W6", "IN_DLL_NW1_CLKOP"],
                        &["IN_PCS_W2_FF_RXCLK_P1", "IN_IO_W2", "IN_DLL_NW1_CLKOS"],
                        &["IN_PCS_W2_FF_RXCLK_P2", "IN_IO_W5", "IN_DCS1"],
                        &["IN_PCS_W3_FF_SYSCLK_P1", "IN_IO_W1", "IN_DCS0"],
                        &["IN_PCS_W3_FF_RXCLK_P1", "IN_IO_W4", "IN_DLL_SW2_CLKOP"],
                        &["IN_PCS_W3_FF_RXCLK_P2", "IN_IO_W0", "IN_DLL_SW2_CLKOS"],
                        &["IN_PCS_W3_FF_RXCLK_P2", "IN_IO_W0", "IN_DLL_SW3_CLKOP"],
                        &["IN_PCS_W3_FF_RXCLK_P1", "IN_IO_W4", "IN_DLL_SW3_CLKOS"],
                        &["IN_PCS_W3_FF_SYSCLK_P1", "IN_IO_W1", "IN_DCS0"],
                        &["IN_PCS_W2_FF_RXCLK_P2", "IN_IO_W5", "IN_DCS1"],
                        &["IN_PCS_W2_FF_RXCLK_P1", "IN_IO_W2", "IN_PLL_SW0_CLKOS"],
                        &["IN_PCS_W2_FF_SYSCLK_P1", "IN_IO_W6", "IN_PLL_SW0_CLKOP"],
                        &["IN_PCS_W1_FF_RXCLK_P2", "IN_IO_W3", "IN_PLL_SW1_CLKOS"],
                        &["IN_PCS_W1_FF_RXCLK_P1", "IN_IO_W7", "IN_PLL_SW1_CLKOP"],
                        &["IN_PCS_W1_FF_SYSCLK_P1", "IN_DLL_SW0_CLKOP"],
                        &["IN_PCS_W0_FF_RXCLK_P2", "IN_DLL_SW0_CLKOS"],
                        &["IN_PCS_W0_FF_RXCLK_P1", "IN_DLL_SW1_CLKOP"],
                        &["IN_PCS_W0_FF_SYSCLK_P1", "IN_DLL_SW1_CLKOS"],
                    ],
                    Dir::E => &[
                        &[
                            "IN_PCS_E0_FF_SYSCLK_P1",
                            "IN_CLKDIV_2_0",
                            "IN_PLL_NE0_CLKOS",
                        ],
                        &["IN_PCS_E0_FF_RXCLK_P1", "IN_CLKDIV_2_1", "IN_PLL_NE0_CLKOP"],
                        &["IN_PCS_E0_FF_RXCLK_P2", "IN_CLKDIV_2_2", "IN_PLL_NE1_CLKOS"],
                        &[
                            "IN_PCS_E1_FF_SYSCLK_P1",
                            "IN_CLKDIV_2_3",
                            "IN_PLL_NE1_CLKOP",
                        ],
                        &["IN_PCS_E1_FF_RXCLK_P1", "IN_IO_E7", "IN_DLL_NE0_CLKOP"],
                        &["IN_PCS_E1_FF_RXCLK_P2", "IN_IO_E3", "IN_DLL_NE0_CLKOS"],
                        &["IN_PCS_E2_FF_SYSCLK_P1", "IN_IO_E6", "IN_DLL_NE1_CLKOP"],
                        &["IN_PCS_E2_FF_RXCLK_P1", "IN_IO_E2", "IN_DLL_NE1_CLKOS"],
                        &["IN_PCS_E2_FF_RXCLK_P2", "IN_IO_E5", "IN_DCS0"],
                        &["IN_PCS_E3_FF_SYSCLK_P1", "IN_IO_E1", "IN_DCS1"],
                        &["IN_PCS_E3_FF_RXCLK_P1", "IN_IO_E4", "IN_DLL_SE2_CLKOP"],
                        &["IN_PCS_E3_FF_RXCLK_P2", "IN_IO_E0", "IN_DLL_SE2_CLKOS"],
                        &["IN_PCS_E3_FF_RXCLK_P2", "IN_IO_E0", "IN_DLL_SE3_CLKOP"],
                        &["IN_PCS_E3_FF_RXCLK_P1", "IN_IO_E4", "IN_DLL_SE3_CLKOS"],
                        &["IN_PCS_E3_FF_SYSCLK_P1", "IN_IO_E1", "IN_DCS1"],
                        &["IN_PCS_E2_FF_RXCLK_P2", "IN_IO_E5", "IN_DCS0"],
                        &["IN_PCS_E2_FF_RXCLK_P1", "IN_IO_E2", "IN_PLL_SE0_CLKOS"],
                        &["IN_PCS_E2_FF_SYSCLK_P1", "IN_IO_E6", "IN_PLL_SE0_CLKOP"],
                        &["IN_PCS_E1_FF_RXCLK_P2", "IN_IO_E3", "IN_PLL_SE1_CLKOS"],
                        &["IN_PCS_E1_FF_RXCLK_P1", "IN_IO_E7", "IN_PLL_SE1_CLKOP"],
                        &["IN_PCS_E1_FF_SYSCLK_P1", "IN_DLL_SE0_CLKOP"],
                        &["IN_PCS_E0_FF_RXCLK_P2", "IN_DLL_SE0_CLKOS"],
                        &["IN_PCS_E0_FF_RXCLK_P1", "IN_DLL_SE1_CLKOP"],
                        &["IN_PCS_E0_FF_SYSCLK_P1", "IN_DLL_SE1_CLKOS"],
                    ],
                    Dir::S => &[
                        &["IN_IO_S3", "IN_CLKDIV_5_0", "IN_DCS1"],
                        &["IN_IO_S2", "IN_CLKDIV_5_1", "IN_DCS0"],
                        &["IN_IO_S1", "IN_CLKDIV_5_2", "IN_DCS1"],
                        &["IN_IO_S0", "IN_CLKDIV_5_3", "IN_DCS0"],
                        &["IN_IO_S11", "IN_CLKDIV_4_0", "IN_DCS0"],
                        &["IN_IO_S10", "IN_CLKDIV_4_1", "IN_DCS1"],
                        &["IN_IO_S9", "IN_CLKDIV_4_2", "IN_DCS0"],
                        &["IN_IO_S8", "IN_CLKDIV_4_3", "IN_DCS1"],
                    ],
                    Dir::N => &[
                        &["IN_IO_N3", "IN_CLKDIV_1_0", "IN_DCS0"],
                        &["IN_IO_N2", "IN_CLKDIV_1_1", "IN_DCS1"],
                        &["IN_IO_N1", "IN_CLKDIV_1_2", "IN_DCS1"],
                        &["IN_IO_N0", "IN_CLKDIV_1_3", "IN_DCS0"],
                    ],
                };
                for (i, &inps) in inps.iter().enumerate() {
                    let prefix = match edge {
                        Dir::N => "VPFS",
                        Dir::S => "VPFN",
                        Dir::W => "HPFE",
                        Dir::E => "HPFW",
                    };
                    let wire_out = self.rc_io_wire(cell, &format!("J{prefix}{i:02}00"));
                    self.add_bel_wire(bcrd, format!("OUT{i}"), wire_out);

                    for &inp in inps {
                        if let Some(wire) = self.naming.try_bel_wire(bcrd, inp) {
                            self.claim_pip(wire_out, wire);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn process_clk_root_scm(&mut self, hpcx: BTreeMap<(DirHV, usize), WireName>) {
        let bcrd = self.chip.bel_clk_root();
        self.name_bel_null(bcrd);
        let cell = bcrd.cell.delta(-1, 0);
        let mut bel = Bel::default();
        let mut inps = vec![];
        for edge in Dir::DIRS {
            let (col, row) = match edge {
                Dir::H(edge) => (self.chip.col_edge(edge), self.chip.row_clk),
                Dir::V(edge) => (self.chip.col_clk, self.chip.row_edge(edge)),
            };
            let cell_tile = CellCoord::new(DieId::from_idx(0), col, row);
            let cell_edge = match edge {
                Dir::H(_) => cell_tile,
                Dir::V(_) => cell_tile.delta(-1, 0),
            };
            let hv = match edge {
                Dir::H(_) => 'H',
                Dir::V(_) => 'V',
            };
            let num_inps = match edge {
                Dir::H(_) => 24,
                Dir::S => 8,
                Dir::N => 4,
            };
            for i in 0..num_inps {
                let wire = self.rc_wire(cell, &format!("J{hv}PF{ndir}{i:02}01", ndir = !edge));
                self.add_bel_wire(bcrd, format!("IN_{edge}{i}"), wire);
                let wire_edge =
                    self.rc_io_wire(cell_edge, &format!("J{hv}PF{ndir}{i:02}00", ndir = !edge));
                self.claim_pip(wire, wire_edge);
                inps.push(wire);
            }
        }
        for pin in ["CIBLLQ", "CIBULQ", "CIBURQ"] {
            let wire = self.rc_wire(cell, &format!("J{pin}"));
            self.add_bel_wire(bcrd, format!("IN_{pin}"), wire);
            inps.push(wire);
            bel.pins
                .insert(format!("IN_{pin}"), self.xlat_int_wire(bcrd, wire));
        }
        for hv in DirHV::DIRS {
            let cidx = match hv {
                DirHV::SW => 0,
                DirHV::SE => 1,
                DirHV::NW => 2,
                DirHV::NE => 3,
            };
            for i in 0..12 {
                let pclk = TileWireCoord::new_idx(cidx, self.intdb.get_wire(&format!("PCLK{i}")));
                bel.pins
                    .insert(format!("PCLK{i}_{hv}"), BelPin::new_in(pclk));
                let pclk = hpcx[&(hv, i)];
                self.add_bel_wire(bcrd, format!("PCLK{i}_{hv}"), pclk);

                for &wire in &inps {
                    self.claim_pip(pclk, wire);
                }
            }
        }
        self.insert_bel(bcrd, bel);
        {
            let bcrd = bcrd.bel(bels::CLKTEST);
            self.name_bel(bcrd, ["TESTCK"]);
            self.insert_simple_bel(bcrd, cell, "TESTCK");
        }
    }
}

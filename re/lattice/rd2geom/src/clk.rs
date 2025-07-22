use std::collections::{BTreeMap, BTreeSet};

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, PllLoc, RowKind, SpecialIoKey, SpecialLocKey},
    tslots,
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, CellSlotId, PinDir, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, DieId},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_clk_ecp(&mut self) {
        let bcrd = self.chip.bel_clk_root();
        let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
        let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_clk - 1, self.chip.row_clk);
        self.name_bel_null(bcrd);

        let mut bel = Bel::default();

        let mut pclk_in = BTreeMap::new();
        let mut sclk_in = BTreeMap::new();
        for &loc in self.chip.special_loc.keys() {
            match loc {
                SpecialLocKey::PclkIn(dir, idx) => {
                    assert_eq!(idx, 0);
                    let name = match dir {
                        Dir::W => "JCIBLLQ",
                        Dir::E => "JCIBURQ",
                        Dir::S => "JCIBLRQ",
                        Dir::N => "JCIBULQ",
                    };
                    let wire = self.rc_wire(cell, name);
                    let pin = format!("PCLK_IN_{dir}");
                    self.add_bel_wire(bcrd, &pin, wire);
                    let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                    bel.pins.insert(pin, bpin);
                    pclk_in.insert(dir, wire);
                }
                SpecialLocKey::SclkIn(dir, idx) => {
                    let name = match dir {
                        Dir::W => format!("JCIBL{idx}"),
                        Dir::E => format!("JCIBR{idx}"),
                        Dir::S => format!("JCIBB{idx}"),
                        Dir::N => format!("JCIBT{idx}"),
                    };
                    let wire = self.rc_wire(cell, &name);
                    let pin = format!("SCLK_IN_{dir}{idx}");
                    self.add_bel_wire(bcrd, &pin, wire);
                    let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                    bel.pins.insert(pin, bpin);
                    sclk_in.insert(loc, wire);
                }
                _ => (),
            }
        }
        let mut io_in = BTreeMap::new();
        for dir in Dir::DIRS {
            let name = match dir {
                Dir::W => "JLPIO",
                Dir::E => "JRPIO",
                Dir::S => "JBPIO",
                Dir::N => "JTPIO",
            };
            let wire = self.rc_wire(cell, name);
            let pin = format!("IO_IN_{dir}");
            self.add_bel_wire(bcrd, &pin, wire);
            let wire_io = self.get_special_io_wire(SpecialIoKey::Clock(dir, 0));
            self.claim_pip(wire, wire_io);
            io_in.insert(dir, wire);
        }
        let mut pll_in = BTreeMap::new();
        for (&loc, &pll_cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            let wire_clkop_pll = self.rc_wire(pll_cell, "JCLKOP");
            let wire_clkos_pll = self.rc_wire(pll_cell, "JCLKOS_PLL3");
            let wire_clkok_pll = self.rc_wire(pll_cell, "JCLKOK_PLL3");
            let (wn_clkop, wn_clkos, wn_clkok) = match loc.quad {
                DirHV::SW => ("JLLMMCLKA", "JLLMNCLKA", "JLFPSC1"),
                DirHV::SE => ("JLRMMCLKA", "JLRMNCLKA", "JRFPSC1"),
                DirHV::NW => ("JULMMCLKA", "JULMNCLKA", "JLFPSC0"),
                DirHV::NE => ("JURMMCLKA", "JURMNCLKA", "JRFPSC0"),
            };
            let wire_clkop = self.rc_wire(cell, wn_clkop);
            let wire_clkos = self.rc_wire(cell, wn_clkos);
            let wire_clkok = self.rc_wire(cell, wn_clkok);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOP"), wire_clkop);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOS"), wire_clkos);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOK"), wire_clkok);
            self.claim_pip(wire_clkop, wire_clkop_pll);
            self.claim_pip(wire_clkos, wire_clkos_pll);
            self.claim_pip(wire_clkok, wire_clkok_pll);
            pll_in.insert(loc, [wire_clkop, wire_clkos, wire_clkok]);
        }

        for (cell_idx, hv) in [DirHV::SW, DirHV::SE, DirHV::NW, DirHV::NE]
            .into_iter()
            .enumerate()
        {
            for wname in ["PCLK0", "PCLK1", "SCLK0", "SCLK1", "SCLK2", "SCLK3"] {
                let wire = self.intdb.get_wire(wname);
                let cell_slot = CellSlotId::from_idx(cell_idx);
                let bpin = BelPin {
                    wires: BTreeSet::from_iter([TileWireCoord {
                        cell: cell_slot,
                        wire,
                    }]),
                    dir: PinDir::Input,
                    is_intf_in: false,
                };
                bel.pins.insert(format!("{wname}_{hv}"), bpin);
                let wire_cell = self.edev.egrid.tile_cell(tcrd, cell_slot);
                let wire = wire_cell.wire(wire);
                let idx: u8 = wname[4..].parse().unwrap();
                if wname.starts_with("PCLK") {
                    for &wf in pll_in.values().flatten() {
                        self.claim_pip_int_out(wire, wf);
                    }
                    let (pclk_in_dir, skip_io_in_dir) = match idx {
                        0 => (Dir::W, Dir::S),
                        1 => (Dir::E, Dir::N),
                        _ => unreachable!(),
                    };
                    self.claim_pip_int_out(wire, pclk_in[&pclk_in_dir]);
                    for dir in Dir::DIRS {
                        if dir == skip_io_in_dir {
                            continue;
                        }
                        self.claim_pip_int_out(wire, io_in[&dir]);
                    }
                } else {
                    let (io_in_dir, sclk_in_idx) = match idx {
                        0 => (Dir::W, [0, 2]),
                        1 => (Dir::N, [0, 2]),
                        2 => (Dir::E, [1, 3]),
                        3 => (Dir::S, [1, 3]),
                        _ => unreachable!(),
                    };
                    self.claim_pip_int_out(wire, io_in[&io_in_dir]);
                    for in_idx in sclk_in_idx {
                        for dir in Dir::DIRS {
                            if dir == io_in_dir && matches!(in_idx, 2 | 3) {
                                continue;
                            }
                            if let Some(&wf) = sclk_in.get(&SpecialLocKey::SclkIn(dir, in_idx)) {
                                self.claim_pip_int_out(wire, wf);
                            }
                        }
                    }
                }
            }
        }

        self.insert_bel(bcrd, bel);

        for (bidx, (cell_idx, ll, i)) in [
            (0, "LL", 0),
            (0, "LL", 1),
            (1, "LR", 0),
            (1, "LR", 1),
            (2, "UL", 0),
            (2, "UL", 1),
            (3, "UR", 0),
            (3, "UR", 1),
        ]
        .into_iter()
        .enumerate()
        {
            let bcrd = tcrd.bel(bels::DCS[bidx]);
            self.name_bel(bcrd, [format!("{ll}DCS{i}")]);
            let mut bel = Bel::default();

            let sel = self.rc_wire(cell, &format!("J{ll}SEL{i}_DCS"));
            self.add_bel_wire(bcrd, "SEL", sel);
            let bpin = self.xlat_int_wire(tcrd, sel).unwrap();
            bel.pins.insert("SEL".into(), bpin);

            let out = self.rc_wire(cell, &format!("{ll}DCSOUT{i}_DCS"));
            self.add_bel_wire(bcrd, "OUT", out);

            let pclk = self.intdb.get_wire(&format!("PCLK{}", i + 2));
            let cell_slot = CellSlotId::from_idx(cell_idx);
            let bpin = BelPin {
                wires: BTreeSet::from_iter([TileWireCoord {
                    cell: cell_slot,
                    wire: pclk,
                }]),
                dir: PinDir::Input,
                is_intf_in: false,
            };
            bel.pins.insert("OUT".into(), bpin);
            let pclk_cell = self.edev.egrid.tile_cell(tcrd, cell_slot);
            let pclk = pclk_cell.wire(pclk);
            self.claim_pip_int_out(pclk, out);

            let clka = self.rc_wire(cell, &format!("{ll}CLK{i}A_DCS"));
            let clkb = self.rc_wire(cell, &format!("{ll}CLK{i}B_DCS"));
            self.add_bel_wire(bcrd, "CLKA", clka);
            self.add_bel_wire(bcrd, "CLKB", clkb);

            let clka_in = self.pips_bwd[&clka].iter().copied().next().unwrap();
            let clkb_in = self.pips_bwd[&clkb].iter().copied().next().unwrap();
            self.add_bel_wire(bcrd, "CLKA_IN", clka_in);
            self.add_bel_wire(bcrd, "CLKB_IN", clkb_in);
            self.claim_pip(clka, clka_in);
            self.claim_pip(clkb, clkb_in);

            if i == 0 {
                self.claim_pip(clka_in, pclk_in[&Dir::S]);
                self.claim_pip(clkb_in, pclk_in[&Dir::E]);
            } else {
                self.claim_pip(clka_in, pclk_in[&Dir::N]);
                self.claim_pip(clkb_in, pclk_in[&Dir::W]);
            }
            self.claim_pip(clka_in, io_in[&Dir::W]);
            self.claim_pip(clka_in, io_in[&Dir::E]);
            self.claim_pip(clka_in, io_in[&Dir::N]);
            self.claim_pip(clkb_in, io_in[&Dir::W]);
            self.claim_pip(clkb_in, io_in[&Dir::E]);
            self.claim_pip(clkb_in, io_in[&Dir::S]);
            for &[_clkop, clkos, clkok] in pll_in.values() {
                self.claim_pip(clka_in, clkos);
                self.claim_pip(clkb_in, clkos);
                self.claim_pip(clka_in, clkok);
                self.claim_pip(clkb_in, clkok);
            }

            self.insert_bel(bcrd, bel);
        }
    }

    fn process_clk_machxo(&mut self) {
        let bcrd = self.chip.bel_clk_root();
        let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
        let cell = if self.chip.rows.len() == 21 {
            CellCoord::new(
                DieId::from_idx(0),
                self.chip.col_clk - 1,
                self.chip.row_clk - 1,
            )
        } else {
            CellCoord::new(DieId::from_idx(0), self.chip.col_clk - 1, self.chip.row_clk)
        };
        self.name_bel_null(bcrd);
        let mut bel = Bel::default();

        let mut inputs_pclk = vec![];
        let mut inputs_sclk = vec![];
        for i in 0..3 {
            for j in 0..4 {
                let wire = self.rc_wire(cell, &format!("JCIBCTL{i}{j}"));
                let pin = format!("CIBCTL{i}{j}");
                self.add_bel_wire(bcrd, &pin, wire);
                let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                bel.pins.insert(pin, bpin);
                inputs_sclk.push(wire);
                if j == 2
                    && self
                        .chip
                        .special_loc
                        .contains_key(&SpecialLocKey::Pll(PllLoc::new(DirHV::NW, 0)))
                {
                    continue;
                }
                if j == 3
                    && self
                        .chip
                        .special_loc
                        .contains_key(&SpecialLocKey::Pll(PllLoc::new(DirHV::SW, 0)))
                {
                    continue;
                }
                let wire = self.rc_wire(cell, &format!("JCIBCLK{i}{j}"));
                let pin = format!("CIBCLK{i}{j}");
                self.add_bel_wire(bcrd, &pin, wire);
                let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                bel.pins.insert(pin, bpin);
                inputs_pclk.push(wire);
            }
        }

        for (wire, key) in [
            ("JGCLK0", SpecialIoKey::Clock(Dir::N, 0)),
            ("JGCLK1", SpecialIoKey::Clock(Dir::N, 1)),
            ("JGCLK2", SpecialIoKey::Clock(Dir::S, 0)),
            ("JGCLK3", SpecialIoKey::Clock(Dir::S, 1)),
        ] {
            let wire = self.rc_wire(cell, wire);
            self.add_bel_wire(bcrd, key.to_string(), wire);
            let wire_io = self.get_special_io_wire(key);
            self.claim_pip(wire, wire_io);
            inputs_pclk.push(wire);
            inputs_sclk.push(wire);
        }

        for (loc, clkop, clkos, clkok) in [
            (
                PllLoc::new(DirHV::SW, 0),
                "JLLMMCLKA",
                "JLLMNCLKA",
                "JLFPSC1",
            ),
            (
                PllLoc::new(DirHV::NW, 0),
                "JULMMCLKA",
                "JULMNCLKA",
                "JLFPSC0",
            ),
        ] {
            let Some(&pll_cell) = self.chip.special_loc.get(&SpecialLocKey::Pll(loc)) else {
                continue;
            };
            let clkop = self.rc_wire(cell, clkop);
            let clkos = self.rc_wire(cell, clkos);
            let clkok = self.rc_wire(cell, clkok);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOP"), clkop);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOS"), clkos);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOK"), clkok);
            let clkop_pll = self.rc_wire(pll_cell, "JCLKOP_PLL");
            let clkos_pll = self.rc_wire(pll_cell, "JCLKOS_PLL");
            let clkok_pll = self.rc_wire(pll_cell, "JCLKOK_PLL");
            self.claim_pip(clkop, clkop_pll);
            self.claim_pip(clkos, clkos_pll);
            self.claim_pip(clkok, clkok_pll);
            inputs_pclk.push(clkop);
            inputs_pclk.push(clkos);
            inputs_pclk.push(clkok);
        }

        for pin in [
            "PCLK0", "PCLK1", "PCLK2", "PCLK3", "SCLK0", "SCLK1", "SCLK2", "SCLK3",
        ] {
            let wire = self.intdb.get_wire(pin);
            let bpin = BelPin {
                wires: BTreeSet::from_iter([TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire,
                }]),
                dir: PinDir::Input,
                is_intf_in: false,
            };
            bel.pins.insert(pin.into(), bpin);
            let wire = tcrd.cell.wire(wire);
            if pin.starts_with("PCLK") {
                for &wf in &inputs_pclk {
                    self.claim_pip_int_out(wire, wf);
                }
            } else {
                for &wf in &inputs_sclk {
                    self.claim_pip_int_out(wire, wf);
                }
            }
        }

        self.insert_bel(bcrd, bel);
    }

    fn process_hsdclk_splitter(&mut self) {
        let hsdclk = [
            self.intdb.get_wire("HSDCLK0"),
            self.intdb.get_wire("HSDCLK1"),
        ];
        for (tcrd, tile) in self.edev.egrid.tiles() {
            if tcrd.slot != tslots::HSDCLK_SPLITTER {
                continue;
            }
            let bcrd = tcrd.bel(bels::HSDCLK_SPLITTER);
            self.name_bel_null(bcrd);
            let cell = tcrd.cell.delta(-1, 0);
            for i in 0..8 {
                let wire_w = tile.cells[CellSlotId::from_idx(i % 4)].wire(hsdclk[i / 4]);
                let wire_e = tile.cells[CellSlotId::from_idx(i % 4 + 4)].wire(hsdclk[i / 4]);
                let wire_l2r = self.rc_wire(cell, &format!("HSSX0{i}00_L2R"));
                let wire_r2l = self.rc_wire(cell, &format!("HSSX0{i}00_R2L"));
                self.add_bel_wire(bcrd, format!("HSDCLK{i}_L2R"), wire_l2r);
                self.add_bel_wire(bcrd, format!("HSDCLK{i}_R2L"), wire_r2l);
                self.claim_pip_int_out(wire_w, wire_r2l);
                self.claim_pip_int_in(wire_r2l, wire_e);
                self.claim_pip_int_out(wire_e, wire_l2r);
                self.claim_pip_int_in(wire_l2r, wire_w);
            }
        }
    }

    fn process_clk_ecp2(&mut self) {
        let mut sdclk_root_out = BTreeMap::new();
        let mut prev_row = BTreeMap::new();
        let mut prev = self.chip.row_s();
        for (row, rd) in &self.chip.rows {
            if matches!(rd.kind, RowKind::Io | RowKind::Ebr | RowKind::Dsp) {
                prev_row.insert(row, prev);
                prev = row;
                let bcrd = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, row)
                    .bel(bels::HSDCLK_ROOT);
                self.name_bel_null(bcrd);
                for h in [DirH::W, DirH::E] {
                    for i in 0..8 {
                        let wire = self.edev.egrid.get_bel_pin(bcrd, &format!("OUT_{h}{i}"))[0];
                        let wire = self.naming.interconnect[&wire];
                        let wire_out = self.pips_bwd[&wire]
                            .iter()
                            .copied()
                            .find(|wn| self.naming.strings[wn.suffix].starts_with("VSC"))
                            .unwrap();
                        sdclk_root_out.insert((row, h, i), wire_out);
                        self.add_bel_wire(bcrd, format!("OUT_{h}{i}"), wire_out);
                        self.claim_pip(wire, wire_out);
                    }
                }
            }
        }

        let bcrd = self.chip.bel_clk_root();
        let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
        let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_clk - 1, self.chip.row_clk);
        self.name_bel_null(bcrd);

        let mut bel = Bel::default();

        let mut sdclk_root = BTreeMap::new();
        for h in [DirH::W, DirH::E] {
            for i in 0..8 {
                let wire = sdclk_root_out[&(self.chip.row_clk, h, i)];
                let hv = DirHV { h, v: DirV::N };
                sdclk_root.insert((hv, i), wire);
                self.add_bel_wire_no_claim(bcrd, format!("SDCLK_ROOT_{hv}{i}"), wire);

                let wire = sdclk_root_out[&(prev_row[&self.chip.row_clk], h, i)];
                let hv = DirHV { h, v: DirV::S };
                let wire = self.pips_bwd[&wire].iter().copied().next().unwrap();
                self.add_bel_wire(bcrd, format!("SDCLK_ROOT_{hv}{i}"), wire);
                sdclk_root.insert((hv, i), wire);
            }
        }

        for (row, rd) in &self.chip.rows {
            if matches!(rd.kind, RowKind::Io | RowKind::Ebr | RowKind::Dsp)
                && row != self.chip.row_s()
            {
                for h in [DirH::W, DirH::E] {
                    for i in 0..8 {
                        let wire_n = sdclk_root_out[&(row, h, i)];
                        let wire_s = sdclk_root_out[&(prev_row[&row], h, i)];
                        if row < self.chip.row_clk {
                            self.claim_pip(wire_s, wire_n);
                        } else if row == self.chip.row_clk {
                            let wire_n = sdclk_root[&(DirHV { h, v: DirV::S }, i)];
                            self.claim_pip(wire_s, wire_n);
                        } else {
                            self.claim_pip(wire_n, wire_s);
                        }
                    }
                }
            }
        }

        let mut pclk_in = BTreeMap::new();
        let mut sclk_in = BTreeMap::new();
        for &loc in self.chip.special_loc.keys() {
            match loc {
                SpecialLocKey::PclkIn(dir, idx) => {
                    let name = match (dir, idx) {
                        (Dir::W, 0) => "JCIBLLQ0",
                        (Dir::W, 1) => "JCIBULQ0",
                        (Dir::E, 0) => "JCIBLRQ2",
                        (Dir::E, 1) => "JCIBLRQ0",
                        (Dir::E, 2) => "JCIBURQ0",
                        (Dir::E, 3) => "JCIBURQ2",
                        (Dir::S, 0) => "JCIBLLQ1",
                        (Dir::S, 1) => "JCIBLRQ1",
                        (Dir::N, 0) => "JCIBULQ1",
                        (Dir::N, 1) => "JCIBURQ1",
                        _ => unreachable!(),
                    };
                    let wire = self.rc_wire(cell, name);
                    let pin = format!("PCLK_IN_{dir}{idx}");
                    self.add_bel_wire(bcrd, &pin, wire);
                    let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                    bel.pins.insert(pin, bpin);
                    pclk_in.insert(loc, wire);
                }
                SpecialLocKey::SclkIn(dir, idx) => {
                    let name = match dir {
                        Dir::W => format!("JCIBL{idx}"),
                        Dir::E => format!("JCIBR{idx}"),
                        Dir::S => format!("JCIBB{idx}"),
                        Dir::N => format!("JCIBT{idx}"),
                    };
                    let wire = self.rc_wire(cell, &name);
                    let pin = format!("SCLK_IN_{dir}{idx}");
                    self.add_bel_wire(bcrd, &pin, wire);
                    let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                    bel.pins.insert(pin, bpin);
                    sclk_in.insert(loc, wire);
                }
                _ => (),
            }
        }
        let mut io_in = BTreeMap::new();
        for dir in Dir::DIRS {
            for idx in 0..2 {
                let name = match dir {
                    Dir::W => format!("JLPIO{idx}"),
                    Dir::E => format!("JRPIO{idx}"),
                    Dir::S => format!("JBPIO{idx}"),
                    Dir::N => format!("JTPIO{idx}"),
                };
                let wire = self.rc_wire(cell, &name);
                let pin = format!("IO_IN_{dir}");
                self.add_bel_wire(bcrd, &pin, wire);
                let loc = SpecialIoKey::Clock(dir, idx);
                let wire_io = self.get_special_io_wire(loc);
                self.claim_pip(wire, wire_io);
                io_in.insert(loc, wire);
            }
        }
        let mut pll_in = BTreeMap::new();
        for (&loc, &cell_loc) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            if loc.quad.v == DirV::S && loc.idx == 0 {
                let cell_pll = match loc.quad.h {
                    DirH::W => cell_loc.delta(2, 0),
                    DirH::E => cell_loc.delta(-2, 0),
                };
                let wire_clkop_pll_in = self.rc_wire(cell_pll, "JCLKOP_PLL");
                let wire_clkos_pll_in = self.rc_wire(cell_pll, "JCLKOS_PLL");
                let wire_clkok_pll_in = self.rc_wire(cell_pll, "JCLKOK_PLL");
                let wire_clkop_dll_in = self.rc_wire(cell_loc, "JCLKOP_DLL");
                let wire_clkos_dll_in = self.rc_wire(cell_loc, "JCLKOS_DLL");
                let wire_cdiv1_in = self.rc_wire(cell_loc, "JCDIV1_CLKDIV");
                let wire_cdiv2_in = self.rc_wire(cell_loc, "JCDIV2_CLKDIV");
                let wire_cdiv4_in = self.rc_wire(cell_loc, "JCDIV4_CLKDIV");
                let wire_cdiv8_in = self.rc_wire(cell_loc, "JCDIV8_CLKDIV");
                let (
                    wn_clkop_pll,
                    wn_clkos_pll,
                    wn_clkok_pll,
                    wn_clkop_dll,
                    wn_clkos_dll,
                    wn_cdiv1,
                    wn_cdiv2,
                    wn_cdiv4,
                    wn_cdiv8,
                ) = match (self.chip.kind, loc.quad.h) {
                    (ChipKind::Ecp2, DirH::W) => (
                        "JLLMMCLKA",
                        "JLLMNCLKA",
                        "JLFPSC2",
                        "JLLMMCLKB",
                        "JLLMNCLKB",
                        "JLCDIV1",
                        "JLCDIV2",
                        "JLCDIV4",
                        "JLCDIV8",
                    ),
                    (ChipKind::Ecp2, DirH::E) => (
                        "JLRMMCLKA",
                        "JLRMNCLKA",
                        "JRFPSC2",
                        "JLRMMCLKB",
                        "JLRMNCLKB",
                        "JRCDIV1",
                        "JRCDIV2",
                        "JRCDIV4",
                        "JRCDIV8",
                    ),
                    (ChipKind::Ecp2M, DirH::W) => (
                        "JLLMMCLKC",
                        "JLLMNCLKC",
                        "JLFPSC3",
                        "JLLMMCLKB",
                        "JLLMNCLKB",
                        "JLCDIV1",
                        "JLCDIV2",
                        "JLCDIV4",
                        "JLCDIV8",
                    ),
                    (ChipKind::Ecp2M, DirH::E) => (
                        "JLRMMCLKC",
                        "JLRMNCLKC",
                        "JRFPSC3",
                        "JLRMMCLKB",
                        "JLRMNCLKB",
                        "JRCDIV1",
                        "JRCDIV2",
                        "JRCDIV4",
                        "JRCDIV8",
                    ),
                    _ => unreachable!(),
                };
                let wire_clkop_pll = self.rc_wire(cell, wn_clkop_pll);
                let wire_clkos_pll = self.rc_wire(cell, wn_clkos_pll);
                let wire_clkok_pll = self.rc_wire(cell, wn_clkok_pll);
                let wire_clkop_dll = self.rc_wire(cell, wn_clkop_dll);
                let wire_clkos_dll = self.rc_wire(cell, wn_clkos_dll);
                let wire_cdiv1 = self.rc_wire(cell, wn_cdiv1);
                let wire_cdiv2 = self.rc_wire(cell, wn_cdiv2);
                let wire_cdiv4 = self.rc_wire(cell, wn_cdiv4);
                let wire_cdiv8 = self.rc_wire(cell, wn_cdiv8);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOP"), wire_clkop_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOS"), wire_clkos_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOK"), wire_clkok_pll);
                self.add_bel_wire(bcrd, format!("DLL_{loc}_CLKOP"), wire_clkop_dll);
                self.add_bel_wire(bcrd, format!("DLL_{loc}_CLKOS"), wire_clkos_dll);
                self.add_bel_wire(bcrd, format!("CLKDIV_{loc}_CDIV1"), wire_cdiv1);
                self.add_bel_wire(bcrd, format!("CLKDIV_{loc}_CDIV2"), wire_cdiv2);
                self.add_bel_wire(bcrd, format!("CLKDIV_{loc}_CDIV4"), wire_cdiv4);
                self.add_bel_wire(bcrd, format!("CLKDIV_{loc}_CDIV8"), wire_cdiv8);
                self.claim_pip(wire_clkop_pll, wire_clkop_pll_in);
                self.claim_pip(wire_clkos_pll, wire_clkos_pll_in);
                self.claim_pip(wire_clkok_pll, wire_clkok_pll_in);
                self.claim_pip(wire_clkop_dll, wire_clkop_dll_in);
                self.claim_pip(wire_clkos_dll, wire_clkos_dll_in);
                self.claim_pip(wire_cdiv1, wire_cdiv1_in);
                self.claim_pip(wire_cdiv2, wire_cdiv2_in);
                self.claim_pip(wire_cdiv4, wire_cdiv4_in);
                self.claim_pip(wire_cdiv8, wire_cdiv8_in);
                pll_in.insert(
                    loc,
                    vec![
                        wire_clkop_pll,
                        wire_clkos_pll,
                        wire_clkok_pll,
                        wire_clkop_dll,
                        wire_clkos_dll,
                        wire_cdiv1,
                        wire_cdiv2,
                        wire_cdiv4,
                        wire_cdiv8,
                    ],
                );
            } else {
                let wire_clkop_pll_in = self.rc_wire(cell_loc, "JCLKOP_SPLL");
                let wire_clkos_pll_in = self.rc_wire(cell_loc, "JCLKOS_SPLL");
                let wire_clkok_pll_in = self.rc_wire(cell_loc, "JCLKOK_SPLL");
                let (wn_clkop_pll, wn_clkos_pll, wn_clkok_pll) = match (loc.quad, loc.idx) {
                    (DirHV::SW, 1) => ("JLLMMCLKA", "JLLMNCLKA", "JLFPSC2"),
                    (DirHV::SE, 1) => ("JLRMMCLKA", "JLRMNCLKA", "JRFPSC2"),
                    (DirHV::NW, 0) => ("JULMMCLKA", "JULMNCLKA", "JLFPSC0"),
                    (DirHV::NE, 0) => ("JURMMCLKA", "JURMNCLKA", "JRFPSC0"),
                    (DirHV::NW, 1) => ("JULMMCLKB", "JULMNCLKB", "JLFPSC1"),
                    (DirHV::NE, 1) => ("JURMMCLKB", "JURMNCLKB", "JRFPSC1"),
                    _ => unreachable!(),
                };
                let wire_clkop_pll = self.rc_wire(cell, wn_clkop_pll);
                let wire_clkos_pll = self.rc_wire(cell, wn_clkos_pll);
                let wire_clkok_pll = self.rc_wire(cell, wn_clkok_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOP"), wire_clkop_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOS"), wire_clkos_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOK"), wire_clkok_pll);
                self.claim_pip(wire_clkop_pll, wire_clkop_pll_in);
                self.claim_pip(wire_clkos_pll, wire_clkos_pll_in);
                self.claim_pip(wire_clkok_pll, wire_clkok_pll_in);
                pll_in.insert(loc, vec![wire_clkop_pll, wire_clkos_pll, wire_clkok_pll]);
            }
        }
        let mut serdes_in = BTreeMap::new();
        if self.chip.kind == ChipKind::Ecp2M {
            for tcname in ["SERDES_S", "SERDES_N"] {
                let tcid = self.intdb.get_tile_class(tcname);
                for &tcrd in &self.edev.egrid.tile_index[tcid] {
                    let bcrd_pcs = tcrd.bel(bels::SERDES);
                    let hv = DirHV {
                        h: if bcrd_pcs.col < self.chip.col_clk {
                            DirH::W
                        } else {
                            DirH::E
                        },
                        v: if bcrd_pcs.row < self.chip.row_clk {
                            DirV::S
                        } else {
                            DirV::N
                        },
                    };
                    let cell_pcs = bcrd_pcs.cell.delta(
                        match hv.h {
                            DirH::W => 12,
                            DirH::E => 13,
                        },
                        match hv.v {
                            DirV::S => -1,
                            DirV::N => 1,
                        },
                    );
                    let corner = match hv {
                        DirHV::SW => "LL",
                        DirHV::SE => "LR",
                        DirHV::NW => "UL",
                        DirHV::NE => "UR",
                    };
                    for fh in ['F', 'H'] {
                        let wire_pcs = self.rc_wire(cell_pcs, &format!("JFF_TX_{fh}_CLK_PCS"));
                        let wire = self.rc_wire(cell, &format!("J{corner}SQ_TX_{fh}_CLK"));
                        self.add_bel_wire(bcrd, format!("SERDES_{hv}_TX_{fh}_CLK"), wire);
                        self.claim_pip(wire, wire_pcs);
                        serdes_in.insert((hv, fh), wire);
                    }
                }
            }
        }

        let mut pclk_outs = BTreeMap::new();

        for (cell_idx, hv) in [DirHV::SW, DirHV::SE, DirHV::NW, DirHV::NE]
            .into_iter()
            .enumerate()
        {
            let cell_slot = CellSlotId::from_idx(cell_idx);
            for i in 0..8 {
                let wire = self.intdb.get_wire(&format!("PCLK{i}"));
                let wire = TileWireCoord {
                    cell: cell_slot,
                    wire,
                };
                let bpin = BelPin {
                    wires: BTreeSet::from_iter([wire]),
                    dir: PinDir::Input,
                    is_intf_in: false,
                };
                bel.pins.insert(format!("PCLK{i}_{hv}"), bpin);
                let wire = self.edev.egrid.tile_wire(tcrd, wire);
                let wire = self.naming.interconnect[&wire];
                let wire_out = self.pips_bwd[&wire].iter().copied().next().unwrap();
                self.add_bel_wire(bcrd, format!("PCLK{i}_{hv}"), wire_out);
                pclk_outs.insert((hv, i), wire_out);
            }
        }

        for hv in DirHV::DIRS {
            for i in 0..6 {
                let pclk_out = pclk_outs[&(hv, i)];
                for &wire in pll_in.values().flatten() {
                    self.claim_pip(pclk_out, wire);
                }
                for (&(_, fh), &wire) in &serdes_in {
                    if fh == ['H', 'F'][i % 2] {
                        self.claim_pip(pclk_out, wire);
                    }
                }

                let loc = [
                    SpecialLocKey::PclkIn(Dir::W, 1),
                    SpecialLocKey::PclkIn(Dir::W, 0),
                    SpecialLocKey::PclkIn(Dir::E, 2),
                    SpecialLocKey::PclkIn(Dir::E, 1),
                    SpecialLocKey::PclkIn(Dir::N, 0),
                    SpecialLocKey::PclkIn(Dir::S, 0),
                ][i];
                self.claim_pip(pclk_out, pclk_in[&loc]);
                let (we, sn) = [(0, 0), (1, 0), (0, 1), (1, 1), (0, 1), (1, 0)][i];
                self.claim_pip(pclk_out, io_in[&SpecialIoKey::Clock(Dir::W, we)]);
                self.claim_pip(pclk_out, io_in[&SpecialIoKey::Clock(Dir::E, we)]);
                self.claim_pip(pclk_out, io_in[&SpecialIoKey::Clock(Dir::S, sn)]);
                self.claim_pip(pclk_out, io_in[&SpecialIoKey::Clock(Dir::N, sn)]);
            }
        }

        for (bidx, (hv, ll, i)) in [
            (DirHV::SW, "LL", 0),
            (DirHV::SW, "LL", 1),
            (DirHV::SE, "LR", 0),
            (DirHV::SE, "LR", 1),
            (DirHV::NW, "UL", 0),
            (DirHV::NW, "UL", 1),
            (DirHV::NE, "UR", 0),
            (DirHV::NE, "UR", 1),
        ]
        .into_iter()
        .enumerate()
        {
            let pclki = i + 6;

            let bcrd = tcrd.bel(bels::DCS[bidx]);
            self.name_bel(bcrd, [format!("{ll}DCS{i}")]);
            let mut bel_dcs = Bel::default();

            let sel = self.rc_wire(cell, &format!("{ll}SEL{i}_DCS"));
            self.add_bel_wire(bcrd, "SEL", sel);
            self.claim_pip(sel, sdclk_root[&(hv, [3, 7][i])]);

            let out = self.rc_wire(cell, &format!("{ll}DCSOUT{i}_DCS"));
            self.add_bel_wire(bcrd, "OUT", out);
            let bpin = bel.pins.remove(&format!("PCLK{pclki}_{hv}")).unwrap();
            bel_dcs.pins.insert("OUT".into(), bpin);
            self.claim_pip(pclk_outs[&(hv, pclki)], out);

            let clka = self.rc_wire(cell, &format!("{ll}CLK{i}A_DCS"));
            let clkb = self.rc_wire(cell, &format!("{ll}CLK{i}B_DCS"));
            self.add_bel_wire(bcrd, "CLKA", clka);
            self.add_bel_wire(bcrd, "CLKB", clkb);
            self.claim_pip(out, clka);
            self.claim_pip(out, clkb);

            let clka_in = self.pips_bwd[&clka].iter().copied().next().unwrap();
            let clkb_in = self.pips_bwd[&clkb].iter().copied().next().unwrap();
            self.add_bel_wire(bcrd, "CLKA_IN", clka_in);
            self.add_bel_wire(bcrd, "CLKB_IN", clkb_in);
            self.claim_pip(clka, clka_in);
            self.claim_pip(clkb, clkb_in);

            if self.chip.kind == ChipKind::Ecp2M {
                for (&loc, ins) in &pll_in {
                    for (i, &wire) in ins.iter().enumerate() {
                        match (i, loc.quad.h) {
                            (0, DirH::W) | (3, DirH::E) => {
                                self.claim_pip(clka_in, wire);
                            }
                            (0, DirH::E) | (3, DirH::W) => {
                                self.claim_pip(clkb_in, wire);
                            }
                            _ => {
                                self.claim_pip(clka_in, wire);
                                self.claim_pip(clkb_in, wire);
                            }
                        }
                    }
                }
                for (&(_, fh), &wire) in &serdes_in {
                    if fh == 'H' {
                        self.claim_pip(clka_in, wire);
                    } else {
                        self.claim_pip(clkb_in, wire);
                    }
                }
            } else {
                for (&loc, ins) in &pll_in {
                    for (i, &wire) in ins.iter().enumerate() {
                        match (i, loc.quad.h) {
                            (2, DirH::W) | (3, DirH::E) => {
                                self.claim_pip(clka_in, wire);
                            }
                            (2, DirH::E) | (3, DirH::W) => {
                                self.claim_pip(clkb_in, wire);
                            }
                            _ => {
                                self.claim_pip(clka_in, wire);
                                self.claim_pip(clkb_in, wire);
                            }
                        }
                    }
                }
            }

            self.claim_pip(clka_in, io_in[&SpecialIoKey::Clock(Dir::W, 0)]);
            self.claim_pip(clka_in, io_in[&SpecialIoKey::Clock(Dir::E, 0)]);
            self.claim_pip(clka_in, io_in[&SpecialIoKey::Clock(Dir::S, 1)]);
            self.claim_pip(clka_in, io_in[&SpecialIoKey::Clock(Dir::N, 1)]);
            self.claim_pip(clkb_in, io_in[&SpecialIoKey::Clock(Dir::W, 1)]);
            self.claim_pip(clkb_in, io_in[&SpecialIoKey::Clock(Dir::E, 1)]);
            self.claim_pip(clkb_in, io_in[&SpecialIoKey::Clock(Dir::S, 0)]);
            self.claim_pip(clkb_in, io_in[&SpecialIoKey::Clock(Dir::N, 0)]);

            if i == 0 {
                self.claim_pip(clka_in, pclk_in[&SpecialLocKey::PclkIn(Dir::W, 1)]);
                self.claim_pip(clka_in, pclk_in[&SpecialLocKey::PclkIn(Dir::N, 1)]);
                self.claim_pip(clkb_in, pclk_in[&SpecialLocKey::PclkIn(Dir::W, 0)]);
                self.claim_pip(clkb_in, pclk_in[&SpecialLocKey::PclkIn(Dir::S, 1)]);
            } else {
                self.claim_pip(clka_in, pclk_in[&SpecialLocKey::PclkIn(Dir::E, 0)]);
                self.claim_pip(clka_in, pclk_in[&SpecialLocKey::PclkIn(Dir::E, 2)]);
                self.claim_pip(clkb_in, pclk_in[&SpecialLocKey::PclkIn(Dir::E, 1)]);
                self.claim_pip(clkb_in, pclk_in[&SpecialLocKey::PclkIn(Dir::E, 3)]);
            }

            self.insert_bel(bcrd, bel_dcs);
        }

        for hv in DirHV::DIRS {
            for i in 0..8 {
                let sdclk_out = sdclk_root[&(hv, i)];
                let (edge, idx) = [
                    (Dir::W, 0),
                    (Dir::W, 1),
                    (Dir::E, 0),
                    (Dir::E, 1),
                    (Dir::N, 0),
                    (Dir::N, 1),
                    (Dir::S, 0),
                    (Dir::S, 1),
                ][i];
                self.claim_pip(sdclk_out, io_in[&SpecialIoKey::Clock(edge, idx)]);
                let (e0, i0, e1, i1, e2, i2) = [
                    (Dir::E, 0, Dir::N, 0, Dir::S, 2),
                    (Dir::E, 1, Dir::N, 1, Dir::S, 3),
                    (Dir::W, 0, Dir::S, 0, Dir::N, 2),
                    (Dir::W, 1, Dir::S, 1, Dir::N, 3),
                    (Dir::S, 0, Dir::E, 0, Dir::W, 2),
                    (Dir::S, 1, Dir::E, 1, Dir::W, 3),
                    (Dir::N, 0, Dir::W, 0, Dir::E, 2),
                    (Dir::N, 1, Dir::W, 1, Dir::E, 3),
                ][i];
                self.claim_pip(sdclk_out, sclk_in[&SpecialLocKey::SclkIn(e0, i0)]);
                self.claim_pip(sdclk_out, sclk_in[&SpecialLocKey::SclkIn(e1, i1)]);
                self.claim_pip(sdclk_out, sclk_in[&SpecialLocKey::SclkIn(e2, i2)]);
            }
        }

        for (col, cd) in &self.chip.columns {
            if !cd.pclk_leaf_break && col != self.chip.col_w() {
                continue;
            }
            let h = if col < self.chip.col_clk {
                DirH::W
            } else {
                DirH::E
            };
            'cells: for mut cell in self.edev.egrid.column(cell.die, col) {
                let v = if cell.row < self.chip.row_clk {
                    DirV::S
                } else {
                    DirV::N
                };
                let hv = DirHV { h, v };
                while !self.edev.egrid.has_bel(cell.bel(bels::INT)) {
                    if cell.col == self.chip.col_e() {
                        continue 'cells;
                    }
                    cell.col += 1;
                    if self.chip.columns[cell.col].pclk_leaf_break {
                        continue 'cells;
                    }
                }
                for i in 0..8 {
                    let pclk = cell.wire(self.intdb.get_wire(&format!("PCLK{i}")));
                    self.claim_pip_int_out(pclk, pclk_outs[&(hv, i)]);
                }
            }
        }

        self.insert_bel(bcrd, bel);
    }

    pub fn process_clk(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => self.process_clk_ecp(),
            ChipKind::MachXo => self.process_clk_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => {
                self.process_hsdclk_splitter();
                self.process_clk_ecp2();
            }
        }
    }

    fn process_clk_zones_ecp2(&mut self) {
        let mut ranges = vec![];
        let mut prev = self.chip.col_w();
        for (col, cd) in &self.chip.columns {
            if cd.pclk_leaf_break {
                ranges.push((prev, col));
                prev = col;
            }
        }
        ranges.push((prev, self.chip.col_e() + 1));
        for (col_w, col_e) in ranges {
            for _ in col_w.range(col_e) {
                self.pclk_cols.push((col_w, col_e));
            }
        }
    }

    pub fn process_clk_zones(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp | ChipKind::MachXo => (),
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.process_clk_zones_ecp2(),
        }
    }
}

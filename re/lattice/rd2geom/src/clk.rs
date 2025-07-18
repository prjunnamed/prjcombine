use std::collections::{BTreeMap, BTreeSet};

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, CellSlotId, PinDir, TileWireCoord},
    dir::{Dir, DirHV},
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
        for (&hv, &pll_cell) in &self.edev.plls {
            let wire_clkop_pll = self.rc_wire(pll_cell, "JCLKOP");
            let wire_clkos_pll = self.rc_wire(pll_cell, "JCLKOS_PLL3");
            let wire_clkok_pll = self.rc_wire(pll_cell, "JCLKOK_PLL3");
            let (wn_clkop, wn_clkos, wn_clkok) = match hv {
                DirHV::SW => ("JLLMMCLKA", "JLLMNCLKA", "JLFPSC1"),
                DirHV::SE => ("JLRMMCLKA", "JLRMNCLKA", "JRFPSC1"),
                DirHV::NW => ("JULMMCLKA", "JULMNCLKA", "JLFPSC0"),
                DirHV::NE => ("JURMMCLKA", "JURMNCLKA", "JRFPSC0"),
            };
            let wire_clkop = self.rc_wire(cell, wn_clkop);
            let wire_clkos = self.rc_wire(cell, wn_clkos);
            let wire_clkok = self.rc_wire(cell, wn_clkok);
            self.add_bel_wire(bcrd, format!("PLL_{hv}_CLKOP"), wire_clkop);
            self.add_bel_wire(bcrd, format!("PLL_{hv}_CLKOS"), wire_clkos);
            self.add_bel_wire(bcrd, format!("PLL_{hv}_CLKOK"), wire_clkok);
            self.claim_pip(wire_clkop, wire_clkop_pll);
            self.claim_pip(wire_clkos, wire_clkos_pll);
            self.claim_pip(wire_clkok, wire_clkok_pll);
            pll_in.insert(hv, [wire_clkop, wire_clkos, wire_clkok]);
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
                let wire_cell = self.edev.egrid.tile(tcrd).cells[cell_slot];
                let wire_cell = CellCoord::new(DieId::from_idx(0), wire_cell.0, wire_cell.1);
                let wire = wire_cell.wire(wire);
                let wire = self.naming.interconnect[&wire];
                let idx: u8 = wname[4..].parse().unwrap();
                if wname.starts_with("PCLK") {
                    for &wf in pll_in.values().flatten() {
                        self.claim_pip(wire, wf);
                    }
                    let (pclk_in_dir, skip_io_in_dir) = match idx {
                        0 => (Dir::W, Dir::S),
                        1 => (Dir::E, Dir::N),
                        _ => unreachable!(),
                    };
                    self.claim_pip(wire, pclk_in[&pclk_in_dir]);
                    for dir in Dir::DIRS {
                        if dir == skip_io_in_dir {
                            continue;
                        }
                        self.claim_pip(wire, io_in[&dir]);
                    }
                } else {
                    let (io_in_dir, sclk_in_idx) = match idx {
                        0 => (Dir::W, [0, 2]),
                        1 => (Dir::N, [0, 2]),
                        2 => (Dir::E, [1, 3]),
                        3 => (Dir::S, [1, 3]),
                        _ => unreachable!(),
                    };
                    self.claim_pip(wire, io_in[&io_in_dir]);
                    for in_idx in sclk_in_idx {
                        for dir in Dir::DIRS {
                            if dir == io_in_dir && matches!(in_idx, 2 | 3) {
                                continue;
                            }
                            if let Some(&wf) = sclk_in.get(&SpecialLocKey::SclkIn(dir, in_idx)) {
                                self.claim_pip(wire, wf);
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
            let pclk_cell = self.edev.egrid.tile(tcrd).cells[cell_slot];
            let pclk_cell = CellCoord::new(DieId::from_idx(0), pclk_cell.0, pclk_cell.1);
            let pclk = pclk_cell.wire(pclk);
            let pclk = self.naming.interconnect[&pclk];
            self.claim_pip(pclk, out);

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
                        .contains_key(&SpecialLocKey::Pll(DirHV::NW))
                {
                    continue;
                }
                if j == 3
                    && self
                        .chip
                        .special_loc
                        .contains_key(&SpecialLocKey::Pll(DirHV::SW))
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
            (DirHV::SW, "JLLMMCLKA", "JLLMNCLKA", "JLFPSC1"),
            (DirHV::NW, "JULMMCLKA", "JULMNCLKA", "JLFPSC0"),
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
            let wire = self.naming.interconnect[&wire];
            if pin.starts_with("PCLK") {
                for &wf in &inputs_pclk {
                    self.claim_pip(wire, wf);
                }
            } else {
                for &wf in &inputs_sclk {
                    self.claim_pip(wire, wf);
                }
            }
        }

        self.insert_bel(bcrd, bel);
    }

    pub fn process_clk(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => {
                self.process_clk_ecp();
            }
            ChipKind::MachXo => {
                self.process_clk_machxo();
            }
        }
    }
}

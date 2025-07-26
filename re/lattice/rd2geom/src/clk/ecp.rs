use std::collections::BTreeMap;

use prjcombine_ecp::{
    bels,
    chip::{SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, CellSlotId, TileWireCoord},
    dir::{Dir, DirHV},
    grid::{CellCoord, DieId},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_clk_ecp(&mut self) {
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
                    let bpin = self.xlat_int_wire(bcrd, wire);
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
                    let bpin = self.xlat_int_wire(bcrd, wire);
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
            let wire_io = self.get_special_io_wire_in(SpecialIoKey::Clock(dir, 0));
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
                let bpin = BelPin::new_in(TileWireCoord {
                    cell: cell_slot,
                    wire,
                });
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

        for (bidx, (ll, slot)) in [
            ("LL", bels::DCS_SW[0]),
            ("LL", bels::DCS_SW[1]),
            ("LR", bels::DCS_SE[0]),
            ("LR", bels::DCS_SE[1]),
            ("UL", bels::DCS_NW[0]),
            ("UL", bels::DCS_NW[1]),
            ("UR", bels::DCS_NE[0]),
            ("UR", bels::DCS_NE[1]),
        ]
        .into_iter()
        .enumerate()
        {
            let i = bidx % 2;
            let bcrd = tcrd.bel(slot);
            self.name_bel(bcrd, [format!("{ll}DCS{i}")]);
            let mut bel = Bel::default();

            let sel = self.rc_wire(cell, &format!("J{ll}SEL{i}_DCS"));
            self.add_bel_wire(bcrd, "SEL", sel);
            let bpin = self.xlat_int_wire(bcrd, sel);
            bel.pins.insert("SEL".into(), bpin);

            let out = self.rc_wire(cell, &format!("{ll}DCSOUT{i}_DCS"));
            self.add_bel_wire(bcrd, "OUT", out);

            let pclk = self.intdb.get_wire(&format!("PCLK{}", i + 2));
            let cell_slot = CellSlotId::from_idx(bidx / 2);
            let bpin = BelPin::new_in(TileWireCoord {
                cell: cell_slot,
                wire: pclk,
            });
            bel.pins.insert("OUT".into(), bpin);
            let pclk_cell = self.edev.egrid.tile_cell(tcrd, cell_slot);
            let pclk = pclk_cell.wire(pclk);
            self.claim_pip_int_out(pclk, out);

            let clka = self.rc_wire(cell, &format!("{ll}CLK{i}A_DCS"));
            let clkb = self.rc_wire(cell, &format!("{ll}CLK{i}B_DCS"));
            self.add_bel_wire(bcrd, "CLKA", clka);
            self.add_bel_wire(bcrd, "CLKB", clkb);

            let clka_in = self.claim_single_in(clka);
            let clkb_in = self.claim_single_in(clkb);
            self.add_bel_wire(bcrd, "CLKA_IN", clka_in);
            self.add_bel_wire(bcrd, "CLKB_IN", clkb_in);

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
}

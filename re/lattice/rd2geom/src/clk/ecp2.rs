use std::collections::BTreeMap;

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, RowKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, DieId},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_clk_ecp2(&mut self) {
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
                let wire = self.find_single_in(wire);
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
                    let bpin = self.xlat_int_wire(bcrd, wire);
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
                    let bpin = self.xlat_int_wire(bcrd, wire);
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
                let wire_io = self.get_special_io_wire_in(loc);
                self.claim_pip(wire, wire_io);
                io_in.insert(loc, wire);
            }
        }
        let mut pll_in = BTreeMap::new();
        for (&loc, &cell_loc) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            if self.chip.kind == ChipKind::Xp2 {
                let wire_clkop_pll_in = self.rc_wire(cell_loc, "JCLKOP_PLL");
                let wire_clkos_pll_in = self.rc_wire(cell_loc, "JCLKOS_PLL");
                let wire_clkok_pll_in = self.rc_wire(cell_loc, "JCLKOK_PLL");
                let wire_clkok2_pll_in = self.rc_wire(cell_loc, "JCLKOK2_PLL");
                let (wn_clkop_pll, wn_clkos_pll, wn_clkok_pll, wn_clkok2_pll) = match loc.quad {
                    DirHV::SW => ("JLLCMCLKA", "JLLCNCLKA", "JLFPSC3", "JLFPSC4"),
                    DirHV::SE => ("JLRCMCLKA", "JLRCNCLKA", "JRFPSC3", "JRFPSC4"),
                    DirHV::NW => ("JULCMCLKA", "JULCNCLKA", "JLFPSC1", "JLFPSC2"),
                    DirHV::NE => ("JURCMCLKA", "JURCNCLKA", "JRFPSC1", "JRFPSC2"),
                };
                let wire_clkop_pll = self.rc_wire(cell, wn_clkop_pll);
                let wire_clkos_pll = self.rc_wire(cell, wn_clkos_pll);
                let wire_clkok_pll = self.rc_wire(cell, wn_clkok_pll);
                let wire_clkok2_pll = self.rc_wire(cell, wn_clkok2_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOP"), wire_clkop_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOS"), wire_clkos_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOK"), wire_clkok_pll);
                self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOK2"), wire_clkok2_pll);
                self.claim_pip(wire_clkop_pll, wire_clkop_pll_in);
                self.claim_pip(wire_clkos_pll, wire_clkos_pll_in);
                self.claim_pip(wire_clkok_pll, wire_clkok_pll_in);
                self.claim_pip(wire_clkok2_pll, wire_clkok2_pll_in);
                pll_in.insert(
                    loc,
                    vec![
                        wire_clkop_pll,
                        wire_clkos_pll,
                        wire_clkok_pll,
                        wire_clkok2_pll,
                    ],
                );
            } else if loc.quad.v == DirV::S && loc.idx == 0 {
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
        let mut clkdiv_in = BTreeMap::new();
        if self.chip.kind == ChipKind::Xp2 {
            for edge in [DirH::W, DirH::E] {
                let cell_loc = self.chip.bel_dqsdll_ecp2(edge).cell;
                let wire_cdiv1_in = self.rc_wire(cell_loc, "JCDIV1_CLKDIV");
                let wire_cdiv2_in = self.rc_wire(cell_loc, "JCDIV2_CLKDIV");
                let wire_cdiv4_in = self.rc_wire(cell_loc, "JCDIV4_CLKDIV");
                let wire_cdiv8_in = self.rc_wire(cell_loc, "JCDIV8_CLKDIV");
                let (wn_cdiv1, wn_cdiv2, wn_cdiv4, wn_cdiv8) = match edge {
                    DirH::W => ("JLCDIV1", "JLCDIV2", "JLCDIV4", "JLCDIV8"),
                    DirH::E => ("JRCDIV1", "JRCDIV2", "JRCDIV4", "JRCDIV8"),
                };
                let wire_cdiv1 = self.rc_wire(cell, wn_cdiv1);
                let wire_cdiv2 = self.rc_wire(cell, wn_cdiv2);
                let wire_cdiv4 = self.rc_wire(cell, wn_cdiv4);
                let wire_cdiv8 = self.rc_wire(cell, wn_cdiv8);
                self.add_bel_wire(bcrd, format!("CLKDIV_{edge}_CDIV1"), wire_cdiv1);
                self.add_bel_wire(bcrd, format!("CLKDIV_{edge}_CDIV2"), wire_cdiv2);
                self.add_bel_wire(bcrd, format!("CLKDIV_{edge}_CDIV4"), wire_cdiv4);
                self.add_bel_wire(bcrd, format!("CLKDIV_{edge}_CDIV8"), wire_cdiv8);
                self.claim_pip(wire_cdiv1, wire_cdiv1_in);
                self.claim_pip(wire_cdiv2, wire_cdiv2_in);
                self.claim_pip(wire_cdiv4, wire_cdiv4_in);
                self.claim_pip(wire_cdiv8, wire_cdiv8_in);
                clkdiv_in.insert(edge, vec![wire_cdiv1, wire_cdiv2, wire_cdiv4, wire_cdiv8]);
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
            for i in 0..8 {
                let wire =
                    TileWireCoord::new_idx(cell_idx, self.intdb.get_wire(&format!("PCLK{i}")));
                bel.pins
                    .insert(format!("PCLK{i}_{hv}"), BelPin::new_in(wire));
                let wire = self.edev.egrid.tile_wire(tcrd, wire);
                let wire = self.naming.interconnect[&wire];
                let wire_out = self.find_single_in(wire);
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
                for &wire in clkdiv_in.values().flatten() {
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

        for (bidx, (hv, ll, slot)) in [
            (DirHV::SW, "LL", bels::DCS_SW[0]),
            (DirHV::SW, "LL", bels::DCS_SW[1]),
            (DirHV::SE, "LR", bels::DCS_SE[0]),
            (DirHV::SE, "LR", bels::DCS_SE[1]),
            (DirHV::NW, "UL", bels::DCS_NW[0]),
            (DirHV::NW, "UL", bels::DCS_NW[1]),
            (DirHV::NE, "UR", bels::DCS_NE[0]),
            (DirHV::NE, "UR", bels::DCS_NE[1]),
        ]
        .into_iter()
        .enumerate()
        {
            let i = bidx % 2;
            let bcrd = tcrd.bel(slot);
            let pclki = i + 6;

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

            let clka_in = self.claim_single_in(clka);
            let clkb_in = self.claim_single_in(clkb);
            self.add_bel_wire(bcrd, "CLKA_IN", clka_in);
            self.add_bel_wire(bcrd, "CLKB_IN", clkb_in);

            for (&loc, ins) in &pll_in {
                for (i, &wire) in ins.iter().enumerate() {
                    match self.chip.kind {
                        ChipKind::Ecp2 => match (i, loc.quad.h) {
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
                        },
                        ChipKind::Ecp2M => match (i, loc.quad.h) {
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
                        },
                        ChipKind::Xp2 => match (i, loc.quad.h) {
                            (0, DirH::W) => {
                                self.claim_pip(clkb_in, wire);
                            }
                            (0, DirH::E) => {
                                self.claim_pip(clka_in, wire);
                            }
                            _ => {
                                self.claim_pip(clka_in, wire);
                                self.claim_pip(clkb_in, wire);
                            }
                        },
                        _ => unreachable!(),
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
            for &wire in clkdiv_in.values().flatten() {
                self.claim_pip(clka_in, wire);
                self.claim_pip(clkb_in, wire);
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
            if !cd.pclk_break && col != self.chip.col_w() {
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
                    if self.chip.columns[cell.col].pclk_break {
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
}

use std::collections::{BTreeMap, btree_map};

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, IoGroupKind, RowKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId},
};
use prjcombine_re_lattice_naming::WireName;
use unnamed_entity::EntityId;

use crate::{ChipContext, chip::ChipExt};

impl ChipContext<'_> {
    pub(super) fn process_hsdclk_root_ecp3(&mut self) -> BTreeMap<(DirHV, usize), WireName> {
        let mut roots = BTreeMap::new();
        let mut vsrx = BTreeMap::new();
        let root_cols = Vec::from_iter(
            self.chip
                .columns
                .iter()
                .filter(|&(col, cd)| cd.sdclk_break && col != self.chip.col_clk)
                .map(|(col, _)| col),
        );
        let root_rows = Vec::from_iter(
            self.chip
                .rows
                .iter()
                .filter(|&(_, rd)| rd.sclk_break || rd.kind == RowKind::Io)
                .map(|(row, _)| row),
        );
        for &row in &root_rows {
            for &col in &root_cols {
                let mut cell = CellCoord::new(DieId::from_idx(0), col, row);
                if !self.edev.has_bel(cell.bel(bels::INT)) {
                    cell.row += 9;
                }
                let bcrd = cell.bel(bels::HSDCLK_ROOT);
                self.name_bel_null(bcrd);
                for i in 0..8 {
                    for h in [DirH::W, DirH::E] {
                        let wire = self.edev.get_bel_pin(bcrd, &format!("OUT_{h}{i}"))[0];
                        let wire = self.naming.interconnect[&wire];
                        let wire_out = self.pips_bwd[&wire]
                            .iter()
                            .copied()
                            .find(|&wn| {
                                self.naming.strings[wn.suffix].starts_with("VSRX")
                                    && self.chip.xlat_rc_wire(wn).col == col - 1
                            })
                            .unwrap();
                        match vsrx.entry((col, row, i)) {
                            btree_map::Entry::Vacant(e) => {
                                e.insert(wire_out);
                                if row == self.chip.row_clk {
                                    self.add_bel_wire_no_claim(bcrd, format!("OUT_{i}"), wire_out);
                                } else {
                                    self.add_bel_wire(bcrd, format!("OUT_{i}"), wire_out);
                                }
                            }
                            btree_map::Entry::Occupied(e) => {
                                assert_eq!(*e.get(), wire_out);
                            }
                        }
                        self.claim_pip(wire, wire_out);
                    }
                }
            }
        }
        for (idx, &row) in root_rows.iter().enumerate() {
            for &col in &root_cols {
                let h = if col < self.chip.col_clk {
                    DirH::W
                } else {
                    DirH::E
                };
                for i in 0..8 {
                    let wire = vsrx[&(col, row, i)];
                    if row < self.chip.row_clk {
                        let row_n = root_rows[idx + 1];
                        if row_n == self.chip.row_clk {
                            let root = self.claim_single_in(wire);
                            let hv = DirHV { h, v: DirV::S };
                            match roots.entry((hv, i)) {
                                btree_map::Entry::Vacant(e) => {
                                    e.insert(root);
                                }
                                btree_map::Entry::Occupied(e) => {
                                    assert_eq!(*e.get(), root);
                                }
                            }
                        } else {
                            let wire_n = vsrx[&(col, row_n, i)];
                            self.claim_pip(wire, wire_n);
                        }
                    } else if row == self.chip.row_clk {
                        let root = self.claim_single_in(wire);
                        let hv = DirHV { h, v: DirV::N };
                        match roots.entry((hv, i)) {
                            btree_map::Entry::Vacant(e) => {
                                e.insert(root);
                            }
                            btree_map::Entry::Occupied(e) => {
                                assert_eq!(*e.get(), root);
                            }
                        }
                    } else {
                        let row_s = root_rows[idx - 1];
                        let wire_s = vsrx[&(col, row_s, i)];
                        if row_s == self.chip.row_clk {
                            assert_eq!(wire, wire_s);
                        } else {
                            self.claim_pip(wire, wire_s);
                        }
                    }
                }
            }
        }
        roots
    }

    pub(super) fn process_clk_ecp3(&mut self, sclk_roots: BTreeMap<(DirHV, usize), WireName>) {
        let bcrd = self.chip.bel_clk_root();
        let cell = bcrd.cell.delta(-1, 0);
        self.name_bel(
            bcrd,
            [
                "LLGND", "LRGND", "ULGND", "URGND", "LLVCC", "LRVCC", "ULVCC", "URVCC",
            ],
        );
        let mut bel = Bel::default();

        let mut pclk_in = BTreeMap::new();
        let mut sclk_in = BTreeMap::new();
        for &loc in self.chip.special_loc.keys() {
            match loc {
                SpecialLocKey::PclkIn(dir, idx) => {
                    let name = match (dir, idx) {
                        (Dir::W, 0) => "JPCLKCIBLLQ0",
                        (Dir::W, 1) => "JPCLKCIBULQ0",
                        (Dir::E, 0) => "JPCLKCIBLRQ2",
                        (Dir::E, 1) => "JPCLKCIBLRQ0",
                        (Dir::E, 2) => "JPCLKCIBURQ0",
                        (Dir::E, 3) => "JPCLKCIBURQ2",
                        (Dir::S, 0) => "JPCLKCIBLLQ1",
                        (Dir::S, 1) => "JPCLKCIBLRQ1",
                        (Dir::N, 0) => "JPCLKCIBULQ1",
                        (Dir::N, 1) => "JPCLKCIBURQ1",
                        _ => unreachable!(),
                    };
                    let wire = self.rc_wire(cell, name);
                    let pin = format!("PCLK_IN_{dir}{idx}");
                    self.add_bel_wire(bcrd, &pin, wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);
                    bel.pins.insert(pin, bpin);
                    pclk_in.insert(loc, wire);
                }
                SpecialLocKey::PclkInMid(idx) => {
                    let name = format!("JPCLKCIBMID{idx}");
                    let wire = self.rc_wire(cell, &name);
                    let pin = format!("PCLK_IN_M{idx}");
                    self.add_bel_wire(bcrd, &pin, wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);
                    bel.pins.insert(pin, bpin);
                    pclk_in.insert(loc, wire);
                }
                SpecialLocKey::SclkIn(dir, idx) => {
                    let name = match dir {
                        Dir::W => format!("JSCLKCIBL{idx}"),
                        Dir::E => format!("JSCLKCIBR{idx}"),
                        Dir::S => format!("JSCLKCIBB{idx}"),
                        Dir::N => format!("JSCLKCIBT{idx}"),
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
                    Dir::N => format!("JTPIO{idx}"),
                    Dir::S => continue,
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
        let mut serdes_in = BTreeMap::new();
        for (col, cd) in &self.chip.columns {
            if cd.io_s == IoGroupKind::Serdes {
                let bank = cd.bank_s.unwrap();
                let which = match bank {
                    50 => "PCSA",
                    51 => "PCSB",
                    52 => "PCSC",
                    53 => "PCSD",
                    _ => unreachable!(),
                };
                let idx = bank - 50;
                let bcrd = self.chip.bel_serdes(DirV::S, col);
                let cell_pcs = bcrd.cell.delta(0, -1);
                for fh in ['F', 'H'] {
                    let wire_pcs = self.rc_wire(cell_pcs, &format!("JFF_TX_{fh}_CLK_0_PCS"));
                    let wire = self.rc_wire(cell, &format!("J{which}_TX_{fh}_CLK"));
                    self.add_bel_wire(bcrd, format!("SERDES{idx}_TX_{fh}_CLK"), wire);
                    self.claim_pip(wire, wire_pcs);
                    serdes_in.insert((idx, fh), wire);
                }
            }
        }
        let mut pll_in = BTreeMap::new();
        let mut dll_in = BTreeMap::new();
        let mut clkdiv_in = BTreeMap::new();
        for (&loc, &cell_loc) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            let mut wires = vec![];
            let cell_pll = match loc.quad.h {
                DirH::W => cell_loc.delta(3, 0),
                DirH::E => cell_loc.delta(-3, 0),
            };
            let corner = match loc.quad {
                DirHV::SW => "LL",
                DirHV::SE => "RL",
                DirHV::NW => "LU",
                DirHV::NE => "RU",
            };
            for (i, name) in ["CLKOP", "CLKOS", "CLKOK", "CLKOK2"]
                .into_iter()
                .enumerate()
            {
                let wire_pll = self.rc_wire(cell_pll, &format!("J{name}_PLL"));
                let wire = self.rc_wire(
                    cell,
                    &format!("J{corner}PLL{ii}", ii = loc.idx * 4 + (i as u8)),
                );
                self.add_bel_wire(bcrd, format!("PLL_{loc}_{name}"), wire);
                self.claim_pip(wire, wire_pll);
                wires.push(wire);
            }
            pll_in.insert(loc, wires);
            if loc.quad.v == DirV::N && loc.idx == 0 {
                let cell_dll = match loc.quad.h {
                    DirH::W => cell_loc.delta(13, 0),
                    DirH::E => cell_loc.delta(-13, 0),
                };
                let lr = match loc.quad.h {
                    DirH::W => 'L',
                    DirH::E => 'R',
                };
                let mut wires = vec![];
                for (i, name) in ["CLKOP", "CLKOS"].into_iter().enumerate() {
                    let wire_dll = self.rc_wire(cell_dll, &format!("J{name}_DLL"));
                    let wire = self.rc_wire(cell, &format!("J{corner}DLL{i}"));
                    self.add_bel_wire(bcrd, format!("DLL_{loc}_{name}"), wire);
                    self.claim_pip(wire, wire_dll);
                    wires.push(wire);
                }
                dll_in.insert(loc.quad.h, wires);
                let mut wires = vec![];
                for name in ["CDIV1", "CDIV2", "CDIV4", "CDIV8"] {
                    let wire_clkdiv = self.rc_wire(cell_dll, &format!("J{name}_CLKDIV"));
                    let wire = self.rc_wire(cell, &format!("J{lr}{name}"));
                    self.add_bel_wire(bcrd, format!("CLKDIV_{h}_{name}", h = loc.quad.h), wire);
                    self.claim_pip(wire, wire_clkdiv);
                    wires.push(wire);
                }
                clkdiv_in.insert(loc.quad.h, wires);
            }
        }

        if self.chip.kind == ChipKind::Ecp3A {
            let cell_dll = cell.with_col(self.chip.col_w() + 14);
            for ei in 0..2 {
                let eip1 = ei + 1;
                let eclk = self.rc_wire(cell, &format!("JBRGECLK{eip1}"));
                self.add_bel_wire(bcrd, format!("ECLK{ei}"), eclk);
                let eclk_in = self.rc_wire(cell, &format!("JLDLLECLK{eip1}"));
                self.add_bel_wire(bcrd, format!("ECLK{ei}_IN"), eclk_in);
                self.claim_pip(eclk, eclk_in);
                let wire_dll = self.rc_wire(cell_dll, &format!("JDLLECLK{eip1}"));
                self.claim_pip(eclk_in, wire_dll);
            }
        }

        self.insert_bel(bcrd, bel);

        for (hv, ll, dcc, dcs) in [
            (DirHV::SW, "LL", bels::DCC_SW, bels::DCS_SW),
            (DirHV::SE, "LR", bels::DCC_SE, bels::DCS_SE),
            (DirHV::NW, "UL", bels::DCC_NW, bels::DCS_NW),
            (DirHV::NE, "UR", bels::DCC_NE, bels::DCS_NE),
        ] {
            for i in 0..8 {
                let wire_out = if i < 6 {
                    let bcrd_dcc = bcrd.bel(dcc[i]);
                    self.name_bel(bcrd_dcc, [format!("{ll}DCC{i}")]);
                    let mut bel = Bel::default();

                    let clki = self.rc_wire(cell, &format!("{ll}CLKI{i}_DCC"));
                    self.add_bel_wire(bcrd_dcc, "CLKI", clki);

                    let clki_in = self.claim_single_in(clki);
                    self.add_bel_wire(bcrd_dcc, "CLKI_IN", clki_in);

                    let wire_ce = self.rc_wire(cell, &format!("J{ll}CE{i}_DCC"));
                    self.add_bel_wire(bcrd_dcc, "CE", wire_ce);
                    bel.pins
                        .insert("CE".into(), self.xlat_int_wire(bcrd_dcc, wire_ce));

                    let wire_out = self.rc_wire(cell, &format!("{ll}CLKO{i}_DCC"));
                    self.add_bel_wire(bcrd_dcc, "OUT", wire_out);
                    self.claim_pip(wire_out, clki);

                    let (wei, ni) = [(0, 0), (1, 0), (0, 1), (1, 1), (0, 1), (1, 0)][i];
                    self.claim_pip(clki_in, io_in[&SpecialIoKey::Clock(Dir::W, wei)]);
                    self.claim_pip(clki_in, io_in[&SpecialIoKey::Clock(Dir::E, wei)]);
                    self.claim_pip(clki_in, io_in[&SpecialIoKey::Clock(Dir::N, ni)]);
                    let loc = [
                        SpecialLocKey::PclkIn(Dir::W, 1),
                        SpecialLocKey::PclkIn(Dir::W, 0),
                        SpecialLocKey::PclkIn(Dir::E, 2),
                        SpecialLocKey::PclkIn(Dir::E, 1),
                        SpecialLocKey::PclkIn(Dir::N, 0),
                        SpecialLocKey::PclkIn(Dir::S, 0),
                    ][i];
                    self.claim_pip(clki_in, pclk_in[&loc]);
                    self.claim_pip(clki_in, pclk_in[&SpecialLocKey::PclkInMid(i as u8)]);

                    for (&(_, hf), &wire) in &serdes_in {
                        if hf == ['H', 'F'][i % 2] {
                            self.claim_pip(clki_in, wire);
                        }
                    }

                    for wire in pll_in.values().flatten().copied() {
                        self.claim_pip(clki_in, wire);
                    }
                    for wire in dll_in.values().flatten().copied() {
                        self.claim_pip(clki_in, wire);
                    }
                    for wire in clkdiv_in.values().flatten().copied() {
                        self.claim_pip(clki_in, wire);
                    }

                    self.insert_bel(bcrd_dcc, bel);
                    wire_out
                } else {
                    let dcsi = i - 6;
                    let bcrd_dcs = bcrd.bel(dcs[dcsi]);
                    self.name_bel(bcrd_dcs, [format!("{ll}DCS{dcsi}")]);
                    let mut bel = Bel::default();

                    let clka = self.rc_wire(cell, &format!("{ll}CLK{dcsi}A_DCS"));
                    let clkb = self.rc_wire(cell, &format!("{ll}CLK{dcsi}B_DCS"));
                    self.add_bel_wire(bcrd_dcs, "CLKA", clka);
                    self.add_bel_wire(bcrd_dcs, "CLKB", clkb);

                    let clka_in = self.claim_single_in(clka);
                    let clkb_in = self.claim_single_in(clkb);
                    self.add_bel_wire(bcrd_dcs, "CLKA_IN", clka_in);
                    self.add_bel_wire(bcrd_dcs, "CLKB_IN", clkb_in);

                    let sel = self.rc_wire(cell, &format!("J{ll}SEL{dcsi}_DCS"));
                    self.add_bel_wire(bcrd_dcs, "SEL", sel);
                    bel.pins
                        .insert("SEL".into(), self.xlat_int_wire(bcrd_dcs, sel));

                    let wire_out = self.rc_wire(cell, &format!("{ll}DCSOUT{dcsi}_DCS"));
                    self.add_bel_wire(bcrd_dcs, "OUT", wire_out);
                    self.claim_pip(wire_out, clka);
                    self.claim_pip(wire_out, clkb);

                    self.claim_pip(clka_in, io_in[&SpecialIoKey::Clock(Dir::W, 0)]);
                    self.claim_pip(clka_in, io_in[&SpecialIoKey::Clock(Dir::E, 0)]);
                    self.claim_pip(clka_in, io_in[&SpecialIoKey::Clock(Dir::N, 1)]);
                    self.claim_pip(clkb_in, io_in[&SpecialIoKey::Clock(Dir::W, 1)]);
                    self.claim_pip(clkb_in, io_in[&SpecialIoKey::Clock(Dir::E, 1)]);
                    self.claim_pip(clkb_in, io_in[&SpecialIoKey::Clock(Dir::N, 0)]);

                    let (loc0a, loc1a, loc2a) = [
                        (
                            SpecialLocKey::PclkIn(Dir::W, 1),
                            SpecialLocKey::PclkIn(Dir::N, 1),
                            SpecialLocKey::PclkInMid(6),
                        ),
                        (
                            SpecialLocKey::PclkIn(Dir::E, 0),
                            SpecialLocKey::PclkIn(Dir::E, 2),
                            SpecialLocKey::PclkInMid(4),
                        ),
                    ][dcsi];
                    self.claim_pip(clka_in, pclk_in[&loc0a]);
                    self.claim_pip(clka_in, pclk_in[&loc1a]);
                    self.claim_pip(clka_in, pclk_in[&loc2a]);
                    let (loc0b, loc1b, loc2b) = [
                        (
                            SpecialLocKey::PclkIn(Dir::W, 0),
                            SpecialLocKey::PclkIn(Dir::S, 1),
                            SpecialLocKey::PclkInMid(1),
                        ),
                        (
                            SpecialLocKey::PclkIn(Dir::E, 1),
                            SpecialLocKey::PclkIn(Dir::E, 3),
                            SpecialLocKey::PclkInMid(7),
                        ),
                    ][dcsi];
                    self.claim_pip(clkb_in, pclk_in[&loc0b]);
                    self.claim_pip(clkb_in, pclk_in[&loc1b]);
                    self.claim_pip(clkb_in, pclk_in[&loc2b]);

                    for (&(_, hf), &wire) in &serdes_in {
                        if hf == 'H' {
                            self.claim_pip(clka_in, wire);
                        } else {
                            self.claim_pip(clkb_in, wire);
                        }
                    }

                    for (&loc, wires) in &pll_in {
                        for (i, &wire) in wires.iter().enumerate() {
                            match (loc.quad.h, i) {
                                (DirH::W, 0) => {
                                    self.claim_pip(clka_in, wire);
                                }
                                (DirH::E, 0) => {
                                    self.claim_pip(clkb_in, wire);
                                }
                                _ => {
                                    self.claim_pip(clka_in, wire);
                                    self.claim_pip(clkb_in, wire);
                                }
                            }
                        }
                    }
                    for (&edge, wires) in &dll_in {
                        for (i, &wire) in wires.iter().enumerate() {
                            match (edge, i) {
                                (DirH::W, 0) => {
                                    self.claim_pip(clka_in, wire);
                                }
                                (DirH::E, 0) => {
                                    self.claim_pip(clkb_in, wire);
                                }
                                _ => {
                                    self.claim_pip(clka_in, wire);
                                    self.claim_pip(clkb_in, wire);
                                }
                            }
                        }
                    }

                    for wire in clkdiv_in.values().flatten().copied() {
                        self.claim_pip(clka_in, wire);
                        self.claim_pip(clkb_in, wire);
                    }

                    self.insert_bel(bcrd_dcs, bel);
                    wire_out
                };
                let wire = self.claim_single_out(wire_out);
                self.add_bel_wire(bcrd, format!("PCLK{i}_{hv}"), wire);
            }
            for i in 0..8 {
                let sdclk_root = sclk_roots[&(hv, i)];
                self.add_bel_wire(bcrd, format!("SDCLK_ROOT_{hv}{i}"), sdclk_root);
                let (edge, idx) = [
                    (Dir::W, 0),
                    (Dir::W, 1),
                    (Dir::E, 0),
                    (Dir::E, 1),
                    (Dir::N, 0),
                    (Dir::N, 1),
                    (Dir::W, 0),
                    (Dir::E, 0),
                ][i];
                self.claim_pip(sdclk_root, io_in[&SpecialIoKey::Clock(edge, idx)]);
                let sources = [
                    [
                        (Dir::W, 1),
                        (Dir::W, 3),
                        (Dir::E, 0),
                        (Dir::E, 2),
                        (Dir::S, 1),
                        (Dir::S, 3),
                        (Dir::N, 0),
                    ],
                    [
                        (Dir::W, 0),
                        (Dir::W, 2),
                        (Dir::E, 1),
                        (Dir::S, 0),
                        (Dir::S, 2),
                        (Dir::N, 1),
                        (Dir::N, 3),
                    ],
                    [
                        (Dir::W, 1),
                        (Dir::W, 3),
                        (Dir::E, 0),
                        (Dir::E, 2),
                        (Dir::S, 3),
                        (Dir::N, 0),
                        (Dir::N, 2),
                    ],
                    [
                        (Dir::W, 2),
                        (Dir::E, 1),
                        (Dir::E, 3),
                        (Dir::S, 0),
                        (Dir::S, 2),
                        (Dir::N, 1),
                        (Dir::N, 3),
                    ],
                    [
                        (Dir::W, 1),
                        (Dir::W, 3),
                        (Dir::E, 0),
                        (Dir::E, 2),
                        (Dir::S, 1),
                        (Dir::S, 3),
                        (Dir::N, 2),
                    ],
                    [
                        (Dir::W, 0),
                        (Dir::W, 2),
                        (Dir::E, 3),
                        (Dir::S, 0),
                        (Dir::S, 2),
                        (Dir::N, 1),
                        (Dir::N, 3),
                    ],
                    [
                        (Dir::W, 1),
                        (Dir::W, 3),
                        (Dir::E, 0),
                        (Dir::E, 2),
                        (Dir::S, 1),
                        (Dir::N, 0),
                        (Dir::N, 2),
                    ],
                    [
                        (Dir::W, 0),
                        (Dir::E, 1),
                        (Dir::E, 3),
                        (Dir::S, 0),
                        (Dir::S, 2),
                        (Dir::N, 1),
                        (Dir::N, 3),
                    ],
                ];
                for (e, si) in sources[i] {
                    self.claim_pip(sdclk_root, sclk_in[&SpecialLocKey::SclkIn(e, si)]);
                }
            }
            for (w, name) in [("TIE0", "GND"), ("TIE1", "VCC")] {
                let col = self.chip.col_edge(hv.h);
                let row = self.chip.row_edge(hv.v);
                let wire = self.naming.interconnect
                    [&CellCoord::new(DieId::from_idx(0), col, row).wire(self.intdb.get_wire(w))];
                let wire = self.find_single_in(wire);
                self.add_bel_wire(bcrd, format!("{name}_{hv}"), wire);
                let (row_s, row_n) = match hv.v {
                    DirV::S => (self.chip.row_s() + 9, self.chip.row_clk),
                    DirV::N => (self.chip.row_clk, self.chip.row_n() + 1),
                };
                for row in row_s.range(row_n) {
                    let wire_int =
                        self.naming.interconnect[&CellCoord::new(DieId::from_idx(0), col, row)
                            .wire(self.intdb.get_wire(w))];
                    self.claim_pip(wire_int, wire);
                }
                for ll in ["LL", "UL", "LR", "UR"] {
                    let wire_in = self.rc_wire(cell, &format!("{ll}{name}I"));
                    self.claim_pip(wire, wire_in);
                }
                let wire_in = self.rc_wire(cell, &format!("{ll}{name}I"));
                self.add_bel_wire(bcrd, format!("{name}_{hv}_IN"), wire_in);
                let wire_in_in = self.rc_wire(
                    match hv.h {
                        DirH::W => cell,
                        DirH::E => cell.delta(1, 0),
                    },
                    &format!("{name}{ll}_{name}"),
                );
                self.add_bel_wire(bcrd, format!("{name}_{hv}_IN_IN"), wire_in_in);
                self.claim_pip(wire_in, wire_in_in);
            }
        }
    }

    pub(super) fn process_pclk_ecp3(&mut self) {
        let mut is_bot = true;
        let mut vprx_wires: BTreeMap<(DirV, ColId, usize), WireName> = BTreeMap::new();
        let mut vprx_source = BTreeMap::new();
        let mut prev = self.chip.col_w();
        for (col, cd) in &self.chip.columns {
            if cd.sdclk_break {
                if col < self.chip.col_clk {
                    vprx_source.insert(prev, col);
                } else if col == self.chip.col_clk {
                    vprx_source.insert(prev, prev);
                } else {
                    if prev == self.chip.col_clk {
                        vprx_source.insert(prev, col);
                    }
                    vprx_source.insert(col, col);
                }
                prev = col;
            }
        }
        for (row, rd) in &self.chip.rows {
            if !rd.pclk_drive {
                continue;
            }
            let mut row_n = row;
            let mut row_s = row - 1;
            while row_n != self.chip.row_n() && !self.chip.rows[row_n + 1].pclk_break {
                row_n += 1;
            }
            row_n += 1;
            while !self.chip.rows[row_s].pclk_break {
                row_s -= 1;
            }
            let mut hpsx_wires: BTreeMap<(ColId, usize), WireName> = BTreeMap::new();
            let mut vptx_wires_n: BTreeMap<(ColId, usize), WireName> = BTreeMap::new();
            let mut vptx_wires_s: BTreeMap<(ColId, usize), WireName> = BTreeMap::new();
            let mut vptx_wires_ss: BTreeMap<(ColId, usize), WireName> = BTreeMap::new();
            let mut col_hpsx = self.chip.col_w();
            for cell in self.edev.row(DieId::from_idx(0), row) {
                if self.chip.columns[cell.col].sdclk_break {
                    col_hpsx = cell.col;
                }
                let idx = self.chip.col_sclk_idx(cell.col);
                let mut pclk_idx = vec![idx, idx + 4];
                if self.edev.has_bel(cell.bel(bels::PCLK_SOURCE_W)) {
                    pclk_idx.extend([
                        (idx + 3) % 4,
                        (idx + 3) % 4 + 4,
                        (idx + 2) % 4,
                        (idx + 2) % 4 + 4,
                    ]);
                }
                if self.edev.has_bel(cell.bel(bels::PCLK_SOURCE_E)) {
                    pclk_idx.extend([(idx + 1) % 4, (idx + 1) % 4 + 4]);
                }
                for &i in &pclk_idx {
                    let wire_n = self.naming.interconnect
                        [&cell.wire(self.intdb.get_wire(&format!("PCLK{i}")))];
                    let wire_n = self.find_single_in(wire_n);
                    self.add_bel_wire(cell.bel(bels::INT), format!("PCLK{i}_N"), wire_n);
                    vptx_wires_n.insert((cell.col, i), wire_n);
                    for row in cell.row.range(row_n) {
                        let wire = self.naming.interconnect[&cell
                            .with_row(row)
                            .wire(self.intdb.get_wire(&format!("PCLK{i}")))];
                        self.claim_pip(wire, wire_n);
                    }

                    let wire_s = self.naming.interconnect[&cell
                        .delta(0, -1)
                        .wire(self.intdb.get_wire(&format!("PCLK{i}")))];
                    let wire_s = self.find_single_in(wire_s);
                    self.add_bel_wire(cell.bel(bels::INT), format!("PCLK{i}_S"), wire_s);
                    vptx_wires_s.insert((cell.col, i), wire_s);
                    for row in row_s.range(cell.row) {
                        let wire = self.naming.interconnect[&cell
                            .with_row(row)
                            .wire(self.intdb.get_wire(&format!("PCLK{i}")))];
                        self.claim_pip(wire, wire_s);
                    }

                    if is_bot
                        && let Some(&wire_ss) = self.naming.interconnect.get(
                            &cell
                                .with_row(self.chip.row_s())
                                .wire(self.intdb.get_wire(&format!("PCLK{i}"))),
                        )
                    {
                        let wire_ss = self.find_single_in(wire_ss);
                        vptx_wires_ss.insert((cell.col, i), wire_ss);
                        self.add_bel_wire(cell.bel(bels::INT), format!("PCLK{i}_SS"), wire_ss);
                        self.claim_pip(wire_ss, wire_s);
                        for row in self.chip.row_s().range(row_s) {
                            let wire = self.naming.interconnect[&cell
                                .with_row(row)
                                .wire(self.intdb.get_wire(&format!("PCLK{i}")))];
                            self.claim_pip(wire, wire_ss);
                        }
                    }
                }

                let (r, c) = self.rc(cell);
                for i in 0..2 {
                    let pclk_i = pclk_idx[i];
                    let bcrd = cell.bel(bels::PCLK_DCC[i]);
                    // what in fuck's name.
                    if self.chip.rows.len() == 53
                        && cell.col.to_idx() == 36
                        && cell.row.to_idx() == 18
                    {
                        if i == 0 {
                            self.name_bel(
                                bcrd,
                                [format!("DCC_R{r}C{c}_0"), format!("DCC_R{r}C{c}_1")],
                            );
                        } else {
                            self.name_bel(
                                bcrd,
                                [format!("DCC_R{r}C{c}C"), format!("DCC_R{r}C{c}D")],
                            );
                        }
                    } else if self.chip.rows.len() == 53
                        && cell.col.to_idx() == 45
                        && cell.row.to_idx() == 18
                    {
                        if i == 0 {
                            self.name_bel(
                                bcrd,
                                [format!("DCC_R{r}C{c}A"), format!("DCC_R{r}C{c}B")],
                            );
                        } else {
                            self.name_bel(
                                bcrd,
                                [format!("DCC_R{r}C{c}C"), format!("DCC_R{r}C{c}D")],
                            );
                        }
                    } else {
                        self.name_bel(
                            bcrd,
                            [format!("DCC_R{r}C{c}_{i}B"), format!("DCC_R{r}C{c}_{i}T")],
                        );
                    }
                    let mut bel = Bel::default();

                    let pclk = self.intdb.get_wire(&format!("PCLK{pclk_i}"));
                    bel.pins.insert(
                        "OUT_S".into(),
                        BelPin::new_out(TileWireCoord::new_idx(1, pclk)),
                    );
                    bel.pins.insert(
                        "OUT_N".into(),
                        BelPin::new_out(TileWireCoord::new_idx(0, pclk)),
                    );
                    let out_s = self.rc_wire(cell, &format!("CLKO{pclk_i}B_DCC"));
                    self.add_bel_wire(bcrd, "OUT_S", out_s);
                    self.claim_pip(vptx_wires_s[&(cell.col, pclk_i)], out_s);
                    let out_n = self.rc_wire(cell, &format!("CLKO{pclk_i}T_DCC"));
                    self.add_bel_wire(bcrd, "OUT_N", out_n);
                    self.claim_pip(vptx_wires_n[&(cell.col, pclk_i)], out_n);

                    // CE
                    let wire_s = self.rc_wire(cell, &format!("JCE{pclk_i}B_DCC"));
                    self.add_bel_wire(bcrd, "CE_S", wire_s);
                    let wire_n = self.rc_wire(cell, &format!("JCE{pclk_i}T_DCC"));
                    self.add_bel_wire(bcrd, "CE_N", wire_n);
                    let bpin = self.xlat_int_wire(bcrd, wire_s);
                    assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_n));
                    bel.pins.insert("CE".into(), bpin);

                    // IN
                    let in_s = self.rc_wire(cell, &format!("CLKI{pclk_i}B_DCC"));
                    self.add_bel_wire(bcrd, "IN_S", in_s);
                    self.claim_pip(out_s, in_s);
                    let in_n = self.rc_wire(cell, &format!("CLKI{pclk_i}T_DCC"));
                    self.add_bel_wire(bcrd, "IN_N", in_n);
                    self.claim_pip(out_n, in_n);

                    let hpsx = self.find_single_in(in_s);
                    self.claim_pip(in_s, hpsx);
                    self.claim_pip(in_n, hpsx);
                    self.add_bel_wire_no_claim(bcrd, "IN", hpsx);

                    let vprx = self.find_single_in(hpsx);
                    self.add_bel_wire_no_claim(bcrd, "IN_IN", vprx);

                    match hpsx_wires.entry((col_hpsx, pclk_i)) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(hpsx);
                            self.claim_node(hpsx);
                            self.claim_pip(hpsx, vprx);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), hpsx);
                        }
                    }

                    let hv = DirHV {
                        h: if cell.col < self.chip.col_clk {
                            DirH::W
                        } else {
                            DirH::E
                        },
                        v: if cell.row < self.chip.row_clk {
                            DirV::S
                        } else {
                            DirV::N
                        },
                    };
                    match vprx_wires.entry((hv.v, vprx_source[&col_hpsx], pclk_i)) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(vprx);
                            self.claim_node(vprx);
                            let root = self
                                .naming
                                .bel_wire(self.chip.bel_clk_root(), &format!("PCLK{pclk_i}_{hv}"));
                            self.claim_pip(vprx, root);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), vprx);
                        }
                    }

                    self.insert_bel(bcrd, bel);
                }
            }

            let mut col_hpsx = self.chip.col_w();
            for cell in self.edev.row(DieId::from_idx(0), row) {
                if self.chip.columns[cell.col].sdclk_break {
                    col_hpsx = cell.col;
                }
                let idx = self.chip.col_sclk_idx(cell.col);
                let (bcrd, pclk_idx) = if self.edev.has_bel(cell.bel(bels::PCLK_SOURCE_W)) {
                    (
                        cell.bel(bels::PCLK_SOURCE_W),
                        vec![
                            (idx + 3) % 4,
                            (idx + 3) % 4 + 4,
                            (idx + 2) % 4,
                            (idx + 2) % 4 + 4,
                        ],
                    )
                } else if self.edev.has_bel(cell.bel(bels::PCLK_SOURCE_E)) {
                    (
                        cell.bel(bels::PCLK_SOURCE_E),
                        vec![(idx + 1) % 4, (idx + 1) % 4 + 4],
                    )
                } else {
                    continue;
                };
                self.name_bel_null(bcrd);
                let mut bel = Bel::default();
                for i in pclk_idx {
                    let pclk = self.intdb.get_wire(&format!("PCLK{i}"));
                    bel.pins.insert(
                        format!("OUT_S{i}"),
                        BelPin::new_out(TileWireCoord::new_idx(1, pclk)),
                    );
                    bel.pins.insert(
                        format!("OUT_N{i}"),
                        BelPin::new_out(TileWireCoord::new_idx(0, pclk)),
                    );
                    let hpsx = hpsx_wires[&(col_hpsx, i)];
                    self.claim_pip(vptx_wires_s[&(cell.col, i)], hpsx);
                    self.claim_pip(vptx_wires_n[&(cell.col, i)], hpsx);
                }
                self.insert_bel(bcrd, bel);
            }

            is_bot = false;
        }
    }
}

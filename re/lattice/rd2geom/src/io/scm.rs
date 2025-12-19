use prjcombine_ecp::{
    bels,
    chip::{IoGroupKind, PllLoc, PllPad, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirV},
    grid::{BelCoord, CellCoord, EdgeIoCoord},
};
use prjcombine_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    pub fn xlat_io_loc_scm(&self, io: EdgeIoCoord) -> (CellCoord, &'static str) {
        let bcrd = self.chip.get_io_loc(io);
        let abcd = ["A", "B", "C", "D"][io.iob().to_idx() % 4];
        let cidx = io.iob().to_idx() / 4;
        match io.edge() {
            Dir::H(edge) => {
                let kind = match edge {
                    DirH::W => self.chip.rows[bcrd.row].io_w,
                    DirH::E => self.chip.rows[bcrd.row].io_e,
                };
                let dy = match kind {
                    IoGroupKind::Quad => 0,
                    IoGroupKind::Dozen => 2 - cidx,
                    _ => unreachable!(),
                };
                (bcrd.cell.delta(0, dy as i32), abcd)
            }
            Dir::V(_) => (bcrd.cell.delta(cidx as i32, 0), abcd),
        }
    }

    pub(super) fn process_eclk_scm(&mut self) {
        for (bank, edge, base_idx) in [
            (1, Dir::N, 0),
            (2, Dir::E, 0),
            (4, Dir::S, 8),
            (5, Dir::S, 0),
            (7, Dir::W, 0),
        ] {
            let cell_tile = self.chip.special_loc[&SpecialLocKey::Bc(bank)];
            let cell = if bank == 1 {
                cell_tile.delta(-1, 0)
            } else {
                cell_tile
            };
            let pll_sources = match bank {
                1 => [].as_slice(),
                7 => [
                    (DirHV::NW, 0),
                    (DirHV::NW, 1),
                    (DirHV::NW, 2),
                    (DirHV::NW, 3),
                    (DirHV::SW, 0),
                    (DirHV::SW, 1),
                    (DirHV::SW, 2),
                    (DirHV::SW, 3),
                ]
                .as_slice(),
                2 => [
                    (DirHV::NE, 0),
                    (DirHV::NE, 1),
                    (DirHV::NE, 2),
                    (DirHV::NE, 3),
                    (DirHV::SE, 0),
                    (DirHV::SE, 1),
                    (DirHV::SE, 2),
                    (DirHV::SE, 3),
                ]
                .as_slice(),
                5 => [
                    (DirHV::SW, 0),
                    (DirHV::SW, 1),
                    (DirHV::SW, 2),
                    (DirHV::SW, 3),
                    (DirHV::SW, 4),
                    (DirHV::SW, 5),
                ]
                .as_slice(),
                4 => [
                    (DirHV::SE, 0),
                    (DirHV::SE, 1),
                    (DirHV::SE, 2),
                    (DirHV::SE, 3),
                    (DirHV::SE, 4),
                    (DirHV::SE, 5),
                ]
                .as_slice(),
                _ => unreachable!(),
            };
            let pll_sources_m: Vec<_> = (0..pll_sources.len())
                .map(|i| {
                    let abcd = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'][i];
                    let prefix = if i < 4 { "XXC" } else { "XXX" };
                    self.rc_io_wire(cell, &format!("J{prefix}MCLK{abcd}"))
                })
                .collect();
            let pll_sources_n: Vec<_> = (0..pll_sources.len())
                .map(|i| {
                    let abcd = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'][i];
                    let prefix = if i < 4 { "XXC" } else { "XXX" };
                    self.rc_io_wire(cell, &format!("J{prefix}NCLK{abcd}"))
                })
                .collect();
            // W:
            // XXCMCLKA: NW CLKOS A     A       0
            // XXCMCLKB: NW CLKOS B     B       2
            // XXCMCLKC: NW CLKOP C     C       4
            // XXCMCLKD: NW CLKOP D     D       6
            // XXXMCLKE: SW CLKOS A     A       0
            // XXXMCLKF: SW CLKOS B     B       2
            // XXXMCLKG: SW CLKOP C     C       4
            // XXXMCLKH: SW CLKOP D     D       6
            // XXCNCLKA: NW CLKOP A     D       1
            // XXCNCLKB: NW CLKOP B     A       3
            // XXCNCLKC: NW CLKOS C     B       5
            // XXCNCLKD: NW CLKOS D     C       7
            // XXXNCLKE: SW CLKOP A     D       1
            // XXXNCLKF: SW CLKOP B     A       3
            // XXXNCLKG: SW CLKOS C     B       5
            // XXXNCLKH: SW CLKOS D     C       7
            for i in 0..4 {
                let bcrd = cell_tile.bel(bels::CLKDIV[i]);
                let abcd = ['A', 'B', 'C', 'D'][i];
                self.name_bel(bcrd, [format!("CLKDIV{bank}{abcd}")]);
                let mut bel = Bel::default();

                let wire = self.rc_io_wire(cell, &format!("JLSR{abcd}_CLKDIV"));
                self.add_bel_wire(bcrd, "LSR", wire);
                bel.pins
                    .insert("LSR".into(), self.xlat_int_wire(bcrd, wire));

                let clki_in = self.rc_io_wire(cell, &format!("CLKI{abcd}"));
                self.add_bel_wire(bcrd, "CLKI_IN", clki_in);
                let clki = self.rc_io_wire(cell, &format!("CLKI{abcd}_CLKDIV"));
                self.add_bel_wire(bcrd, "CLKI", clki);
                self.claim_pip(clki, clki_in);

                let clki_int = self.rc_io_wire(cell, &format!("JCIB_{ii}", ii = i % 2));
                self.claim_pip(clki_in, clki_int);
                let clki_io = self.rc_io_wire(cell, &format!("JPIO_{i}"));
                self.claim_pip(clki_in, clki_io);

                for wire in [
                    pll_sources_m.get(i),
                    pll_sources_m.get(i + 4),
                    pll_sources_n.get((i + 1) % 4),
                    pll_sources_n.get((i + 1) % 4 + 4),
                ] {
                    let Some(&wire) = wire else { continue };
                    self.claim_pip(clki_in, wire);
                }

                let clko = self.rc_io_wire(cell, &format!("JCLKO{abcd}_CLKDIV"));
                self.add_bel_wire(bcrd, "CLKO", clko);
                self.claim_pip(clko, clki);

                let elsr = self.rc_io_wire(cell, &format!("ELSR{abcd}_CLKDIV"));
                self.add_bel_wire(bcrd, "ELSR", elsr);
                let elsr_out = self.rc_io_wire(cell, &format!("ELSR{abcd}"));
                self.add_bel_wire(bcrd, "ELSR_OUT", elsr_out);
                self.claim_pip(elsr_out, elsr);

                self.insert_bel(bcrd, bel);
            }
            {
                let bcrd = cell_tile.bel(bels::ECLK_ROOT);
                self.name_bel_null(bcrd);
                let mut bel = Bel::default();

                let mut cibs = vec![];
                for i in 0..2 {
                    let cib = self.rc_io_wire(cell, &format!("JCIB_{i}"));
                    self.add_bel_wire(bcrd, format!("CIB{i}"), cib);
                    cibs.push(cib);
                    bel.pins
                        .insert(format!("CIB{i}"), self.xlat_int_wire(bcrd, cib));
                }

                for (i, &(hv, pll_idx)) in pll_sources.iter().enumerate() {
                    let cell_pll = cell.with_cr(self.chip.col_edge(hv.h), self.chip.row_edge(hv.v));
                    let (kind, sidx, clkop, clkos) = if pll_idx < 2 {
                        ("PLL", pll_idx, pll_sources_n[i], pll_sources_m[i])
                    } else {
                        ("DLL", pll_idx - 2, pll_sources_m[i], pll_sources_n[i])
                    };
                    self.add_bel_wire(bcrd, format!("{hv}_{kind}{sidx}_CLKOP"), clkop);
                    self.add_bel_wire(bcrd, format!("{hv}_{kind}{sidx}_CLKOS"), clkos);
                    let abcd = ['A', 'B', 'C', 'D', 'E', 'F'][pll_idx];
                    let clkop_pll = self.rc_corner_wire(cell_pll, &format!("JCLKOP{abcd}_{kind}"));
                    let clkos_pll = self.rc_corner_wire(cell_pll, &format!("JCLKOS{abcd}_{kind}"));
                    self.claim_pip(clkop, clkop_pll);
                    self.claim_pip(clkos, clkos_pll);
                }

                let mut eclks = vec![];
                for i in 0..8 {
                    let i: usize = i;
                    let pio = self.rc_io_wire(cell, &format!("JPIO_{i}"));
                    self.add_bel_wire(bcrd, format!("PIO{i}"), pio);

                    let io = self.chip.special_io[&SpecialIoKey::Clock(edge, base_idx + i as u8)];
                    let (cell_io, abcd) = self.xlat_io_loc_scm(io);
                    let wire_io = self.rc_io_wire(cell_io, &format!("JINDDCK{abcd}"));
                    self.claim_pip(pio, wire_io);

                    let eclk = self.pips_fwd[&pio]
                        .iter()
                        .copied()
                        .find(|w| !self.naming.strings[w.suffix].starts_with("CLKI"))
                        .unwrap();
                    self.add_bel_wire(bcrd, format!("ECLK{i}"), eclk);
                    eclks.push(eclk);
                    self.claim_pip(eclk, pio);

                    for &wire in &cibs {
                        self.claim_pip(eclk, wire);
                    }

                    let pll_sources = if i.is_multiple_of(2) {
                        &pll_sources_m
                    } else {
                        &pll_sources_n
                    };
                    for idx in [i / 2, i / 2 + 4] {
                        if let Some(&wire) = pll_sources.get(idx) {
                            self.claim_pip(eclk, wire);
                        }
                    }

                    let abcd = ['A', 'B', 'C', 'D'][i % 4];
                    let elsr = self.rc_io_wire(cell, &format!("ELSR{abcd}"));
                    self.claim_pip(eclk, elsr);
                }

                self.insert_bel(bcrd, bel);
            }
        }
    }

    pub(super) fn process_pll_scm(&mut self) {
        for hv in DirHV::DIRS {
            let corner = match hv {
                DirHV::SW => "LL",
                DirHV::SE => "LR",
                DirHV::NW => "UL",
                DirHV::NE => "UR",
            };
            let cell_tile = &self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(hv, 0))];
            let cell = cell_tile.with_row(self.chip.row_edge(hv.v));
            {
                let bcrd = cell_tile.bel(bels::PLL_SMI);
                self.name_bel_null(bcrd);
                let mut bel = Bel::default();
                for pin in [
                    "SMIRD", "SMIWR", "SMICLK", "SMIRSTN", "SMIWDATA", "SMIADDR0", "SMIADDR1",
                    "SMIADDR2", "SMIADDR3", "SMIADDR4", "SMIADDR5", "SMIADDR6", "SMIADDR7",
                    "SMIADDR8", "SMIADDR9",
                ] {
                    let wire = self.rc_corner_wire(cell, &format!("J{pin}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }
                self.insert_bel(bcrd, bel);

                for edge in ['H', 'V'] {
                    let (bank, lrb) = match (hv, edge) {
                        (DirHV::SW, 'H') => (5, 'B'),
                        (DirHV::SW, 'V') => (6, 'L'),
                        (DirHV::SE, 'H') => (4, 'B'),
                        (DirHV::SE, 'V') => (3, 'R'),
                        (DirHV::NW, 'H') => continue,
                        (DirHV::NW, 'V') => (7, 'L'),
                        (DirHV::NE, 'H') => continue,
                        (DirHV::NE, 'V') => (2, 'R'),
                        _ => unreachable!(),
                    };
                    let bcrd_eclk = self.chip.bel_eclk_root_bank(bank);
                    for i in 0..8 {
                        let wire = self.rc_corner_wire(cell, &format!("J{edge}ECX{i:02}00"));
                        self.add_bel_wire(bcrd, format!("ECLK_{edge}{i}"), wire);
                        let wire_in = self.rc_corner_wire(cell, &format!("J{edge}EC{lrb}{i:02}00"));
                        self.add_bel_wire(bcrd, format!("ECLK_{edge}{i}_IN"), wire_in);
                        self.claim_pip(wire, wire_in);
                        let wire_eclk = self.naming.bel_wire(bcrd_eclk, &format!("ECLK{i}"));
                        self.claim_pip_bi(wire_in, wire_eclk);
                    }
                }
            }
            for idx in 0..6 {
                let bcrd = cell_tile.bel(if idx < 2 {
                    bels::PLL[idx]
                } else {
                    bels::DLL[idx - 2]
                });
                if !self.edev.has_bel(bcrd) {
                    continue;
                }
                let abcd = ['A', 'B', 'C', 'D', 'E', 'F'][idx];
                let kind = if idx < 2 { "PLL" } else { "DLL" };
                self.name_bel(bcrd, [format!("{kind}_{corner}C{abcd}",)]);
                let mut bel = Bel::default();

                for pin in [
                    "SMIRD", "SMIWR", "SMICLK", "SMIRSTN", "SMIWDATA", "SMIADDR0", "SMIADDR1",
                    "SMIADDR2", "SMIADDR3", "SMIADDR4", "SMIADDR5", "SMIADDR6", "SMIADDR7",
                    "SMIADDR8", "SMIADDR9",
                ] {
                    let wire = self.rc_corner_wire(cell, &format!("{pin}{abcd}_{kind}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    let wire_in = self.rc_corner_wire(cell, &format!("{pin}{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_IN"), wire_in);
                    self.claim_pip(wire, wire_in);
                    let wire_common = self.rc_corner_wire(cell, &format!("J{pin}"));
                    self.claim_pip(wire_in, wire_common);
                }

                let pins = if idx < 2 {
                    ["RST"].as_slice()
                } else {
                    ["RST", "DTCCST0", "DTCCST1", "UDDCNTL"].as_slice()
                };
                for &pin in pins {
                    let pin_alt0 = if pin == "RST" && kind == "DLL" {
                        "RSTN"
                    } else {
                        pin
                    };
                    let pin_alt1 = if pin == "RST" { "RSTN" } else { pin };
                    let wire = self.rc_corner_wire(cell, &format!("{pin_alt0}{abcd}_{kind}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    let wire_in = self.rc_corner_wire(cell, &format!("{pin_alt0}{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_IN"), wire_in);
                    self.claim_pip(wire, wire_in);
                    let wire_cib = self.rc_corner_wire(cell, &format!("JCIB_{pin_alt1}{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_CIB"), wire_cib);
                    self.claim_pip(wire_in, wire_cib);
                    bel.pins
                        .insert(pin.into(), self.xlat_int_wire(bcrd, wire_cib));
                }

                let pins = if idx < 2 {
                    ["LOCK", "SMIRDATA", "TPREF", "TPFB"].as_slice()
                } else {
                    ["LOCK", "SMIRDATA"].as_slice()
                };

                for &pin in pins {
                    let wire = self.rc_corner_wire(cell, &format!("{pin}{abcd}_{kind}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    let wire_out = self.rc_corner_wire(cell, &format!("{pin}{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_OUT"), wire_out);
                    self.claim_pip(wire_out, wire);
                    let wire_cib = self.rc_corner_wire(cell, &format!("JCIB_{pin}{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_CIB"), wire_cib);
                    self.claim_pip(wire_cib, wire_out);
                    bel.pins
                        .insert(pin.into(), self.xlat_int_wire(bcrd, wire_cib));
                }

                for pin in ["CLKOP", "CLKOS"] {
                    let wire = self.rc_corner_wire(cell, &format!("J{pin}{abcd}_{kind}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    let wire_out = self.rc_corner_wire(cell, &format!("{pin}{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_OUT"), wire_out);
                    self.claim_pip(wire_out, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }

                for pin in ["CLKI", "CLKFB"] {
                    let wire = self.rc_corner_wire(cell, &format!("{pin}{abcd}_{kind}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    let wire_in = self.rc_corner_wire(cell, &format!("{pin}{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_IN"), wire_in);
                    self.claim_pip(wire, wire_in);

                    let infb = if pin == "CLKI" { "IN" } else { "FB" };

                    let wire_int = self.rc_corner_wire(cell, &format!("JSC_{infb}_{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_INT"), wire_int);
                    self.claim_pip(wire_in, wire_int);
                    bel.pins
                        .insert(pin.into(), self.xlat_int_wire(bcrd, wire_int));

                    let wire_io = self.rc_corner_wire(cell, &format!("JPIO_{infb}_{abcd}"));
                    self.add_bel_wire(bcrd, format!("{pin}_IO"), wire_io);
                    self.claim_pip(wire_in, wire_io);
                    let pad = match (idx, pin) {
                        (0, "CLKI") => PllPad::PllIn0,
                        (1, "CLKI") => PllPad::PllIn1,
                        (2, "CLKI") => PllPad::DllIn0,
                        (3, "CLKI") => PllPad::DllIn1,
                        (4, "CLKI") => PllPad::DllIn2,
                        (5, "CLKI") => PllPad::DllIn3,
                        (0, "CLKFB") => PllPad::PllIn1,
                        (1, "CLKFB") => PllPad::PllIn0,
                        (2, "CLKFB") => PllPad::DllIn1,
                        (3, "CLKFB") => PllPad::DllIn0,
                        (4, "CLKFB") => PllPad::DllIn3,
                        (5, "CLKFB") => PllPad::DllIn2,
                        _ => unreachable!(),
                    };
                    let io = self.chip.special_io[&SpecialIoKey::Pll(pad, PllLoc::new(hv, 0))];
                    let (cell_io, abcd_io) = self.xlat_io_loc_scm(io);
                    let wire_src = self.rc_io_wire(cell_io, &format!("JINDDCK{abcd_io}"));
                    self.claim_pip(wire_io, wire_src);

                    for edge in ['H', 'V'] {
                        if hv.v == DirV::N && edge == 'H' {
                            continue;
                        }
                        for j in 0..8 {
                            let wire_eclk =
                                self.rc_corner_wire(cell, &format!("J{edge}ECX{j:02}00"));
                            self.claim_pip(wire_in, wire_eclk);
                        }
                    }
                    if pin == "CLKFB" && kind == "PLL" {
                        let clkintfb = self.rc_corner_wire(cell, &format!("CLKINTFB{abcd}_PLL"));
                        self.add_bel_wire(bcrd, "CLKINTFB", clkintfb);
                        let clkintfb_out = self.rc_corner_wire(cell, &format!("CLKINTFB{abcd}"));
                        self.add_bel_wire(bcrd, "CLKINTFB_OUT", clkintfb_out);
                        self.claim_pip(clkintfb_out, clkintfb);
                        self.claim_pip(wire_in, clkintfb_out);
                    }
                }

                if kind == "DLL" {
                    for j in 0..9 {
                        let wire = self.rc_corner_wire(cell, &format!("DCNTL{j}{abcd}_{kind}"));
                        self.add_bel_wire(bcrd, format!("DCNTL{j}"), wire);
                        let wire_out = self.rc_corner_wire(cell, &format!("DCNTL{j}{abcd}"));
                        self.add_bel_wire(bcrd, format!("DCNTL{j}_OUT"), wire_out);
                        self.claim_pip(wire_out, wire);
                    }

                    let clkiduty = self.rc_corner_wire(cell, &format!("CLKIDUTY{abcd}_{kind}"));
                    self.add_bel_wire(bcrd, "CLKIDUTY", clkiduty);
                    let clkiduty_in = self.rc_corner_wire(cell, &format!("CLKIDUTY{abcd}"));
                    self.add_bel_wire(bcrd, "CLKIDUTY_IN", clkiduty_in);
                    self.claim_pip(clkiduty, clkiduty_in);
                    for src_idx in [idx & 1, idx ^ 1] {
                        let src_abcd = ['A', 'B', 'C', 'D', 'E', 'F'][src_idx];
                        for src_pin in ["CLKOP", "CLKOS"] {
                            let wire = self.rc_corner_wire(cell, &format!("{src_pin}{src_abcd}"));
                            self.claim_pip(clkiduty_in, wire);
                        }
                    }
                }

                self.insert_bel(bcrd, bel);
            }
            for i in 0..2 {
                let bcrd = cell_tile.bel(bels::DLL_DCNTL[i]);
                self.name_bel_null(bcrd);
                let mut bel = Bel::default();

                for j in 0..9 {
                    let wire_in = self.rc_corner_wire(cell, &format!("JDCNTL{i}_IN_{j}"));
                    self.add_bel_wire(bcrd, format!("DCNTL_IN{j}"), wire_in);
                    bel.pins
                        .insert(format!("DCNTL_IN{j}"), self.xlat_int_wire(bcrd, wire_in));

                    let wire_out = self.rc_corner_wire(cell, &format!("JDCNTL{i}_OUT_{j}"));
                    self.add_bel_wire(bcrd, format!("DCNTL_OUT{j}"), wire_out);
                    bel.pins
                        .insert(format!("DCNTL_OUT{j}"), self.xlat_int_wire(bcrd, wire_out));

                    let abcd = ['C', 'D'][i];
                    let wire_dll = self.rc_corner_wire(cell, &format!("DCNTL{j}{abcd}"));
                    self.claim_pip(wire_out, wire_dll);
                    if hv.v == DirV::S {
                        let abcd = ['E', 'F'][i];
                        let wire_dll = self.rc_corner_wire(cell, &format!("DCNTL{j}{abcd}"));
                        self.claim_pip(wire_out, wire_dll);
                    }

                    for edge in ['H', 'V'] {
                        if edge == 'H' && hv == DirHV::NW {
                            continue;
                        }
                        let wire_edge = self.rc_corner_wire(cell, &format!("JDCNTL{i}_{edge}_{j}"));
                        self.add_bel_wire(bcrd, format!("DCNTL_{edge}{j}"), wire_edge);
                        self.claim_pip(wire_edge, wire_in);

                        let abcd = ['C', 'D'][i];
                        let wire_dll = self.rc_corner_wire(cell, &format!("DCNTL{j}{abcd}"));
                        self.claim_pip(wire_edge, wire_dll);
                        if hv.v == DirV::S {
                            let abcd = ['E', 'F'][i];
                            let wire_dll = self.rc_corner_wire(cell, &format!("DCNTL{j}{abcd}"));
                            self.claim_pip(wire_edge, wire_dll);
                        }

                        let wire_edge_mid = self.claim_single_out(wire_edge);
                        self.add_bel_wire(bcrd, format!("DCNTL_{edge}{j}_MID"), wire_edge_mid);
                        let wire_edge_out = self.claim_single_out(wire_edge_mid);
                        self.add_bel_wire(bcrd, format!("DCNTL_{edge}{j}_OUT"), wire_edge_out);
                    }
                }

                self.insert_bel(bcrd, bel);
            }
            if hv == DirHV::SW {
                let bcrd = cell_tile.bel(bels::RNET);
                self.name_bel(bcrd, ["RNET"]);
                self.insert_simple_bel(bcrd, cell, "RNET");
            }
            if hv == DirHV::NW {
                for (slot, name, suffix) in [
                    (bels::M0, "M0PAD", "M0"),
                    (bels::M1, "M1PAD", "M1"),
                    (bels::M2, "M2PAD", "M2"),
                    (bels::M3, "M3PAD", "M3"),
                ] {
                    let bcrd = cell_tile.bel(slot);
                    self.name_bel(bcrd, [name]);
                    self.insert_simple_bel(bcrd, cell, suffix);
                }
                {
                    let bcrd = cell_tile.bel(bels::RESETN);
                    self.name_bel(bcrd, ["RESETN"]);
                    self.insert_simple_bel(bcrd, cell, "RSTN");
                }
                {
                    let bcrd = cell_tile.bel(bels::RDCFGN);
                    self.name_bel(bcrd, ["RDCFGN"]);
                    let mut bel = Bel::default();

                    let rdcfgn = self.rc_corner_wire(cell, "RDCFGN_RDCFGN");
                    self.add_bel_wire(bcrd, "RDCFGN", rdcfgn);

                    let rdcfgn_out = self.rc_corner_wire(cell, "JRDCFGN");
                    self.add_bel_wire(bcrd, "RDCFGN_OUT", rdcfgn_out);
                    self.claim_pip(rdcfgn_out, rdcfgn);
                    bel.pins
                        .insert("RDCFGN".into(), self.xlat_int_wire(bcrd, rdcfgn_out));

                    let tsalln_io = self.rc_corner_wire(cell, "JTSALLN");
                    self.add_bel_wire(bcrd, "TSALLN_IO", tsalln_io);
                    self.claim_pip(tsalln_io, rdcfgn);

                    let cell_rdbk = self.chip.special_loc[&SpecialLocKey::Config];
                    let tsalln = self.rc_wire(cell_rdbk, "JTSALLN_RDBK");
                    self.add_bel_wire(bcrd, "TSALLN", tsalln);
                    self.claim_pip(tsalln, tsalln_io);
                    bel.pins
                        .insert("TSALLN".into(), self.xlat_int_wire(bcrd, tsalln));

                    self.insert_bel(bcrd, bel);
                }
            }
            if hv == DirHV::NE {
                let bcrd = cell_tile.bel(bels::CCLK);
                self.name_bel(bcrd, ["CCLK"]);
                self.insert_simple_bel(bcrd, cell, "CCLK");
                for (slot, name) in [(bels::TCK, "TCK"), (bels::TMS, "TMS"), (bels::TDI, "TDI")] {
                    let bcrd = cell_tile.bel(slot);
                    self.name_bel(bcrd, [name]);
                    let wire = self.rc_corner_wire(cell, &format!("J{name}_{name}"));
                    self.add_bel_wire(bcrd, name, wire);
                    let mut bel = Bel::default();
                    bel.pins
                        .insert(name.into(), self.xlat_int_wire_filter(bcrd, wire));
                    self.insert_bel(bcrd, bel);
                }
            }
            if let Some(name) = match hv {
                DirHV::SW => Some("PROMON2V"),
                DirHV::SE => Some("PROMON1V"),
                _ => None,
            } {
                let bcrd = cell_tile.bel(bels::PROMON);
                self.name_bel(bcrd, [name]);
                self.insert_simple_bel(bcrd, cell, name);
            }
        }
    }

    fn process_single_io_scm(&mut self, bcrd: BelCoord) {
        let io = self.chip.get_io_crd(bcrd);
        let bank = self.chip.get_io_bank(io);
        let idx = io.iob().to_idx();
        let (cell, abcd) = self.xlat_io_loc_scm(io);
        let (r, c) = self.rc(cell);
        match io.edge() {
            Dir::W => {
                self.name_bel(bcrd, [format!("PL{r}{abcd}"), format!("IOLOGICL{r}{abcd}")]);
            }
            Dir::E => {
                self.name_bel(bcrd, [format!("PR{r}{abcd}"), format!("IOLOGICR{r}{abcd}")]);
            }
            Dir::S => {
                self.name_bel(bcrd, [format!("PB{c}{abcd}"), format!("IOLOGICB{c}{abcd}")]);
            }
            Dir::N => {
                self.name_bel(bcrd, [format!("PT{c}{abcd}"), format!("IOLOGICT{c}{abcd}")]);
            }
        }
        let iol = if idx.is_multiple_of(2) && io.edge() != Dir::N {
            "IOLOGICE"
        } else {
            "IOLOGIC"
        };
        let mut bel = Bel::default();
        let mut pins = vec![
            "LSR", "CE", "CLK", "OPOS0", "OPOS1", "OPOS2", "OPOS3", "ONEG0", "ONEG1", "ONEG2",
            "ONEG3", "IPOS0", "IPOS1", "IPOS2", "IPOS3", "INEG0", "INEG1", "INEG2", "INEG3",
            "INFF", "UP",
        ];
        if iol == "IOLOGICE" {
            pins.extend(["LOCK", "RUNAIL"]);
        }
        for pin in pins {
            let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_{iol}"));
            self.add_bel_wire(bcrd, pin, wire);
            bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
        }

        let paddi_pio = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDI_PIO", paddi_pio);
        let paddi = self.rc_io_wire(cell, &format!("JPADDI{abcd}"));
        self.add_bel_wire(bcrd, "PADDI", paddi);
        self.claim_pip(paddi, paddi_pio);
        let di_iol = self.rc_io_wire(cell, &format!("DI{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "DI_IOLOGIC", di_iol);
        self.claim_pip(di_iol, paddi_pio);
        let indd_iol = self.rc_io_wire(cell, &format!("JINDD{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "INDD_IOLOGIC", indd_iol);
        let indd = self.rc_io_wire(cell, &format!("JINDD{abcd}"));
        self.add_bel_wire(bcrd, "INDD", indd);
        self.claim_pip(indd, indd_iol);
        let inddck = self.rc_io_wire(cell, &format!("JINDDCK{abcd}"));
        self.add_bel_wire(bcrd, "INDDCK", inddck);
        self.claim_pip(inddck, paddi);
        self.claim_pip(inddck, indd);
        bel.pins
            .insert("INDDCK".into(), self.xlat_int_wire(bcrd, inddck));

        let paddo = self.rc_io_wire(cell, &format!("JPADDO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDO", paddo);
        let bpin = self.xlat_int_wire(bcrd, paddo);
        assert_eq!(bel.pins["OPOS0"], bpin);

        let paddt = self.rc_io_wire(cell, &format!("JPADDT{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDT", paddt);
        let td_iol = self.rc_io_wire(cell, &format!("JTD{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "TD_IOLOGIC", td_iol);
        let td_int = self.rc_io_wire(cell, &format!("JTD_{abcd}_INT"));
        self.add_bel_wire(bcrd, "TD_INT", td_int);
        self.claim_pip(paddt, td_int);
        self.claim_pip(td_iol, td_int);
        let td = self.rc_io_wire(cell, &format!("JTD_{abcd}"));
        self.add_bel_wire(bcrd, "TD", td);
        self.claim_pip(td_int, td);
        let mut bpin = self.xlat_int_wire(bcrd, td);
        if io.edge() != Dir::N {
            for w in ["IO_T_W", "IO_T_E"] {
                let wire = TileWireCoord::new_idx(0, self.intdb.get_wire(w));
                bpin.wires.insert(wire);
                let tcrd = self.edev.get_tile_by_bel(bcrd);
                let wire = self.io_int_names[&self.edev.tile_wire(tcrd, wire)];
                self.claim_pip(td_int, wire);
            }
        }
        bel.pins.insert("TD".into(), bpin);

        let ioldo_pio = self.rc_io_wire(cell, &format!("IOLDO{abcd}_PIO"));
        let ioldo_iol = self.rc_io_wire(cell, &format!("IOLDO{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLDO_PIO", ioldo_pio);
        self.add_bel_wire(bcrd, "IOLDO_IOLOGIC", ioldo_iol);
        self.claim_pip(ioldo_pio, ioldo_iol);

        let iolto_pio = self.rc_io_wire(cell, &format!("IOLTO{abcd}_PIO"));
        let iolto_iol = self.rc_io_wire(cell, &format!("IOLTO{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLTO_PIO", iolto_pio);
        self.add_bel_wire(bcrd, "IOLTO_IOLOGIC", iolto_iol);
        self.claim_pip(iolto_pio, iolto_iol);

        for pin in ["ECLK", "ELSR"] {
            let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_{iol}"));
            self.add_bel_wire(bcrd, pin, wire);
            let wire_in = self.rc_io_wire(cell, &format!("J{pin}{abcd}"));
            self.add_bel_wire(bcrd, format!("{pin}_IN"), wire_in);
            self.claim_pip(wire, wire_in);

            let bcrd_eclk = self.chip.bel_eclk_root_bank(bank);
            for i in 0..8 {
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, &format!("ECLK{i}"));
                self.claim_pip(wire_in, wire_eclk);
            }
        }

        let (hv, edge) = match bank {
            1 => (DirHV::NE, 'H'),
            2 => (DirHV::NE, 'V'),
            3 => (DirHV::SE, 'V'),
            4 => (DirHV::SE, 'H'),
            5 => (DirHV::SW, 'H'),
            6 => (DirHV::SW, 'V'),
            7 => (DirHV::NW, 'V'),
            _ => unreachable!(),
        };
        let cell_dll_dcntl = self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(hv, 0))];
        for i in 0..9 {
            let wire = self.rc_io_wire(cell, &format!("JDCNTL{i}{abcd}_{iol}"));
            self.add_bel_wire(bcrd, format!("DCNTL{i}"), wire);
            let wire_in = self.rc_io_wire(cell, &format!("JDCNTL{i}{abcd}"));
            self.add_bel_wire(bcrd, format!("DCNTL{i}_IN"), wire_in);
            self.claim_pip(wire, wire_in);
            for j in 0..2 {
                let bel_dll_dcntl = cell_dll_dcntl.bel(bels::DLL_DCNTL[j]);
                let wire_dll = self
                    .naming
                    .bel_wire(bel_dll_dcntl, &format!("DCNTL_{edge}{i}_OUT"));
                self.claim_pip(wire_in, wire_dll);
            }
        }

        self.insert_bel(bcrd, bel);
    }

    fn process_pictest_scm(&mut self, bcrd: BelCoord) {
        let idx = bels::PICTEST
            .iter()
            .position(|&slot| slot == bcrd.slot)
            .unwrap();
        let edge = if bcrd.col == self.chip.col_w() {
            Dir::W
        } else if bcrd.col == self.chip.col_e() {
            Dir::E
        } else if bcrd.row == self.chip.row_s() {
            Dir::S
        } else if bcrd.row == self.chip.row_n() {
            Dir::N
        } else {
            unreachable!()
        };
        let cell = match edge {
            Dir::H(edge) => {
                let io_kind = match edge {
                    DirH::W => self.chip.rows[bcrd.row].io_w,
                    DirH::E => self.chip.rows[bcrd.row].io_e,
                };
                match io_kind {
                    IoGroupKind::Quad => bcrd.cell,
                    IoGroupKind::Dozen => bcrd.cell.delta(0, 2 - idx as i32),
                    _ => unreachable!(),
                }
            }
            Dir::V(_) => bcrd.cell.delta(idx as i32, 0),
        };
        let (r, c) = self.rc(cell);
        match edge {
            Dir::W => {
                self.name_bel(bcrd, [format!("LPICTEST{r}")]);
            }
            Dir::E => {
                self.name_bel(bcrd, [format!("RPICTEST{r}")]);
            }
            Dir::S => {
                self.name_bel(bcrd, [format!("BPICTEST{c}")]);
            }
            Dir::N => {
                self.name_bel(bcrd, [format!("TPICTEST{c}")]);
            }
        }
        self.insert_simple_bel(bcrd, cell, "PICTEST");
    }

    pub(super) fn process_io_scm(&mut self) {
        for (tcname, num_io) in [
            ("IO_W4", 4),
            ("IO_W12", 12),
            ("IO_E4", 4),
            ("IO_E12", 12),
            ("IO_S4", 4),
            ("IO_S12", 12),
            ("IO_N4", 4),
            ("IO_N8", 8),
            ("IO_N12", 12),
        ] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                for i in 0..num_io {
                    let bcrd = tcrd.bel(bels::IO[i]);
                    self.process_single_io_scm(bcrd);
                }
                for i in 0..num_io / 4 {
                    let bcrd = tcrd.bel(bels::PICTEST[i]);
                    self.process_pictest_scm(bcrd);
                }
            }
        }
    }
}

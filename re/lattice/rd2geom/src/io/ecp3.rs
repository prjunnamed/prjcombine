use prjcombine_ecp::{
    bels,
    chip::{ChipKind, IoGroupKind, IoKind, PllLoc, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::Bel,
    dir::{Dir, DirH, DirHV, DirV},
    grid::{BelCoord, EdgeIoCoord, TileIobId},
};
use prjcombine_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_single_io_ecp3(&mut self, bcrd: BelCoord) {
        let io = self.chip.get_io_crd(bcrd);
        let cell;
        let abcd;
        match io {
            EdgeIoCoord::W(_, iob) => {
                if iob.to_idx() < 2 {
                    cell = bcrd.cell.delta(0, 2);
                    abcd = ["A", "B"][iob.to_idx()];
                    let (r, _) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PL{r}{abcd}"), format!("IOL_L{r}{abcd}")]);
                } else if iob.to_idx() < 4 {
                    cell = bcrd.cell;
                    abcd = ["A", "B"][iob.to_idx() - 2];
                    let (r, _) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PL{r}{abcd}"), format!("IOL_L{r}{abcd}")]);
                } else {
                    cell = bcrd.cell.delta(-1, 0);
                    abcd = ["EA", "EB", "EC", "ED"][iob.to_idx() - 4];
                    let e_abcd = ["E_A", "E_B", "E_C", "E_D"][iob.to_idx() - 4];
                    let (r, _) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PL{r}{e_abcd}"), format!("IOL_L{r}{abcd}")]);
                }
            }
            EdgeIoCoord::E(_, iob) => {
                if iob.to_idx() < 2 {
                    cell = bcrd.cell.delta(0, 2);
                    abcd = ["A", "B"][iob.to_idx()];
                    let (r, _) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PR{r}{abcd}"), format!("IOL_R{r}{abcd}")]);
                } else if iob.to_idx() < 4 {
                    cell = bcrd.cell;
                    abcd = ["A", "B"][iob.to_idx() - 2];
                    let (r, _) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PR{r}{abcd}"), format!("IOL_R{r}{abcd}")]);
                } else {
                    cell = bcrd.cell.delta(1, 0);
                    abcd = ["EA", "EB", "EC", "ED"][iob.to_idx() - 4];
                    let e_abcd = ["E_A", "E_B", "E_C", "E_D"][iob.to_idx() - 4];
                    let (r, _) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PR{r}{e_abcd}"), format!("IOL_R{r}{abcd}")]);
                }
            }
            EdgeIoCoord::S(_, iob) => {
                if iob.to_idx() < 2 {
                    cell = bcrd.cell;
                    abcd = ["A", "B"][iob.to_idx()];
                    let (_, c) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PB{c}{abcd}"), format!("IOL_B{c}{abcd}")]);
                } else {
                    cell = bcrd.cell.delta(2, 0);
                    abcd = ["A", "B"][iob.to_idx() - 2];
                    let (_, c) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PB{c}{abcd}"), format!("IOL_B{c}{abcd}")]);
                }
            }
            EdgeIoCoord::N(_, iob) => {
                if iob.to_idx() < 2 {
                    cell = bcrd.cell;
                    abcd = ["A", "B"][iob.to_idx()];
                    let (_, c) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PT{c}{abcd}"), format!("IOL_T{c}{abcd}")]);
                } else {
                    cell = bcrd.cell.delta(2, 0);
                    abcd = ["A", "B"][iob.to_idx() - 2];
                    let (_, c) = self.rc(cell);
                    self.name_bel(bcrd, [format!("PT{c}{abcd}"), format!("IOL_T{c}{abcd}")]);
                }
            }
        }
        let kind = self.chip.get_io_kind(io);
        let (iol, iol_alt) = match kind {
            IoKind::Dummy => unreachable!(),
            IoKind::Io => ("IOLOGIC", "IOLOGIC"),
            IoKind::Sio => ("SIOLOGIC", "SIOLOGIC"),
            IoKind::Xsio => ("XSIOLOGIC", "XSIOLOGIC"),
            IoKind::IoPll => ("IOLOGIC", "XSIOLOGIC"),
            IoKind::Dqs => ("DQSIOL", "DQSIOL"),
            IoKind::SDqs => ("SDQSIOL", "SDQSIOL"),
        };

        let mut bel = Bel::default();

        let paddi = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDI", paddi);
        let paddo = self.rc_io_wire(cell, &format!("JPADDO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDO", paddo);
        let paddt = self.rc_io_wire(cell, &format!("JPADDT{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDT", paddt);

        let di = self.rc_io_wire(cell, &format!("DI{abcd}_{iol_alt}"));
        self.add_bel_wire(bcrd, "DI", di);
        self.claim_pip(di, paddi);

        let ioldo_pio = self.rc_io_wire(cell, &format!("IOLDO{abcd}_PIO"));
        let ioldo_iol = self.rc_io_wire(cell, &format!("IOLDO{abcd}_{iol_alt}"));
        self.add_bel_wire(bcrd, "IOLDO_PIO", ioldo_pio);
        self.add_bel_wire(bcrd, "IOLDO_IOLOGIC", ioldo_iol);
        self.claim_pip(ioldo_pio, ioldo_iol);

        let iolto_pio = self.rc_io_wire(cell, &format!("IOLTO{abcd}_PIO"));
        let iolto_iol = self.rc_io_wire(cell, &format!("IOLTO{abcd}_{iol_alt}"));
        self.add_bel_wire(bcrd, "IOLTO_PIO", iolto_pio);
        self.add_bel_wire(bcrd, "IOLTO_IOLOGIC", iolto_iol);
        self.claim_pip(iolto_pio, iolto_iol);

        let mut pins = vec![
            "LSR", "CE", "CLK", "INB", "INDD", "DEL0", "DEL1", "DEL2", "DEL3",
        ];

        if kind != IoKind::IoPll {
            pins.extend(["OPOSA", "TS"]);
        }

        if !matches!(kind, IoKind::IoPll | IoKind::Xsio) {
            pins.extend(["ONEGB"]);
        }

        if matches!(kind, IoKind::Io | IoKind::Dqs) {
            pins.extend(["OPOSB"]);
        }
        if kind == IoKind::Io {
            pins.extend(["ONEGA"]);
        }

        if kind != IoKind::Xsio {
            pins.extend(["IPA", "IPB", "INA"]);
        }

        for pin in pins {
            let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_{iol}"));
            self.add_bel_wire(bcrd, pin, wire);
            bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
        }

        {
            let bpin = self.xlat_int_wire(bcrd, paddi);
            assert_eq!(bpin, bel.pins["INDD"]);
        }

        if kind == IoKind::IoPll {
            for pin in ["OPOSA", "OPOSB", "ONEGA", "ONEGB", "TS"] {
                let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_{iol}"));
                self.add_bel_wire(bcrd, pin, wire);
            }
        } else {
            let bpin = self.xlat_int_wire(bcrd, paddo);
            assert_eq!(bpin, bel.pins["OPOSA"]);
            let bpin = self.xlat_int_wire(bcrd, paddt);
            assert_eq!(bpin, bel.pins["TS"]);
        }

        if matches!(kind, IoKind::Dqs | IoKind::SDqs) {
            let dqsw = self.rc_io_wire(cell, &format!("JDQSW{abcd}_{iol}"));
            self.add_bel_wire(bcrd, "DQSW", dqsw);
            let bcrd_dqs = bcrd.bel(bels::DQS0);
            let dqsw_dqs = self.naming.bel_wire(bcrd_dqs, "DQSW");
            self.claim_pip(dqsw, dqsw_dqs);

            let dqstclko = self.rc_io_wire(cell, &format!("DQSTCLKO{abcd}_{iol}"));
            self.add_bel_wire(bcrd, "DQSTCLKO", dqstclko);
            let dqstclki = self.rc_io_wire(cell, &format!("DQSTCLKI{abcd}_{iol}"));
            self.add_bel_wire(bcrd, "DQSTCLKI", dqstclki);
            self.claim_pip(dqstclki, dqstclko);
        }

        if kind != IoKind::Xsio {
            let bcrd_dqs = self.edev.dqs[&bcrd.cell].bel(bels::DQS0);
            let mut pins = vec!["DQCLK1", "DDRCLKPOL", "DDRLAT"];
            if self.chip.kind == ChipKind::Ecp3A {
                pins.push("ECLKDQSR");
            }
            if matches!(kind, IoKind::Io | IoKind::Dqs | IoKind::IoPll) {
                pins.push("DQCLK0");
            }
            for pin in pins {
                let wire = self.rc_io_wire(cell, &format!("{pin}{abcd}_{iol}"));
                self.add_bel_wire(bcrd, pin, wire);
                let wire_dqs = self.naming.bel_wire(bcrd_dqs, &format!("{pin}_OUT"));
                self.claim_pip(wire, wire_dqs);
            }

            if self.chip.kind != ChipKind::Ecp3A {
                let wire = self.rc_io_wire(cell, &format!("ECLKDQSR{abcd}_{iol}"));
                self.add_bel_wire(bcrd, "ECLKDQSR", wire);
                let wire_in = self.rc_io_wire(cell, &format!("ECLKDQSR{abcd}"));
                self.add_bel_wire(bcrd, "ECLKDQSR_IN", wire_in);
                self.claim_pip(wire, wire_in);
                let wire_dqs = self.naming.bel_wire(bcrd_dqs, "ECLKDQSR_OUT");
                self.claim_pip(wire_in, wire_dqs);

                let bcrd_eclk = self.chip.bel_eclksync(io.edge(), 1);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKO_OUT");
                self.claim_pip(wire_in, wire_eclk);
            }

            let eclk = self.rc_io_wire(cell, &format!("ECLK{abcd}_{iol}"));
            self.add_bel_wire(bcrd, "ECLK", eclk);
            let eclk_in = self.rc_io_wire(cell, &format!("ECLK{abcd}"));
            self.add_bel_wire(bcrd, "ECLK_IN", eclk_in);
            self.claim_pip(eclk, eclk_in);

            for i in 0..2 {
                let bcrd_eclk = self.chip.bel_eclksync(io.edge(), i);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKO_OUT");
                self.claim_pip(eclk_in, wire_eclk);
            }
        }

        if matches!(kind, IoKind::Sio | IoKind::SDqs) && self.chip.kind == ChipKind::Ecp3 {
            let dqclk1 = self.rc_io_wire(cell, &format!("DQCLK1{abcd}_{iol}"));
            let dqclk1_in = self.rc_io_wire(cell, &format!("JDQCLK1{abcd}"));
            self.claim_pip(dqclk1, dqclk1_in);
            self.add_bel_wire(bcrd, "DQCLK1_IN", dqclk1_in);
            let bpin = self.xlat_int_wire(bcrd, dqclk1_in);
            assert_eq!(bel.pins["CLK"], bpin);
        }

        self.insert_bel(bcrd, bel);
    }

    fn process_dqs_ecp3(&mut self, bcrd_io: BelCoord) {
        let io = self.chip.get_io_crd(bcrd_io);
        let bcrd_dqs = bcrd_io.bel(bels::DQS0);
        let kind = self.chip.get_io_kind(io);
        let cell = match io.edge() {
            Dir::H(_) => bcrd_dqs.cell.delta(0, 1),
            Dir::V(_) => bcrd_dqs.cell.delta(1, 0),
        };
        let (r, c) = self.rc(cell);
        match io.edge() {
            Dir::W => self.name_bel(bcrd_dqs, [format!("DQS_L{r}")]),
            Dir::E => self.name_bel(bcrd_dqs, [format!("DQS_R{r}")]),
            Dir::N => self.name_bel(bcrd_dqs, [format!("DQS_T{c}")]),
            _ => unreachable!(),
        }
        let suffix = match kind {
            IoKind::Io | IoKind::Dqs => "DQS",
            IoKind::SDqs => "SDQS",
            _ => unreachable!(),
        };
        let mut pins = vec![
            ("DQCLK1", "DQCLK1"),
            (
                "DDRLAT",
                if self.chip.kind == ChipKind::Ecp3A && kind != IoKind::SDqs {
                    "DDRLAT"
                } else {
                    "JDDRLAT"
                },
            ),
            ("DDRCLKPOL", "JDDRCLKPOL"),
            ("ECLKDQSR", "JECLKDQSR"),
        ];
        if kind != IoKind::SDqs {
            pins.extend([("DQCLK0", "DQCLK0")]);
        }
        for (pin, name) in pins {
            let wire = self.rc_io_wire(cell, &format!("{name}_{suffix}"));
            if !name.starts_with('J') {
                self.add_bel_wire(bcrd_dqs, pin, wire);
            }
            let wire_out = self.pips_fwd[&wire]
                .iter()
                .copied()
                .find(|w| !self.int_wires.contains_key(w))
                .unwrap();
            self.claim_pip(wire_out, wire);
            self.add_bel_wire(bcrd_dqs, format!("{pin}_OUT"), wire_out);
        }
        for pin in ["ECLK", "ECLKW"] {
            let wire = self.rc_io_wire(cell, &format!("{pin}_{suffix}"));
            self.add_bel_wire(bcrd_dqs, pin, wire);
            let wire_in = self.rc_io_wire(cell, pin);
            self.add_bel_wire(bcrd_dqs, format!("{pin}_IN"), wire_in);
            self.claim_pip(wire, wire_in);

            for i in 0..2 {
                let bcrd_eclk = self.chip.bel_eclksync(io.edge(), i);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKO_OUT");
                self.claim_pip(wire_in, wire_eclk);
            }
        }
        let dqsi = self.rc_io_wire(cell, &format!("JDQSI_{suffix}"));
        self.add_bel_wire(bcrd_dqs, "DQSI", dqsi);
        if kind != IoKind::Io {
            let wire_io = self.get_io_wire_in(io);
            self.claim_pip(dqsi, wire_io);
        }
        let dqsdel = self.rc_io_wire(cell, &format!("JDQSDEL_{suffix}"));
        self.add_bel_wire(bcrd_dqs, "DQSDEL", dqsdel);
        let bcrd_dqsdll = self.chip.bel_dqsdll(cell);
        let dqsdel_dqsdll = self.naming.bel_wire(bcrd_dqsdll, "DQSDEL");
        self.claim_pip(dqsdel, dqsdel_dqsdll);

        self.insert_simple_bel(bcrd_dqs, cell, suffix);
        if matches!(io.edge(), Dir::H(_)) && self.chip.kind == ChipKind::Ecp3A {
            let bcrd_dqstest = bcrd_io.bel(bels::DQSTEST);
            match io.edge() {
                Dir::W => self.name_bel(bcrd_dqstest, [format!("DQSTEST_L{r}")]),
                Dir::E => self.name_bel(bcrd_dqstest, [format!("DQSTEST_R{r}")]),
                _ => unreachable!(),
            }
            self.insert_simple_bel(bcrd_dqstest, cell, "DQSTEST");
        }
    }

    pub(super) fn process_io_ecp3(&mut self) {
        // DQS
        for (row, rd) in &self.chip.rows {
            match rd.io_w {
                IoGroupKind::QuadDqs | IoGroupKind::QuadDqsDummy => {
                    let io = EdgeIoCoord::W(row, TileIobId::from_idx(2));
                    let bcrd = self.chip.get_io_loc(io);
                    self.process_dqs_ecp3(bcrd);
                }
                _ => (),
            }
            match rd.io_e {
                IoGroupKind::QuadDqs | IoGroupKind::QuadDqsDummy => {
                    let io = EdgeIoCoord::E(row, TileIobId::from_idx(2));
                    let bcrd = self.chip.get_io_loc(io);
                    self.process_dqs_ecp3(bcrd);
                }
                _ => (),
            }
        }
        for (col, cd) in &self.chip.columns {
            match cd.io_n {
                IoGroupKind::QuadDqs | IoGroupKind::QuadDqsDummy => {
                    let io = EdgeIoCoord::N(col, TileIobId::from_idx(2));
                    let bcrd = self.chip.get_io_loc(io);
                    self.process_dqs_ecp3(bcrd);
                }
                _ => (),
            }
        }
        // actual IO
        for (row, rd) in &self.chip.rows {
            match rd.io_w {
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadDqsDummy => {
                    for iob in 0..4 {
                        let io = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                        let bcrd = self.chip.get_io_loc(io);
                        self.process_single_io_ecp3(bcrd);
                    }
                }
                _ => (),
            }
            match rd.io_e {
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadDqsDummy => {
                    for iob in 0..4 {
                        let io = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                        let bcrd = self.chip.get_io_loc(io);
                        self.process_single_io_ecp3(bcrd);
                    }
                }
                _ => (),
            }
        }
        for (col, cd) in &self.chip.columns {
            match cd.io_s {
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadDqsDummy => {
                    for iob in 0..4 {
                        let io = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                        let bcrd = self.chip.get_io_loc(io);
                        self.process_single_io_ecp3(bcrd);
                    }
                }
                _ => (),
            }
            match cd.io_n {
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadDqsDummy => {
                    for iob in 0..4 {
                        let io = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                        let bcrd = self.chip.get_io_loc(io);
                        self.process_single_io_ecp3(bcrd);
                    }
                }
                _ => (),
            }
        }
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            for iob in 4..8 {
                let io = match loc.quad.h {
                    DirH::W => EdgeIoCoord::W(cell.row, TileIobId::from_idx(iob)),
                    DirH::E => EdgeIoCoord::E(cell.row, TileIobId::from_idx(iob)),
                };
                let bcrd = self.chip.get_io_loc(io);
                self.process_single_io_ecp3(bcrd);
            }
        }
    }

    pub(super) fn process_eclk_ecp3(&mut self) {
        for (edge, lrt) in [(Dir::W, 'L'), (Dir::E, 'R'), (Dir::N, 'T')] {
            for idx in 0..2 {
                let idxp1 = idx + 1;
                let bcrd = self.chip.bel_eclksync(edge, idx);
                let cell = bcrd.cell;
                self.name_bel(bcrd, [format!("{lrt}ECLKSYNC{idxp1}")]);
                let mut bel = Bel::default();

                let stop = self.rc_wire(cell, &format!("JSTOP{idxp1}_ECLKSYNC"));
                self.add_bel_wire(bcrd, "STOP", stop);
                bel.pins
                    .insert("STOP".into(), self.xlat_int_wire(bcrd, stop));

                let eclko = self.rc_wire(cell, &format!("JECLKO{idxp1}_ECLKSYNC"));
                self.add_bel_wire(bcrd, "ECLKO", eclko);

                let eclk_name = format!("{lrt}ECLK{idxp1}");
                let eclko_out = self.pips_fwd[&eclko]
                    .iter()
                    .copied()
                    .find(|wn| self.naming.strings[wn.suffix].starts_with(&eclk_name))
                    .unwrap();
                self.claim_pip(eclko_out, eclko);
                self.add_bel_wire(bcrd, "ECLKO_OUT", eclko_out);

                let eclki = self.rc_wire(cell, &format!("ECLKI{idxp1}_ECLKSYNC"));
                self.add_bel_wire(bcrd, "ECLKI", eclki);
                self.claim_pip(eclko, eclki);

                let eclki_in = self.claim_single_in(eclki);
                self.add_bel_wire(bcrd, "ECLKI_IN", eclki_in);

                let eclki_io = self.rc_wire(cell, &format!("JPIO{idxp1}"));
                self.add_bel_wire(bcrd, "ECLKI_IO", eclki_io);
                self.claim_pip(eclki_in, eclki_io);
                let wire_io = self.get_special_io_wire_in(SpecialIoKey::Clock(edge, idx as u8));
                self.claim_pip(eclki_io, wire_io);

                let eclki_int = self.rc_wire(cell, &format!("JCIBCLK{idxp1}"));
                self.add_bel_wire(bcrd, "ECLKI_INT", eclki_int);
                self.claim_pip(eclki_in, eclki_int);
                bel.pins
                    .insert("ECLKI".into(), self.xlat_int_wire(bcrd, eclki_int));

                if self.chip.kind == ChipKind::Ecp3A {
                    let eclki_center = self.rc_wire(cell, &format!("JBRGECLK{idxp1}"));
                    self.add_bel_wire(bcrd, "ECLKI_CENTER", eclki_center);
                    self.claim_pip(eclki_in, eclki_center);
                    let wire_center = self.rc_wire(
                        self.chip.bel_clk_root().delta(-1, 0),
                        &format!("JBRGECLK{idxp1}"),
                    );
                    self.claim_pip(eclki_center, wire_center);
                }

                if edge == Dir::N {
                    let cell_dll_w =
                        self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(DirHV::NW, 0))];
                    let cell_dll_e =
                        self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(DirHV::NE, 0))];
                    let cell_pll_w = self
                        .chip
                        .special_loc
                        .get(&SpecialLocKey::Pll(PllLoc::new(DirHV::NW, 1)))
                        .copied()
                        .unwrap_or(cell_dll_w)
                        .delta(3, 0);
                    let cell_pll_e = self
                        .chip
                        .special_loc
                        .get(&SpecialLocKey::Pll(PllLoc::new(DirHV::NE, 1)))
                        .copied()
                        .unwrap_or(cell_dll_e)
                        .delta(-3, 0);
                    let cell_dll_w = cell_dll_w.delta(13, 0);
                    let cell_dll_e = cell_dll_e.delta(-13, 0);
                    for (tgt, pin, name, cell_src, name_src) in [
                        (
                            0,
                            "ECLKI_PLL_W_CLKOP",
                            "JPLLCLKOP1",
                            cell_pll_w,
                            "JCLKOP_PLL",
                        ),
                        (
                            1,
                            "ECLKI_PLL_E_CLKOP",
                            "JPLLCLKOP2",
                            cell_pll_e,
                            "JCLKOP_PLL",
                        ),
                        (
                            1,
                            "ECLKI_PLL_W_CLKOS",
                            "JPLLCLKOS2",
                            cell_pll_w,
                            "JCLKOS_PLL",
                        ),
                        (
                            0,
                            "ECLKI_PLL_E_CLKOS",
                            "JPLLCLKOS1",
                            cell_pll_e,
                            "JCLKOS_PLL",
                        ),
                        (
                            1,
                            "ECLKI_DLL_W_CLKOS",
                            "JDLLCLKOS2",
                            cell_dll_w,
                            "JCLKOS_DLL",
                        ),
                        (
                            0,
                            "ECLKI_DLL_E_CLKOS",
                            "JDLLCLKOS1",
                            cell_dll_e,
                            "JCLKOS_DLL",
                        ),
                        (0, "ECLKI_DLLDEL_W", "JDLLDEL1", cell_dll_w, "JCLKO_DLLDEL"),
                        (1, "ECLKI_DLLDEL_E", "JDLLDEL2", cell_dll_e, "JCLKO_DLLDEL"),
                    ] {
                        if tgt != idx {
                            continue;
                        }
                        let eclki_pll = self.rc_wire(cell, name);
                        self.add_bel_wire(bcrd, pin, eclki_pll);
                        self.claim_pip(eclki_in, eclki_pll);
                        let wire = self.rc_wire(cell_src, name_src);
                        self.claim_pip(eclki_pll, wire);
                    }
                } else {
                    let Dir::H(edge) = edge else {
                        unreachable!();
                    };
                    let eclki_dll = self.rc_wire(cell, &format!("JDLLECLK{idxp1}"));
                    self.add_bel_wire(bcrd, "ECLKI_DLL", eclki_dll);
                    self.claim_pip(eclki_in, eclki_dll);
                    let cell_dll = match edge {
                        DirH::W => cell.delta(14, 0),
                        DirH::E => cell.delta(-14, 0),
                    };
                    let wire_dll = self.rc_wire(cell_dll, &format!("JDLLECLK{idxp1}"));
                    self.claim_pip(eclki_dll, wire_dll);

                    if self.chip.kind == ChipKind::Ecp3A
                        && let Some(cell_pll) = self
                            .chip
                            .special_loc
                            .get(&SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::S, 0)))
                    {
                        let cell_pll = match edge {
                            DirH::W => cell_pll.delta(3, 0),
                            DirH::E => cell_pll.delta(-3, 0),
                        };
                        let pin = ["CLKOS", "CLKOP"][idx];
                        let eclki_pll = self.rc_wire(cell, &format!("JPLL{pin}"));
                        self.add_bel_wire(bcrd, format!("ECLKI_PLL_{pin}"), eclki_pll);
                        self.claim_pip(eclki_in, eclki_pll);
                        let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                        self.claim_pip(eclki_pll, wire_pll);
                    }
                }

                self.insert_bel(bcrd, bel);
            }
        }
    }

    pub(super) fn process_dqsdll_ecp3(&mut self) {
        for edge in [DirH::W, DirH::E] {
            let bcrd = self.chip.bel_dqsdll_ecp2(edge);
            let cell = match edge {
                DirH::W => bcrd.cell.delta(2, 0),
                DirH::E => bcrd.cell.delta(-2, 0),
            };
            self.name_bel(
                bcrd,
                [match edge {
                    DirH::W => "LDQSDLL",
                    DirH::E => "RDQSDLL",
                }],
            );
            let mut bel = self.extract_simple_bel(bcrd, cell, "DQSDLL");
            let dqsdel = self.rc_wire(cell, "JDQSDEL_DQSDLL");
            self.add_bel_wire(bcrd, "DQSDEL", dqsdel);
            if self.chip.kind == ChipKind::Ecp3A {
                let clk = self.rc_wire(cell, "CLK_DQSDLL");
                self.add_bel_wire(bcrd, "CLK", clk);

                let clk_in = self.rc_wire(cell, "DQSDLLCLK");
                self.add_bel_wire(bcrd, "CLK_IN", clk_in);
                self.claim_pip(clk, clk_in);

                let clk_int = self.rc_wire(cell, "JCIBCLK");
                self.add_bel_wire(bcrd, "CLK_INT", clk_int);
                self.claim_pip(clk_in, clk_int);
                bel.pins
                    .insert("CLK".into(), self.xlat_int_wire(bcrd, clk_int));

                let clk_eclk = self.rc_wire(cell, "JECLK");
                self.add_bel_wire(bcrd, "CLK_ECLK", clk_eclk);
                self.claim_pip(clk_in, clk_eclk);
                let bcrd_eclk = self.chip.bel_eclksync(Dir::H(edge), 0);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKO");
                self.claim_pip(clk_eclk, wire_eclk);
            }
            self.insert_bel(bcrd, bel);

            let bcrd = bcrd.bel(bels::DQSDLLTEST);
            self.name_bel(
                bcrd,
                [match edge {
                    DirH::W => "LDQSDLLTEST",
                    DirH::E => "RDQSDLLTEST",
                }],
            );
            self.insert_simple_bel(bcrd, cell, "DQSDLLTEST");
        }
    }

    pub(super) fn process_eclk_tap_ecp3(&mut self) {
        let tcid = self.intdb.get_tile_class("ECLK_TAP");
        for &tcrd in &self.edev.tile_index[tcid] {
            let bcrd = tcrd.bel(bels::ECLK_TAP);
            let edge = if tcrd.col == self.chip.col_w() {
                Dir::W
            } else if tcrd.col == self.chip.col_e() {
                Dir::E
            } else if tcrd.row == self.chip.row_n() {
                Dir::N
            } else {
                unreachable!()
            };
            for i in 0..2 {
                let bel_eclksync = self.chip.bel_eclksync(edge, i);
                let eclk_out = self.edev.get_bel_pin(bcrd, &format!("ECLK{i}"))[0];
                let eclk = self.naming.bel_wire(bel_eclksync, "ECLKO_OUT");
                self.claim_pip_int_out(eclk_out, eclk);
            }
        }
    }
}

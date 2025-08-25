use prjcombine_ecp::{
    bels,
    chip::{IoKind, PllLoc, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::Bel,
    dir::{Dir, DirHV},
    grid::{BelCoord, CellCoord, ColId, DieId, EdgeIoCoord},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_bc_crosslink(&mut self) {
        for (&key, &cell_loc) in &self.chip.special_loc {
            let SpecialLocKey::Bc(bank) = key else {
                continue;
            };
            let (cell, is_corner) = if bank == 2 {
                (cell_loc.with_col(self.chip.col_w()), true)
            } else {
                (cell_loc, false)
            };
            for (bel, name, pin_out, pin_in, name_fake) in [
                (bels::BCINRD, "BCINRD", "INRDENO", "INRDENI", "INRD"),
                (bels::BCLVDSO, "BCLVDSO", "LVDSENO", "LVDSENI", "LVDS"),
            ] {
                let bcrd = cell_loc.bel(bel);
                self.name_bel(bcrd, [format!("{name}{bank}")]);

                let wire_in = if is_corner {
                    self.rc_corner_wire(cell, &format!("J{pin_in}_{name}"))
                } else {
                    self.rc_io_wire(cell, &format!("J{pin_in}_{name}"))
                };
                self.add_bel_wire(bcrd, pin_in, wire_in);
                let mut bel = Bel::default();
                bel.pins
                    .insert(pin_in.into(), self.xlat_int_wire(bcrd, wire_in));
                self.insert_bel(bcrd, bel);

                let wire_out = if is_corner {
                    self.rc_corner_wire(cell, &format!("{pin_out}_{name}"))
                } else {
                    self.rc_io_wire(cell, &format!("{pin_out}_{name}"))
                };
                self.add_bel_wire(bcrd, pin_out, wire_out);
                self.claim_pip(wire_out, wire_in);

                let wire_out_out = self.claim_single_out(wire_out);
                self.add_bel_wire(bcrd, format!("{pin_out}_OUT"), wire_out_out);

                if bank == 2 {
                    // uh. what. what the fuck.
                    let wire_fake = self.rc_io_wire(cell.with_col(ColId::from_idx(13)), name_fake);
                    self.add_bel_wire(bcrd, format!("{pin_out}_FAKE"), wire_fake);
                }
            }
        }
    }

    pub(super) fn process_eclk_crosslink(&mut self) {
        let cell_tile = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_s());
        let cell = cell_tile.delta(-1, 0);
        for (bank, bank_idx, lr, olr) in [(2, 0, 'L', 'R'), (1, 1, 'R', 'L')] {
            let mut eclki_bpin = None;
            for eclk_idx in 0..2 {
                let bcrd = cell_tile.bel(bels::ECLKSYNC[bank_idx * 2 + eclk_idx]);
                self.name_bel(bcrd, [format!("ECLKSYNC{eclk_idx}_BK{bank}")]);
                let mut bel = Bel::default();

                let stop = self.rc_io_wire(cell, &format!("JSTOP_ECLKSYNC{lr}{eclk_idx}"));
                self.add_bel_wire(bcrd, "STOP", stop);
                bel.pins
                    .insert("STOP".into(), self.xlat_int_wire(bcrd, stop));

                let eclki = self.rc_io_wire(cell, &format!("ECLKI_ECLKSYNC{lr}{eclk_idx}"));
                self.add_bel_wire(bcrd, "ECLKI", eclki);
                let eclki_in = self.claim_single_in(eclki);
                self.add_bel_wire(bcrd, "ECLKI_IN", eclki_in);

                let eclki_int = self.rc_io_wire(cell, &format!("J{lr}ECLKCIB"));
                self.add_bel_wire_no_claim(bcrd, "ECLKI_INT", eclki_int);
                self.claim_pip(eclki_in, eclki_int);
                if eclk_idx == 0 {
                    self.claim_node(eclki_int);
                    let bpin = self.xlat_int_wire(bcrd, eclki_int);
                    eclki_bpin = Some(bpin.clone());
                    bel.pins.insert("ECLKI".into(), bpin);
                } else {
                    bel.pins.insert("ECLKI".into(), eclki_bpin.take().unwrap());
                }

                let wire_eclkfb = self.rc_io_wire(cell, "JECLKFB");
                self.add_bel_wire_no_claim(bcrd, "ECLKFB", wire_eclkfb);
                if bank_idx == 0 && eclk_idx == 0 {
                    self.claim_node(wire_eclkfb);
                }

                for pin in ["CLKOP", "CLKOS"] {
                    let eclki_pll = self.rc_io_wire(cell, &format!("J{pin}"));
                    self.add_bel_wire_no_claim(bcrd, format!("PLL_IN_{pin}"), eclki_pll);
                    self.claim_pip(eclki_in, eclki_pll);
                    if bank_idx == 0 && eclk_idx == 0 {
                        self.claim_node(eclki_pll);
                        let cell_pll =
                            self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(DirHV::SE, 0))];
                        let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                        self.claim_pip(eclki_pll, wire_pll);
                        self.claim_pip(wire_eclkfb, eclki_pll);
                    }
                }

                for i in 0..4 {
                    let name = ["20", "21", "10", "11"][i];
                    let pclk = self.rc_io_wire(cell, &format!("JPCLK{name}"));
                    self.add_bel_wire_no_claim(bcrd, format!("IO_IN_S{i}"), pclk);
                    self.claim_pip(eclki_in, pclk);
                    if bank_idx == 0 && eclk_idx == 0 {
                        self.claim_node(pclk);
                        let cell_dlldel = bcrd.cell.delta(-2 + (i as i32), 0);
                        let wire_dlldel = self.rc_io_wire(cell_dlldel, "JINCK");
                        self.claim_pip(pclk, wire_dlldel);
                    }
                }

                let eclko = self.rc_io_wire(cell, &format!("ECLKO_ECLKSYNC{lr}{eclk_idx}"));
                self.add_bel_wire(bcrd, "ELCKO", eclko);
                self.claim_pip(eclko, eclki);

                let eclko_out = self.rc_io_wire(cell, &format!("{lr}SYNCECLK{eclk_idx}"));
                self.add_bel_wire(bcrd, "ELCKO_OUT", eclko_out);
                self.claim_pip(eclko_out, eclko);

                let eclk_mux = self.rc_io_wire(cell, &format!("JBANK{bank}ECLK{eclk_idx}"));
                self.add_bel_wire(bcrd, "ECLK_MUX", eclk_mux);
                self.claim_pip(eclk_mux, eclko_out);
                let eclko_other = self.rc_io_wire(cell, &format!("{olr}SYNCECLK{eclk_idx}"));
                self.claim_pip(eclk_mux, eclko_other);
                bel.pins
                    .insert("ECLK".into(), self.xlat_int_wire(bcrd, eclk_mux));

                let eclk = self.pips_fwd[&eclk_mux]
                    .iter()
                    .copied()
                    .find(|wn| self.naming.strings[wn.suffix].starts_with("BANK"))
                    .unwrap();
                self.add_bel_wire(bcrd, "ECLK", eclk);
                self.claim_pip(eclk, eclk_mux);

                self.insert_bel(bcrd, bel);
            }
        }

        {
            let bcrd = cell_tile.bel(bels::CLKTEST_ECLK);
            self.name_bel(bcrd, ["CLKTEST_ECLK"]);
            let mut bel = Bel::default();

            for i in 0..6 {
                let wire = self.rc_io_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                bel.pins
                    .insert(format!("TESTIN{i}"), self.xlat_int_wire(bcrd, wire));
            }

            self.insert_bel(bcrd, bel);
        }
    }

    pub(super) fn process_ddrdll_crosslink(&mut self) {
        for (&key, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Bc(bank) = key else {
                continue;
            };
            let bcrd = cell.bel(bels::DDRDLL);
            self.name_bel(bcrd, [format!("DDRDLL{bank}")]);
            let mut bel = self.extract_simple_bel(bcrd, cell, "DDRDLL");

            let clk = self.rc_io_wire(cell, "CLK_DDRDLL");
            self.add_bel_wire(bcrd, "CLK", clk);
            let clk_in = self.claim_single_in(clk);
            self.add_bel_wire(bcrd, "CLK_IN", clk_in);

            let clk_int = self.rc_io_wire(cell, "JCIBCLK");
            self.add_bel_wire(bcrd, "CLK_INT", clk_int);
            self.claim_pip(clk_in, clk_int);
            bel.pins
                .insert("CLK".into(), self.xlat_int_wire(bcrd, clk_int));

            for i in 0..2 {
                let bcrd_eclk = self.chip.bel_eclksync_bank(bank, i);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
                self.claim_pip(clk_in, wire_eclk);
            }

            let ddrdel = self.rc_io_wire(cell, "DDRDEL_DDRDLL");
            self.add_bel_wire(bcrd, "DDRDEL", ddrdel);
            let ddrdel_out = self.claim_single_out(ddrdel);
            self.add_bel_wire(bcrd, "DDRDEL_OUT", ddrdel_out);

            self.insert_bel(bcrd, bel);
        }
    }

    pub(super) fn process_dlldel_crosslink(&mut self) {
        let cell_tile = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_s());
        for (name, idx) in [("20", 0), ("21", 1), ("10", 2), ("11", 3)] {
            let Some(&io) = self
                .chip
                .special_io
                .get(&SpecialIoKey::Clock(Dir::S, idx as u8))
            else {
                continue;
            };
            let cell = cell_tile.delta(-2 + (idx as i32), 0);
            let bcrd = cell_tile.bel(bels::DLLDEL[idx]);
            self.name_bel(bcrd, [format!("DLLDEL_{name}")]);

            let (cell_io, abcd) = self.xlat_io_loc_crosslink(io);
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

            for bank in [1, 2] {
                let bcrd_ddrdll = self.chip.special_loc[&SpecialLocKey::Bc(bank)].bel(bels::DDRDLL);
                let wire_ddrdll = self.naming.bel_wire(bcrd_ddrdll, "DDRDEL_OUT");
                self.claim_pip(ddrdel_in, wire_ddrdll);
            }

            self.insert_simple_bel(bcrd, cell, "DLLDEL");
        }
    }

    pub fn xlat_io_loc_crosslink(&self, io: EdgeIoCoord) -> (CellCoord, &'static str) {
        let bcrd = self.chip.get_io_loc(io);
        if self.chip.get_io_kind(io) == IoKind::Io {
            (bcrd.cell, ["A", "B", "C", "D"][io.iob().to_idx()])
        } else {
            (bcrd.cell, "")
        }
    }

    fn process_single_io_crosslink(&mut self, bcrd: BelCoord) {
        let io = self.chip.get_io_crd(bcrd);
        let bank = self.chip.get_io_bank(io);
        let idx = io.iob().to_idx();
        let (cell, abcd) = self.xlat_io_loc_crosslink(io);
        let (_r, c) = self.rc(cell);
        self.name_bel(bcrd, [format!("PB{c}{abcd}"), format!("IOL_B{c}{abcd}")]);
        let kind = self.chip.get_io_kind(io);
        let iol = match kind {
            IoKind::Io => "IOLOGIC",
            IoKind::Sio => "SIOLOGIC",
            _ => unreachable!(),
        };
        let mut bel = Bel::default();

        let mut pins = vec!["LSR", "CE", "CLK", "TXDATA0", "TSDATA", "INFF"];
        if kind == IoKind::Io {
            pins.extend([
                "RXDATA0",
                "RXDATA1",
                "RXDATA2",
                "RXDATA3",
                "RXDATA4",
                "RXDATA5",
                "RXDATA6",
                "RXDATA7",
                "TXDATA1",
                "TXDATA2",
                "TXDATA3",
                "TXDATA4",
                "TXDATA5",
                "TXDATA6",
                "TXDATA7",
                "CFLAG",
                "DIRECTION",
                "MOVE",
                "LOADN",
                "SLIP",
            ]);
            if matches!(idx, 0 | 2) {
                pins.extend([
                    "RXDATA8", "RXDATA9", "RXDATA10", "RXDATA11", "RXDATA12", "RXDATA13",
                    "RXDATA14", "RXDATA15", "TXDATA8", "TXDATA9", "TXDATA10", "TXDATA11",
                    "TXDATA12", "TXDATA13", "TXDATA14", "TXDATA15",
                ]);

                let wire = self.rc_io_wire(cell, &format!("JHSSEL{abcd}_PIO"));
                self.add_bel_wire(bcrd, "HSSEL", wire);
                bel.pins
                    .insert("HSSEL".into(), self.xlat_int_wire(bcrd, wire));
            }
        }

        for pin in pins {
            let wire = self.rc_io_wire(cell, &format!("J{pin}{abcd}_{iol}"));
            self.add_bel_wire(bcrd, pin, wire);
            bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
        }

        if kind == IoKind::Io && matches!(idx, 1 | 3) {
            for pin in [
                "RXDATA8", "RXDATA9", "RXDATA10", "RXDATA11", "RXDATA12", "RXDATA13", "RXDATA14",
                "RXDATA15", "TXDATA8", "TXDATA9", "TXDATA10", "TXDATA11", "TXDATA12", "TXDATA13",
                "TXDATA14", "TXDATA15",
            ] {
                let wire = self.rc_io_wire(cell, &format!("{pin}{abcd}_{iol}"));
                self.add_bel_wire(bcrd, pin, wire);
            }
        }
        if !(kind == IoKind::Io && matches!(idx, 0 | 2)) {
            let wire = self.rc_io_wire(cell, &format!("HSSEL{abcd}_PIO"));
            self.add_bel_wire(bcrd, "HSSEL", wire);
        }

        let paddi_pio = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDI_PIO", paddi_pio);
        let jdi = self.rc_io_wire(cell, &format!("JDI{abcd}"));
        self.add_bel_wire(bcrd, "JDI", jdi);
        bel.pins.insert("DI".into(), self.xlat_int_wire(bcrd, jdi));
        let di_iol = self.rc_io_wire(cell, &format!("DI{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "DI", di_iol);
        let paddilp_pio = self.rc_io_wire(cell, &format!("PADDILP{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDILP_PIO", paddilp_pio);
        if kind == IoKind::Io {
            let paddi_iol = self.rc_io_wire(cell, &format!("PADDI{abcd}_{iol}"));
            self.add_bel_wire(bcrd, "PADDI_IOLOGIC", paddi_iol);
            self.claim_pip(paddi_iol, paddi_pio);
            let paddi = self.rc_io_wire(cell, &format!("PADDI{abcd}"));
            self.add_bel_wire(bcrd, "PADDI", paddi);
            self.claim_pip(paddi, paddi_pio);

            let paddilp = self.rc_io_wire(cell, &format!("PADDILP{abcd}"));
            self.add_bel_wire(bcrd, "PADDILP", paddilp);
            self.claim_pip(paddilp, paddilp_pio);

            let indd_iol = self.rc_io_wire(cell, &format!("INDD{abcd}_{iol}"));
            self.add_bel_wire(bcrd, "INDD_IOLOGIC", indd_iol);
            let paddidel = self.rc_io_wire(cell, &format!("PADDIDEL{abcd}"));
            self.add_bel_wire(bcrd, "PADDIDEL", paddidel);
            self.claim_pip(paddidel, indd_iol);

            self.claim_pip(jdi, paddi);
            self.claim_pip(jdi, paddilp);
            self.claim_pip(jdi, paddidel);

            let di = self.rc_io_wire(cell, &format!("DI{abcd}"));
            self.add_bel_wire(bcrd, "DI", di);

            self.claim_pip(di, paddi);
            self.claim_pip(di, paddidel);
            self.claim_pip(di_iol, di);
        } else {
            self.claim_pip(jdi, paddi_pio);
            self.claim_pip(di_iol, paddi_pio);
        }

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
        assert_eq!(self.xlat_int_wire(bcrd, paddt), bel.pins["TSDATA"]);

        let ioldo_iol = self.rc_io_wire(cell, &format!("IOLDO{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLDO_IOLOGIC", ioldo_iol);
        let ioldo_pio = self.rc_io_wire(cell, &format!("IOLDO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "IOLDO_PIO", ioldo_pio);
        if kind == IoKind::Io {
            let ioldoi_iol = self.rc_io_wire(cell, &format!("IOLDOI{abcd}_{iol}"));
            self.add_bel_wire(bcrd, "IOLDOI_IOLOGIC", ioldoi_iol);
            self.claim_pip(ioldoi_iol, ioldo_iol);
            let ioldod_iol = self.rc_io_wire(cell, &format!("IOLDOD{abcd}_{iol}"));
            self.add_bel_wire(bcrd, "IOLDOD_IOLOGIC", ioldod_iol);
            let ioldo = self.rc_io_wire(cell, &format!("IOLDO{abcd}"));
            self.add_bel_wire(bcrd, "IOLDO", ioldo);
            self.claim_pip(ioldo, ioldo_iol);
            self.claim_pip(ioldo, ioldod_iol);
            self.claim_pip(ioldo_pio, ioldo);
        } else {
            self.claim_pip(ioldo_pio, ioldo_iol);
        }

        let iolto_iol = self.rc_io_wire(cell, &format!("IOLTO{abcd}_{iol}"));
        self.add_bel_wire(bcrd, "IOLTO_IOLOGIC", iolto_iol);
        let iolto_pio = self.rc_io_wire(cell, &format!("IOLTO{abcd}_PIO"));
        self.add_bel_wire(bcrd, "IOLTO_PIO", iolto_pio);
        self.claim_pip(iolto_pio, iolto_iol);

        if kind == IoKind::Io {
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

        for (pin, bslot_rc, pin_src, pin_fake) in [
            ("INRD", bels::BCINRD, "INRDENO_OUT", "INRDENO_FAKE"),
            ("LVDS", bels::BCLVDSO, "LVDSENO_OUT", "LVDSENO_FAKE"),
        ] {
            let wire = self.rc_io_wire(cell, &format!("{pin}{abcd}_PIO"));
            self.add_bel_wire(bcrd, pin, wire);
            if kind == IoKind::Io {
                let cell_rc = self.chip.special_loc[&SpecialLocKey::Bc(bank)];
                let bcrd_rc = cell_rc.bel(bslot_rc);
                let wire_src = self.naming.bel_wire(
                    bcrd_rc,
                    if bank == 2 && cell.col.to_idx() >= 11 {
                        pin_fake
                    } else {
                        pin_src
                    },
                );
                self.claim_pip(wire, wire_src);
            }
        }

        self.insert_bel(bcrd, bel);
    }

    pub(super) fn process_io_crosslink(&mut self) {
        for (tcname, num_io) in [("IO_S4", 4), ("IO_S1A", 1), ("IO_S1B", 1)] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                for i in 0..num_io {
                    let bcrd = tcrd.bel(bels::IO[i]);
                    self.process_single_io_crosslink(bcrd);
                }
            }
        }
    }

    pub(super) fn process_clkdiv_crosslink(&mut self) {
        let cell_tile = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_s());
        let cell = cell_tile.delta(-1, 0);
        for i in 0..4 {
            let bcrd = cell_tile.bel(bels::CLKDIV[i]);
            self.name_bel(bcrd, [format!("CLKDIV{i}")]);
            let mut bel = Bel::default();

            for pin in ["ALIGNWD", "RST", "CDIVX"] {
                let wire = self.rc_io_wire(cell, &format!("J{pin}_CLKDIV{i}"));
                self.add_bel_wire(bcrd, pin, wire);
                bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
            }

            let clki = self.rc_io_wire(cell, &format!("CLKI_CLKDIV{i}"));
            self.add_bel_wire(bcrd, "CLKI", clki);

            let bcrd_eclk = bcrd.bel(bels::ECLKSYNC[i]);
            let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK_MUX");
            self.claim_pip(clki, wire_eclk);

            self.insert_bel(bcrd, bel);
        }
    }
}

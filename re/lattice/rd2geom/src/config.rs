use prjcombine_ecp::{
    bels,
    chip::{ChipKind, MachXo2Kind, PllLoc, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    dir::{DirH, DirHV, DirV},
    grid::{CellCoord, DieId},
};
use prjcombine_re_lattice_naming::WireName;
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_config_ecp(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];

        let bcrd = cell.bel(bels::JTAG);
        self.name_bel(bcrd, ["JTAG", "TCK", "TMS", "TDI", "TDO"]);
        self.insert_simple_bel(bcrd, cell, "JTAG");
        for pin in ["TCK", "TMS", "TDI", "TDO"] {
            let wire = self.rc_wire(cell, &format!("J{pin}_JTAG"));
            let wire_pin = WireName {
                r: 0,
                c: self.chip.columns.len() as u8 + 1,
                suffix: self.naming.strings.get(&format!("J{pin}_{pin}")).unwrap(),
            };
            self.add_bel_wire(bcrd, pin, wire);
            self.add_bel_wire(bcrd, format!("{pin}_{pin}"), wire_pin);
            if pin == "TDO" {
                self.claim_pip(wire_pin, wire);
            } else {
                self.claim_pip(wire, wire_pin);
            }
        }

        let bcrd = cell.bel(bels::START);
        self.name_bel(bcrd, ["START"]);
        self.insert_simple_bel(bcrd, cell, "START");

        let bcrd = cell.bel(bels::RDBK);
        self.name_bel(bcrd, ["RDBK"]);
        for (pin, wire) in [
            ("CAPTINPUT", "JCAPTINPUT_RDBK"),
            ("CAPTCLK", "JCAPTCLK_RDBK"),
        ] {
            let wire = self.rc_wire(cell, wire);
            self.add_bel_wire(bcrd, pin, wire);
        }

        let bcrd = cell.bel(bels::OSC);
        self.name_bel(bcrd, ["OSC"]);
        if self.chip.kind == ChipKind::Ecp {
            self.insert_simple_bel(bcrd, cell, "OSC");
        } else {
            let wire = self.rc_wire(cell, "JCFGCLK_OSC");
            self.add_bel_wire(bcrd, "CFGCLK", wire);
        }

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell, "GSR");
    }

    fn process_config_machxo(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];

        let bcrd = cell.bel(bels::JTAG);
        self.name_bel(bcrd, ["JTAG", "TCK", "TMS", "TDI", "TDO", "TSALL"]);
        self.insert_simple_bel(bcrd, cell, "JTAG");
        for pin in ["TCK", "TMS", "TDI", "TDO"] {
            let wire = self.rc_wire(cell, &format!("J{pin}_JTAG"));
            let wire_pin = WireName {
                r: self.chip.rows.len() as u8 + 1,
                c: 0,
                suffix: self.naming.strings.get(&format!("J{pin}_{pin}")).unwrap(),
            };
            self.add_bel_wire(bcrd, pin, wire);
            self.add_bel_wire(bcrd, format!("{pin}_{pin}"), wire_pin);
            if pin == "TDO" {
                self.claim_pip(wire_pin, wire);
            } else {
                self.claim_pip(wire, wire_pin);
            }
        }
        let tsall_io = self.chip.special_io[&SpecialIoKey::TsAll];
        let tsall_cell = self.chip.get_io_loc(tsall_io).cell;
        let tsalli = self.rc_wire(tsall_cell, "JTSALLI_TSALL");
        self.add_bel_wire(bcrd, "TSALLI", tsalli);
        let wire_io = self.get_special_io_wire_in(SpecialIoKey::TsAll);
        self.claim_pip(tsalli, wire_io);

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell, "GSR");
        let gsrpadn = self.rc_wire(cell, "JGSRPADN_GSR");
        self.add_bel_wire(bcrd, "GSRPADN", gsrpadn);
        let wire_io = self.get_special_io_wire_in(SpecialIoKey::Gsr);
        self.claim_pip(gsrpadn, wire_io);
    }

    fn process_osc_machxo(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Osc];
        let bcrd = cell.bel(bels::OSC);
        self.name_bel(bcrd, ["OSC"]);
        self.insert_simple_bel(bcrd, cell, "OSC");
    }

    fn process_config_ecp2(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];

        let bcrd = cell.bel(bels::JTAG);
        self.name_bel(bcrd, ["JTAG", "TCK", "TMS", "TDI", "TDO"]);
        self.insert_simple_bel(bcrd, cell, "JTAG");
        for pin in ["TCK", "TMS", "TDI", "TDO"] {
            let wire = self.rc_wire(cell, &format!("J{pin}_JTAG"));
            let wire_pin = WireName {
                r: self.chip.rows.len() as u8 + 1,
                c: 0,
                suffix: self.naming.strings.get(&format!("J{pin}_{pin}")).unwrap(),
            };
            self.add_bel_wire(bcrd, pin, wire);
            self.add_bel_wire(bcrd, format!("{pin}_{pin}"), wire_pin);
            if pin == "TDO" {
                self.claim_pip(wire_pin, wire);
            } else {
                self.claim_pip(wire, wire_pin);
            }
        }

        let bcrd = cell.bel(bels::START);
        self.name_bel(bcrd, ["START"]);
        self.insert_simple_bel(bcrd, cell, "START");

        let bcrd = cell.bel(bels::OSC);
        self.name_bel(bcrd, ["OSC"]);
        self.insert_simple_bel(bcrd, cell, "OSC");

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell, "GSR");

        let bcrd = cell.bel(bels::SED);
        self.name_bel(bcrd, ["SED"]);
        self.insert_simple_bel(bcrd, cell, "SED");

        let bcrd = cell.bel(bels::SPIM);
        self.name_bel(bcrd, ["SPIM"]);
        self.insert_simple_bel(bcrd, cell, "SPIM");
    }

    fn process_config_xp2(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Osc];
        let bcrd = cell.bel(bels::OSC);
        let cell = cell.delta(0, 1);
        self.name_bel(bcrd, ["OSC"]);
        self.insert_simple_bel(bcrd, cell, "OSC");

        let cell = self.chip.bel_dqsdll_ecp2(DirH::E).cell;

        let bcrd = cell.bel(bels::WAKEUP);
        self.name_bel(bcrd, ["WAKEUP"]);
        self.insert_simple_bel(bcrd, cell, "WAKEUP");

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell, "GSR");

        let bcrd = cell.bel(bels::STF);
        self.name_bel(bcrd, ["STF"]);
        self.insert_simple_bel(bcrd, cell, "STF");

        let bcrd = cell.bel(bels::START);
        self.name_bel(bcrd, ["START"]);
        self.insert_simple_bel(bcrd, cell, "START");

        let bcrd = cell.bel(bels::SSPI);
        self.name_bel(bcrd, ["SSPICIB", "SSPIPIN"]);
        self.insert_simple_bel(bcrd, cell, "SSPICIB");
        for (pin, key) in [
            ("CLK", SpecialIoKey::Cclk),
            ("CS", SpecialIoKey::SpiPCsB),
            ("SI", SpecialIoKey::SpiSdi),
            ("SO", SpecialIoKey::SpiSdo),
        ] {
            let wire = self.rc_wire(cell, &format!("J{pin}_SSPIPIN"));
            self.add_bel_wire(bcrd, format!("{pin}_IO"), wire);
            if pin == "SO" {
                let wire_io = self.get_special_io_wire_out(key);
                self.claim_pip(wire_io, wire);
            } else {
                let wire_io = self.get_special_io_wire_in(key);
                self.claim_pip(wire, wire_io);
            }
        }

        let cell_cfg = self.chip.special_loc[&SpecialLocKey::Config];

        let bcrd = cell_cfg.bel(bels::SED);
        self.name_bel(bcrd, ["SED"]);
        self.insert_simple_bel(bcrd, cell, "SED");

        let bcrd = cell_cfg.bel(bels::JTAG);
        self.name_bel(bcrd, ["JTAG", "TCK", "TMS", "TDI", "TDO"]);
        self.insert_simple_bel(bcrd, cell, "JTAG");
        for pin in ["TCK", "TMS", "TDI", "TDO"] {
            let wire = self.rc_wire(cell, &format!("J{pin}_JTAG"));
            let wire_pin = self.rc_io_wire(cell, &format!("J{pin}_{pin}"));
            self.add_bel_wire(bcrd, pin, wire);
            self.add_bel_wire(bcrd, format!("{pin}_{pin}"), wire_pin);
            if pin == "TDO" {
                self.claim_pip(wire_pin, wire);
            } else {
                self.claim_pip(wire, wire_pin);
            }
        }
    }

    fn process_config_ecp3(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];

        let bcrd = cell.bel(bels::JTAG);
        self.name_bel(bcrd, ["JTAG", "TCK", "TMS", "TDI", "TDO"]);
        self.insert_simple_bel(bcrd, cell.delta(3, 0), "JTAG");
        for pin in ["TCK", "TMS", "TDI", "TDO"] {
            let wire = self.rc_wire(cell.delta(3, 0), &format!("J{pin}_JTAG"));
            let wire_pin = WireName {
                r: 0,
                c: 0,
                suffix: self.naming.strings.get(&format!("J{pin}_{pin}")).unwrap(),
            };
            self.add_bel_wire(bcrd, pin, wire);
            self.add_bel_wire(bcrd, format!("{pin}_{pin}"), wire_pin);
            // lmao yes they reversed this
            if pin == "TDO" {
                self.claim_pip(wire, wire_pin);
            } else {
                self.claim_pip(wire_pin, wire);
            }
        }

        let bcrd = cell.bel(bels::START);
        self.name_bel(bcrd, ["START"]);
        self.insert_simple_bel(bcrd, cell.delta(3, 0), "START");

        let bcrd = cell.bel(bels::OSC);
        self.name_bel(bcrd, ["OSC"]);
        self.insert_simple_bel(bcrd, cell.delta(3, 0), "OSC");

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell.delta(3, 0), "GSR");

        let bcrd = cell.bel(bels::SED);
        self.name_bel(bcrd, ["SED"]);
        self.insert_simple_bel(bcrd, cell.delta(4, 0), "SED");

        let bcrd = cell.bel(bels::AMBOOT);
        self.name_bel(bcrd, ["AMBOOT"]);
        self.insert_simple_bel(bcrd, cell.delta(4, 0), "AMBOOT");

        let bcrd = cell.bel(bels::PERREG);
        self.name_bel(bcrd, ["PERREG"]);
        self.insert_simple_bel(bcrd, cell.delta(5, 0), "PERREG");

        for h in [DirH::W, DirH::E] {
            let col = self.chip.col_edge(h);
            for v in [DirV::S, DirV::N] {
                let lr = match h {
                    DirH::W => 'L',
                    DirH::E => 'R',
                };
                let lu = match v {
                    DirV::S => 'L',
                    DirV::N => 'U',
                };
                let row = self.chip.row_edge(v);
                let cell = CellCoord::new(DieId::from_idx(0), col, row);
                for (bel, name) in [(bels::TESTIN, "TESTIN"), (bels::TESTOUT, "TESTOUT")] {
                    let bcrd = cell.bel(bel);
                    self.name_bel(bcrd, [format!("{lu}{lr}{name}")]);
                    self.insert_simple_bel(bcrd, bcrd.cell, name);
                }
                if h == DirH::E && v == DirV::S {
                    let bcrd = cell.bel(bels::DTS);
                    self.name_bel(bcrd, ["DTS"]);
                    self.insert_simple_bel(bcrd, bcrd.cell, "DTS");
                }
            }
        }
    }

    fn process_config_machxo2(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];

        let bcrd = cell.bel(bels::JTAG);
        self.name_bel(bcrd, ["JTAG"]);
        self.insert_simple_bel(bcrd, cell, "JTAG");
        for (pin, key) in [
            ("TCK", SpecialIoKey::Tck),
            ("TMS", SpecialIoKey::Tms),
            ("TDI", SpecialIoKey::Tdi),
            ("TDO", SpecialIoKey::Tdo),
        ] {
            let wire = self.rc_wire(cell, &format!("J{pin}_JTAG"));
            self.add_bel_wire(bcrd, pin, wire);
            if pin == "TDO" {
                let io = self.chip.special_io[&key];
                let bcrd_io = self.chip.get_io_loc(io);
                let wire_io = self.rc_io_wire(
                    bcrd_io.cell,
                    &format!(
                        "JPADDO{abcd}",
                        abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx()],
                    ),
                );
                self.claim_pip(wire, wire_io);
            } else {
                // yeah they not good at it
                let wire_io = self.get_special_io_wire_in(key);
                self.claim_pip(wire_io, wire);
            }
        }

        let bcrd_osc = cell.bel(bels::OSC);
        self.name_bel(bcrd_osc, ["OSC"]);
        self.insert_simple_bel(bcrd_osc, cell, "OSC");

        let bcrd = cell.bel(bels::START);
        self.name_bel(bcrd, ["START"]);
        self.insert_simple_bel(bcrd, cell, "START");

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell, "GSR");

        let bcrd_efb = cell.bel(bels::EFB);
        self.name_bel(bcrd_efb, ["EFB"]);
        self.insert_simple_bel(bcrd_efb, cell, "EFB");
        for (pin, key) in [
            ("UFMSN", SpecialIoKey::SpiPCsB),
            ("SPIMOSII", SpecialIoKey::SpiCopi),
            ("SPIMISOI", SpecialIoKey::SpiCipo),
            ("SPISCKI", SpecialIoKey::Cclk),
            ("I2C1SCLI", SpecialIoKey::I2cScl),
            ("I2C1SDAI", SpecialIoKey::I2cSda),
        ] {
            let wire = self.rc_wire(cell, &format!("J{pin}_EFB"));
            self.add_bel_wire(bcrd_efb, pin, wire);
            let wire_io = self.get_special_io_wire_in(key);
            self.claim_pip(wire, wire_io);
        }
        for (pin_o, pin_oe, key) in [
            ("I2C1SCLO", "I2C1SCLOEN", SpecialIoKey::I2cScl),
            ("I2C1SDAO", "I2C1SDAOEN", SpecialIoKey::I2cSda),
            ("SPIMCSN0", "SPIMCSN0", SpecialIoKey::SpiCCsB),
            ("SPISCKO", "SPISCKEN", SpecialIoKey::Cclk),
            ("SPIMISOO", "SPIMISOEN", SpecialIoKey::SpiCipo),
            ("SPIMOSIO", "SPIMOSIEN", SpecialIoKey::SpiCopi),
        ] {
            let wire_o = self.rc_wire(cell, &format!("J{pin_o}_EFB"));
            let wire_oe = self.rc_wire(cell, &format!("J{pin_oe}_EFB"));
            self.add_bel_wire(bcrd_efb, pin_o, wire_o);
            if wire_o != wire_oe {
                self.add_bel_wire(bcrd_efb, pin_oe, wire_oe);
            }
            let io = self.chip.special_io[&key];
            let bcrd_io = self.chip.get_io_loc(io);
            let wire_io_o = self.rc_io_wire(
                bcrd_io.cell,
                &format!(
                    "JPADDO{abcd}",
                    abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx()],
                ),
            );
            self.claim_pip(wire_io_o, wire_o);
            let wire_io_oe = self.rc_io_wire(
                bcrd_io.cell,
                &format!(
                    "JPADDT{abcd}",
                    abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx()],
                ),
            );
            self.claim_pip(wire_io_oe, wire_oe);
        }
        // bug? idk perhaps.
        let wire = self.rc_wire(cell, "JSPICSNEN_EFB");
        self.add_bel_wire(bcrd_efb, "SPICSNEN", wire);

        for pin in [
            "PLLDATO0",
            "PLLDATO1",
            "PLLDATO2",
            "PLLDATO3",
            "PLLDATO4",
            "PLLDATO5",
            "PLLDATO6",
            "PLLDATO7",
            "PLLADRO0",
            "PLLADRO1",
            "PLLADRO2",
            "PLLADRO3",
            "PLLADRO4",
            "PLLWEO",
            "PLLCLKO",
            "PLLRSTO",
            "PLL0STBO",
            "PLL0ACKI",
            "PLL0DATI0",
            "PLL0DATI1",
            "PLL0DATI2",
            "PLL0DATI3",
            "PLL0DATI4",
            "PLL0DATI5",
            "PLL0DATI6",
            "PLL0DATI7",
            "PLL1STBO",
            "PLL1ACKI",
            "PLL1DATI0",
            "PLL1DATI1",
            "PLL1DATI2",
            "PLL1DATI3",
            "PLL1DATI4",
            "PLL1DATI5",
            "PLL1DATI6",
            "PLL1DATI7",
        ] {
            let wire = self.rc_wire(cell, &format!("J{pin}_EFB"));
            self.add_bel_wire(bcrd_efb, pin, wire);
        }

        let has_2pll = self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::Pll(PllLoc::new(DirHV::NE, 0)));

        let has_mux = matches!(
            self.chip.kind,
            ChipKind::MachXo2(MachXo2Kind::MachXo2 | MachXo2Kind::MachXo3L)
        ) && has_2pll;

        for (pin, pin_pll) in [
            ("ACKI", "ACK"),
            ("DATI0", "DATO0"),
            ("DATI1", "DATO1"),
            ("DATI2", "DATO2"),
            ("DATI3", "DATO3"),
            ("DATI4", "DATO4"),
            ("DATI5", "DATO5"),
            ("DATI6", "DATO6"),
            ("DATI7", "DATO7"),
        ] {
            for (i, hv) in [(0, DirHV::NW), (1, DirHV::NE)] {
                let Some(&cell_pll) = self
                    .chip
                    .special_loc
                    .get(&SpecialLocKey::Pll(PllLoc::new(hv, 0)))
                else {
                    continue;
                };
                let wire_pll = self.rc_wire(cell_pll, &format!("JPLL{pin_pll}_PLL"));
                if has_mux {
                    let wire = self.rc_wire(cell, &format!("JPLL{i}{pin}"));
                    self.add_bel_wire(bcrd, format!("PLL{i}{pin}_IN"), wire);
                    self.claim_pip(wire, wire_pll);
                } else {
                    let wire = self.rc_wire(cell, &format!("JPLL{i}{pin}_EFB"));
                    self.claim_pip(wire, wire_pll);
                }
            }
            if has_2pll {
                for i in 0..2 {
                    let wire = self.rc_wire(cell, &format!("JPLL{i}{pin}_EFB"));
                    let wire_mux = self.rc_wire(cell, &format!("PLL{i}{pin}MUX"));
                    self.add_bel_wire(bcrd, format!("PLL{i}{pin}_MUX"), wire_mux);
                    self.claim_pip(wire, wire_mux);
                    if has_mux {
                        for j in 0..2 {
                            let wire_in = self.rc_wire(cell, &format!("JPLL{j}{pin}"));
                            self.claim_pip(wire_mux, wire_in);
                        }
                    }
                }
            }
        }

        if has_2pll {
            for i in 0..2 {
                let wire = self.rc_wire(cell, &format!("JPLL{i}STBO_EFB"));
                let wire_out = self.rc_wire(cell, &format!("PLL{i}STBO"));
                self.add_bel_wire(bcrd, format!("PLL{i}STBO_OUT"), wire_out);
                self.claim_pip(wire_out, wire);
            }
        }
        if has_mux {
            for i in 0..2 {
                let wire_mux = self.rc_wire(cell, &format!("JPLL{i}STBOMUX"));
                self.add_bel_wire(bcrd, format!("PLL{i}STBO_MUX"), wire_mux);
                for j in 0..2 {
                    let wire_out = self.rc_wire(cell, &format!("PLL{j}STBO"));
                    self.claim_pip(wire_mux, wire_out);
                }
            }
        }

        let bcrd = cell.bel(bels::PCNTR);
        self.name_bel(bcrd, ["PCNTR"]);
        let mut bel = self.extract_simple_bel(bcrd, cell, "PCNTR");
        for pin in ["CFGWAKE", "CFGSTDBY"] {
            let wire_efb = self.rc_wire(cell, &format!("{pin}_EFB"));
            self.add_bel_wire(bcrd_efb, pin, wire_efb);
            let wire = self.rc_wire(cell, &format!("{pin}_PCNTR"));
            self.add_bel_wire(bcrd, pin, wire);
            self.claim_pip(wire, wire_efb);
        }
        let clk = self.rc_wire(cell, "CLK_PCNTR");
        self.add_bel_wire(bcrd, "CLK", clk);
        let clk_in = self.rc_wire(cell, "PCNTRCLK");
        self.add_bel_wire(bcrd, "CLK_IN", clk_in);
        self.claim_pip(clk, clk_in);
        let clk_int = self.rc_wire(cell, "JCIBCLK");
        self.add_bel_wire(bcrd, "CLK_INT", clk_int);
        self.claim_pip(clk_in, clk_int);
        bel.pins
            .insert("CLK".into(), self.xlat_int_wire(bcrd, clk_int));
        let clk_osc = self.rc_wire(cell, "JOSCCLK");
        self.add_bel_wire(bcrd, "CLK_OSC", clk_osc);
        self.claim_pip(clk_in, clk_osc);
        let wire_osc = self.rc_wire(cell, "JOSC_OSC");
        self.claim_pip(clk_osc, wire_osc);
        self.insert_bel(bcrd, bel);

        let bcrd = cell.bel(bels::TSALL);
        self.name_bel(bcrd, ["TSALL"]);
        self.insert_simple_bel(bcrd, cell, "TSALL");

        let bcrd = cell.bel(bels::SED);
        self.name_bel(bcrd, ["SED"]);
        self.insert_simple_bel(bcrd, cell, "SED");
        let wire_osc = self.rc_wire(bcrd_osc.cell, "SEDSTDBY_OSC");
        self.add_bel_wire(bcrd_osc, "SEDSTDBY", wire_osc);
        let wire_osc_in = self.rc_wire(bcrd_osc.cell, "JSTDBY_OSC");
        self.claim_pip(wire_osc, wire_osc_in);
        let wire = self.rc_wire(cell, "SEDSTDBY_SED");
        self.add_bel_wire(bcrd, "SEDSTDBY", wire);
        self.claim_pip(wire, wire_osc);

        if matches!(
            self.chip.kind,
            ChipKind::MachXo2(MachXo2Kind::MachXo3D | MachXo2Kind::MachNx)
        ) {
            let bcrd = cell.bel(bels::ESB);
            let cell = cell.delta(1, 0);
            let (r, c) = self.rc(cell);
            self.name_bel(bcrd, [format!("ESB_R{r}C{c}")]);
            self.insert_simple_bel(bcrd, cell, "ESB");

            let wire_osc = self.rc_wire(bcrd_osc.cell, "JOSCESB_OSC");
            self.add_bel_wire(bcrd_osc, "OSCESB", wire_osc);
            let wire = self.rc_wire(cell, "JOSCCLK_ESB");
            self.add_bel_wire(bcrd, "OSCCLK", wire);
            self.claim_pip(wire, wire_osc);
        }
    }

    fn process_config_ecp4(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];

        let bcrd = cell.bel(bels::JTAG);
        self.name_bel(bcrd, ["JTAG", "TCK", "TMS", "TDI", "TDO"]);
        self.insert_simple_bel(bcrd, cell, "JTAG");
        for pin in ["TCK", "TMS", "TDI", "TDO"] {
            let wire = self.rc_wire(cell, &format!("J{pin}_JTAG"));
            let wire_pin = WireName {
                r: self.chip.rows.len() as u8,
                c: 0,
                suffix: self.naming.strings.get(&format!("J{pin}_{pin}")).unwrap(),
            };
            self.add_bel_wire(bcrd, pin, wire);
            self.add_bel_wire(bcrd, format!("{pin}_{pin}"), wire_pin);
            if pin == "TDO" {
                self.claim_pip(wire_pin, wire);
            } else {
                self.claim_pip(wire, wire_pin);
            }
        }

        let bcrd_osc = cell.bel(bels::OSC);
        self.name_bel(bcrd_osc, ["OSC"]);
        self.insert_simple_bel(bcrd_osc, cell, "OSC");

        let bcrd = cell.bel(bels::START);
        self.name_bel(bcrd, ["START"]);
        self.insert_simple_bel(bcrd, cell, "START");

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell, "GSR");

        let bcrd_efb = cell.bel(bels::EFB);
        self.name_bel(bcrd_efb, ["EFB", "CCLK"]);
        self.insert_simple_bel(bcrd_efb, cell, "EFB");

        let cell_cclk = cell.with_col(self.chip.col_w());
        for (pin, pin_cclk) in [
            ("SPISCKEN", "PADDT"),
            ("SPISCKO", "PADDO"),
            ("SPISCKI", "PADDI"),
        ] {
            let wire = self.rc_io_wire(cell, &format!("J{pin}_EFB"));
            self.add_bel_wire(bcrd_efb, pin, wire);
            let wire_cclk = self.rc_io_wire(cell_cclk, &format!("J{pin_cclk}_CCLK"));
            self.add_bel_wire(bcrd_efb, format!("{pin}_CCLK"), wire_cclk);
            if pin_cclk == "PADDI" {
                self.claim_pip(wire, wire_cclk);
            } else {
                self.claim_pip(wire_cclk, wire);
            }
        }

        for (key, pin_i, pin_o, pin_oe) in [
            (SpecialIoKey::WriteN, "I2C1SCLI", "I2C1SCLO", "I2C1SCLOEN"),
            (SpecialIoKey::Cs1N, "I2C1SDAI", "I2C1SDAO", "I2C1SDAOEN"),
            (SpecialIoKey::D(0), "SPIMOSII", "SPIMOSIO", "SPIMOSIEN"),
            (SpecialIoKey::D(1), "SPIMISOI", "SPIMISOO", "SPIMISOEN"),
            (SpecialIoKey::Di, "", "SPIMCSN0", "SPICSNEN"),
        ] {
            let (cell_io, abcd) = self.xlat_io_loc_ecp4(self.chip.special_io[&key]);
            for (pin, wn_io) in [
                (pin_i, format!("JPADDI{abcd}_PIO")),
                (pin_o, format!("JPADDO{abcd}")),
                (pin_oe, format!("JPADDT{abcd}")),
            ] {
                if pin.is_empty() {
                    continue;
                }
                let wire = self.rc_io_wire(cell, &format!("J{pin}_EFB"));
                self.add_bel_wire(bcrd_efb, pin, wire);
                let wire_io = self.rc_io_wire(cell_io, &wn_io);
                if pin == pin_i {
                    self.claim_pip(wire, wire_io);
                } else {
                    self.claim_pip(wire_io, wire);
                }
            }
        }

        let cell_asb = cell.with_col(self.chip.col_w());
        for i in 0..8 {
            let wire = self.rc_io_wire(cell, &format!("JSCIDATI0_{i}_EFB"));
            self.add_bel_wire(bcrd_efb, format!("SCIDATI0_{i}"), wire);
            let wire_asb = self.rc_io_sn_wire(cell_asb, &format!("JSCIDATO{i}_ASB"));
            self.claim_pip(wire, wire_asb);
        }
        for (pin, pin_asb) in [
            ("SCIINT0", "SCIINT"),
            ("SCIRTYI0", "SCIRTYO"),
            ("SCIACKI0", "SCIACKO"),
        ] {
            let wire = self.rc_io_wire(cell, &format!("J{pin}_EFB"));
            self.add_bel_wire(bcrd_efb, pin, wire);
            let wire_asb = self.rc_io_sn_wire(cell_asb, &format!("J{pin_asb}_ASB"));
            self.claim_pip(wire, wire_asb);
        }
        for (pin, range) in [("SCISTBO", 0..64), ("SCIDATO", 0..8), ("SCIADRO", 0..12)] {
            for i in range {
                let wire = self.rc_io_wire(cell, &format!("J{pin}{i}_EFB"));
                self.add_bel_wire(bcrd_efb, format!("{pin}{i}"), wire);
            }
        }
        for pin in [
            "SCIWEO",
            "SCICYCO",
            "SCIRSTO",
            "SCICLKO",
            "SCISLEEP",
            "SCIINITEN0",
            "SCIINITEN1",
            "SCIRSTN",
        ] {
            let wire = self.rc_io_wire(cell, &format!("J{pin}_EFB"));
            self.add_bel_wire(bcrd_efb, pin, wire);
        }

        for (pin, range) in [
            ("SCIRTYI", 1..64),
            ("SCIACKI", 1..64),
            ("SCIINT", 1..64),
            ("SWSIMADDR", 0..32),
        ] {
            for i in range {
                let wire = self.rc_io_wire(cell, &format!("{pin}{i}_EFB"));
                self.add_bel_wire(bcrd_efb, format!("{pin}{i}"), wire);
            }
        }
        for i in 1..64 {
            for j in 0..8 {
                let wire = self.rc_io_wire(cell, &format!("SCIDATI{i}_{j}_EFB"));
                self.add_bel_wire(bcrd_efb, format!("SCIDATI{i}_{j}"), wire);
            }
        }

        let bcrd = cell.bel(bels::PCNTR);
        self.name_bel(bcrd, ["PCNTR"]);
        let mut bel = self.extract_simple_bel(bcrd, cell, "PCNTR");
        for pin in ["CFGWAKE", "CFGSTDBY"] {
            let wire_efb = self.rc_io_wire(cell, &format!("J{pin}_EFB"));
            self.add_bel_wire(bcrd_efb, pin, wire_efb);
            let wire = self.rc_wire(cell, &format!("J{pin}_PCNTR"));
            self.add_bel_wire(bcrd, pin, wire);
            self.claim_pip(wire, wire_efb);
        }
        let clk = self.rc_wire(cell, "CLK_PCNTR");
        self.add_bel_wire(bcrd, "CLK", clk);
        let clk_in = self.rc_wire(cell, "PCNTRCLK");
        self.add_bel_wire(bcrd, "CLK_IN", clk_in);
        self.claim_pip(clk, clk_in);
        let clk_int = self.rc_wire(cell, "JCIBCLK");
        self.add_bel_wire(bcrd, "CLK_INT", clk_int);
        self.claim_pip(clk_in, clk_int);
        bel.pins
            .insert("CLK".into(), self.xlat_int_wire(bcrd, clk_int));
        let clk_osc = self.rc_wire(cell, "JOSCCLK");
        self.add_bel_wire(bcrd, "CLK_OSC", clk_osc);
        self.claim_pip(clk_in, clk_osc);
        let wire_osc = self.rc_wire(cell, "JOSC_OSC");
        self.claim_pip(clk_osc, wire_osc);
        self.insert_bel(bcrd, bel);

        let bcrd = cell.bel(bels::SED);
        self.name_bel(bcrd, ["SED"]);
        self.insert_simple_bel(bcrd, cell, "SED");
        let wire_osc = self.rc_wire(bcrd_osc.cell, "SEDSTDBY_OSC");
        self.add_bel_wire(bcrd_osc, "SEDSTDBY", wire_osc);
        let wire_osc_in = self.rc_wire(bcrd_osc.cell, "JSTDBY_OSC");
        self.claim_pip(wire_osc, wire_osc_in);
        let wire = self.rc_wire(cell, "SEDSTDBY_SED");
        self.add_bel_wire(bcrd, "SEDSTDBY", wire);
        self.claim_pip(wire, wire_osc);
    }

    fn process_config_ecp5(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];

        let bcrd = cell.bel(bels::JTAG);
        self.name_bel(bcrd, ["JTAG", "TCK", "TMS", "TDI", "TDO"]);
        self.insert_simple_bel(bcrd, cell, "JTAG");
        for pin in ["TCK", "TMS", "TDI", "TDO"] {
            let wire = self.rc_wire(cell, &format!("J{pin}_JTAG"));
            let wire_pin = WireName {
                r: self.chip.rows.len() as u8 + 1,
                c: 0,
                suffix: self.naming.strings.get(&format!("J{pin}_{pin}")).unwrap(),
            };
            self.add_bel_wire(bcrd, pin, wire);
            self.add_bel_wire(bcrd, format!("{pin}_{pin}"), wire_pin);
            if pin == "TDO" {
                self.claim_pip(wire_pin, wire);
            } else {
                self.claim_pip(wire, wire_pin);
            }
        }

        let bcrd_osc = cell.bel(bels::OSC);
        self.name_bel(bcrd_osc, ["OSC"]);
        self.insert_simple_bel(bcrd_osc, cell, "OSC");

        let bcrd = cell.bel(bels::START);
        self.name_bel(bcrd, ["START"]);
        self.insert_simple_bel(bcrd, cell, "START");

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell, "GSR");

        let bcrd = cell.bel(bels::CCLK);
        self.name_bel(bcrd, ["CCLK"]);
        self.insert_simple_bel(bcrd, cell.with_col(self.chip.col_w()), "CCLK");

        let bcrd = cell.bel(bels::SED);
        self.name_bel(bcrd, ["SED"]);
        self.insert_simple_bel(bcrd, cell, "SED");
        let wire_osc = self.rc_wire(cell, "SEDSTDBY_OSC");
        self.add_bel_wire(bcrd_osc, "SEDSTDBY", wire_osc);
        let wire_osc_in = self.rc_wire(cell, "JSTDBY_OSC");
        self.claim_pip(wire_osc, wire_osc_in);
        let wire = self.rc_wire(cell, "SEDSTDBY_SED");
        self.add_bel_wire(bcrd, "SEDSTDBY", wire);
        self.claim_pip(wire, wire_osc);
    }

    pub fn process_config(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => {
                self.process_config_ecp();
            }
            ChipKind::MachXo => {
                self.process_config_machxo();
                self.process_osc_machxo();
            }
            ChipKind::Ecp2 | ChipKind::Ecp2M => {
                self.process_config_ecp2();
            }
            ChipKind::Xp2 => {
                self.process_config_xp2();
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.process_config_ecp3();
            }
            ChipKind::MachXo2(_) => {
                self.process_config_machxo2();
            }
            ChipKind::Ecp4 => {
                self.process_config_ecp4();
            }
            ChipKind::Ecp5 => {
                self.process_config_ecp5();
            }
        }
    }
}

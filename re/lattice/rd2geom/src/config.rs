use prjcombine_ecp::{
    bels,
    chip::{ChipKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::dir::DirH;
use prjcombine_re_lattice_naming::WireName;

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
        }
    }
}

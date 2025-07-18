use prjcombine_ecp::{
    bels,
    chip::{ChipKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_re_lattice_naming::WireName;

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_config_ecp(&mut self) {
        let cell = self.edev.config;

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
        let wire_io = self.get_special_io_wire(SpecialIoKey::TsAll);
        self.claim_pip(tsalli, wire_io);

        let bcrd = cell.bel(bels::GSR);
        self.name_bel(bcrd, ["GSR"]);
        self.insert_simple_bel(bcrd, cell, "GSR");
        let gsrpadn = self.rc_wire(cell, "JGSRPADN_GSR");
        self.add_bel_wire(bcrd, "GSRPADN", gsrpadn);
        let wire_io = self.get_special_io_wire(SpecialIoKey::Gsr);
        self.claim_pip(gsrpadn, wire_io);
    }

    fn process_osc_machxo(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Osc];
        let bcrd = cell.bel(bels::OSC);
        self.name_bel(bcrd, ["OSC"]);
        self.insert_simple_bel(bcrd, cell, "OSC");
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
        }
    }
}

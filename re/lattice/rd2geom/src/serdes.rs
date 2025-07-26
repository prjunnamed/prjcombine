use prjcombine_ecp::{bels, chip::ChipKind};
use prjcombine_interconnect::dir::{DirH, DirHV, DirV};

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_serdes_ecp2(&mut self) {
        for tcname in ["SERDES_S", "SERDES_N"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.egrid.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::SERDES);
                let hv = DirHV {
                    h: if bcrd.col < self.chip.col_clk {
                        DirH::W
                    } else {
                        DirH::E
                    },
                    v: if bcrd.row < self.chip.row_clk {
                        DirV::S
                    } else {
                        DirV::N
                    },
                };
                let cell = bcrd.cell.delta(
                    match hv.h {
                        DirH::W => 12,
                        DirH::E => 13,
                    },
                    match hv.v {
                        DirV::S => -1,
                        DirV::N => 1,
                    },
                );
                let cell_apio = bcrd.cell.delta(
                    match hv.h {
                        DirH::W => 12,
                        DirH::E => 13,
                    },
                    match hv.v {
                        DirV::S => -7,
                        DirV::N => 7,
                    },
                );
                let corner = match hv {
                    DirHV::SW => "LL",
                    DirHV::SE => "LR",
                    DirHV::NW => "UL",
                    DirHV::NE => "UR",
                };
                self.name_bel(
                    bcrd,
                    [
                        format!("{corner}PCS"),
                        format!("{corner}C_SQ_REFCLKP"),
                        format!("{corner}C_SQ_REFCLKN"),
                        format!("{corner}C_SQ_HDINP0"),
                        format!("{corner}C_SQ_HDINN0"),
                        format!("{corner}C_SQ_HDINP1"),
                        format!("{corner}C_SQ_HDINN1"),
                        format!("{corner}C_SQ_HDINP2"),
                        format!("{corner}C_SQ_HDINN2"),
                        format!("{corner}C_SQ_HDINP3"),
                        format!("{corner}C_SQ_HDINN3"),
                        format!("{corner}C_SQ_HDOUTP0"),
                        format!("{corner}C_SQ_HDOUTN0"),
                        format!("{corner}C_SQ_HDOUTP1"),
                        format!("{corner}C_SQ_HDOUTN1"),
                        format!("{corner}C_SQ_HDOUTP2"),
                        format!("{corner}C_SQ_HDOUTN2"),
                        format!("{corner}C_SQ_HDOUTP3"),
                        format!("{corner}C_SQ_HDOUTN3"),
                    ],
                );
                for (dx, name) in [
                    (-11, "IN3"),
                    (-10, "OUT3"),
                    (-6, "IN2"),
                    (-5, "OUT2"),
                    (0, "REFCLK"),
                    (5, "IN1"),
                    (6, "OUT1"),
                    (10, "IN0"),
                    (11, "OUT0"),
                ] {
                    for pin in [
                        "JINPUTA_APIO",
                        "JOUTPUTA_APIO",
                        "JCLOCKA_APIO",
                        "JINPUTB_APIO",
                        "JOUTPUTB_APIO",
                        "JCLOCKB_APIO",
                    ] {
                        let wire = self.rc_io_wire(cell_apio.delta(dx, 0), pin);
                        self.add_bel_wire(bcrd, format!("{name}_{pin}"), wire);
                    }
                }
                for (dx, wp, wn) in [
                    (-11, "HDINP3", "HDINN3"),
                    (-6, "HDINP2", "HDINN2"),
                    (0, "REFCLKP", "REFCLKN"),
                    (5, "HDINP1", "HDINN1"),
                    (10, "HDINP0", "HDINN0"),
                ] {
                    let wire_apio = self.rc_io_wire(cell_apio.delta(dx, 0), "JINPUTA_APIO");
                    let wire_pcs = self.rc_wire(cell, &format!("J{wp}_PCS"));
                    self.claim_pip(wire_pcs, wire_apio);
                    self.add_bel_wire(bcrd, wp, wire_pcs);
                    let wire_apio = self.rc_io_wire(cell_apio.delta(dx, 0), "JINPUTB_APIO");
                    let wire_pcs = self.rc_wire(cell, &format!("J{wn}_PCS"));
                    self.claim_pip(wire_pcs, wire_apio);
                    self.add_bel_wire(bcrd, wn, wire_pcs);
                }
                for (dx, wp, wn) in [
                    (-10, "HDOUTP3", "HDOUTN3"),
                    (-5, "HDOUTP2", "HDOUTN2"),
                    (6, "HDOUTP1", "HDOUTN1"),
                    (11, "HDOUTP0", "HDOUTN0"),
                ] {
                    let wire_apio = self.rc_io_wire(cell_apio.delta(dx, 0), "JOUTPUTA_APIO");
                    let wire_pcs = self.rc_wire(cell, &format!("J{wp}_PCS"));
                    self.claim_pip(wire_apio, wire_pcs);
                    self.add_bel_wire(bcrd, wp, wire_pcs);
                    let wire_apio = self.rc_io_wire(cell_apio.delta(dx, 0), "JOUTPUTB_APIO");
                    let wire_pcs = self.rc_wire(cell, &format!("J{wn}_PCS"));
                    self.claim_pip(wire_apio, wire_pcs);
                    self.add_bel_wire(bcrd, wn, wire_pcs);
                }
                self.insert_simple_bel(bcrd, cell, "PCS");
            }
        }
    }

    fn process_serdes_ecp3(&mut self) {
        let tcid = self.intdb.get_tile_class("SERDES");
        for &tcrd in &self.edev.egrid.tile_index[tcid] {
            let bcrd = tcrd.bel(bels::SERDES);
            let bank = self.chip.columns[bcrd.cell.col].bank_s.unwrap();
            let name = match bank {
                50 => "PCSA",
                51 => "PCSB",
                52 => "PCSC",
                53 => "PCSD",
                _ => unreachable!(),
            };
            let cell = bcrd.cell.delta(0, -1);
            self.name_bel(
                bcrd,
                [
                    name.to_string(),
                    format!("{name}_REFCLKP"),
                    format!("{name}_REFCLKN"),
                    format!("{name}_HDINP0"),
                    format!("{name}_HDINN0"),
                    format!("{name}_HDINP1"),
                    format!("{name}_HDINN1"),
                    format!("{name}_HDINP2"),
                    format!("{name}_HDINN2"),
                    format!("{name}_HDINP3"),
                    format!("{name}_HDINN3"),
                    format!("{name}_HDOUTP0"),
                    format!("{name}_HDOUTN0"),
                    format!("{name}_HDOUTP1"),
                    format!("{name}_HDOUTN1"),
                    format!("{name}_HDOUTP2"),
                    format!("{name}_HDOUTN2"),
                    format!("{name}_HDOUTP3"),
                    format!("{name}_HDOUTN3"),
                ],
            );
            for (dx, name) in [
                (0, "IN3"),
                (1, "OUT3"),
                (2, "OUT2"),
                (3, "IN2"),
                (4, "REFCLK"),
                (5, "IN1"),
                (6, "OUT1"),
                (7, "OUT0"),
                (8, "IN0"),
            ] {
                for pin in [
                    "JINPUTA_APIO",
                    "JOUTPUTA_APIO",
                    "JCLOCKA_APIO",
                    "JINPUTB_APIO",
                    "JOUTPUTB_APIO",
                    "JCLOCKB_APIO",
                ] {
                    let wire = self.rc_io_wire(cell.delta(dx, -8), pin);
                    self.add_bel_wire(bcrd, format!("{name}_{pin}"), wire);
                }
            }
            for (dx, wp, wn) in [
                (0, "HDINP3", "HDINN3"),
                (3, "HDINN2", "HDINP2"),
                (4, "REFCLKP", "REFCLKN"),
                (5, "HDINP1", "HDINN1"),
                (8, "HDINN0", "HDINP0"),
            ] {
                let wire_apio = self.rc_io_wire(cell.delta(dx, -8), "JINPUTA_APIO");
                let wire_pcs = self.rc_wire(cell, &format!("J{wp}_PCS"));
                self.claim_pip(wire_pcs, wire_apio);
                self.add_bel_wire(bcrd, wp, wire_pcs);
                let wire_apio = self.rc_io_wire(cell.delta(dx, -8), "JINPUTB_APIO");
                let wire_pcs = self.rc_wire(cell, &format!("J{wn}_PCS"));
                self.claim_pip(wire_pcs, wire_apio);
                self.add_bel_wire(bcrd, wn, wire_pcs);
            }
            for (dx, wp, wn) in [
                (1, "HDOUTP3", "HDOUTN3"),
                (2, "HDOUTN2", "HDOUTP2"),
                (6, "HDOUTP1", "HDOUTN1"),
                (7, "HDOUTN0", "HDOUTP0"),
            ] {
                let wire_apio = self.rc_io_wire(cell.delta(dx, -8), "JOUTPUTA_APIO");
                let wire_pcs = self.rc_wire(cell, &format!("J{wp}_PCS"));
                self.claim_pip(wire_apio, wire_pcs);
                self.add_bel_wire(bcrd, wp, wire_pcs);
                let wire_apio = self.rc_io_wire(cell.delta(dx, -8), "JOUTPUTB_APIO");
                let wire_pcs = self.rc_wire(cell, &format!("J{wn}_PCS"));
                self.claim_pip(wire_apio, wire_pcs);
                self.add_bel_wire(bcrd, wn, wire_pcs);
            }
            let wire = self.rc_wire(cell, "JREFCLK_TO_NQ_PCS");
            self.add_bel_wire(bcrd, "REFCLK_TO_NQ_PCS", wire);
            let wire = self.rc_wire(cell, "JREFCLK_FROM_NQ_PCS");
            self.add_bel_wire(bcrd, "REFCLK_FROM_NQ_PCS", wire);
            if let Some(cell_prev) = self.edev.egrid.cell_delta(bcrd.cell, -36, 0)
                && self.edev.egrid.has_bel(cell_prev.bel(bels::SERDES))
            {
                let wire_prev = self.rc_wire(cell.delta(-36, 0), "JREFCLK_TO_NQ_PCS");
                self.claim_pip(wire, wire_prev);
            }
            self.insert_simple_bel(bcrd, cell, "PCS");
        }
    }

    pub fn process_serdes(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp2M => {
                self.process_serdes_ecp2();
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.process_serdes_ecp3();
            }
            _ => (),
        }
    }
}

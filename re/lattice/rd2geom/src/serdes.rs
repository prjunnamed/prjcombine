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

    pub fn process_serdes(&mut self) {
        #[allow(clippy::single_match)]
        match self.chip.kind {
            ChipKind::Ecp2M => {
                self.process_serdes_ecp2();
            }
            _ => (),
        }
    }
}

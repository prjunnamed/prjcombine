use prjcombine_ecp::chip::{ChipKind, SpecialIoKey};
use prjcombine_interconnect::{dir::Dir, grid::EdgeIoCoord};
use prjcombine_re_lattice_naming::WireName;
use unnamed_entity::EntityId;

use crate::ChipContext;

mod ecp;
mod ecp3;
mod machxo;
mod machxo2;

impl ChipContext<'_> {
    pub fn process_io(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => {
                self.process_dqsdll_ecp();
                self.process_io_ecp();
            }
            ChipKind::MachXo => self.process_io_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => {
                self.process_eclk_ecp2();
                self.process_eclk_tap_ecp2();
                self.process_dqsdll_ecp2();
                self.process_io_ecp();
            }
            ChipKind::Xp2 => {
                self.process_eclk_xp2();
                self.process_eclk_tap_ecp2();
                self.process_dqsdll_ecp2();
                self.process_io_ecp();
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.process_eclk_ecp3();
                self.process_eclk_tap_ecp3();
                self.process_dqsdll_ecp3();
                self.process_io_ecp3();
            }
            ChipKind::MachXo2(_) => {
                self.process_bc_machxo2();
                self.process_eclk_machxo2();
                self.process_dqsdll_machxo2();
                self.process_dqs_machxo2();
                self.process_io_machxo2();
                self.process_icc_machxo2();
            }
        }
    }

    pub fn get_io_wire_in(&self, io: EdgeIoCoord) -> WireName {
        let bel = self.chip.get_io_loc(io);
        match self.chip.kind {
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                if io.iob().to_idx() >= 4 {
                    let cell = match io.edge() {
                        Dir::W => bel.cell.delta(-1, 0),
                        Dir::E => bel.cell.delta(1, 0),
                        _ => unreachable!(),
                    };
                    let abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx() - 4];
                    self.rc_io_wire(cell, &format!("JPADDIE{abcd}_PIO"))
                } else if io.iob().to_idx() >= 2 {
                    let cell = match io.edge() {
                        Dir::H(_) => bel.cell,
                        Dir::V(_) => bel.cell.delta(2, 0),
                    };
                    let abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx() - 2];
                    self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"))
                } else {
                    let cell = match io.edge() {
                        Dir::H(_) => bel.cell.delta(0, 2),
                        Dir::V(_) => bel.cell,
                    };
                    let abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx()];
                    self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"))
                }
            }
            _ => {
                let abcd = ['A', 'B', 'C', 'D', 'E', 'F'][io.iob().to_idx()];
                self.rc_io_wire(bel.cell, &format!("JPADDI{abcd}_PIO"))
            }
        }
    }

    pub fn get_special_io_wire_in(&self, key: SpecialIoKey) -> WireName {
        self.get_io_wire_in(self.chip.special_io[&key])
    }

    pub fn get_io_wire_out(&self, io: EdgeIoCoord) -> WireName {
        let bel = self.chip.get_io_loc(io);
        let abcd = ['A', 'B', 'C', 'D', 'E', 'F'][io.iob().to_idx()];
        self.rc_io_wire(bel.cell, &format!("JPADDO{abcd}_PIO"))
    }

    pub fn get_special_io_wire_out(&self, key: SpecialIoKey) -> WireName {
        self.get_io_wire_out(self.chip.special_io[&key])
    }
}

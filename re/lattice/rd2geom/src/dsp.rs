use prjcombine_ecp::chip::ChipKind;

use crate::ChipContext;

mod ecp;
mod ecp3;
mod ecp4;

impl ChipContext<'_> {
    pub fn process_dsp(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                self.process_dsp_ecp()
            }
            ChipKind::Xp | ChipKind::MachXo | ChipKind::MachXo2(_) | ChipKind::Crosslink => (),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.process_dsp_ecp3(),
            ChipKind::Ecp4 | ChipKind::Ecp5 => self.process_dsp_ecp4(),
        }
    }
}

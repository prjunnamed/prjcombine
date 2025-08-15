use prjcombine_ecp::chip::ChipKind;

use crate::ChipContext;

mod crosslink;
mod ecp;
mod ecp2;
mod ecp3;
mod ecp4;
mod ecp5;
mod machxo;
mod machxo2;
mod xp2;

impl ChipContext<'_> {
    pub fn process_pll(&mut self) {
        match self.chip.kind {
            ChipKind::Scm => {
                // handled in io
            }
            ChipKind::Ecp | ChipKind::Xp => self.process_pll_ecp(),
            ChipKind::MachXo => self.process_pll_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.process_pll_ecp2(),
            ChipKind::Xp2 => {
                self.process_pll_xp2();
                self.process_clkdiv_xp2();
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.process_pll_ecp3();
                self.process_dll_ecp3();
                self.process_clkdiv_ecp3();
            }
            ChipKind::MachXo2(_) => self.process_pll_machxo2(),
            ChipKind::Ecp4 => self.process_pll_ecp4(),
            ChipKind::Ecp5 => self.process_pll_ecp5(),
            ChipKind::Crosslink => self.process_pll_crosslink(),
        }
    }
}

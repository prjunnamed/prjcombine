use prjcombine_ecp::{bels, chip::ChipKind};

use crate::ChipContext;

impl ChipContext<'_> {
    pub fn process_ebr(&mut self) {
        let tcid = self.intdb.get_tile_class("EBR");
        for &tcrd in &self.edev.egrid.tile_index[tcid] {
            let bcrd = tcrd.bel(bels::EBR0);
            let cell = if self.chip.kind == ChipKind::MachXo {
                tcrd.cell.delta(0, 3)
            } else {
                tcrd.cell
            };
            let (r, c) = self.rc(cell);
            self.name_bel(bcrd, [format!("EBR_R{r}C{c}")]);
            self.insert_simple_bel(bcrd, cell, "EBR");
        }
    }
}

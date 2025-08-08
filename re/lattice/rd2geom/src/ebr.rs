use prjcombine_ecp::{bels, chip::ChipKind};

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_ebr_ecp(&mut self) {
        let tiles = if matches!(self.chip.kind, ChipKind::MachXo2(_)) {
            ["EBR", "EBR_N"].as_slice()
        } else {
            ["EBR"].as_slice()
        };
        for &tcname in tiles {
            let tcid = self.intdb.get_tile_class(tcname);
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

    fn process_ebr_ecp4(&mut self) {
        let tcid = self.intdb.get_tile_class("EBR");
        for &tcrd in &self.edev.egrid.tile_index[tcid] {
            for i in 0..4 {
                let bcrd = tcrd.bel(bels::EBR[i]);
                let cell = tcrd.delta(2 * (i as i32), 0);
                let (r, c) = self.rc(cell);
                self.name_bel(bcrd, [format!("EBR_R{r}C{c}")]);
                self.insert_simple_bel(bcrd, cell, "EBR");
            }
        }
    }

    pub fn process_ebr(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp
            | ChipKind::Xp
            | ChipKind::MachXo
            | ChipKind::Ecp2
            | ChipKind::Ecp2M
            | ChipKind::Xp2
            | ChipKind::Ecp3
            | ChipKind::Ecp3A
            | ChipKind::MachXo2(_) => self.process_ebr_ecp(),
            ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink => self.process_ebr_ecp4(),
        }
    }
}

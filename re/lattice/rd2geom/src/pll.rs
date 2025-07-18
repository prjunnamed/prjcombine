use prjcombine_ecp::{
    bels,
    chip::{ChipKind, SpecialIoKey},
};
use prjcombine_interconnect::dir::DirHV;

use crate::ChipContext;

impl ChipContext<'_> {
    pub fn process_pll_ecp(&mut self) {
        for (&loc, &cell) in &self.edev.plls {
            let bcrd = cell.bel(bels::PLL);
            let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
            let (r, c) = self.rc(cell);
            self.name_bel(bcrd, [format!("PLL3_R{r}C{c}")]);
            let mut bel = self.extract_simple_bel(bcrd, cell, "PLL3");

            let clki = self.rc_wire(cell, "CLKI");
            let clki0 = self.rc_wire(cell, "JCLKI0");
            let clki1 = self.rc_wire(cell, "JCLKI1");
            let clki2 = self.rc_wire(cell, "JCLKI2");
            let clki3 = self.rc_wire(cell, "JCLKI3");
            let clki_pll = self.rc_wire(cell, "CLKI_PLL3");
            self.add_bel_wire(bcrd, "CLKI", clki);
            self.add_bel_wire(bcrd, "CLKI0", clki0);
            self.add_bel_wire(bcrd, "CLKI1", clki1);
            self.add_bel_wire(bcrd, "CLKI2", clki2);
            self.add_bel_wire(bcrd, "CLKI3", clki3);
            self.add_bel_wire(bcrd, "CLKI_PLL", clki_pll);
            bel.pins
                .insert("CLKI0".into(), self.xlat_int_wire(tcrd, clki0).unwrap());
            bel.pins
                .insert("CLKI1".into(), self.xlat_int_wire(tcrd, clki1).unwrap());
            bel.pins
                .insert("CLKI2".into(), self.xlat_int_wire(tcrd, clki2).unwrap());
            let wire_io = self.get_special_io_wire(SpecialIoKey::PllIn(loc));
            self.claim_pip(clki3, wire_io);
            self.claim_pip(clki, clki0);
            self.claim_pip(clki, clki1);
            self.claim_pip(clki, clki2);
            self.claim_pip(clki, clki3);
            self.claim_pip(clki_pll, clki);

            let clkop_pll = self.rc_wire(cell, "CLKOP_PLL3");
            let clkop = self.rc_wire(cell, "JCLKOP");
            self.add_bel_wire(bcrd, "CLKOP_PLL", clkop_pll);
            self.add_bel_wire(bcrd, "CLKOP", clkop);
            self.claim_pip(clkop, clkop_pll);
            bel.pins
                .insert("CLKOP".into(), self.xlat_int_wire(tcrd, clkop).unwrap());

            let clkfb = self.rc_wire(cell, "CLKFB");
            let clkfb0 = self.rc_wire(cell, "JCLKFB0");
            let clkfb1 = self.rc_wire(cell, "JCLKFB1");
            let clkfb3 = self.rc_wire(cell, "JCLKFB3");
            let clkfb_pll = self.rc_wire(cell, "CLKFB_PLL3");
            self.add_bel_wire(bcrd, "CLKFB", clkfb);
            self.add_bel_wire(bcrd, "CLKFB0", clkfb0);
            self.add_bel_wire(bcrd, "CLKFB1", clkfb1);
            self.add_bel_wire(bcrd, "CLKFB3", clkfb3);
            self.add_bel_wire(bcrd, "CLKFB_PLL", clkfb_pll);
            bel.pins
                .insert("CLKFB0".into(), self.xlat_int_wire(tcrd, clkfb0).unwrap());
            bel.pins
                .insert("CLKFB1".into(), self.xlat_int_wire(tcrd, clkfb1).unwrap());
            let wire_io = self.get_special_io_wire(SpecialIoKey::PllFb(loc));
            self.claim_pip(clkfb3, wire_io);
            self.claim_pip(clkfb, clkfb0);
            self.claim_pip(clkfb, clkfb1);
            self.claim_pip(clkfb, clkop);
            self.claim_pip(clkfb, clkfb3);
            self.claim_pip(clkfb_pll, clkfb);

            self.insert_bel(bcrd, bel);
        }
    }

    pub fn process_pll_machxo(&mut self) {
        for tcname in ["PLL_S", "PLL_N"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.egrid.tile_index[tcid] {
                let cell = tcrd.cell;
                let bcrd = cell.bel(bels::PLL);
                let (r, c) = self.rc(tcrd.cell);
                self.name_bel(bcrd, [format!("PLL3_R{r}C{c}")]);
                let mut bel = self.extract_simple_bel(bcrd, cell, "PLL");

                let pll_loc = if tcname == "PLL_S" {
                    DirHV::SW
                } else {
                    DirHV::NW
                };

                let clki = self.rc_wire(cell, "CLKI");
                let clki0 = self.rc_wire(cell, "JCLKI0");
                let clki1 = self.rc_wire(cell, "JCLKI1");
                let clki2 = self.rc_wire(cell, "JCLKI2");
                let clki3 = self.rc_wire(cell, "JCLKI3");
                let clki_pll = self.rc_wire(cell, "CLKI_PLL");
                self.add_bel_wire(bcrd, "CLKI", clki);
                self.add_bel_wire(bcrd, "CLKI0", clki0);
                self.add_bel_wire(bcrd, "CLKI1", clki1);
                self.add_bel_wire(bcrd, "CLKI2", clki2);
                self.add_bel_wire(bcrd, "CLKI3", clki3);
                self.add_bel_wire(bcrd, "CLKI_PLL", clki_pll);
                bel.pins
                    .insert("CLKI0".into(), self.xlat_int_wire(tcrd, clki0).unwrap());
                bel.pins
                    .insert("CLKI1".into(), self.xlat_int_wire(tcrd, clki1).unwrap());
                bel.pins
                    .insert("CLKI2".into(), self.xlat_int_wire(tcrd, clki2).unwrap());
                let wire_io = self.get_special_io_wire(SpecialIoKey::PllIn(pll_loc));
                self.claim_pip(clki3, wire_io);
                self.claim_pip(clki, clki0);
                self.claim_pip(clki, clki1);
                self.claim_pip(clki, clki2);
                self.claim_pip(clki, clki3);
                self.claim_pip(clki_pll, clki);

                let clkintfb_pll = self.rc_wire(cell, "CLKINTFB_PLL");
                let clkintfb = self.rc_wire(cell, "CLKINTFB");
                let clkfb = self.rc_wire(cell, "CLKFB");
                let clkfb0 = self.rc_wire(cell, "JCLKFB0");
                let clkfb1 = self.rc_wire(cell, "JCLKFB1");
                let clkfb3 = self.rc_wire(cell, "JCLKFB3");
                let clkfb_pll = self.rc_wire(cell, "CLKFB_PLL");
                self.add_bel_wire(bcrd, "CLKINTFB", clkintfb);
                self.add_bel_wire(bcrd, "CLKINTFB_PLL", clkintfb_pll);
                self.add_bel_wire(bcrd, "CLKFB", clkfb);
                self.add_bel_wire(bcrd, "CLKFB0", clkfb0);
                self.add_bel_wire(bcrd, "CLKFB1", clkfb1);
                self.add_bel_wire(bcrd, "CLKFB3", clkfb3);
                self.add_bel_wire(bcrd, "CLKFB_PLL", clkfb_pll);
                bel.pins
                    .insert("CLKFB0".into(), self.xlat_int_wire(tcrd, clkfb0).unwrap());
                bel.pins
                    .insert("CLKFB1".into(), self.xlat_int_wire(tcrd, clkfb1).unwrap());
                let wire_io = self.get_special_io_wire(SpecialIoKey::PllFb(pll_loc));
                self.claim_pip(clkfb3, wire_io);
                self.claim_pip(clkintfb, clkintfb_pll);
                self.claim_pip(clkfb, clkfb0);
                self.claim_pip(clkfb, clkfb1);
                self.claim_pip(clkfb, clkintfb);
                self.claim_pip(clkfb, clkfb3);
                self.claim_pip(clkfb_pll, clkfb);

                self.insert_bel(bcrd, bel);
            }
        }
    }

    pub fn process_pll(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => self.process_pll_ecp(),
            ChipKind::MachXo => self.process_pll_machxo(),
        }
    }
}

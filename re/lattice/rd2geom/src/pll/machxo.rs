use prjcombine_ecp::{
    bels,
    chip::{PllLoc, PllPad, SpecialIoKey},
};
use prjcombine_interconnect::dir::DirHV;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pll_machxo(&mut self) {
        for tcname in ["PLL_S", "PLL_N"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                let cell = tcrd.cell;
                let bcrd = cell.bel(bels::PLL0);
                let (r, c) = self.rc(tcrd.cell);
                self.name_bel(bcrd, [format!("PLL3_R{r}C{c}")]);
                let mut bel = self.extract_simple_bel(bcrd, cell, "PLL");

                let pll_loc = if tcname == "PLL_S" {
                    PllLoc::new(DirHV::SW, 0)
                } else {
                    PllLoc::new(DirHV::NW, 0)
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
                    .insert("CLKI0".into(), self.xlat_int_wire(bcrd, clki0));
                bel.pins
                    .insert("CLKI1".into(), self.xlat_int_wire(bcrd, clki1));
                bel.pins
                    .insert("CLKI2".into(), self.xlat_int_wire(bcrd, clki2));
                let wire_io =
                    self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn0, pll_loc));
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
                    .insert("CLKFB0".into(), self.xlat_int_wire(bcrd, clkfb0));
                bel.pins
                    .insert("CLKFB1".into(), self.xlat_int_wire(bcrd, clkfb1));
                let wire_io =
                    self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllFb, pll_loc));
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
}

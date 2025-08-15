use prjcombine_ecp::{
    bels,
    chip::{PllLoc, PllPad, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::dir::{Dir, DirH, DirHV, DirV};

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pll_xp2(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            let bcrd = cell.bel(bels::PLL0);
            self.name_bel(
                bcrd,
                [match loc.quad {
                    DirHV::SW => "LLPLL",
                    DirHV::SE => "LRPLL",
                    DirHV::NW => "ULPLL",
                    DirHV::NE => "URPLL",
                }],
            );
            let mut bel = self.extract_simple_bel(bcrd, cell, "PLL");

            let io_pll_in0 = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn0, loc));
            let io_pll_fb = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllFb, loc));

            let bel_eclk_root = self.chip.bel_eclk_root(Dir::H(loc.quad.h));
            let eclk0_in = self.naming.bel_wire(bel_eclk_root, "ECLK0_IN");

            let clki = self.rc_wire(cell, "CLKI");
            let clki0 = self.rc_wire(cell, "JPLLCLKI0");
            let clki1 = self.rc_wire(cell, "JPLLCLKI1");
            let clki2 = self.rc_wire(cell, "JPLLCLKI2");
            let clki3 = self.rc_wire(cell, "JPLLCLKI3");
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
            self.claim_pip(clki3, io_pll_in0);
            self.claim_pip(clki, clki0);
            self.claim_pip(clki, clki1);
            self.claim_pip(clki, clki2);
            self.claim_pip(clki, clki3);
            self.claim_pip(clki_pll, clki);

            let clkfb = self.rc_wire(cell, "CLKFB");
            let clkfb0 = self.rc_wire(cell, "JPLLCLKFB0");
            let clkfb1 = self.rc_wire(cell, "JPLLCLKFB1");
            let clkfb3 = self.rc_wire(cell, "JPLLCLKFB3");
            let clkfb_pll = self.rc_wire(cell, "CLKFB_PLL");
            let clkintfb = self.rc_wire(cell, "PLLCLKINTFB");
            let clkintfb_pll = self.rc_wire(cell, "CLKINTFB_PLL");
            self.add_bel_wire(bcrd, "CLKFB", clkfb);
            self.add_bel_wire(bcrd, "CLKFB0", clkfb0);
            self.add_bel_wire(bcrd, "CLKFB1", clkfb1);
            self.add_bel_wire(bcrd, "CLKFB3", clkfb3);
            self.add_bel_wire(bcrd, "CLKFB_PLL", clkfb_pll);
            self.add_bel_wire(bcrd, "CLKINTFB", clkintfb);
            self.add_bel_wire(bcrd, "CLKINTFB_PLL", clkintfb_pll);
            bel.pins
                .insert("CLKFB0".into(), self.xlat_int_wire(bcrd, clkfb0));
            self.claim_pip(clkfb1, io_pll_fb);
            self.claim_pip(clkfb3, eclk0_in);
            self.claim_pip(clkfb, clkfb0);
            self.claim_pip(clkfb, clkfb1);
            self.claim_pip(clkfb, clkfb3);
            self.claim_pip(clkfb, clkintfb);
            self.claim_pip(clkfb_pll, clkfb);
            self.claim_pip(clkintfb, clkintfb_pll);

            self.insert_bel(bcrd, bel);
        }
    }

    pub(super) fn process_clkdiv_xp2(&mut self) {
        for edge in [DirH::W, DirH::E] {
            let bcrd = self.chip.bel_dqsdll_ecp2(edge).bel(bels::CLKDIV0);
            let cell = bcrd.cell;
            self.name_bel(
                bcrd,
                [match edge {
                    DirH::W => "LCLKDIV",
                    DirH::E => "RCLKDIV",
                }],
            );
            self.insert_simple_bel(bcrd, cell, "CLKDIV");

            let bel_eclk_root = self.chip.bel_eclk_root(Dir::H(edge));
            let eclk0_in = self.naming.bel_wire(bel_eclk_root, "ECLK0_IN");
            let eclk1_in = self.naming.bel_wire(bel_eclk_root, "ECLK1_IN");

            let eclk0 = self.rc_wire(cell, "JFRC0");
            let eclk1 = self.rc_wire(cell, "JFRC1");
            self.add_bel_wire(bcrd, "ECLK0", eclk0);
            self.add_bel_wire(bcrd, "ECLK1", eclk1);
            self.claim_pip(eclk0, eclk0_in);
            self.claim_pip(eclk1, eclk1_in);

            let clki_in = self.rc_wire(cell, "CLKDIVCLKI");
            self.add_bel_wire(bcrd, "CLKI_IN", clki_in);
            self.claim_pip(clki_in, eclk0);
            self.claim_pip(clki_in, eclk1);

            for v in [DirV::S, DirV::N] {
                let pll_loc = PllLoc::new_hv(edge, v, 0);
                if let Some(&cell_pll) = self.chip.special_loc.get(&SpecialLocKey::Pll(pll_loc)) {
                    let clkop_in = self.rc_wire(cell_pll, "JCLKOP_PLL");
                    let idx = match pll_loc.quad {
                        DirHV::SW => 1,
                        DirHV::SE => 0,
                        DirHV::NW => 0,
                        DirHV::NE => 1,
                    };
                    let clkop = self.rc_wire(cell, &format!("JCLKOP{idx}"));
                    self.add_bel_wire(
                        bcrd,
                        format!("PLL_{quad}_CLKOP", quad = pll_loc.quad),
                        clkop,
                    );
                    self.claim_pip(clkop, clkop_in);
                    self.claim_pip(clki_in, clkop);
                }
            }

            let clki = self.rc_wire(cell, "CLKI_CLKDIV");
            self.add_bel_wire(bcrd, "CLKI", clki);
            self.claim_pip(clki, clki_in);
        }
    }
}

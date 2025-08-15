use prjcombine_ecp::{
    bels,
    chip::{PllLoc, PllPad, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::Bel,
    dir::{Dir, DirHV},
};

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pll_crosslink(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(DirHV::SE, 0))];

        let bcrd_pllrefcs = cell.bel(bels::PLLREFCS0);

        self.name_bel(bcrd_pllrefcs, ["PLLREFCS"]);
        let mut bel_pllrefcs = Bel::default();

        let sel = self.rc_wire(cell, "JSEL_PLLREFCS");
        self.add_bel_wire(bcrd_pllrefcs, "SEL", sel);
        bel_pllrefcs
            .pins
            .insert("SEL".into(), self.xlat_int_wire(bcrd_pllrefcs, sel));

        let pllcsout = self.rc_wire(cell, "PLLCSOUT_PLLREFCS");
        self.add_bel_wire(bcrd_pllrefcs, "PLLCSOUT", pllcsout);

        for i in 0..2 {
            let clk_in = self.rc_wire(cell, &format!("REFCLK{i}"));
            self.add_bel_wire(bcrd_pllrefcs, format!("CLK{i}_IN"), clk_in);

            let clk_int = self.rc_wire(cell, &format!("JREFCLK{i}_1"));
            self.add_bel_wire(bcrd_pllrefcs, format!("CLK{i}_INT"), clk_int);
            bel_pllrefcs.pins.insert(
                format!("CLK{i}"),
                self.xlat_int_wire(bcrd_pllrefcs, clk_int),
            );
            self.claim_pip(clk_in, clk_int);

            let io_sources: &[_] = match i {
                0 => &[
                    (2, SpecialIoKey::Clock(Dir::S, 2)),
                    (3, SpecialIoKey::Clock(Dir::S, 3)),
                    (4, SpecialIoKey::Clock(Dir::S, 4)),
                    (
                        5,
                        SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::SE, 0)),
                    ),
                ],
                1 => &[
                    (2, SpecialIoKey::Clock(Dir::S, 0)),
                    (3, SpecialIoKey::Clock(Dir::S, 1)),
                    (4, SpecialIoKey::Clock(Dir::S, 5)),
                ],
                _ => unreachable!(),
            };

            for &(j, key) in io_sources {
                let clk_io = self.rc_wire(cell, &format!("JREFCLK{i}_{j}"));
                self.add_bel_wire(bcrd_pllrefcs, format!("REFCLK{i}_{j}"), clk_io);
                let io = self.chip.special_io[&key];
                let (cell_io, abcd) = self.xlat_io_loc_crosslink(io);
                let paddi_pio = self.rc_io_wire(cell_io, &format!("JPADDI{abcd}_PIO"));
                self.claim_pip(clk_io, paddi_pio);
                self.claim_pip(clk_in, clk_io);
            }

            let clk = self.rc_wire(cell, &format!("CLK{i}_PLLREFCS"));
            self.add_bel_wire(bcrd_pllrefcs, format!("CLK{i}"), clk);
            self.claim_pip(clk, clk_in);
            self.claim_pip(pllcsout, clk);
        }

        self.insert_bel(bcrd_pllrefcs, bel_pllrefcs);

        // actual PLL

        let bcrd = cell.bel(bels::PLL0);
        self.name_bel(bcrd, ["PLL"]);
        let mut bel = self.extract_simple_bel(bcrd, cell, "PLL");

        let clki = self.rc_wire(cell, "CLKI_PLL");
        self.add_bel_wire(bcrd, "CLKI", clki);
        self.claim_pip(clki, pllcsout);

        for pin in ["REFCLK", "CLKOP", "CLKOS", "CLKOS2", "CLKOS3"] {
            let wire = self.rc_wire(cell, &format!("J{pin}_PLL"));
            self.claim_pip(wire, clki);
        }

        let clkintfb = self.rc_wire(cell, "CLKINTFB_PLL");
        self.add_bel_wire(bcrd, "CLKINTFB", clkintfb);
        let clkintfb_out = self.rc_wire(cell, "CLKINTFB");
        self.add_bel_wire(bcrd, "CLKINTFB_OUT", clkintfb_out);
        self.claim_pip(clkintfb_out, clkintfb);

        let clkfb_int = self.rc_wire(cell, "JCLKFB2");
        self.add_bel_wire(bcrd, "CLKFB_INT", clkfb_int);
        bel.pins
            .insert("CLKFB".into(), self.xlat_int_wire(bcrd, clkfb_int));

        let clkfb_eclk = self.rc_wire(cell, "JCLKFB1");
        self.add_bel_wire(bcrd, "CLKFB_ECLK", clkfb_eclk);

        let cell_eclk = cell.with_col(self.chip.col_clk - 1);
        let wire_eclk = self.rc_io_wire(cell_eclk, "JECLKFB");
        self.claim_pip(clkfb_eclk, wire_eclk);

        let clkfb_in = self.rc_wire(cell, "CLKFB");
        self.add_bel_wire(bcrd, "CLKFB_IN", clkfb_in);
        self.claim_pip(clkfb_in, clkintfb_out);
        self.claim_pip(clkfb_in, clkfb_int);
        self.claim_pip(clkfb_in, clkfb_eclk);

        let clkfb = self.rc_wire(cell, "CLKFB_PLL");
        self.add_bel_wire(bcrd, "CLKFB", clkfb);
        self.claim_pip(clkfb, clkfb_in);

        self.insert_bel(bcrd, bel);
    }
}

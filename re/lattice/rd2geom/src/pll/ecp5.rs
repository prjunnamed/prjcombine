use prjcombine_ecp::{
    bels,
    chip::{PllLoc, PllPad, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::LegacyBel,
    dir::{Dir, DirH, DirHV, DirV},
};

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pll_ecp5(&mut self) {
        for hv in DirHV::DIRS {
            let Some(&cell) = self
                .chip
                .special_loc
                .get(&SpecialLocKey::Pll(PllLoc::new(hv, 0)))
            else {
                continue;
            };
            let corner = match hv {
                DirHV::SW => "BL",
                DirHV::SE => "BR",
                DirHV::NW => "TL",
                DirHV::NE => "TR",
            };

            let bcrd_pllrefcs = cell.bel(bels::PLLREFCS0);

            self.name_bel(bcrd_pllrefcs, [format!("PLLREFCS_{corner}0")]);
            let mut bel_pllrefcs = LegacyBel::default();

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

                let clk_int = self.rc_wire(cell, &format!("JREFCLK{i}_0"));
                self.add_bel_wire(bcrd_pllrefcs, format!("CLK{i}_INT"), clk_int);
                bel_pllrefcs.pins.insert(
                    format!("CLK{i}"),
                    self.xlat_int_wire(bcrd_pllrefcs, clk_int),
                );
                self.claim_pip(clk_in, clk_int);

                let mut io_sources =
                    vec![(3, SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(hv, 0)))];
                match hv.v {
                    DirV::S => {
                        io_sources.extend([
                            (1, SpecialIoKey::Clock(Dir::H(hv.h), 0)),
                            (2, SpecialIoKey::Clock(Dir::H(hv.h), 1)),
                        ]);
                    }
                    DirV::N => {
                        io_sources.extend([
                            (1, SpecialIoKey::Clock(Dir::H(hv.h), 2)),
                            (2, SpecialIoKey::Clock(Dir::H(hv.h), 3)),
                            (4, SpecialIoKey::Pll(PllPad::PllIn1, PllLoc::new(hv, 0))),
                            (
                                5,
                                SpecialIoKey::Clock(
                                    Dir::N,
                                    match hv.h {
                                        DirH::W => 1,
                                        DirH::E => 3,
                                    },
                                ),
                            ),
                            (
                                6,
                                SpecialIoKey::Clock(
                                    Dir::N,
                                    match hv.h {
                                        DirH::W => 0,
                                        DirH::E => 2,
                                    },
                                ),
                            ),
                        ]);
                    }
                }

                for &(j, key) in &io_sources {
                    let clk_io = self.rc_wire(cell, &format!("JREFCLK{i}_{j}"));
                    self.add_bel_wire(bcrd_pllrefcs, format!("REFCLK{i}_{j}"), clk_io);
                    let io = self.chip.special_io[&key];
                    let (cell_io, abcd) = self.xlat_io_loc_ecp5(io);
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
            self.name_bel(bcrd, [format!("PLL_{corner}0")]);
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

            let clkfb_int = self.rc_wire(cell, "JCLKFB3");
            self.add_bel_wire(bcrd, "CLKFB_INT", clkfb_int);
            bel.pins
                .insert("CLKFB".into(), self.xlat_int_wire(bcrd, clkfb_int));

            let clkfb_in = self.rc_wire(cell, "CLKFB");
            self.add_bel_wire(bcrd, "CLKFB_IN", clkfb_in);
            self.claim_pip(clkfb_in, clkintfb_out);
            self.claim_pip(clkfb_in, clkfb_int);

            let bank = match hv {
                DirHV::SW => 6,
                DirHV::SE => 3,
                DirHV::NW => 7,
                DirHV::NE => 2,
            };
            for j in 0..2 {
                let i = 1 + j;
                let clkfb_eclk = self.rc_wire(cell, &format!("JCLKFB{i}"));
                self.add_bel_wire(bcrd, format!("CLKFB{i}"), clkfb_eclk);
                let bcrd_eclk = self.chip.bel_eclksync_bank(bank, j);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK_MUX");
                self.claim_pip(clkfb_eclk, wire_eclk);
                self.claim_pip(clkfb_in, clkfb_eclk);
            }

            let clkfb = self.rc_wire(cell, "CLKFB_PLL");
            self.add_bel_wire(bcrd, "CLKFB", clkfb);
            self.claim_pip(clkfb, clkfb_in);

            self.insert_bel(bcrd, bel);
        }
    }
}

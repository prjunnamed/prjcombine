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
    pub(super) fn process_pll_ecp4(&mut self) {
        for hv in DirHV::DIRS {
            let cell_tile = self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(hv, 0))];
            let corner = match hv {
                DirHV::SW => "BL",
                DirHV::SE => "BR",
                DirHV::NW => "TL",
                DirHV::NE => "TR",
            };
            for idx in 0..2 {
                let cell = match hv.v {
                    DirV::S => cell_tile.delta(0, 2 - idx as i32),
                    DirV::N => cell_tile.delta(0, -1 - idx as i32),
                };

                let bcrd_pllrefcs = cell_tile.bel(bels::PLLREFCS[idx]);
                self.name_bel(bcrd_pllrefcs, [format!("PLLREFCS_{corner}{idx}")]);
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

                    for j in [2, 3] {
                        let clk_int = self.rc_wire(cell, &format!("JREFCLK{i}_{j}"));
                        self.add_bel_wire(bcrd_pllrefcs, format!("REFCLK{i}_{j}"), clk_int);
                        bel_pllrefcs.pins.insert(
                            format!("REFCLK{i}_{j}"),
                            self.xlat_int_wire(bcrd_pllrefcs, clk_int),
                        );
                        self.claim_pip(clk_in, clk_int);
                    }

                    let mut io_sources = vec![
                        (0, SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(hv, 0))),
                        (1, SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(hv, 1))),
                    ];
                    io_sources.extend(match hv.v {
                        DirV::S => [
                            (4, SpecialIoKey::Clock(Dir::H(hv.h), 0)),
                            (5, SpecialIoKey::Clock(Dir::H(hv.h), 1)),
                        ],
                        DirV::N => [
                            (4, SpecialIoKey::Clock(Dir::H(hv.h), 2)),
                            (5, SpecialIoKey::Clock(Dir::H(hv.h), 3)),
                        ],
                    });
                    if hv.v == DirV::N {
                        let has_bank0 = self.chip.special_loc.contains_key(&SpecialLocKey::Bc(0));
                        io_sources.extend(match hv.h {
                            DirH::W => [
                                (
                                    6,
                                    SpecialIoKey::Clock(
                                        Dir::N,
                                        if has_bank0 && i == 0 { 0 } else { 2 },
                                    ),
                                ),
                                (
                                    7,
                                    SpecialIoKey::Clock(
                                        Dir::N,
                                        if has_bank0 && i == 0 { 1 } else { 3 },
                                    ),
                                ),
                            ],
                            DirH::E => [
                                (
                                    6,
                                    SpecialIoKey::Clock(
                                        Dir::N,
                                        if has_bank0 && i == 0 { 6 } else { 4 },
                                    ),
                                ),
                                (
                                    7,
                                    SpecialIoKey::Clock(
                                        Dir::N,
                                        if has_bank0 && i == 0 { 7 } else { 5 },
                                    ),
                                ),
                            ],
                        });
                    }

                    for &(j, key) in &io_sources {
                        let clk_io = self.rc_wire(cell, &format!("JREFCLK{i}_{j}"));
                        self.add_bel_wire(bcrd_pllrefcs, format!("REFCLK{i}_{j}"), clk_io);
                        let io = self.chip.special_io[&key];
                        let (cell_io, abcd) = self.xlat_io_loc_ecp4(io);
                        let paddi_pio = self.rc_io_wire(cell_io, &format!("JPADDI{abcd}_PIO"));
                        self.claim_pip(clk_io, paddi_pio);
                        self.claim_pip(clk_in, clk_io);
                    }

                    if hv == DirHV::SW {
                        let clk_osc = self.rc_wire(cell, &format!("JREFCLK{i}_6"));
                        self.add_bel_wire(bcrd_pllrefcs, format!("REFCLK{i}_6"), clk_osc);
                        self.claim_pip(clk_in, clk_osc);
                        let cell_osc = self.chip.special_loc[&SpecialLocKey::Config];
                        let wire_osc = self.rc_wire(cell_osc, "JOSC_OSC");
                        self.claim_pip(clk_osc, wire_osc);

                        let clk_asb = self.rc_wire(cell, &format!("JREFCLK{i}_7"));
                        self.add_bel_wire(bcrd_pllrefcs, format!("REFCLK{i}_7"), clk_asb);
                        self.claim_pip(clk_in, clk_asb);
                        let wire_asb = self.rc_io_sn_wire(cell_tile, "JQ0P_CORECLK_ASB");
                        self.claim_pip(clk_asb, wire_asb);
                    }

                    let clk = self.rc_wire(cell, &format!("CLK{i}_PLLREFCS"));
                    self.add_bel_wire(bcrd_pllrefcs, format!("CLK{i}"), clk);
                    self.claim_pip(clk, clk_in);
                    self.claim_pip(pllcsout, clk);
                }

                self.insert_bel(bcrd_pllrefcs, bel_pllrefcs);

                // actual PLL

                let bcrd = cell_tile.bel(bels::PLL[idx]);
                self.name_bel(bcrd, [format!("PLL_{corner}{idx}")]);
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

                let clkfb2 = self.rc_wire(cell, "JCLKFB2");
                self.add_bel_wire(bcrd, "CLKFB2", clkfb2);
                bel.pins
                    .insert("CLKFB2".into(), self.xlat_int_wire(bcrd, clkfb2));

                let clkfb_in = self.rc_wire(cell, "CLKFB");
                self.add_bel_wire(bcrd, "CLKFB_IN", clkfb_in);
                self.claim_pip(clkfb_in, clkintfb_out);
                self.claim_pip(clkfb_in, clkfb2);

                for i in 0..2 {
                    let clkfb_io = self.rc_wire(cell, &format!("JCLKFB{i}"));
                    self.add_bel_wire(bcrd, format!("CLKFB{i}"), clkfb_io);
                    let io =
                        self.chip.special_io[&SpecialIoKey::Pll(PllPad::PllFb, PllLoc::new(hv, i))];
                    let (cell_io, abcd) = self.xlat_io_loc_ecp4(io);
                    let paddi_pio = self.rc_io_wire(cell_io, &format!("JPADDI{abcd}_PIO"));
                    self.claim_pip(clkfb_io, paddi_pio);
                    self.claim_pip(clkfb_in, clkfb_io);
                }

                let eclk_sources = match hv {
                    DirHV::SW => [(3, 6)].as_slice(),
                    DirHV::SE => [(3, 5)].as_slice(),
                    DirHV::NW => [(3, 7), (7, 1)].as_slice(),
                    DirHV::NE => [(3, 4), (7, 2)].as_slice(),
                };
                for &(base, bank) in eclk_sources {
                    for j in 0..4 {
                        let i = base + j;
                        let clkfb_eclk = self.rc_wire(cell, &format!("JCLKFB{i}"));
                        self.add_bel_wire(bcrd, format!("CLKFB{i}"), clkfb_eclk);
                        let bcrd_eclk = self.chip.bel_eclksync_bank(bank, j);
                        let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK_MUX");
                        self.claim_pip(clkfb_eclk, wire_eclk);
                        self.claim_pip(clkfb_in, clkfb_eclk);
                    }
                }

                let clkfb = self.rc_wire(cell, "CLKFB_PLL");
                self.add_bel_wire(bcrd, "CLKFB", clkfb);
                self.claim_pip(clkfb, clkfb_in);

                self.insert_bel(bcrd, bel);
            }
        }
    }
}

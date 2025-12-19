use prjcombine_ecp::{
    bels,
    chip::{ChipKind, MachXo2Kind, PllLoc, PllPad, SpecialIoKey, SpecialLocKey},
};
use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::LegacyBel,
    dir::{Dir, DirHV},
};

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pll_machxo2(&mut self) {
        for (lr, idx, loc) in [
            ('L', 0, PllLoc::new(DirHV::NW, 0)),
            ('R', 1, PllLoc::new(DirHV::NE, 0)),
        ] {
            let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::Pll(loc)) else {
                continue;
            };

            let bcrd_pllrefcs = cell.bel(bels::PLLREFCS0);
            self.name_bel(bcrd_pllrefcs, [format!("{lr}PLLREFCS")]);
            let mut bel_pllrefcs = LegacyBel::default();

            let sel = self.rc_wire(cell, "JSEL_PLLREFCS");
            self.add_bel_wire(bcrd_pllrefcs, "SEL", sel);
            bel_pllrefcs
                .pins
                .insert("SEL".into(), self.xlat_int_wire(bcrd_pllrefcs, sel));

            let refclk0 = self.rc_wire(cell, "JREFCLK0");
            self.add_bel_wire(bcrd_pllrefcs, "REFCLK0", refclk0);
            let cell_config = self.chip.special_loc[&SpecialLocKey::Config];
            let wire_osc = self.rc_wire(cell_config, "JOSC_OSC");
            self.claim_pip(refclk0, wire_osc);

            let refclk1_0 = self.rc_wire(cell, "JREFCLK1_0");
            self.add_bel_wire(bcrd_pllrefcs, "REFCLK1_0", refclk1_0);
            bel_pllrefcs.pins.insert(
                "REFCLK1_0".into(),
                self.xlat_int_wire(bcrd_pllrefcs, refclk1_0),
            );
            let refclk1_1 = self.rc_wire(cell, "JREFCLK1_1");
            self.add_bel_wire(bcrd_pllrefcs, "REFCLK1_1", refclk1_1);
            bel_pllrefcs.pins.insert(
                "REFCLK1_1".into(),
                self.xlat_int_wire(bcrd_pllrefcs, refclk1_1),
            );

            let refclk2_0 = self.rc_wire(cell, "JREFCLK2_0");
            self.add_bel_wire(bcrd_pllrefcs, "REFCLK2_0", refclk2_0);
            bel_pllrefcs.pins.insert(
                "REFCLK2_0".into(),
                self.xlat_int_wire(bcrd_pllrefcs, refclk2_0),
            );
            let refclk2_1 = self.rc_wire(cell, "JREFCLK2_1");
            self.add_bel_wire(bcrd_pllrefcs, "REFCLK2_1", refclk2_1);
            bel_pllrefcs.pins.insert(
                "REFCLK2_1".into(),
                self.xlat_int_wire(bcrd_pllrefcs, refclk2_1),
            );

            let mut refclk_io = vec![];
            for (i, key) in [
                (3, SpecialIoKey::Pll(PllPad::PllIn0, loc)),
                (4, SpecialIoKey::Clock(Dir::S, 1)),
                (5, SpecialIoKey::Clock(Dir::N, 1)),
                (6, SpecialIoKey::Clock(Dir::S, 0)),
                (7, SpecialIoKey::Clock(Dir::N, 0)),
            ] {
                let refclk = self.rc_wire(cell, &format!("JREFCLK{i}"));
                self.add_bel_wire(bcrd_pllrefcs, format!("REFCLK{i}"), refclk);
                let io = self.chip.special_io[&key];
                let cell_io = self.chip.get_io_loc(io).cell;
                let wire_io = self.rc_io_wire(
                    cell_io,
                    &format!("JDI{abcd}", abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx()]),
                );
                self.claim_pip(refclk, wire_io);
                refclk_io.push(refclk);
            }

            let clk0_in = self.rc_wire(cell, "REFCLK0");
            self.add_bel_wire(bcrd_pllrefcs, "CLK0_IN", clk0_in);
            let clk1_in = self.rc_wire(cell, "REFCLK1");
            self.add_bel_wire(bcrd_pllrefcs, "CLK1_IN", clk1_in);
            self.claim_pip(clk0_in, refclk0);
            self.claim_pip(clk1_in, refclk0);
            self.claim_pip(clk0_in, refclk1_0);
            self.claim_pip(clk1_in, refclk1_1);
            self.claim_pip(clk0_in, refclk2_0);
            self.claim_pip(clk1_in, refclk2_1);
            for wire in refclk_io {
                self.claim_pip(clk0_in, wire);
                self.claim_pip(clk1_in, wire);
            }

            let clk0 = self.rc_wire(cell, "CLK0_PLLREFCS");
            self.add_bel_wire(bcrd_pllrefcs, "CLK0", clk0);
            self.claim_pip(clk0, clk0_in);
            let clk1 = self.rc_wire(cell, "CLK1_PLLREFCS");
            self.add_bel_wire(bcrd_pllrefcs, "CLK1", clk1);
            self.claim_pip(clk1, clk1_in);

            let pllcsout = self.rc_wire(cell, "PLLCSOUT_PLLREFCS");
            self.add_bel_wire(bcrd_pllrefcs, "PLLCSOUT", pllcsout);
            self.claim_pip(pllcsout, clk0);

            self.insert_bel(bcrd_pllrefcs, bel_pllrefcs);

            let bcrd = cell.bel(bels::PLL0);
            self.name_bel(bcrd, [format!("{lr}PLL")]);
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

            let clkfb0 = self.rc_wire(cell, "JCLKFB0");
            self.add_bel_wire(bcrd, "CLKFB0", clkfb0);
            let io = self.chip.special_io[&SpecialIoKey::Pll(PllPad::PllFb, loc)];
            let cell_io = self.chip.get_io_loc(io).cell;
            let wire_io = self.rc_io_wire(
                cell_io,
                &format!("JDI{abcd}", abcd = ['A', 'B', 'C', 'D'][io.iob().to_idx()]),
            );
            self.claim_pip(clkfb0, wire_io);

            let clkfb1 = self.rc_wire(cell, "JCLKFB1");
            self.add_bel_wire(bcrd, "CLKFB1", clkfb1);
            bel.pins
                .insert("CLKFB1".into(), self.xlat_int_wire(bcrd, clkfb1));

            let clkfb2 = self.rc_wire(cell, "JCLKFB2");
            self.add_bel_wire(bcrd, "CLKFB2", clkfb2);
            let clkfb4 = self.rc_wire(cell, "JCLKFB4");
            self.add_bel_wire(bcrd, "CLKFB4", clkfb4);
            let cell_clkfb = self.chip.bel_eclksync(Dir::S, 0).cell;
            let wire0_clkfb = self.rc_wire(cell_clkfb, "JPLLCLKFB0");
            let wire1_clkfb = self.rc_wire(cell_clkfb, "JPLLCLKFB1");
            self.claim_pip(clkfb4, wire0_clkfb);
            self.claim_pip(clkfb2, wire1_clkfb);

            let clkfb_in = self.rc_wire(cell, "CLKFB");
            self.add_bel_wire(bcrd, "CLKFB_IN", clkfb_in);
            self.claim_pip(clkfb_in, clkintfb_out);
            self.claim_pip(clkfb_in, clkfb0);
            self.claim_pip(clkfb_in, clkfb1);
            self.claim_pip(clkfb_in, clkfb2);
            self.claim_pip(clkfb_in, clkfb4);
            for i in 0..2 {
                let bcrd_eclksync = self.chip.bel_eclksync(Dir::N, i);
                let wire_eclk = self.naming.bel_wire(bcrd_eclksync, "ECLKO_OUT");
                self.claim_pip(clkfb_in, wire_eclk);
            }

            let clkfb = self.rc_wire(cell, "CLKFB_PLL");
            self.add_bel_wire(bcrd, "CLKFB", clkfb);
            self.claim_pip(clkfb, clkfb_in);

            for (pin, pin_efb) in [
                ("PLLADDR0", "PLLADRO0"),
                ("PLLADDR1", "PLLADRO1"),
                ("PLLADDR2", "PLLADRO2"),
                ("PLLADDR3", "PLLADRO3"),
                ("PLLADDR4", "PLLADRO4"),
                ("PLLDATI0", "PLLDATO0"),
                ("PLLDATI1", "PLLDATO1"),
                ("PLLDATI2", "PLLDATO2"),
                ("PLLDATI3", "PLLDATO3"),
                ("PLLDATI4", "PLLDATO4"),
                ("PLLDATI5", "PLLDATO5"),
                ("PLLDATI6", "PLLDATO6"),
                ("PLLDATI7", "PLLDATO7"),
                ("PLLWE", "PLLWEO"),
                ("PLLRST", "PLLRSTO"),
                ("PLLCLK", "PLLCLKO"),
            ] {
                let wire = self.rc_wire(cell, &format!("J{pin}_PLL"));
                let wire_efb = self.rc_wire(cell_config, &format!("J{pin_efb}_EFB"));
                self.add_bel_wire(bcrd, pin, wire);
                self.claim_pip(wire, wire_efb);
            }
            let has_mux = matches!(
                self.chip.kind,
                ChipKind::MachXo2(MachXo2Kind::MachXo2 | MachXo2Kind::MachXo3L)
            ) && self
                .chip
                .special_loc
                .contains_key(&SpecialLocKey::Pll(PllLoc::new(DirHV::NE, 0)));
            let wire = self.rc_wire(cell, "JPLLSTB_PLL");
            self.add_bel_wire(bcrd, "PLLSTB", wire);
            let wire_efb = self.rc_wire(
                cell_config,
                &if has_mux {
                    format!("JPLL{idx}STBOMUX")
                } else {
                    format!("JPLL{idx}STBO_EFB")
                },
            );
            self.claim_pip(wire, wire_efb);
            for pin in [
                "PLLDATO0", "PLLDATO1", "PLLDATO2", "PLLDATO3", "PLLDATO4", "PLLDATO5", "PLLDATO6",
                "PLLDATO7", "PLLACK",
            ] {
                let wire = self.rc_wire(cell, &format!("J{pin}_PLL"));
                self.add_bel_wire(bcrd, pin, wire);
            }

            self.insert_bel(bcrd, bel);
        }
    }
}

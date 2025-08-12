use prjcombine_ecp::chip::{PllLoc, SpecialIoKey, SpecialLocKey};
use prjcombine_interconnect::{
    db::{Bel, BelPin, TileWireCoord},
    dir::{Dir, DirHV},
    grid::{CellCoord, DieId},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_clk_machxo(&mut self) {
        let bcrd = self.chip.bel_clk_root();
        let cell = if self.chip.rows.len() == 21 {
            CellCoord::new(
                DieId::from_idx(0),
                self.chip.col_clk - 1,
                self.chip.row_clk - 1,
            )
        } else {
            CellCoord::new(DieId::from_idx(0), self.chip.col_clk - 1, self.chip.row_clk)
        };
        self.name_bel_null(bcrd);
        let mut bel = Bel::default();

        let mut inputs_pclk = vec![];
        let mut inputs_sclk = vec![];
        for i in 0..3 {
            for j in 0..4 {
                let wire = self.rc_wire(cell, &format!("JCIBCTL{i}{j}"));
                let pin = format!("CIBCTL{i}{j}");
                self.add_bel_wire(bcrd, &pin, wire);
                let bpin = self.xlat_int_wire(bcrd, wire);
                bel.pins.insert(pin, bpin);
                inputs_sclk.push(wire);
                if j == 2
                    && self
                        .chip
                        .special_loc
                        .contains_key(&SpecialLocKey::Pll(PllLoc::new(DirHV::NW, 0)))
                {
                    continue;
                }
                if j == 3
                    && self
                        .chip
                        .special_loc
                        .contains_key(&SpecialLocKey::Pll(PllLoc::new(DirHV::SW, 0)))
                {
                    continue;
                }
                let wire = self.rc_wire(cell, &format!("JCIBCLK{i}{j}"));
                let pin = format!("CIBCLK{i}{j}");
                self.add_bel_wire(bcrd, &pin, wire);
                let bpin = self.xlat_int_wire(bcrd, wire);
                bel.pins.insert(pin, bpin);
                inputs_pclk.push(wire);
            }
        }

        for (wire, key) in [
            ("JGCLK0", SpecialIoKey::Clock(Dir::N, 0)),
            ("JGCLK1", SpecialIoKey::Clock(Dir::N, 1)),
            ("JGCLK2", SpecialIoKey::Clock(Dir::S, 0)),
            ("JGCLK3", SpecialIoKey::Clock(Dir::S, 1)),
        ] {
            let wire = self.rc_wire(cell, wire);
            self.add_bel_wire(bcrd, key.to_string(), wire);
            let wire_io = self.get_special_io_wire_in(key);
            self.claim_pip(wire, wire_io);
            inputs_pclk.push(wire);
            inputs_sclk.push(wire);
        }

        for (loc, clkop, clkos, clkok) in [
            (
                PllLoc::new(DirHV::SW, 0),
                "JLLMMCLKA",
                "JLLMNCLKA",
                "JLFPSC1",
            ),
            (
                PllLoc::new(DirHV::NW, 0),
                "JULMMCLKA",
                "JULMNCLKA",
                "JLFPSC0",
            ),
        ] {
            let Some(&pll_cell) = self.chip.special_loc.get(&SpecialLocKey::Pll(loc)) else {
                continue;
            };
            let clkop = self.rc_wire(cell, clkop);
            let clkos = self.rc_wire(cell, clkos);
            let clkok = self.rc_wire(cell, clkok);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOP"), clkop);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOS"), clkos);
            self.add_bel_wire(bcrd, format!("PLL_{loc}_CLKOK"), clkok);
            let clkop_pll = self.rc_wire(pll_cell, "JCLKOP_PLL");
            let clkos_pll = self.rc_wire(pll_cell, "JCLKOS_PLL");
            let clkok_pll = self.rc_wire(pll_cell, "JCLKOK_PLL");
            self.claim_pip(clkop, clkop_pll);
            self.claim_pip(clkos, clkos_pll);
            self.claim_pip(clkok, clkok_pll);
            inputs_pclk.push(clkop);
            inputs_pclk.push(clkos);
            inputs_pclk.push(clkok);
        }

        for pin in [
            "PCLK0", "PCLK1", "PCLK2", "PCLK3", "SCLK0", "SCLK1", "SCLK2", "SCLK3",
        ] {
            let wire = self.intdb.get_wire(pin);
            bel.pins
                .insert(pin.into(), BelPin::new_in(TileWireCoord::new_idx(0, wire)));
            let wire = bcrd.cell.wire(wire);
            if pin.starts_with("PCLK") {
                for &wf in &inputs_pclk {
                    self.claim_pip_int_out(wire, wf);
                }
            } else {
                for &wf in &inputs_sclk {
                    self.claim_pip_int_out(wire, wf);
                }
            }
        }

        self.insert_bel(bcrd, bel);
    }
}

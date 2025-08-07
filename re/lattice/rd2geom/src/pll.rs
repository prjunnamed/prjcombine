use prjcombine_ecp::{
    bels,
    chip::{
        ChipKind, IoGroupKind, MachXo2Kind, PllLoc, PllPad, RowKind, SpecialIoKey, SpecialLocKey,
    },
};
use prjcombine_interconnect::{
    db::Bel,
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, DieId},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_pll_ecp(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            let bcrd = cell.bel(bels::PLL0);
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
                .insert("CLKI0".into(), self.xlat_int_wire(bcrd, clki0));
            bel.pins
                .insert("CLKI1".into(), self.xlat_int_wire(bcrd, clki1));
            bel.pins
                .insert("CLKI2".into(), self.xlat_int_wire(bcrd, clki2));
            let wire_io = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn0, loc));
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
                .insert("CLKOP".into(), self.xlat_int_wire(bcrd, clkop));

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
                .insert("CLKFB0".into(), self.xlat_int_wire(bcrd, clkfb0));
            bel.pins
                .insert("CLKFB1".into(), self.xlat_int_wire(bcrd, clkfb1));
            let wire_io = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllFb, loc));
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

    fn process_pll_ecp2(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            if loc.quad.v == DirV::S && loc.idx == 0 {
                // GPLL + DLL + CLKDIV + DLLDEL
                let cell_pll = match loc.quad.h {
                    DirH::W => cell.delta(2, 0),
                    DirH::E => cell.delta(-2, 0),
                };

                let bcrd_pll = cell.bel(bels::PLL0);
                let bcrd_dll = cell.bel(bels::DLL);
                let bcrd_dlldel = cell.bel(bels::DLLDEL0);
                let bcrd_clkdiv = cell.bel(bels::CLKDIV0);
                let bcrd_eclk = cell.bel(bels::ECLK_ALT_ROOT);

                let (r, c) = self.rc(cell_pll);
                self.name_bel(bcrd_pll, [format!("PLL_R{r}C{c}")]);

                let (r, c) = self.rc(cell);
                self.name_bel(bcrd_dll, [format!("DLL_R{r}C{c}")]);
                self.name_bel(bcrd_dlldel, [format!("DLLDEL_R{r}C{c}")]);
                self.name_bel(bcrd_clkdiv, [format!("CLKDIV_R{r}C{c}")]);
                self.name_bel_null(bcrd_eclk);

                let mut bel_pll = self.extract_simple_bel(bcrd_pll, cell_pll, "PLL");
                let mut bel_dll = self.extract_simple_bel(bcrd_dll, cell, "DLL");
                self.insert_simple_bel(bcrd_dlldel, cell, "DLLDEL");
                self.insert_simple_bel(bcrd_clkdiv, cell, "CLKDIV");
                let mut bel_eclk = Bel::default();

                let io_pll_in0 =
                    self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn0, loc));
                let io_pll_in1 =
                    self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn1, loc));
                let io_dll_in0 =
                    self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::DllIn0, loc));
                let io_dll_in1 =
                    self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::DllIn1, loc));
                let io_pll_fb = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllFb, loc));
                let io_dll_fb = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::DllFb, loc));
                assert_eq!(io_pll_in0, io_dll_in1);
                assert_eq!(io_pll_in1, io_dll_in0);

                let bel_eclk_root = self.chip.bel_eclk_root(Dir::H(loc.quad.h));
                let eclk0_in = self.naming.bel_wire(bel_eclk_root, "ECLK0_IN");

                // edge clocks
                let eclk0_out = self.rc_wire(cell, "JFRC0");
                let eclk1_out = self.rc_wire(cell, "JFRC1");
                self.add_bel_wire(bcrd_eclk, "ECLK0_OUT", eclk0_out);
                self.add_bel_wire(bcrd_eclk, "ECLK1_OUT", eclk1_out);

                let eclk0_io = self.rc_wire(cell, "JPIO0");
                let eclk1_io = self.rc_wire(cell, "JPIO1");
                self.add_bel_wire(bcrd_eclk, "ECLK0_IO", eclk0_io);
                self.add_bel_wire(bcrd_eclk, "ECLK1_IO", eclk1_io);
                self.claim_pip(eclk0_out, eclk0_io);
                self.claim_pip(eclk1_out, eclk1_io);
                self.claim_pip(eclk0_io, io_pll_in0);
                self.claim_pip(eclk1_io, io_pll_in0);

                let eclk0_int = self.rc_wire(cell, "JCIBCLK0");
                let eclk1_int = self.rc_wire(cell, "JCIBCLK1");
                self.add_bel_wire(bcrd_eclk, "ECLK0_INT", eclk0_int);
                self.add_bel_wire(bcrd_eclk, "ECLK1_INT", eclk1_int);
                bel_eclk
                    .pins
                    .insert("ECLK0_IN".into(), self.xlat_int_wire(bcrd_eclk, eclk0_int));
                bel_eclk
                    .pins
                    .insert("ECLK1_IN".into(), self.xlat_int_wire(bcrd_eclk, eclk1_int));
                self.claim_pip(eclk0_out, eclk0_int);
                self.claim_pip(eclk1_out, eclk1_int);

                // PLL

                let clki = self.rc_wire(cell_pll, "CLKI");
                let clki0 = self.rc_wire(cell_pll, "JPLLCLKI0");
                let clki1 = self.rc_wire(cell_pll, "JPLLCLKI1");
                let clki2 = self.rc_wire(cell_pll, "JPLLCLKI2");
                let clki3 = self.rc_wire(cell_pll, "JPLLCLKI3");
                let clki_pll = self.rc_wire(cell_pll, "CLKI_PLL");
                self.add_bel_wire(bcrd_pll, "CLKI", clki);
                self.add_bel_wire(bcrd_pll, "CLKI0", clki0);
                self.add_bel_wire(bcrd_pll, "CLKI1", clki1);
                self.add_bel_wire(bcrd_pll, "CLKI2", clki2);
                self.add_bel_wire(bcrd_pll, "CLKI3", clki3);
                self.add_bel_wire(bcrd_pll, "CLKI_PLL", clki_pll);
                bel_pll
                    .pins
                    .insert("CLKI1".into(), self.xlat_int_wire(bcrd_pll, clki1));
                bel_pll
                    .pins
                    .insert("CLKI2".into(), self.xlat_int_wire(bcrd_pll, clki2));
                self.claim_pip(clki0, io_pll_in1);
                self.claim_pip(clki3, io_pll_in0);
                self.claim_pip(clki, clki0);
                self.claim_pip(clki, clki1);
                self.claim_pip(clki, clki2);
                self.claim_pip(clki, clki3);
                self.claim_pip(clki_pll, clki);

                let clkop_pll = self.rc_wire(cell_pll, "JCLKOP_PLL");
                let pll_clkop = self.rc_wire(cell, "JPLLCLKOP");
                let clkos_pll = self.rc_wire(cell_pll, "JCLKOS_PLL");
                let pll_clkos = self.rc_wire(cell, "JPLLCLKOS");
                self.add_bel_wire(bcrd_pll, "CLKOP_OUT", pll_clkop);
                self.add_bel_wire(bcrd_pll, "CLKOS_OUT", pll_clkos);
                self.claim_pip(pll_clkop, clkop_pll);
                self.claim_pip(pll_clkos, clkos_pll);

                self.claim_pip(eclk0_out, pll_clkop);
                self.claim_pip(eclk1_out, pll_clkos);

                let clkfb = self.rc_wire(cell_pll, "CLKFB");
                let clkfb0 = self.rc_wire(cell_pll, "JPLLCLKFB0");
                let clkfb1 = self.rc_wire(cell_pll, "JPLLCLKFB1");
                let clkfb3 = self.rc_wire(cell_pll, "JPLLCLKFB3");
                let clkfb_pll = self.rc_wire(cell_pll, "CLKFB_PLL");
                let clkintfb = self.rc_wire(cell_pll, "PLLCLKINTFB");
                let clkintfb_pll = self.rc_wire(cell_pll, "CLKINTFB_PLL");
                self.add_bel_wire(bcrd_pll, "CLKFB", clkfb);
                self.add_bel_wire(bcrd_pll, "CLKFB0", clkfb0);
                self.add_bel_wire(bcrd_pll, "CLKFB1", clkfb1);
                self.add_bel_wire(bcrd_pll, "CLKFB3", clkfb3);
                self.add_bel_wire(bcrd_pll, "CLKFB_PLL", clkfb_pll);
                self.add_bel_wire(bcrd_pll, "CLKINTFB", clkintfb);
                self.add_bel_wire(bcrd_pll, "CLKINTFB_PLL", clkintfb_pll);
                bel_pll
                    .pins
                    .insert("CLKFB0".into(), self.xlat_int_wire(bcrd_pll, clkfb0));
                self.claim_pip(clkfb1, io_pll_fb);
                self.claim_pip(clkfb3, eclk0_in);
                self.claim_pip(clkfb, clkfb0);
                self.claim_pip(clkfb, clkfb1);
                self.claim_pip(clkfb, clkintfb);
                self.claim_pip(clkfb, clkfb3);
                self.claim_pip(clkfb_pll, clkfb);
                self.claim_pip(clkintfb, clkintfb_pll);

                // DLL
                let dll_clki = self.rc_wire(cell, "DLLCLKI");
                let dlldel_clki = self.rc_wire(cell, "DLLDELCLKI");
                let dll_clki0 = self.rc_wire(cell, "JDLLCLKI0");
                let dll_clki1 = self.rc_wire(cell, "JDLLCLKI1");
                let dll_clki2 = self.rc_wire(cell, "JDLLCLKI2");
                let dll_clki3 = self.rc_wire(cell, "JDLLCLKI3");
                let clki_dll = self.rc_wire(cell, "CLKI_DLL");
                let clki_dlldel = self.rc_wire(cell, "CLKI_DLLDEL");
                self.add_bel_wire(bcrd_dll, "CLKI0", dll_clki0);
                self.add_bel_wire(bcrd_dll, "CLKI1", dll_clki1);
                self.add_bel_wire(bcrd_dll, "CLKI2", dll_clki2);
                self.add_bel_wire(bcrd_dll, "CLKI3", dll_clki3);
                self.add_bel_wire(bcrd_dll, "CLKI", dll_clki);
                self.add_bel_wire(bcrd_dll, "CLKI_DLL", clki_dll);
                self.add_bel_wire(bcrd_dlldel, "CLKI", dlldel_clki);
                self.add_bel_wire(bcrd_dlldel, "CLKI_DLL", clki_dlldel);
                bel_dll
                    .pins
                    .insert("CLKI1".into(), self.xlat_int_wire(bcrd_dll, dll_clki1));
                bel_dll
                    .pins
                    .insert("CLKI2".into(), self.xlat_int_wire(bcrd_dll, dll_clki2));
                self.claim_pip(dll_clki0, io_dll_in0);
                self.claim_pip(dll_clki3, io_dll_in1);
                self.claim_pip(dll_clki, dll_clki0);
                self.claim_pip(dll_clki, dll_clki1);
                self.claim_pip(dll_clki, dll_clki2);
                self.claim_pip(dll_clki, dll_clki3);
                self.claim_pip(clki_dll, dll_clki);
                self.claim_pip(dlldel_clki, dll_clki0);
                self.claim_pip(dlldel_clki, dll_clki1);
                self.claim_pip(dlldel_clki, dll_clki2);
                self.claim_pip(dlldel_clki, dll_clki3);
                self.claim_pip(clki_dlldel, dlldel_clki);

                let clkiduty = self.rc_wire(cell, "JCLKIDUTY_DLL");
                self.add_bel_wire(bcrd_dll, "CLKIDUTY", clkiduty);

                let clkop_dll = self.rc_wire(cell, "JCLKOP_DLL");
                let dll_clkop = self.rc_wire(cell, "DLLCLKOP");
                let clkos_dll = self.rc_wire(cell, "JCLKOS_DLL");
                let dll_clkos = self.rc_wire(cell, "DLLCLKOS");
                self.add_bel_wire(bcrd_dll, "CLKOP_OUT", dll_clkop);
                self.add_bel_wire(bcrd_dll, "CLKOS_OUT", dll_clkos);
                self.claim_pip(dll_clkop, clkop_dll);
                self.claim_pip(dll_clkos, clkos_dll);

                self.claim_pip(eclk0_out, dll_clkop);
                self.claim_pip(eclk1_out, dll_clkos);

                let dll_clkfb = self.rc_wire(cell, "DLLCLKFB");
                let dll_clkfb0 = self.rc_wire(cell, "JDLLCLKFB0");
                let dll_clkfb1 = self.rc_wire(cell, "JDLLCLKFB1");
                let dll_clkfb2 = self.rc_wire(cell, "JDLLCLKFB2");
                let dll_clkfb3 = self.rc_wire(cell, "JDLLCLKFB3");
                let clkfb_dll = self.rc_wire(cell, "CLKFB_DLL");
                self.add_bel_wire(bcrd_dll, "CLKFB", dll_clkfb);
                self.add_bel_wire(bcrd_dll, "CLKFB0", dll_clkfb0);
                self.add_bel_wire(bcrd_dll, "CLKFB1", dll_clkfb1);
                self.add_bel_wire(bcrd_dll, "CLKFB2", dll_clkfb2);
                self.add_bel_wire(bcrd_dll, "CLKFB3", dll_clkfb3);
                self.add_bel_wire(bcrd_dll, "CLKFB_DLL", clkfb_dll);
                bel_dll
                    .pins
                    .insert("CLKFB3".into(), self.xlat_int_wire(bcrd_dll, dll_clkfb3));
                self.claim_pip(dll_clkfb0, eclk0_in);
                self.claim_pip(dll_clkfb1, io_dll_fb);
                self.claim_pip(dll_clkfb2, clkop_dll);
                self.claim_pip(dll_clkfb, dll_clkfb0);
                self.claim_pip(dll_clkfb, dll_clkfb1);
                self.claim_pip(dll_clkfb, dll_clkfb2);
                self.claim_pip(dll_clkfb, dll_clkfb3);
                self.claim_pip(clkfb_dll, dll_clkfb);

                // DLLDEL
                for i in 0..9 {
                    // ???
                    let wire_dll = self.rc_wire(cell, &format!("JDCNTL{i}_DLL"));
                    let wire_dlldel = self.rc_wire(cell, &format!("JDCNTL{i}_DLLDEL"));
                    self.add_bel_wire(bcrd_dlldel, format!("DCNTL{i}"), wire_dlldel);
                    self.claim_pip(wire_dll, wire_dlldel);
                }
                let dlldel_bypass = self.rc_wire(cell, "BYPASS");
                let bypass_dlldel = self.rc_wire(cell, "BYPASS_DLLDEL");
                let dlldel_clko = self.rc_wire(cell, "DLLDEL");
                let clko_dlldel = self.rc_wire(cell, "JCLKO_DLLDEL");
                self.add_bel_wire(bcrd_dlldel, "BYPASS", dlldel_bypass);
                self.add_bel_wire(bcrd_dlldel, "BYPASS_DLLDEL", bypass_dlldel);
                self.add_bel_wire(bcrd_dlldel, "CLKO", dlldel_clko);
                self.add_bel_wire(bcrd_dlldel, "CLKO_DLLDEL", clko_dlldel);
                self.claim_pip(dlldel_bypass, bypass_dlldel);
                self.claim_pip(dlldel_clko, clko_dlldel);
                self.claim_pip(eclk0_out, dlldel_bypass);
                self.claim_pip(eclk1_out, dlldel_bypass);
                self.claim_pip(eclk0_out, dlldel_clko);
                self.claim_pip(eclk1_out, dlldel_clko);

                // CLKDIV
                let clki_clkdiv = self.rc_wire(cell, "CLKI_CLKDIV");
                let clki_clkdiv_in = self.rc_wire(cell, "CLKDIVCLKI");
                self.add_bel_wire(bcrd_clkdiv, "CLKI", clki_clkdiv);
                self.add_bel_wire(bcrd_clkdiv, "CLKI_IN", clki_clkdiv_in);
                self.claim_pip(clki_clkdiv, clki_clkdiv_in);
                self.claim_pip(clki_clkdiv_in, eclk0_out);
                self.claim_pip(clki_clkdiv_in, eclk1_out);
                self.claim_pip(clki_clkdiv_in, pll_clkop);
                self.claim_pip(clki_clkdiv_in, dll_clkop);

                self.insert_bel(bcrd_pll, bel_pll);
                self.insert_bel(bcrd_dll, bel_dll);
                self.insert_bel(bcrd_eclk, bel_eclk);
            } else {
                // SPLL

                let bcrd = cell.bel(bels::SPLL);
                let (r, c) = self.rc(cell);
                self.name_bel(bcrd, [format!("SPLL_R{r}C{c}")]);
                let mut bel = self.extract_simple_bel(bcrd, cell, "SPLL");

                let io_pll_in0 =
                    self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn0, loc));
                let io_pll_in1 =
                    self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn1, loc));
                let io_pll_fb = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllFb, loc));

                let clki = self.rc_wire(cell, "CLKI");
                let clki0 = self.rc_wire(cell, "JSPLLCLKI0");
                let clki1 = self.rc_wire(cell, "JSPLLCLKI1");
                let clki2 = self.rc_wire(cell, "JSPLLCLKI2");
                let clki3 = self.rc_wire(cell, "JSPLLCLKI3");
                let clki_pll = self.rc_wire(cell, "CLKI_SPLL");
                self.add_bel_wire(bcrd, "CLKI", clki);
                self.add_bel_wire(bcrd, "CLKI0", clki0);
                self.add_bel_wire(bcrd, "CLKI1", clki1);
                self.add_bel_wire(bcrd, "CLKI2", clki2);
                self.add_bel_wire(bcrd, "CLKI3", clki3);
                self.add_bel_wire(bcrd, "CLKI_PLL", clki_pll);
                bel.pins
                    .insert("CLKI1".into(), self.xlat_int_wire(bcrd, clki1));
                bel.pins
                    .insert("CLKI2".into(), self.xlat_int_wire(bcrd, clki2));
                self.claim_pip(clki0, io_pll_in1);
                self.claim_pip(clki3, io_pll_in0);
                self.claim_pip(clki, clki0);
                self.claim_pip(clki, clki1);
                self.claim_pip(clki, clki2);
                self.claim_pip(clki, clki3);
                self.claim_pip(clki_pll, clki);

                let clkfb = self.rc_wire(cell, "CLKFB");
                let clkfb0 = self.rc_wire(cell, "JSPLLCLKFB0");
                let clkfb1 = self.rc_wire(cell, "JSPLLCLKFB1");
                let clkfb_pll = self.rc_wire(cell, "CLKFB_SPLL");
                let clkintfb = self.rc_wire(cell, "SPLLCLKINTFB");
                let clkintfb_pll = self.rc_wire(cell, "CLKINTFB_SPLL");
                self.add_bel_wire(bcrd, "CLKFB", clkfb);
                self.add_bel_wire(bcrd, "CLKFB0", clkfb0);
                self.add_bel_wire(bcrd, "CLKFB1", clkfb1);
                self.add_bel_wire(bcrd, "CLKFB_PLL", clkfb_pll);
                self.add_bel_wire(bcrd, "CLKINTFB", clkintfb);
                self.add_bel_wire(bcrd, "CLKINTFB_PLL", clkintfb_pll);
                bel.pins
                    .insert("CLKFB0".into(), self.xlat_int_wire(bcrd, clkfb0));
                self.claim_pip(clkfb1, io_pll_fb);
                self.claim_pip(clkfb, clkfb0);
                self.claim_pip(clkfb, clkfb1);
                self.claim_pip(clkfb, clkintfb);
                self.claim_pip(clkfb_pll, clkfb);
                self.claim_pip(clkintfb, clkintfb_pll);

                self.insert_bel(bcrd, bel);
            }
        }
    }

    fn process_pll_xp2(&mut self) {
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

    fn process_clkdiv_xp2(&mut self) {
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

    fn process_pll_ecp3(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            let bcrd = cell.bel(bels::PLL0);
            let cell = match loc.quad.h {
                DirH::W => cell.delta(3, 0),
                DirH::E => cell.delta(-3, 0),
            };
            let (r, c) = self.rc(cell);
            self.name_bel(bcrd, [format!("PLL_R{r}C{c}")]);
            let mut bel = self.extract_simple_bel(bcrd, cell, "PLL");

            let clki_pll = self.rc_wire(cell, "CLKI_PLL");
            let clki = self.rc_wire(cell, "CLKI");
            self.add_bel_wire(bcrd, "CLKI_PLL", clki_pll);
            self.add_bel_wire(bcrd, "CLKI", clki);
            self.claim_pip(clki_pll, clki);
            for i in [1, 2, 5] {
                if i == 5 && self.chip.kind == ChipKind::Ecp3 {
                    continue;
                }
                let clki_int = self.rc_wire(cell, &format!("JPLLCLKI{i}"));
                self.add_bel_wire(bcrd, format!("CLKI{i}"), clki_int);
                self.claim_pip(clki, clki_int);
                bel.pins
                    .insert(format!("CLKI{i}"), self.xlat_int_wire(bcrd, clki_int));
            }
            let mut io_ins = vec![(3, SpecialIoKey::Pll(PllPad::PllIn0, loc))];
            if loc.quad.v == DirV::N && loc.idx == 0 {
                io_ins.push((0, SpecialIoKey::Pll(PllPad::DllIn0, loc)));
            }
            if loc.quad.v == DirV::N
                && (loc.idx == 1
                    || !self
                        .chip
                        .special_loc
                        .contains_key(&SpecialLocKey::Pll(PllLoc::new(loc.quad, 1))))
            {
                io_ins.push((
                    4,
                    SpecialIoKey::Clock(
                        Dir::N,
                        match loc.quad.h {
                            DirH::W => 0,
                            DirH::E => 1,
                        },
                    ),
                ));
            }
            for (i, key) in io_ins {
                let clki_io = self.rc_wire(cell, &format!("JPLLCLKI{i}"));
                self.add_bel_wire(bcrd, format!("CLKI{i}"), clki_io);
                self.claim_pip(clki, clki_io);
                let wire_io = self.get_special_io_wire_in(key);
                self.claim_pip(clki_io, wire_io);
            }

            let clkfb_pll = self.rc_wire(cell, "CLKFB_PLL");
            let clkfb = self.rc_wire(cell, "CLKFB");
            self.add_bel_wire(bcrd, "CLKFB_PLL", clkfb_pll);
            self.add_bel_wire(bcrd, "CLKFB", clkfb);
            self.claim_pip(clkfb_pll, clkfb);

            for i in [0, 6] {
                if i == 6 && self.chip.kind == ChipKind::Ecp3 {
                    continue;
                }
                let clkfb_int = self.rc_wire(cell, &format!("JPLLCLKFB{i}"));
                self.add_bel_wire(bcrd, format!("CLKFB{i}"), clkfb_int);
                self.claim_pip(clkfb, clkfb_int);
                bel.pins
                    .insert(format!("CLKFB{i}"), self.xlat_int_wire(bcrd, clkfb_int));
            }

            let clkfb_io = self.rc_wire(cell, "JPLLCLKFB1");
            self.add_bel_wire(bcrd, "CLKFB1", clkfb_io);
            self.claim_pip(clkfb, clkfb_io);
            let wire_io = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllFb, loc));
            self.claim_pip(clkfb_io, wire_io);

            let clkintfb_pll = self.rc_wire(cell, "CLKINTFB_PLL");
            self.add_bel_wire(bcrd, "CLKINTFB_PLL", clkintfb_pll);
            let clkintfb = self.rc_wire(cell, "PLLCLKINTFB");
            self.add_bel_wire(bcrd, "CLKINTFB", clkintfb);
            self.claim_pip(clkintfb, clkintfb_pll);
            self.claim_pip(clkfb, clkintfb);

            let mut eclk_fb = vec![(3, Dir::H(loc.quad.h), 0)];
            if loc.quad.v == DirV::N
                && (loc.idx == 1
                    || !self
                        .chip
                        .special_loc
                        .contains_key(&SpecialLocKey::Pll(PllLoc::new(loc.quad, 1))))
            {
                eclk_fb.extend([(4, Dir::N, 0), (5, Dir::N, 1)]);
            }
            for (i, edge, ei) in eclk_fb {
                let clkfb_eclk = self.rc_wire(cell, &format!("JPLLCLKFB{i}"));
                self.add_bel_wire(bcrd, format!("CLKFB{i}"), clkfb_eclk);
                self.claim_pip(clkfb, clkfb_eclk);
                let bcrd_eclk = self.chip.bel_eclksync(edge, ei);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKO");
                self.claim_pip(clkfb_eclk, wire_eclk);
            }

            for pin in ["CLKOP", "CLKOS", "CLKOK"] {
                let wire = self.rc_wire(cell, &format!("J{pin}_PLL"));
                self.claim_pip(wire, clki_pll);
            }

            self.insert_bel(bcrd, bel);
        }
    }

    fn process_dll_ecp3(&mut self) {
        for edge in [DirH::W, DirH::E] {
            let bcrd_dll = CellCoord::new(
                DieId::from_idx(0),
                match edge {
                    DirH::W => self.chip.col_w() + 1,
                    DirH::E => self.chip.col_e() - 1,
                },
                self.chip.row_clk,
            )
            .bel(bels::DLL);
            let bcrd_dlldel = bcrd_dll.bel(bels::DLLDEL0);
            let cell = match edge {
                DirH::W => bcrd_dll.cell.delta(13, 0),
                DirH::E => bcrd_dll.cell.delta(-13, 0),
            };
            let cell_pll = match edge {
                DirH::W => bcrd_dll.cell.delta(3, 0),
                DirH::E => bcrd_dll.cell.delta(-3, 0),
            };
            let (r, c) = self.rc(cell);
            self.name_bel(bcrd_dll, [format!("DLL_R{r}C{c}")]);
            self.name_bel(bcrd_dlldel, [format!("DLLDEL_R{r}C{c}")]);
            let mut bel_dll = self.extract_simple_bel(bcrd_dll, cell, "DLL");
            self.insert_simple_bel(bcrd_dlldel, cell, "DLLDEL");

            for i in 0..6 {
                let wire_dll = self.rc_wire(cell, &format!("JDCNTL{i}_DLL"));
                let wire_dlldel = self.rc_wire(cell, &format!("JDCNTL{i}_DLLDEL"));
                self.add_bel_wire(bcrd_dlldel, format!("DCNTL{i}"), wire_dlldel);
                self.claim_pip(wire_dlldel, wire_dll);
            }

            let clki_dll = self.rc_wire(cell, "CLKI_DLL");
            self.add_bel_wire(bcrd_dll, "CLKI_DLL", clki_dll);
            let clki_dlldel = self.rc_wire(cell, "CLKI_DLLDEL");
            self.add_bel_wire(bcrd_dlldel, "CLKI", clki_dlldel);
            let clki = self.rc_wire(cell, "DLLCLKI");
            self.add_bel_wire(bcrd_dll, "CLKI", clki);
            self.claim_pip(clki_dll, clki);
            self.claim_pip(clki_dlldel, clki);

            for i in [1, 2, 5] {
                if i == 5 && self.chip.kind == ChipKind::Ecp3 {
                    continue;
                }
                let clki_int = self.rc_wire(cell, &format!("JDLLCLKI{i}"));
                self.add_bel_wire(bcrd_dll, format!("CLKI{i}"), clki_int);
                self.claim_pip(clki, clki_int);
                bel_dll
                    .pins
                    .insert(format!("CLKI{i}"), self.xlat_int_wire(bcrd_dll, clki_int));
            }
            let loc = PllLoc::new_hv(edge, DirV::N, 0);
            let io_ins = [
                (0, SpecialIoKey::Pll(PllPad::DllIn0, loc)),
                (3, SpecialIoKey::Pll(PllPad::PllIn0, loc)),
                (
                    4,
                    SpecialIoKey::Clock(
                        Dir::N,
                        match loc.quad.h {
                            DirH::W => 0,
                            DirH::E => 1,
                        },
                    ),
                ),
            ];
            for (i, key) in io_ins {
                let clki_io = self.rc_wire(cell, &format!("JDLLCLKI{i}"));
                self.add_bel_wire(bcrd_dll, format!("CLKI{i}"), clki_io);
                self.claim_pip(clki, clki_io);
                let wire_io = self.get_special_io_wire_in(key);
                self.claim_pip(clki_io, wire_io);
            }
            if self.chip.kind == ChipKind::Ecp3A {
                let clki_pll = self.rc_wire(cell, "JDLLCLKI6");
                self.add_bel_wire(bcrd_dll, "CLKI6", clki_pll);
                self.claim_pip(clki, clki_pll);
                let wire_pll = self.rc_wire(cell_pll, "JCLKOP_PLL");
                self.claim_pip(clki_pll, wire_pll);
            }

            let clkfb_dll = self.rc_wire(cell, "CLKFB_DLL");
            let clkfb = self.rc_wire(cell, "DLLCLKFB");
            self.add_bel_wire(bcrd_dll, "CLKFB_DLL", clkfb_dll);
            self.add_bel_wire(bcrd_dll, "CLKFB", clkfb);
            self.claim_pip(clkfb_dll, clkfb);

            for i in [3, 5] {
                if i == 5 && self.chip.kind == ChipKind::Ecp3 {
                    continue;
                }
                let clkfb_int = self.rc_wire(cell, &format!("JDLLCLKFB{i}"));
                self.add_bel_wire(bcrd_dll, format!("CLKFB{i}"), clkfb_int);
                self.claim_pip(clkfb, clkfb_int);
                bel_dll
                    .pins
                    .insert(format!("CLKFB{i}"), self.xlat_int_wire(bcrd_dll, clkfb_int));
            }

            let clkfb_io = self.rc_wire(cell, "JDLLCLKFB1");
            self.add_bel_wire(bcrd_dll, "CLKFB1", clkfb_io);
            self.claim_pip(clkfb, clkfb_io);
            let wire_io = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::DllFb, loc));
            self.claim_pip(clkfb_io, wire_io);

            let clkfb_clkop = self.rc_wire(cell, "JDLLCLKFB2");
            self.add_bel_wire(bcrd_dll, "CLKFB2", clkfb_clkop);
            self.claim_pip(clkfb, clkfb_clkop);
            let clkop = self.rc_wire(cell, "JCLKOP_DLL");
            self.claim_pip(clkfb_clkop, clkop);

            let eclk_fb = [
                (0, Dir::H(loc.quad.h), 0),
                (
                    4,
                    Dir::N,
                    match loc.quad.h {
                        DirH::W => 0,
                        DirH::E => 1,
                    },
                ),
            ];
            for (i, edge, ei) in eclk_fb {
                let clkfb_eclk = self.rc_wire(cell, &format!("JDLLCLKFB{i}"));
                self.add_bel_wire(bcrd_dll, format!("CLKFB{i}"), clkfb_eclk);
                self.claim_pip(clkfb, clkfb_eclk);
                let bcrd_eclk = self.chip.bel_eclksync(edge, ei);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKO");
                self.claim_pip(clkfb_eclk, wire_eclk);
            }

            let clko_dlldel = self.rc_wire(cell, "JCLKO_DLLDEL");
            let dlldel = self.rc_wire(cell, "DLLDEL");
            self.add_bel_wire(bcrd_dlldel, "CLKO_OUT", dlldel);
            self.claim_pip(dlldel, clko_dlldel);

            self.insert_bel(bcrd_dll, bel_dll);

            let bcrd = bcrd_dll.bel(bels::ECLK_ALT_ROOT);
            let mut bel = Bel::default();
            self.name_bel_null(bcrd);

            for ei in 0..2 {
                let eip1 = ei + 1;
                let eclk_out = self.rc_wire(cell, &format!("JDLLECLK{eip1}"));
                self.add_bel_wire(bcrd, format!("ECLK{ei}_OUT"), eclk_out);

                let eclk_int = self.rc_wire(cell, &format!("JCIBCLK{eip1}"));
                self.add_bel_wire(bcrd, format!("ECLK{ei}_INT"), eclk_int);
                self.claim_pip(eclk_out, eclk_int);
                bel.pins
                    .insert(format!("ECLK{ei}_IN"), self.xlat_int_wire(bcrd, eclk_int));

                let eclk_io = self.rc_wire(cell, &format!("JPLLPIO{eip1}"));
                self.add_bel_wire(bcrd, format!("ECLK{ei}_IO"), eclk_io);
                self.claim_pip(eclk_out, eclk_io);
                let wire_io = self.get_special_io_wire_in(SpecialIoKey::Pll(PllPad::PllIn0, loc));
                self.claim_pip(eclk_io, wire_io);

                let pin = ["CLKOP", "CLKOS"][ei];

                let eclk_pll = self.rc_wire(cell, &format!("JPLL{pin}"));
                self.add_bel_wire(bcrd, format!("ECLK{ei}_PLL_{pin}"), eclk_pll);
                self.claim_pip(eclk_out, eclk_pll);
                let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                self.claim_pip(eclk_pll, wire_pll);

                let eclk_dll = self.rc_wire(cell, &format!("DLL{pin}"));
                self.add_bel_wire(bcrd, format!("ECLK{ei}_DLL_{pin}"), eclk_dll);
                self.claim_pip(eclk_out, eclk_dll);
                let wire_dll = self.rc_wire(cell, &format!("J{pin}_DLL"));
                self.claim_pip(eclk_dll, wire_dll);

                self.claim_pip(eclk_out, dlldel);
            }

            self.insert_bel(bcrd, bel);
        }
    }

    fn process_clkdiv_ecp3(&mut self) {
        for edge in [DirH::W, DirH::E] {
            let bcrd = CellCoord::new(
                DieId::from_idx(0),
                match edge {
                    DirH::W => self.chip.col_w() + 1,
                    DirH::E => self.chip.col_e() - 1,
                },
                self.chip.row_clk,
            )
            .bel(bels::CLKDIV0);
            let cell = match edge {
                DirH::W => bcrd.cell.delta(13, 0),
                DirH::E => bcrd.cell.delta(-13, 0),
            };
            let (r, c) = self.rc(cell);
            self.name_bel(
                bcrd,
                [format!("CLKDIV_R{r}C{c}"), format!("CLKDIVTEST_R{r}C{c}")],
            );
            self.insert_simple_bel(bcrd, cell, "CLKDIV");

            let clk = self.rc_wire(cell, "CLKI_CLKDIV");
            self.add_bel_wire(bcrd, "CLK", clk);
            let clk_in = self.rc_wire(cell, "CLKDIVCLKI");
            self.add_bel_wire(bcrd, "CLK_IN", clk_in);
            self.claim_pip(clk, clk_in);

            for ei in 0..2 {
                let eip1 = ei + 1;
                let clk_eclk = self.rc_wire(cell, &format!("JECLK{eip1}"));
                self.add_bel_wire(bcrd, format!("CLK_ECLK{ei}"), clk_eclk);
                self.claim_pip(clk_in, clk_eclk);
                let bcrd_eclk = self.chip.bel_eclksync(Dir::H(edge), ei);
                let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLKO");
                self.claim_pip(clk_eclk, wire_eclk);

                let dlleclk = self.rc_wire(cell, &format!("JDLLECLK{eip1}"));
                let clki_test = self.rc_wire(cell, &format!("CLKI{eip1}_CLKDIVTEST"));
                self.add_bel_wire(bcrd, format!("CLKI{eip1}_CLKDIVTEST"), clki_test);
                self.claim_pip(clki_test, dlleclk);

                let clko_test = self.rc_wire(cell, &format!("CLKO{eip1}_CLKDIVTEST"));
                self.add_bel_wire(bcrd, format!("CLKO{eip1}_CLKDIVTEST"), clko_test);
                let clk_dlleclk = self.rc_wire(cell, &format!("DLLECLKI{eip1}"));
                self.add_bel_wire(bcrd, format!("CLK_DLLECLK{ei}"), clk_dlleclk);
                self.claim_pip(clk_dlleclk, clko_test);
                self.claim_pip(clk_in, clk_dlleclk);
            }

            let clk_pll = self.rc_wire(cell, "JPLLCLKOP");
            self.claim_pip(clk_in, clk_pll);

            let clk_dll = self.rc_wire(cell, "DLLCLKOP");
            self.claim_pip(clk_in, clk_dll);
        }
    }

    fn process_clkdiv_machxo2(&mut self) {
        let is_smol = self.chip.rows[self.chip.row_clk].kind != RowKind::Ebr;
        if is_smol {
            return;
        }
        for edge in [DirV::S, DirV::N] {
            for idx in 0..2 {
                let bcrd = self
                    .chip
                    .bel_eclksync(Dir::V(edge), 0)
                    .bel(bels::CLKDIV[idx]);
                let cell = bcrd.cell;
                self.name_bel(
                    bcrd,
                    [format!(
                        "{bt}CLKDIV{idx}",
                        bt = match edge {
                            DirV::S => 'B',
                            DirV::N => 'T',
                        }
                    )],
                );
                let mut bel = Bel::default();
                for pin in ["ALIGNWD", "RST", "CDIV1", "CDIVX"] {
                    let wire = self.rc_wire(cell, &format!("J{pin}{idx}_CLKDIV"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }
                self.insert_bel(bcrd, bel);

                let clki = self.rc_wire(cell, &format!("CLKI{idx}_CLKDIV"));
                self.add_bel_wire(bcrd, "CLKI", clki);
                let wire_eclk = self.rc_wire(cell, &format!("JECLKO{idx}_ECLKSYNC"));
                self.claim_pip(clki, wire_eclk);
            }
        }
    }

    fn process_pll_machxo2(&mut self) {
        for (lr, idx, loc) in [
            ('L', 0, PllLoc::new(DirHV::NW, 0)),
            ('R', 1, PllLoc::new(DirHV::NE, 0)),
        ] {
            let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::Pll(loc)) else {
                continue;
            };

            let bcrd_pllrefcs = cell.bel(bels::PLLREFCS0);
            self.name_bel(bcrd_pllrefcs, [format!("{lr}PLLREFCS")]);
            let mut bel_pllrefcs = Bel::default();

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

    fn process_clkdiv_ecp4(&mut self) {
        let cell_n = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_n());
        let cell_w = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk);
        let cell_e = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for (lrt, cell_tile, cell) in [
            ('T', cell_n, cell_n.delta(-1, 0)),
            ('L', cell_w, cell_w),
            ('R', cell_e, cell_e),
        ] {
            for i in 0..4 {
                let bcrd = cell_tile.bel(bels::CLKDIV[i]);
                self.name_bel(bcrd, [format!("CLKDIV_{lrt}{i}")]);
                let mut bel = Bel::default();

                for pin in ["ALIGNWD", "RST", "CDIVX"] {
                    let wire = self.rc_io_wire(cell, &format!("J{pin}_CLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }

                let clki = self.rc_io_wire(cell, &format!("CLKI_CLKDIV{i}"));
                self.add_bel_wire(bcrd, "CLKI", clki);
                let clki_in = self.claim_single_in(clki);
                self.add_bel_wire(bcrd, "CLKI_IN", clki_in);

                for bank_idx in 0..4 {
                    let bcrd_eclk = bcrd.bel(bels::ECLKSYNC[bank_idx * 4 + i]);
                    if !self.edev.egrid.has_bel(bcrd_eclk) {
                        continue;
                    }
                    let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
                    self.claim_pip(clki_in, wire_eclk);
                }

                self.insert_bel(bcrd, bel);
            }
        }

        let num_quads = if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesSingle)
        {
            1
        } else if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesDouble)
        {
            2
        } else if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesTriple)
        {
            3
        } else {
            unreachable!()
        };
        let cell_pcs = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_s());

        let cell_tile = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_s());
        let cell = cell_tile.delta(-1, 0);
        for i in 0..4 {
            let bcrd = cell_tile.bel(bels::PCSCLKDIV[i]);
            self.name_bel(bcrd, [format!("PCSCLKDIV{i}")]);

            let clki = self.rc_wire(cell, &format!("CLKI_PCSCLKDIV{i}"));
            self.add_bel_wire(bcrd, "CLKI", clki);
            let clki_in = self.claim_single_in(clki);
            self.add_bel_wire(bcrd, "CLKI_IN", clki_in);

            if i < num_quads {
                let mut bel = Bel::default();
                for pin in ["RST", "SEL0", "SEL1", "SEL2"] {
                    let wire = self.rc_wire(cell, &format!("J{pin}_PCSCLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }
                let clki_int = self.rc_wire(cell, &format!("JPCSCDIVCIB{i}"));
                self.add_bel_wire(bcrd, "CLKI_INT", clki_int);
                self.claim_pip(clki_in, clki_int);
                bel.pins
                    .insert("CLKI".into(), self.xlat_int_wire(bcrd, clki_int));

                let abcd = ['A', 'B', 'C', 'D'][i];
                for j in 0..4 {
                    let clki_rxclk = self.rc_wire(cell, &format!("JPCS{abcd}RXCLK{j}"));
                    self.add_bel_wire(bcrd, format!("CLKI_RXCLK{j}"), clki_rxclk);
                    self.claim_pip(clki_in, clki_rxclk);
                    let wire_pcs = self.rc_io_sn_wire(cell_pcs, &format!("JQ{i}CH{j}_FOPCLKB_ASB"));
                    self.claim_pip(clki_rxclk, wire_pcs);

                    let clki_txclk = self.rc_wire(cell, &format!("JPCS{abcd}TXCLK{j}"));
                    self.add_bel_wire(bcrd, format!("CLKI_TXCLK{j}"), clki_txclk);
                    self.claim_pip(clki_in, clki_txclk);
                    let wire_pcs = self.rc_io_sn_wire(cell_pcs, &format!("JQ{i}CH{j}_FOPCLKA_ASB"));
                    self.claim_pip(clki_txclk, wire_pcs);
                }

                self.insert_bel(bcrd, bel);
            } else {
                for pin in ["RST", "SEL0", "SEL1", "SEL2"] {
                    let wire = self.rc_wire(cell, &format!("J{pin}_PCSCLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                }
            }

            for pin in ["CDIV1", "CDIVX"] {
                let wire = self.rc_wire(cell, &format!("{pin}_PCSCLKDIV{i}"));
                self.add_bel_wire(bcrd, pin, wire);
            }
        }
    }

    fn process_pll_ecp4(&mut self) {
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

    fn process_clkdiv_ecp5(&mut self) {
        let cell_w = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk);
        let cell_e = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk);
        for (lr, cell) in [('L', cell_w), ('R', cell_e)] {
            for i in 0..2 {
                let bcrd = cell.bel(bels::CLKDIV[i]);
                self.name_bel(bcrd, [format!("CLKDIV_{lr}{i}")]);
                let mut bel = Bel::default();

                for pin in ["ALIGNWD", "RST", "CDIVX"] {
                    let wire = self.rc_io_wire(cell, &format!("J{pin}_CLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }

                let clki = self.rc_io_wire(cell, &format!("CLKI_CLKDIV{i}"));
                self.add_bel_wire(bcrd, "CLKI", clki);
                let clki_in = self.claim_single_in(clki);
                self.add_bel_wire(bcrd, "CLKI_IN", clki_in);

                for bank_idx in 0..2 {
                    let bcrd_eclk = bcrd.bel(bels::ECLKSYNC[bank_idx * 2 + i]);
                    let wire_eclk = self.naming.bel_wire(bcrd_eclk, "ECLK");
                    self.claim_pip(clki_in, wire_eclk);
                }

                self.insert_bel(bcrd, bel);
            }
        }
        if self.skip_serdes {
            return;
        }

        let serdes_cols = Vec::from_iter(
            self.chip
                .columns
                .ids()
                .filter(|&col| self.chip.columns[col].io_s == IoGroupKind::Serdes),
        );

        let cell_tile = CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_s());
        let cell = cell_tile.delta(-1, 0);
        for i in 0..2 {
            let bcrd = cell_tile.bel(bels::PCSCLKDIV[i]);
            self.name_bel(bcrd, [format!("PCSCLKDIV{i}")]);

            let clki = self.rc_wire(cell, &format!("CLKI_PCSCLKDIV{i}"));
            self.add_bel_wire(bcrd, "CLKI", clki);
            let clki_in = self.claim_single_in(clki);
            self.add_bel_wire(bcrd, "CLKI_IN", clki_in);

            if let Some(&col_pcs) = serdes_cols.get(i) {
                let cell_pcs = cell.with_col(col_pcs);
                let mut bel = Bel::default();
                for pin in ["RST", "SEL0", "SEL1", "SEL2"] {
                    let wire = self.rc_wire(cell, &format!("J{pin}_PCSCLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                    bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
                }
                let clki_int = self.rc_wire(cell, &format!("JPCSCDIVCIB{i}"));
                self.add_bel_wire(bcrd, "CLKI_INT", clki_int);
                self.claim_pip(clki_in, clki_int);
                bel.pins
                    .insert("CLKI".into(), self.xlat_int_wire(bcrd, clki_int));

                let abcd = ['A', 'B'][i];
                for ch in 0..2 {
                    let clki_rxclk = self.rc_wire(cell, &format!("JPCS{abcd}RXCLK{ch}"));
                    self.add_bel_wire(bcrd, format!("CLKI_RXCLK{ch}"), clki_rxclk);
                    self.claim_pip(clki_in, clki_rxclk);
                    let wire_pcs = self.rc_io_wire(cell_pcs, &format!("JCH{ch}_FF_RX_PCLK_DCU"));
                    self.claim_pip(clki_rxclk, wire_pcs);

                    let clki_txclk = self.rc_wire(cell, &format!("JPCS{abcd}TXCLK{ch}"));
                    self.add_bel_wire(bcrd, format!("CLKI_TXCLK{ch}"), clki_txclk);
                    self.claim_pip(clki_in, clki_txclk);
                    let wire_pcs = self.rc_io_wire(cell_pcs, &format!("JCH{ch}_FF_TX_PCLK_DCU"));
                    self.claim_pip(clki_txclk, wire_pcs);
                }

                self.insert_bel(bcrd, bel);
            } else {
                for pin in ["RST", "SEL0", "SEL1", "SEL2"] {
                    let wire = self.rc_wire(cell, &format!("J{pin}_PCSCLKDIV{i}"));
                    self.add_bel_wire(bcrd, pin, wire);
                }
            }

            for pin in ["CDIV1", "CDIVX"] {
                let wire = self.rc_wire(cell, &format!("{pin}_PCSCLKDIV{i}"));
                self.add_bel_wire(bcrd, pin, wire);
            }
        }
    }

    fn process_pll_ecp5(&mut self) {
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

    pub fn process_pll(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => self.process_pll_ecp(),
            ChipKind::MachXo => self.process_pll_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.process_pll_ecp2(),
            ChipKind::Xp2 => {
                self.process_pll_xp2();
                self.process_clkdiv_xp2();
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.process_pll_ecp3();
                self.process_dll_ecp3();
                self.process_clkdiv_ecp3();
            }
            ChipKind::MachXo2(_) => {
                self.process_clkdiv_machxo2();
                self.process_pll_machxo2();
            }
            ChipKind::Ecp4 => {
                self.process_clkdiv_ecp4();
                self.process_pll_ecp4();
            }
            ChipKind::Ecp5 => {
                self.process_clkdiv_ecp5();
                self.process_pll_ecp5();
            }
        }
    }
}

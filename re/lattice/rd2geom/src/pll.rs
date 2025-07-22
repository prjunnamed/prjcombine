use prjcombine_ecp::{
    bels,
    chip::{ChipKind, PllLoc, PllPad, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::Bel,
    dir::{Dir, DirH, DirHV, DirV},
};

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_pll_ecp(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
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
            let wire_io = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllIn0, loc));
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
            let wire_io = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllFb, loc));
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
                    .insert("CLKI0".into(), self.xlat_int_wire(tcrd, clki0).unwrap());
                bel.pins
                    .insert("CLKI1".into(), self.xlat_int_wire(tcrd, clki1).unwrap());
                bel.pins
                    .insert("CLKI2".into(), self.xlat_int_wire(tcrd, clki2).unwrap());
                let wire_io = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllIn0, pll_loc));
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
                let wire_io = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllFb, pll_loc));
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

                let bcrd_pll = cell.bel(bels::PLL);
                let bcrd_dll = cell.bel(bels::DLL);
                let bcrd_dlldel = cell.bel(bels::DLLDEL);
                let bcrd_clkdiv = cell.bel(bels::CLKDIV);
                let bcrd_eclk = cell.bel(bels::ECLK_ALT_ROOT);
                let tcrd = self.edev.egrid.get_tile_by_bel(bcrd_pll);

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

                let io_pll_in0 = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllIn0, loc));
                let io_pll_in1 = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllIn1, loc));
                let io_dll_in0 = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::DllIn0, loc));
                let io_dll_in1 = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::DllIn1, loc));
                let io_pll_fb = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllFb, loc));
                let io_dll_fb = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::DllFb, loc));
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
                bel_eclk.pins.insert(
                    "ECLK0_IN".into(),
                    self.xlat_int_wire(tcrd, eclk0_int).unwrap(),
                );
                bel_eclk.pins.insert(
                    "ECLK1_IN".into(),
                    self.xlat_int_wire(tcrd, eclk1_int).unwrap(),
                );
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
                    .insert("CLKI1".into(), self.xlat_int_wire(tcrd, clki1).unwrap());
                bel_pll
                    .pins
                    .insert("CLKI2".into(), self.xlat_int_wire(tcrd, clki2).unwrap());
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
                    .insert("CLKFB0".into(), self.xlat_int_wire(tcrd, clkfb0).unwrap());
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
                    .insert("CLKI1".into(), self.xlat_int_wire(tcrd, dll_clki1).unwrap());
                bel_dll
                    .pins
                    .insert("CLKI2".into(), self.xlat_int_wire(tcrd, dll_clki2).unwrap());
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
                bel_dll.pins.insert(
                    "CLKFB3".into(),
                    self.xlat_int_wire(tcrd, dll_clkfb3).unwrap(),
                );
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
                let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
                let (r, c) = self.rc(cell);
                self.name_bel(bcrd, [format!("SPLL_R{r}C{c}")]);
                let mut bel = self.extract_simple_bel(bcrd, cell, "SPLL");

                let io_pll_in0 = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllIn0, loc));
                let io_pll_in1 = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllIn1, loc));
                let io_pll_fb = self.get_special_io_wire(SpecialIoKey::Pll(PllPad::PllFb, loc));

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
                    .insert("CLKI1".into(), self.xlat_int_wire(tcrd, clki1).unwrap());
                bel.pins
                    .insert("CLKI2".into(), self.xlat_int_wire(tcrd, clki2).unwrap());
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
                    .insert("CLKFB0".into(), self.xlat_int_wire(tcrd, clkfb0).unwrap());
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

    pub fn process_pll(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => self.process_pll_ecp(),
            ChipKind::MachXo => self.process_pll_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.process_pll_ecp2(),
        }
    }
}

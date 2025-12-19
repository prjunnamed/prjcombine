use prjcombine_ecp::{
    bels,
    chip::{ChipKind, PllLoc, PllPad, SpecialIoKey, SpecialLocKey},
};
use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::Bel,
    dir::{Dir, DirH, DirV},
    grid::{CellCoord, DieId},
};

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pll_ecp3(&mut self) {
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

    pub(super) fn process_dll_ecp3(&mut self) {
        for edge in [DirH::W, DirH::E] {
            let bcrd_dll = CellCoord::new(
                DieId::from_idx(0),
                match edge {
                    DirH::W => self.chip.col_w() + 1,
                    DirH::E => self.chip.col_e() - 1,
                },
                self.chip.row_clk,
            )
            .bel(bels::DLL0);
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

    pub(super) fn process_clkdiv_ecp3(&mut self) {
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
}

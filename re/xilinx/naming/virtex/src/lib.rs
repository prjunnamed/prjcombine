use prjcombine_interconnect::grid::{ColId, DieId, EdgeIoCoord, ExpandedDieRef, LayerId, RowId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex::{
    chip::{Chip, ChipKind, DisabledPart},
    expanded::ExpandedDevice,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub grid: &'a Chip,
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn get_io_name(&'a self, io: EdgeIoCoord) -> &'a str {
        let die = self.edev.egrid.die(DieId::from_idx(0));
        let (col, row, bel) = self.grid.get_io_loc(io);
        let nnode = &self.ngrid.nodes[&(die.die, col, row, LayerId::from_idx(0))];
        &nnode.bels[bel]
    }
}

struct Namer<'a> {
    edev: &'a ExpandedDevice<'a>,
    grid: &'a Chip,
    die: ExpandedDieRef<'a, 'a>,
    ngrid: ExpandedGridNaming<'a>,
    clut: EntityPartVec<ColId, usize>,
    bramclut: EntityPartVec<ColId, usize>,
    brambelclut: EntityPartVec<ColId, usize>,
    clkclut: EntityPartVec<ColId, usize>,
    rlut: EntityVec<RowId, usize>,
}

impl Namer<'_> {
    fn fill_rlut(&mut self) {
        let n = self.grid.rows;
        for row in self.die.rows() {
            self.rlut.push(n - row.to_idx() - 1);
        }
    }

    fn fill_clut(&mut self) {
        let mut c = 0;
        let mut bramc = 0;
        let mut brambelc = 0;
        for col in self.die.cols() {
            if self.grid.cols_bram.contains(&col) {
                self.bramclut.insert(col, bramc);
                bramc += 1;
                if !self.edev.disabled.contains(&DisabledPart::Bram(col)) {
                    self.brambelclut.insert(col, brambelc);
                    brambelc += 1;
                }
            } else {
                self.clut.insert(col, c);
                c += 1;
            }
        }
    }

    fn fill_clkclut(&mut self) {
        let mut cc = 1;
        for &(col_m, _, _) in &self.grid.cols_clkv {
            if col_m != self.grid.col_lio() + 1
                && col_m != self.grid.col_rio() - 1
                && col_m != self.grid.col_clk()
            {
                self.clkclut.insert(col_m, cc);
                cc += 1;
            }
        }
    }

    fn fill_io(&mut self) {
        let mut ctr_pad = 1;
        let mut ctr_empty = 1;
        for col in self.die.cols() {
            let row = self.grid.row_tio();
            if self.grid.cols_bram.contains(&col) {
                continue;
            }
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            let nnode = self
                .ngrid
                .nodes
                .get_mut(&(self.die.die, col, row, LayerId::from_idx(0)))
                .unwrap();
            nnode.add_bel(3, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            nnode.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
        }
        for row in self.die.rows().rev() {
            let col = self.grid.col_rio();
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            let nnode = self
                .ngrid
                .nodes
                .get_mut(&(self.die.die, col, row, LayerId::from_idx(0)))
                .unwrap();
            nnode.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            nnode.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(3, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
        }
        for col in self.die.cols().rev() {
            let row = self.grid.row_bio();
            if self.grid.cols_bram.contains(&col) {
                continue;
            }
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            let nnode = self
                .ngrid
                .nodes
                .get_mut(&(self.die.die, col, row, LayerId::from_idx(0)))
                .unwrap();
            nnode.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            nnode.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(3, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
        }
        for row in self.die.rows() {
            let col = self.grid.col_lio();
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            let nnode = self
                .ngrid
                .nodes
                .get_mut(&(self.die.die, col, row, LayerId::from_idx(0)))
                .unwrap();
            nnode.add_bel(3, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            nnode.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
        }
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let grid = edev.chip;
    let mut namer = Namer {
        edev,
        grid,
        die: egrid.die(DieId::from_idx(0)),
        ngrid: ExpandedGridNaming::new(ndb, egrid),
        clut: EntityPartVec::new(),
        bramclut: EntityPartVec::new(),
        brambelclut: EntityPartVec::new(),
        clkclut: EntityPartVec::new(),
        rlut: EntityVec::new(),
    };

    namer.fill_clut();
    namer.fill_clkclut();
    namer.fill_rlut();
    let bram_mid = grid.cols_bram.len() / 2;

    for die in egrid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    match &kind[..] {
                        "CNR.BL" => {
                            let nnode = namer.ngrid.name_node(nloc, "CNR.BL", ["BL".into()]);
                            nnode.add_bel(0, "CAPTURE".to_string());
                        }
                        "CNR.TL" => {
                            let nnode = namer.ngrid.name_node(nloc, "CNR.TL", ["TL".into()]);
                            nnode.add_bel(0, "STARTUP".to_string());
                            nnode.add_bel(1, "BSCAN".to_string());
                        }
                        "CNR.BR" => {
                            namer.ngrid.name_node(nloc, "CNR.BR", ["BR".into()]);
                        }
                        "CNR.TR" => {
                            namer.ngrid.name_node(nloc, "CNR.TR", ["TR".into()]);
                        }
                        "IO.L" => {
                            let c = namer.clut[col];
                            let r = namer.rlut[row];
                            let nnode = namer.ngrid.name_node(nloc, "IO.L", [format!("LR{r}")]);
                            nnode.add_bel(4, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(5, format!("TBUF_R{r}C{c}.0"));
                        }
                        "IO.R" => {
                            let c = namer.clut[col];
                            let r = namer.rlut[row];
                            let nnode = namer.ngrid.name_node(nloc, "IO.R", [format!("RR{r}")]);
                            nnode.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                        }
                        "IO.B" => {
                            let c = namer.clut[col];
                            namer.ngrid.name_node(nloc, "IO.B", [format!("BC{c}")]);
                        }
                        "IO.T" => {
                            let c = namer.clut[col];
                            namer.ngrid.name_node(nloc, "IO.T", [format!("TC{c}")]);
                        }
                        "CLB" => {
                            let c = namer.clut[col];
                            let r = namer.rlut[row];
                            let nnode = namer.ngrid.name_node(nloc, "CLB", [format!("R{r}C{c}")]);
                            nnode.add_bel(0, format!("CLB_R{r}C{c}.S0"));
                            nnode.add_bel(1, format!("CLB_R{r}C{c}.S1"));
                            if c % 2 == 1 {
                                nnode.add_bel(2, format!("TBUF_R{r}C{c}.0"));
                                nnode.add_bel(3, format!("TBUF_R{r}C{c}.1"));
                            } else {
                                nnode.add_bel(2, format!("TBUF_R{r}C{c}.1"));
                                nnode.add_bel(3, format!("TBUF_R{r}C{c}.0"));
                            }
                        }
                        "BRAM_BOT" => {
                            let name = if grid.kind == ChipKind::Virtex {
                                if col == grid.col_lio() + 1 {
                                    "LBRAM_BOT".to_string()
                                } else {
                                    "RBRAM_BOT".to_string()
                                }
                            } else {
                                let c = namer.bramclut[col];
                                format!("BRAM_BOTC{c}")
                            };
                            let c = namer.bramclut[col];
                            let naming = if c + 2 == bram_mid
                                || c == bram_mid + 1
                                || col == grid.col_lio() + 1
                                || col == grid.col_rio() - 1
                            {
                                "BRAM_BOT.BOT"
                            } else {
                                "BRAM_BOT.BOTP"
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "BRAM_TOP" => {
                            let name = if grid.kind == ChipKind::Virtex {
                                if col == grid.col_lio() + 1 {
                                    "LBRAM_TOP".to_string()
                                } else {
                                    "RBRAM_TOP".to_string()
                                }
                            } else {
                                let c = namer.bramclut[col];
                                format!("BRAM_TOPC{c}")
                            };
                            let c = namer.bramclut[col];
                            let naming = if c + 2 == bram_mid
                                || c == bram_mid + 1
                                || col == grid.col_lio() + 1
                                || col == grid.col_rio() - 1
                            {
                                "BRAM_TOP.TOP"
                            } else {
                                "BRAM_TOP.TOPP"
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "LBRAM" | "RBRAM" | "MBRAM" => {
                            let r = namer.rlut[row];
                            let c = namer.bramclut[col];
                            let mut names = vec![if grid.kind == ChipKind::Virtex {
                                format!("{kind}R{r}")
                            } else {
                                format!("BRAMR{r}C{c}")
                            }];
                            if r >= 5 {
                                let pr = r - 4;
                                if grid.kind == ChipKind::Virtex {
                                    names.push(format!("{kind}R{pr}"));
                                } else {
                                    names.push(format!("BRAMR{pr}C{c}"));
                                }
                            };
                            let br = (grid.rows - 1 - row.to_idx() - 4) / 4;
                            let bc = namer.brambelclut[col];
                            let nnode = namer.ngrid.name_node(nloc, kind, names);
                            nnode.add_bel(0, format!("RAMB4_R{br}C{bc}"));
                        }
                        "CLKB" | "CLKB_2DLL" | "CLKB_4DLL" => {
                            let nnode = namer.ngrid.name_node(nloc, kind, ["BM".into()]);
                            nnode.add_bel(0, "GCLKPAD0".to_string());
                            nnode.add_bel(1, "GCLKPAD1".to_string());
                            nnode.add_bel(2, "GCLKBUF0".to_string());
                            nnode.add_bel(3, "GCLKBUF1".to_string());
                        }
                        "CLKT" | "CLKT_2DLL" | "CLKT_4DLL" => {
                            let nnode = namer.ngrid.name_node(nloc, kind, ["TM".into()]);
                            nnode.add_bel(0, "GCLKPAD2".to_string());
                            nnode.add_bel(1, "GCLKPAD3".to_string());
                            nnode.add_bel(2, "GCLKBUF2".to_string());
                            nnode.add_bel(3, "GCLKBUF3".to_string());
                        }
                        "DLL.BOT" => {
                            let (naming, name, bname) = if col < grid.col_clk() {
                                ("DLL.BL", "LBRAM_BOT", "DLL1")
                            } else {
                                ("DLL.BR", "RBRAM_BOT", "DLL0")
                            };
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, naming, [name.into(), "BM".into()]);
                            nnode.add_bel(0, bname.to_string());
                        }
                        "DLL.TOP" => {
                            let (naming, name, bname) = if col < grid.col_clk() {
                                ("DLL.TL", "LBRAM_TOP", "DLL3")
                            } else {
                                ("DLL.TR", "RBRAM_TOP", "DLL2")
                            };
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, naming, [name.into(), "TM".into()]);
                            nnode.add_bel(0, bname.to_string());
                        }
                        "DLLS.BOT" | "DLLP.BOT" | "DLLS.TOP" | "DLLP.TOP" => {
                            let c = namer.bramclut[col];
                            let sp = if kind.starts_with("DLLS") { "S" } else { "P" };
                            let spn = if edev.disabled.contains(&DisabledPart::PrimaryDlls) {
                                ""
                            } else {
                                sp
                            };
                            let bt = if row == grid.row_bio() { 'B' } else { 'T' };
                            let name = if row == grid.row_bio() {
                                format!("BRAM_BOTC{c}")
                            } else {
                                format!("BRAM_TOPC{c}")
                            };
                            let lr = if col < grid.col_clk() { 'L' } else { 'R' };
                            let dll = match (lr, bt) {
                                ('R', 'B') => 0,
                                ('L', 'B') => 1,
                                ('R', 'T') => 2,
                                ('L', 'T') => 3,
                                _ => unreachable!(),
                            };
                            let naming = if grid.cols_bram.len() == 4 && sp == "S" {
                                format!("DLL{sp}.{bt}{lr}.GCLK")
                            } else {
                                format!("DLL{sp}.{bt}{lr}")
                            };
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, &naming, [name, format!("{bt}M")]);
                            nnode.add_bel(0, format!("DLL{dll}{spn}"));
                        }
                        "CLKL" => {
                            let nnode = namer.ngrid.name_node(nloc, "CLKL", ["LM".into()]);
                            nnode.add_bel(0, "LPCILOGIC".to_string());
                        }
                        "CLKR" => {
                            let nnode = namer.ngrid.name_node(nloc, "CLKR", ["RM".into()]);
                            nnode.add_bel(0, "RPCILOGIC".to_string());
                        }
                        "CLKV_BRAM_BOT" => {
                            let name = if grid.kind == ChipKind::Virtex {
                                let lr = if col < grid.col_clk() { 'L' } else { 'R' };
                                format!("{lr}BRAM_BOT")
                            } else {
                                let c = namer.bramclut[col];
                                format!("BRAM_BOTC{c}")
                            };
                            namer.ngrid.name_node(nloc, "CLKV_BRAM_BOT", [name]);
                        }
                        "CLKV_BRAM_TOP" => {
                            let name = if grid.kind == ChipKind::Virtex {
                                let lr = if col < grid.col_clk() { 'L' } else { 'R' };
                                format!("{lr}BRAM_TOP")
                            } else {
                                let c = namer.bramclut[col];
                                format!("BRAM_TOPC{c}")
                            };
                            namer.ngrid.name_node(nloc, "CLKV_BRAM_TOP", [name]);
                        }
                        "CLKV.NULL" => {
                            let (name, naming) = if col == grid.col_clk() {
                                if row == grid.row_bio() {
                                    ("BM".to_string(), "CLKV.CLKB")
                                } else {
                                    ("TM".to_string(), "CLKV.CLKT")
                                }
                            } else {
                                let c = namer.clkclut[col];
                                if row == grid.row_bio() {
                                    (format!("GCLKBC{c}"), "CLKV.GCLKB")
                                } else {
                                    (format!("GCLKTC{c}"), "CLKV.GCLKT")
                                }
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "CLKV.CLKV" => {
                            let r = namer.rlut[row];
                            namer
                                .ngrid
                                .name_node(nloc, "CLKV.CLKV", [format!("VMR{r}")]);
                        }
                        "CLKV.GCLKV" => {
                            let r = namer.rlut[row];
                            let c = namer.clkclut[col];
                            namer
                                .ngrid
                                .name_node(nloc, "CLKV.GCLKV", [format!("GCLKVR{r}C{c}")]);
                        }
                        "BRAM_CLKH" => {
                            let name = if grid.kind == ChipKind::Virtex {
                                if col == grid.col_lio() + 1 {
                                    "LBRAMM".to_string()
                                } else {
                                    "RBRAMM".to_string()
                                }
                            } else {
                                let c = namer.bramclut[col];
                                format!("BRAMMC{c}")
                            };
                            namer.ngrid.name_node(nloc, "BRAM_CLKH", [name]);
                        }
                        "CLKC" => {
                            namer.ngrid.name_node(nloc, "CLKC", ["M".into()]);
                        }
                        "GCLKC" => {
                            let c = namer.clkclut[col];
                            namer.ngrid.name_node(nloc, "GCLKC", [format!("GCLKCC{c}")]);
                        }

                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    namer.fill_io();

    ExpandedNamedDevice {
        edev,
        ngrid: namer.ngrid,
        grid,
    }
}

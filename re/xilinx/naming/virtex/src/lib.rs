use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex::{
    bels,
    chip::{Chip, ChipKind, DisabledPart},
    expanded::ExpandedDevice,
    tslots,
};

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub grid: &'a Chip,
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn get_io_name(&'a self, io: EdgeIoCoord) -> &'a str {
        let bel = self.grid.get_io_loc(io);
        self.ngrid.get_bel_name(bel).unwrap()
    }
}

struct Namer<'a> {
    edev: &'a ExpandedDevice<'a>,
    chip: &'a Chip,
    die: DieId,
    ngrid: ExpandedGridNaming<'a>,
    clut: EntityPartVec<ColId, usize>,
    bramclut: EntityPartVec<ColId, usize>,
    brambelclut: EntityPartVec<ColId, usize>,
    clkclut: EntityPartVec<ColId, usize>,
    rlut: EntityVec<RowId, usize>,
}

impl Namer<'_> {
    fn fill_rlut(&mut self) {
        let n = self.chip.rows;
        for row in self.edev.rows(self.die) {
            self.rlut.push(n - row.to_idx() - 1);
        }
    }

    fn fill_clut(&mut self) {
        let mut c = 0;
        let mut bramc = 0;
        let mut brambelc = 0;
        for col in self.edev.cols(self.die) {
            if self.chip.cols_bram.contains(&col) {
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
        for &(col_m, _, _) in &self.chip.cols_clkv {
            if col_m != self.chip.col_w() + 1
                && col_m != self.chip.col_e() - 1
                && col_m != self.chip.col_clk()
            {
                self.clkclut.insert(col_m, cc);
                cc += 1;
            }
        }
    }

    fn fill_io(&mut self) {
        let mut ctr_pad = 1;
        let mut ctr_empty = 1;
        let die = DieId::from_idx(0);
        for col in self.edev.cols(self.die) {
            let row = self.chip.row_n();
            if self.chip.cols_bram.contains(&col) {
                continue;
            }
            if col == self.chip.col_w() || col == self.chip.col_e() {
                continue;
            }
            let ntile = self
                .ngrid
                .tiles
                .get_mut(&CellCoord::new(die, col, row).tile(tslots::MAIN))
                .unwrap();
            ntile.add_bel(bels::IO3, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            ntile.add_bel(bels::IO2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
        }
        for row in self.edev.rows(self.die).rev() {
            let col = self.chip.col_e();
            if row == self.chip.row_s() || row == self.chip.row_n() {
                continue;
            }
            let ntile = self
                .ngrid
                .tiles
                .get_mut(&CellCoord::new(die, col, row).tile(tslots::MAIN))
                .unwrap();
            ntile.add_bel(bels::IO0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            ntile.add_bel(bels::IO1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO3, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
        }
        for col in self.edev.cols(self.die).rev() {
            let row = self.chip.row_s();
            if self.chip.cols_bram.contains(&col) {
                continue;
            }
            if col == self.chip.col_w() || col == self.chip.col_e() {
                continue;
            }
            let ntile = self
                .ngrid
                .tiles
                .get_mut(&CellCoord::new(die, col, row).tile(tslots::MAIN))
                .unwrap();
            ntile.add_bel(bels::IO0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            ntile.add_bel(bels::IO1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO3, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
        }
        for row in self.edev.rows(self.die) {
            let col = self.chip.col_w();
            if row == self.chip.row_s() || row == self.chip.row_n() {
                continue;
            }
            let ntile = self
                .ngrid
                .tiles
                .get_mut(&CellCoord::new(die, col, row).tile(tslots::MAIN))
                .unwrap();
            ntile.add_bel(bels::IO3, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            ntile.add_bel(bels::IO0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
        }
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let chip = edev.chip;
    let mut namer = Namer {
        edev,
        chip,
        die: DieId::from_idx(0),
        ngrid: ExpandedGridNaming::new(ndb, edev),
        clut: EntityPartVec::new(),
        bramclut: EntityPartVec::new(),
        brambelclut: EntityPartVec::new(),
        clkclut: EntityPartVec::new(),
        rlut: EntityVec::new(),
    };

    namer.fill_clut();
    namer.fill_clkclut();
    namer.fill_rlut();
    let bram_mid = chip.cols_bram.len() / 2;

    for (tcrd, tile) in edev.tiles() {
        let CellCoord { col, row, .. } = tcrd.cell;
        let kind = edev.db.tile_classes.key(tile.class);
        match &kind[..] {
            "CNR.BL" => {
                let ntile = namer.ngrid.name_tile(tcrd, "CNR.BL", ["BL".into()]);
                ntile.add_bel(bels::CAPTURE, "CAPTURE".to_string());
            }
            "CNR.TL" => {
                let ntile = namer.ngrid.name_tile(tcrd, "CNR.TL", ["TL".into()]);
                ntile.add_bel(bels::STARTUP, "STARTUP".to_string());
                ntile.add_bel(bels::BSCAN, "BSCAN".to_string());
            }
            "CNR.BR" => {
                namer.ngrid.name_tile(tcrd, "CNR.BR", ["BR".into()]);
            }
            "CNR.TR" => {
                namer.ngrid.name_tile(tcrd, "CNR.TR", ["TR".into()]);
            }
            "IO.L" => {
                let c = namer.clut[col];
                let r = namer.rlut[row];
                let ntile = namer.ngrid.name_tile(tcrd, "IO.L", [format!("LR{r}")]);
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.0"));
            }
            "IO.R" => {
                let c = namer.clut[col];
                let r = namer.rlut[row];
                let ntile = namer.ngrid.name_tile(tcrd, "IO.R", [format!("RR{r}")]);
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
            }
            "IO.B" => {
                let c = namer.clut[col];
                namer.ngrid.name_tile(tcrd, "IO.B", [format!("BC{c}")]);
            }
            "IO.T" => {
                let c = namer.clut[col];
                namer.ngrid.name_tile(tcrd, "IO.T", [format!("TC{c}")]);
            }
            "CLB" => {
                let c = namer.clut[col];
                let r = namer.rlut[row];
                let ntile = namer.ngrid.name_tile(tcrd, "CLB", [format!("R{r}C{c}")]);
                ntile.add_bel(bels::SLICE0, format!("CLB_R{r}C{c}.S0"));
                ntile.add_bel(bels::SLICE1, format!("CLB_R{r}C{c}.S1"));
                if c % 2 == 1 {
                    ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                    ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                } else {
                    ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.1"));
                    ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.0"));
                }
            }
            "BRAM_BOT" => {
                let name = if chip.kind == ChipKind::Virtex {
                    if col == chip.col_w() + 1 {
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
                    || col == chip.col_w() + 1
                    || col == chip.col_e() - 1
                {
                    "BRAM_BOT.BOT"
                } else {
                    "BRAM_BOT.BOTP"
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "BRAM_TOP" => {
                let name = if chip.kind == ChipKind::Virtex {
                    if col == chip.col_w() + 1 {
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
                    || col == chip.col_w() + 1
                    || col == chip.col_e() - 1
                {
                    "BRAM_TOP.TOP"
                } else {
                    "BRAM_TOP.TOPP"
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "LBRAM" | "RBRAM" | "MBRAM" => {
                let r = namer.rlut[row];
                let c = namer.bramclut[col];
                let mut names = vec![if chip.kind == ChipKind::Virtex {
                    format!("{kind}R{r}")
                } else {
                    format!("BRAMR{r}C{c}")
                }];
                if r >= 5 {
                    let pr = r - 4;
                    if chip.kind == ChipKind::Virtex {
                        names.push(format!("{kind}R{pr}"));
                    } else {
                        names.push(format!("BRAMR{pr}C{c}"));
                    }
                };
                let br = (chip.rows - 1 - row.to_idx() - 4) / 4;
                let bc = namer.brambelclut[col];
                let ntile = namer.ngrid.name_tile(tcrd, kind, names);
                ntile.add_bel(bels::BRAM, format!("RAMB4_R{br}C{bc}"));
            }
            "CLKB" | "CLKB_2DLL" | "CLKB_4DLL" => {
                let ntile = namer.ngrid.name_tile(tcrd, kind, ["BM".into()]);
                ntile.add_bel(bels::GCLK_IO0, "GCLKPAD0".to_string());
                ntile.add_bel(bels::GCLK_IO1, "GCLKPAD1".to_string());
                ntile.add_bel(bels::BUFG0, "GCLKBUF0".to_string());
                ntile.add_bel(bels::BUFG1, "GCLKBUF1".to_string());
            }
            "CLKT" | "CLKT_2DLL" | "CLKT_4DLL" => {
                let ntile = namer.ngrid.name_tile(tcrd, kind, ["TM".into()]);
                ntile.add_bel(bels::GCLK_IO0, "GCLKPAD2".to_string());
                ntile.add_bel(bels::GCLK_IO1, "GCLKPAD3".to_string());
                ntile.add_bel(bels::BUFG0, "GCLKBUF2".to_string());
                ntile.add_bel(bels::BUFG1, "GCLKBUF3".to_string());
            }
            "DLL.BOT" => {
                let (naming, name, bname) = if col < chip.col_clk() {
                    ("DLL.BL", "LBRAM_BOT", "DLL1")
                } else {
                    ("DLL.BR", "RBRAM_BOT", "DLL0")
                };
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, naming, [name.into(), "BM".into()]);
                ntile.add_bel(bels::DLL, bname.to_string());
            }
            "DLL.TOP" => {
                let (naming, name, bname) = if col < chip.col_clk() {
                    ("DLL.TL", "LBRAM_TOP", "DLL3")
                } else {
                    ("DLL.TR", "RBRAM_TOP", "DLL2")
                };
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, naming, [name.into(), "TM".into()]);
                ntile.add_bel(bels::DLL, bname.to_string());
            }
            "DLLS.BOT" | "DLLP.BOT" | "DLLS.TOP" | "DLLP.TOP" => {
                let c = namer.bramclut[col];
                let sp = if kind.starts_with("DLLS") { "S" } else { "P" };
                let spn = if edev.disabled.contains(&DisabledPart::PrimaryDlls) {
                    ""
                } else {
                    sp
                };
                let bt = if row == chip.row_s() { 'B' } else { 'T' };
                let name = if row == chip.row_s() {
                    format!("BRAM_BOTC{c}")
                } else {
                    format!("BRAM_TOPC{c}")
                };
                let lr = if col < chip.col_clk() { 'L' } else { 'R' };
                let dll = match (lr, bt) {
                    ('R', 'B') => 0,
                    ('L', 'B') => 1,
                    ('R', 'T') => 2,
                    ('L', 'T') => 3,
                    _ => unreachable!(),
                };
                let naming = if chip.cols_bram.len() == 4 && sp == "S" {
                    format!("DLL{sp}.{bt}{lr}.GCLK")
                } else {
                    format!("DLL{sp}.{bt}{lr}")
                };
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, &naming, [name, format!("{bt}M")]);
                ntile.add_bel(bels::DLL, format!("DLL{dll}{spn}"));
            }
            "CLKL" => {
                let ntile = namer.ngrid.name_tile(tcrd, "CLKL", ["LM".into()]);
                ntile.add_bel(bels::PCILOGIC, "LPCILOGIC".to_string());
            }
            "CLKR" => {
                let ntile = namer.ngrid.name_tile(tcrd, "CLKR", ["RM".into()]);
                ntile.add_bel(bels::PCILOGIC, "RPCILOGIC".to_string());
            }
            "CLKV_BRAM_S" => {
                let name = if chip.kind == ChipKind::Virtex {
                    let lr = if col < chip.col_clk() { 'L' } else { 'R' };
                    format!("{lr}BRAM_BOT")
                } else {
                    let c = namer.bramclut[col];
                    format!("BRAM_BOTC{c}")
                };
                namer.ngrid.name_tile(tcrd, "CLKV_BRAM_S", [name]);
            }
            "CLKV_BRAM_N" => {
                let name = if chip.kind == ChipKind::Virtex {
                    let lr = if col < chip.col_clk() { 'L' } else { 'R' };
                    format!("{lr}BRAM_TOP")
                } else {
                    let c = namer.bramclut[col];
                    format!("BRAM_TOPC{c}")
                };
                namer.ngrid.name_tile(tcrd, "CLKV_BRAM_N", [name]);
            }
            "CLKV.NULL" => {
                let (name, naming) = if col == chip.col_clk() {
                    if row == chip.row_s() {
                        ("BM".to_string(), "CLKV.CLKB")
                    } else {
                        ("TM".to_string(), "CLKV.CLKT")
                    }
                } else {
                    let c = namer.clkclut[col];
                    if row == chip.row_s() {
                        (format!("GCLKBC{c}"), "CLKV.GCLKB")
                    } else {
                        (format!("GCLKTC{c}"), "CLKV.GCLKT")
                    }
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "CLKV.CLKV" => {
                let r = namer.rlut[row];
                namer
                    .ngrid
                    .name_tile(tcrd, "CLKV.CLKV", [format!("VMR{r}")]);
            }
            "CLKV.GCLKV" => {
                let r = namer.rlut[row];
                let c = namer.clkclut[col];
                namer
                    .ngrid
                    .name_tile(tcrd, "CLKV.GCLKV", [format!("GCLKVR{r}C{c}")]);
            }
            "BRAM_CLKH" => {
                let name = if chip.kind == ChipKind::Virtex {
                    if col == chip.col_w() + 1 {
                        "LBRAMM".to_string()
                    } else {
                        "RBRAMM".to_string()
                    }
                } else {
                    let c = namer.bramclut[col];
                    format!("BRAMMC{c}")
                };
                namer.ngrid.name_tile(tcrd, "BRAM_CLKH", [name]);
            }
            "CLKC" => {
                namer.ngrid.name_tile(tcrd, "CLKC", ["M".into()]);
            }
            "GCLKC" => {
                let c = namer.clkclut[col];
                namer.ngrid.name_tile(tcrd, "GCLKC", [format!("GCLKCC{c}")]);
            }

            _ => panic!("umm {kind}?"),
        }
    }

    namer.fill_io();

    ExpandedNamedDevice {
        edev,
        ngrid: namer.ngrid,
        grid: chip,
    }
}

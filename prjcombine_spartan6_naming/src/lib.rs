use std::cmp::Ordering;

use prjcombine_int::{
    db::BelId,
    grid::{ColId, DieId, EdgeIoCoord, RowId},
};
use prjcombine_spartan6::{
    expanded::ExpandedDevice,
    grid::{ColumnIoKind, ColumnKind, DcmKind, DisabledPart, Grid, Gts, PllKind},
};
use prjcombine_xilinx_naming::{
    db::NamingDb,
    grid::{BelGrid, ExpandedGridNaming},
};
use unnamed_entity::{EntityId, EntityVec};

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub grid: &'a Grid,
}

pub struct Gt<'a> {
    pub col: ColId,
    pub row: RowId,
    pub bank: u32,
    pub pads_clk: Vec<(&'a str, &'a str)>,
    pub pads_tx: Vec<(&'a str, &'a str)>,
    pub pads_rx: Vec<(&'a str, &'a str)>,
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn get_io_name(&'a self, io: EdgeIoCoord) -> &'a str {
        let die = DieId::from_idx(0);
        let (col, row, bel) = self.grid.get_io_loc(io);
        let layer = self
            .edev
            .egrid
            .find_node_loc(die, (col, row), |node| {
                self.edev.egrid.db.nodes.key(node.kind) == "IOB"
            })
            .unwrap()
            .0;
        let nnode = &self.ngrid.nodes[&(die, col, row, layer)];
        &nnode.bels[bel]
    }

    pub fn get_gt(&'a self, col: ColId, row: RowId) -> Gt<'a> {
        let die = DieId::from_idx(0);
        let layer = self
            .edev
            .egrid
            .find_node_loc(die, (col, row), |node| {
                self.edev.egrid.db.nodes.key(node.kind) == "GTP"
            })
            .unwrap()
            .0;
        let nnode = &self.ngrid.nodes[&(die, col, row, layer)];
        Gt {
            col,
            row,
            bank: if row < self.grid.row_clk() {
                if col < self.grid.col_clk {
                    245
                } else {
                    267
                }
            } else {
                if col < self.grid.col_clk {
                    101
                } else {
                    123
                }
            },
            pads_clk: vec![
                (
                    &nnode.bels[BelId::from_idx(4)],
                    &nnode.bels[BelId::from_idx(5)],
                ),
                (
                    &nnode.bels[BelId::from_idx(6)],
                    &nnode.bels[BelId::from_idx(7)],
                ),
            ],
            pads_tx: vec![
                (
                    &nnode.bels[BelId::from_idx(8)],
                    &nnode.bels[BelId::from_idx(9)],
                ),
                (
                    &nnode.bels[BelId::from_idx(10)],
                    &nnode.bels[BelId::from_idx(11)],
                ),
            ],
            pads_rx: vec![
                (
                    &nnode.bels[BelId::from_idx(0)],
                    &nnode.bels[BelId::from_idx(1)],
                ),
                (
                    &nnode.bels[BelId::from_idx(2)],
                    &nnode.bels[BelId::from_idx(3)],
                ),
            ],
        }
    }

    pub fn get_gts(&'a self) -> Vec<Gt<'a>> {
        if self.edev.disabled.contains(&DisabledPart::Gtp) {
            vec![]
        } else {
            match self.grid.gts {
                Gts::None => vec![],
                Gts::Single(cl) => vec![self.get_gt(cl, self.grid.row_tio_outer())],
                Gts::Double(cl, cr) => vec![
                    self.get_gt(cl, self.grid.row_tio_outer()),
                    self.get_gt(cr, self.grid.row_tio_outer()),
                ],
                Gts::Quad(cl, cr) => vec![
                    self.get_gt(cl, self.grid.row_tio_outer()),
                    self.get_gt(cr, self.grid.row_tio_outer()),
                    self.get_gt(cl, self.grid.row_bio_outer()),
                    self.get_gt(cr, self.grid.row_bio_outer()),
                ],
            }
        }
    }
}

struct Namer<'a> {
    grid: &'a Grid,
    ngrid: ExpandedGridNaming<'a>,
    tiexlut: EntityVec<ColId, usize>,
    rxlut: EntityVec<ColId, usize>,
    rylut: EntityVec<RowId, usize>,
    ioxlut: EntityVec<ColId, usize>,
    ioylut: EntityVec<RowId, usize>,
    slice_grid: BelGrid,
    bram_grid: BelGrid,
    dsp_grid: BelGrid,
    gtp_grid: BelGrid,
}

impl Namer<'_> {
    fn fill_rxlut(&mut self) {
        let mut rx = 2;
        for &cd in self.grid.columns.values() {
            self.rxlut.push(rx);
            match cd.kind {
                ColumnKind::CleXL | ColumnKind::CleXM => rx += 2,
                ColumnKind::CleClk => rx += 4,
                _ => rx += 3,
            }
        }
    }

    fn fill_rylut(&mut self) {
        let mut ry = 2;
        for row in self.grid.rows.ids() {
            if row == self.grid.row_clk() {
                ry += 1;
            }
            if row.to_idx() % 16 == 8 {
                ry += 1;
            }
            self.rylut.push(ry);
            ry += 1;
        }
    }

    fn fill_ioxlut(&mut self) {
        let mut iox = 0;
        for &cd in self.grid.columns.values() {
            self.ioxlut.push(iox);
            if cd.kind == ColumnKind::Io
                || cd.bio != ColumnIoKind::None
                || cd.tio != ColumnIoKind::None
            {
                iox += 1;
            }
        }
    }

    fn fill_ioylut(&mut self) {
        let mut ioy = 0;
        for (row, &rd) in &self.grid.rows {
            self.ioylut.push(ioy);
            if row == self.grid.row_bio_outer()
                || row == self.grid.row_bio_inner()
                || row == self.grid.row_tio_inner()
                || row == self.grid.row_tio_outer()
                || rd.lio
                || rd.rio
            {
                ioy += 1;
            }
        }
    }

    fn fill_tiexlut(&mut self) {
        let mut tie_x = 0;
        for &cd in self.grid.columns.values() {
            self.tiexlut.push(tie_x);
            tie_x += 1;
            if cd.kind == ColumnKind::Io
                || cd.tio != ColumnIoKind::None
                || cd.bio != ColumnIoKind::None
            {
                tie_x += 1;
            }
            if cd.kind == ColumnKind::CleClk {
                tie_x += 1;
            }
        }
    }

    fn get_ioi_name(&self, col: ColId, row: RowId) -> (&'static str, String) {
        let is_brk = row.to_idx() % 16 == 0 && row != self.grid.row_clk();
        let cd = self.grid.columns[col];
        let naming = if col == self.grid.col_lio() {
            if is_brk {
                "LIOI_BRK"
            } else {
                "LIOI"
            }
        } else if col == self.grid.col_rio() {
            if is_brk {
                "RIOI_BRK"
            } else {
                "RIOI"
            }
        } else if row == self.grid.row_bio_outer() {
            if cd.bio == ColumnIoKind::Inner {
                "BIOI_OUTER_UNUSED"
            } else {
                "BIOI_OUTER"
            }
        } else if row == self.grid.row_bio_inner() {
            if cd.bio == ColumnIoKind::Outer {
                "BIOI_INNER_UNUSED"
            } else {
                "BIOI_INNER"
            }
        } else if row == self.grid.row_tio_inner() {
            if cd.tio == ColumnIoKind::Outer {
                "TIOI_INNER_UNUSED"
            } else {
                "TIOI_INNER"
            }
        } else if row == self.grid.row_tio_outer() {
            if cd.tio == ColumnIoKind::Inner {
                "TIOI_OUTER_UNUSED"
            } else {
                "TIOI_OUTER"
            }
        } else {
            unreachable!()
        };
        let x = col.to_idx();
        let y = row.to_idx();
        (naming, format!("{naming}_X{x}Y{y}"))
    }

    fn get_lterm_name(&self, col: ColId, row: RowId) -> (&'static str, String) {
        if col == self.grid.col_lio() {
            let rx = self.rxlut[col];
            let ry = self.rylut[row];
            let ltt = if row == self.grid.row_bio_outer() || row == self.grid.row_tio_outer() {
                "CNR_TL_LTERM"
            } else {
                "IOI_LTERM"
            };
            let txtra = if row == self.grid.row_clk() - 2 {
                "_LOWER_BOT"
            } else if row == self.grid.row_clk() - 1 {
                "_LOWER_TOP"
            } else if row == self.grid.row_clk() + 2 {
                "_UPPER_BOT"
            } else if row == self.grid.row_clk() + 3 {
                "_UPPER_TOP"
            } else {
                ""
            };
            let name = format!("{ltt}{txtra}_X{rx}Y{ry}", rx = rx - 1);
            ("TERM.W", name)
        } else {
            let name = if row < self.grid.row_bot() + 8 || row >= self.grid.row_top() - 8 {
                let ry = self.rylut[row];
                let rx = if col < self.grid.col_clk {
                    self.rxlut[col] - 1
                } else {
                    self.rxlut[col] - 2
                };
                format!("INT_LTERM_X{rx}Y{ry}")
            } else {
                let rx = self.rxlut[col] - 1;
                let ry = self.rylut[row];
                format!("INT_INTERFACE_LTERM_X{rx}Y{ry}")
            };
            ("TERM.W.INTF", name)
        }
    }

    fn get_rterm_name(&self, col: ColId, row: RowId) -> (&'static str, String) {
        if col == self.grid.col_rio() {
            let rx = self.rxlut[col];
            let ry = self.rylut[row];
            let rtt = if row == self.grid.row_bio_outer()
                || row == self.grid.row_bio_inner()
                || row == self.grid.row_tio_inner()
                || row == self.grid.row_tio_outer()
            {
                "CNR_TR_RTERM"
            } else {
                "IOI_RTERM"
            };
            let txtra = if row == self.grid.row_clk() - 2 {
                "_LOWER_BOT"
            } else if row == self.grid.row_clk() - 1 {
                "_LOWER_TOP"
            } else if row == self.grid.row_clk() + 2 {
                "_UPPER_BOT"
            } else if row == self.grid.row_clk() + 3 {
                "_UPPER_TOP"
            } else {
                ""
            };
            let name = format!("{rtt}{txtra}_X{rx}Y{ry}", rx = rx + 3);
            ("TERM.E", name)
        } else {
            let name = if row < self.grid.row_bot() + 8 || row >= self.grid.row_top() - 8 {
                let ry = self.rylut[row];
                let rx = if col < self.grid.col_clk {
                    self.rxlut[col] + 6
                } else {
                    self.rxlut[col] + 5
                };
                format!("INT_RTERM_X{rx}Y{ry}")
            } else {
                let rx = self.rxlut[col] + 1;
                let ry = self.rylut[row];
                format!("INT_INTERFACE_RTERM_X{rx}Y{ry}")
            };
            ("TERM.E.INTF", name)
        }
    }

    fn get_ioi_bterm_name(&self, col: ColId) -> String {
        let row = self.grid.row_bio_outer();
        let rx = self.rxlut[col] + 1;
        let ry = self.rylut[row] - 1;
        if col == self.grid.col_clk || col == self.grid.col_clk + 1 {
            format!("IOI_BTERM_REGB_X{rx}Y{ry}")
        } else {
            format!("IOI_BTERM_CLB_X{rx}Y{ry}")
        }
    }

    fn get_ioi_tterm_name(&self, col: ColId) -> String {
        let row = self.grid.row_tio_outer();
        let rx = self.rxlut[col] + 1;
        let ry = self.rylut[row] + 1;
        if col == self.grid.col_clk || col == self.grid.col_clk + 1 {
            format!("IOI_TTERM_REGT_X{rx}Y{ry}")
        } else {
            format!("IOI_TTERM_CLB_X{rx}Y{ry}")
        }
    }

    fn get_hclk_ioi_name(&self, col: ColId, row: RowId) -> String {
        let kind = if row <= self.grid.row_clk() {
            match row.cmp(&self.grid.rows_pci_ce_split.0) {
                Ordering::Less => "BOT_DN",
                Ordering::Equal => "BOT_SPLIT",
                Ordering::Greater => "BOT_UP",
            }
        } else {
            match row.cmp(&self.grid.rows_pci_ce_split.1) {
                Ordering::Less => "TOP_DN",
                Ordering::Equal => "TOP_SPLIT",
                Ordering::Greater => "TOP_UP",
            }
        };
        let lr = if col == self.grid.col_lio() { 'L' } else { 'R' };
        let x = col.to_idx();
        let y = row.to_idx();
        format!("HCLK_IOI{lr}_{kind}_X{x}Y{y}", y = y - 1)
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let grid = edev.grid;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);
    ngrid.tie_kind = Some("TIEOFF".to_string());
    ngrid.tie_pin_pullup = Some("KEEP1".to_string());
    ngrid.tie_pin_gnd = Some("HARD0".to_string());
    ngrid.tie_pin_vcc = Some("HARD1".to_string());
    let slice_grid = ngrid.bel_grid(|_, name, _| name.starts_with("CLE"));
    let bram_grid = ngrid.bel_grid(|_, name, _| name == "BRAM");
    let dsp_grid = ngrid.bel_grid(|_, name, _| name == "DSP");
    let gtp_grid = ngrid.bel_grid(|_, name, _| name == "GTP");
    let mut namer = Namer {
        grid,
        ngrid,
        tiexlut: EntityVec::new(),
        rxlut: EntityVec::new(),
        rylut: EntityVec::new(),
        ioxlut: EntityVec::new(),
        ioylut: EntityVec::new(),
        slice_grid,
        bram_grid,
        dsp_grid,
        gtp_grid,
    };

    namer.fill_rxlut();
    namer.fill_rylut();
    namer.fill_ioxlut();
    namer.fill_ioylut();
    namer.fill_tiexlut();

    for die in egrid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    match &kind[..] {
                        "INT" => {
                            let cd = grid.columns[col];
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let mut is_brk = y % 16 == 0;
                            if y == 0
                                && !matches!(
                                    cd.kind,
                                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
                                )
                            {
                                is_brk = false;
                            }
                            if row == grid.row_clk() && cd.kind == ColumnKind::Io {
                                is_brk = false;
                            }
                            let bram = if cd.kind == ColumnKind::Bram {
                                if is_brk {
                                    "_BRAM_BRK"
                                } else {
                                    "_BRAM"
                                }
                            } else {
                                ""
                            };
                            let name = format!("INT{bram}_X{x}Y{y}");
                            let mut naming = if is_brk { "INT.BRK" } else { "INT" };
                            for &hole in &edev.site_holes {
                                if hole.contains(col, row)
                                    && col == hole.col_r - 1
                                    && hole.col_l != hole.col_r - 1
                                {
                                    let is_brk = y % 16 == 0 && y != 0;
                                    naming = if is_brk { "INT.TERM.BRK" } else { "INT.TERM" };
                                }
                            }
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let tie_x = namer.tiexlut[col];
                            let tie_y = y * 2;
                            nnode.tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                        }
                        "INT.IOI" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let is_brk = y % 16 == 0 && row != grid.row_clk() && y != 0;
                            let name = if is_brk {
                                format!("INT_X{x}Y{y}")
                            } else if col == grid.col_lio() {
                                format!("LIOI_INT_X{x}Y{y}")
                            } else {
                                format!("IOI_INT_X{x}Y{y}")
                            };
                            let naming = if is_brk { "INT.IOI.BRK" } else { "INT.IOI" };
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let tie_x = namer.tiexlut[col];
                            let tie_y = y * 2;
                            nnode.tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                        }
                        "INTF" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let mut name = format!("INT_INTERFACE_X{x}Y{y}");
                            let mut naming = "INTF";
                            if col == grid.col_lio() {
                                if row == grid.row_bio_outer() {
                                    name = format!("LL_X{x}Y{y}");
                                    naming = "INTF.CNR";
                                } else if row == grid.row_tio_outer() {
                                    name = format!("UL_X{x}Y{y}");
                                    naming = "INTF.CNR";
                                } else {
                                    let is_brk = y % 16 == 0 && row != grid.row_clk();
                                    let carry = if is_brk { "_CARRY" } else { "" };
                                    name = format!("INT_INTERFACE{carry}_X{x}Y{y}");
                                }
                            } else if col == grid.col_rio() {
                                if row == grid.row_bio_outer() {
                                    name = format!("LR_LOWER_X{x}Y{y}");
                                    naming = "INTF.CNR";
                                } else if row == grid.row_bio_inner() {
                                    name = format!("LR_UPPER_X{x}Y{y}");
                                    naming = "INTF.CNR";
                                } else if row == grid.row_tio_inner() {
                                    name = format!("UR_LOWER_X{x}Y{y}");
                                    naming = "INTF.CNR";
                                } else if row == grid.row_tio_outer() {
                                    name = format!("UR_UPPER_X{x}Y{y}");
                                    naming = "INTF.CNR";
                                } else {
                                    let is_brk = y % 16 == 0 && row != grid.row_clk();
                                    let carry = if is_brk { "_CARRY" } else { "" };
                                    name = format!("INT_INTERFACE{carry}_X{x}Y{y}");
                                }
                            } else if col == grid.col_clk && row == grid.row_clk() {
                                name = format!("INT_INTERFACE_REGC_X{x}Y{y}");
                                naming = "INTF.REGC";
                            }
                            for &hole in &edev.site_holes {
                                if hole.contains(col, row) && hole.col_l != hole.col_r - 1 {
                                    let ry = namer.rylut[row];
                                    if col == hole.col_l {
                                        let rx = namer.rxlut[col] + 1;
                                        name = format!("INT_INTERFACE_RTERM_X{rx}Y{ry}");
                                        naming = "INTF.RTERM";
                                    } else if col == hole.col_r - 1 {
                                        let rx = namer.rxlut[col] - 1;
                                        name = format!("INT_INTERFACE_LTERM_X{rx}Y{ry}");
                                        naming = "INTF.LTERM";
                                    }
                                }
                            }
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "INTF.IOI" => {
                            let (_, name) = namer.get_ioi_name(col, row);
                            let naming = "INTF.IOI";
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "INTF.CMT" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let name = format!("INT_INTERFACE_CARRY_X{x}Y{y}");
                            let naming = "INTF";
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "INTF.CMT.IOI" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let name = format!("INT_INTERFACE_IOI_X{x}Y{y}");
                            let naming = "INTF";
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "CLEXL" | "CLEXM" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, kind, [format!("{kind}_X{x}Y{y}")]);
                            let sx = namer.slice_grid.xlut[col] * 2;
                            let sy = namer.slice_grid.ylut[row];
                            nnode.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                            nnode.add_bel(1, format!("SLICE_X{sx1}Y{sy}", sx1 = sx + 1));
                        }
                        "BRAM" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "BRAM",
                                [format!("BRAMSITE2_X{x}Y{y}")],
                            );
                            let bx = namer.bram_grid.xlut[col];
                            let by = namer.bram_grid.ylut[row] * 2;
                            nnode.add_bel(0, format!("RAMB16_X{bx}Y{by}"));
                            nnode.add_bel(1, format!("RAMB8_X{bx}Y{by}"));
                            nnode.add_bel(2, format!("RAMB8_X{bx}Y{by}", by = by + 1));
                        }
                        "DSP" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, "DSP", [format!("MACCSITE2_X{x}Y{y}")]);
                            let dx = namer.dsp_grid.xlut[col];
                            let dy = namer.dsp_grid.ylut[row];
                            nnode.add_bel(0, format!("DSP48_X{dx}Y{dy}"));
                        }
                        "PCIE" => {
                            let x = col.to_idx() + 2;
                            let y = row.to_idx() - 1;
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, "PCIE", [format!("PCIE_TOP_X{x}Y{y}")]);
                            nnode.add_bel(0, "PCIE_X0Y0".to_string());
                        }
                        "IOI.LR" | "IOI.BT" => {
                            let (naming, name) = namer.get_ioi_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let iox = namer.ioxlut[col];
                            let ioy = namer.ioylut[row];
                            let tiex = namer.tiexlut[col] + 1;
                            let tiey = row.to_idx() * 2;
                            nnode.add_bel(0, format!("ILOGIC_X{iox}Y{y}", y = ioy * 2));
                            nnode.add_bel(1, format!("ILOGIC_X{iox}Y{y}", y = ioy * 2 + 1));
                            nnode.add_bel(2, format!("OLOGIC_X{iox}Y{y}", y = ioy * 2));
                            nnode.add_bel(3, format!("OLOGIC_X{iox}Y{y}", y = ioy * 2 + 1));
                            nnode.add_bel(4, format!("IODELAY_X{iox}Y{y}", y = ioy * 2));
                            nnode.add_bel(5, format!("IODELAY_X{iox}Y{y}", y = ioy * 2 + 1));
                            nnode.add_bel(6, format!("TIEOFF_X{tiex}Y{tiey}"));
                        }
                        "IOB" => {
                            let cd = grid.columns[col];
                            let (naming, kind) = if col == grid.col_lio() {
                                if row == grid.row_clk() - 1 {
                                    ("LIOB_RDY", "LIOB_RDY")
                                } else if row == grid.row_clk() + 2 {
                                    ("LIOB_PCI", "LIOB_PCI")
                                } else {
                                    ("LIOB", "LIOB")
                                }
                            } else if col == grid.col_rio() {
                                if row == grid.row_clk() - 1 {
                                    ("RIOB_PCI", "RIOB_PCI")
                                } else if row == grid.row_clk() + 2 {
                                    ("RIOB_RDY", "RIOB_RDY")
                                } else {
                                    ("RIOB", "RIOB")
                                }
                            } else if row == grid.row_bio_outer() {
                                (
                                    "BIOB_OUTER",
                                    if cd.bio == ColumnIoKind::Outer {
                                        "BIOB_SINGLE_ALT"
                                    } else {
                                        "BIOB"
                                    },
                                )
                            } else if row == grid.row_bio_inner() {
                                (
                                    "BIOB_INNER",
                                    if cd.bio == ColumnIoKind::Inner {
                                        "BIOB_SINGLE"
                                    } else {
                                        "BIOB"
                                    },
                                )
                            } else if row == grid.row_tio_inner() {
                                (
                                    "TIOB_INNER",
                                    if cd.tio == ColumnIoKind::Inner {
                                        unreachable!()
                                    } else {
                                        "TIOB"
                                    },
                                )
                            } else if row == grid.row_tio_outer() {
                                (
                                    "TIOB_OUTER",
                                    if cd.tio == ColumnIoKind::Outer {
                                        "TIOB_SINGLE"
                                    } else {
                                        "TIOB"
                                    },
                                )
                            } else {
                                unreachable!()
                            };
                            let x = col.to_idx();
                            let mut y = row.to_idx();
                            if kind.starts_with('T') {
                                y = grid.row_tio_outer().to_idx();
                            }
                            if kind.starts_with('B') {
                                y = 0;
                            }
                            let name = format!("{kind}_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "CMT_DCM" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let (dy, (_, kind)) = grid
                                .get_dcms()
                                .into_iter()
                                .enumerate()
                                .find(|&(_, (row_dcm, _))| row == row_dcm)
                                .unwrap();
                            let naming = match kind {
                                DcmKind::Bot => "CMT_DCM_BOT",
                                DcmKind::BotMid => "CMT_DCM2_BOT",
                                DcmKind::Top => "CMT_DCM_TOP",
                                DcmKind::TopMid => "CMT_DCM2_TOP",
                            };
                            let name = format!("{naming}_X{x}Y{y}");
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            nnode.add_bel(0, format!("DCM_X0Y{y}", y = dy * 2));
                            nnode.add_bel(1, format!("DCM_X0Y{y}", y = dy * 2 + 1));
                        }
                        "DCM_BUFPLL_BUF_BOT"
                        | "DCM_BUFPLL_BUF_BOT_MID"
                        | "DCM_BUFPLL_BUF_TOP"
                        | "DCM_BUFPLL_BUF_TOP_MID" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let naming = match &kind[..] {
                                "DCM_BUFPLL_BUF_BOT" => "CMT_DCM_BOT",
                                "DCM_BUFPLL_BUF_BOT_MID" => "CMT_DCM2_BOT",
                                "DCM_BUFPLL_BUF_TOP" => "CMT_DCM_TOP",
                                "DCM_BUFPLL_BUF_TOP_MID" => "CMT_DCM2_TOP",
                                _ => unreachable!(),
                            };
                            let name = format!("{naming}_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        "CMT_PLL" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let (py, (_, kind)) = grid
                                .get_plls()
                                .into_iter()
                                .enumerate()
                                .find(|&(_, (row_pll, _))| row == row_pll)
                                .unwrap();
                            let naming = match kind {
                                PllKind::BotOut0 => "CMT_PLL2_BOT",
                                PllKind::BotOut1 => {
                                    if grid.rows.len() < 128 {
                                        "CMT_PLL_BOT"
                                    } else {
                                        "CMT_PLL1_BOT"
                                    }
                                }
                                PllKind::BotNoOut => "CMT_PLL3_BOT",
                                PllKind::TopOut0 => "CMT_PLL2_TOP",
                                PllKind::TopOut1 => "CMT_PLL_TOP",
                                PllKind::TopNoOut => "CMT_PLL3_TOP",
                            };
                            let name = format!("{naming}_X{x}Y{y}");
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            nnode.add_bel(0, format!("PLL_ADV_X0Y{py}"));
                            nnode.add_bel(
                                1,
                                format!(
                                    "TIEOFF_X{x}Y{y}",
                                    x = namer.tiexlut[col] + 2,
                                    y = row.to_idx() * 2 + 1
                                ),
                            );
                        }
                        "PLL_BUFPLL_B" | "PLL_BUFPLL_T" | "PLL_BUFPLL_OUT0" | "PLL_BUFPLL_OUT1" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let (_, pllkind) = grid
                                .get_plls()
                                .into_iter()
                                .find(|&(row_pll, _)| row == row_pll)
                                .unwrap();
                            let naming = match pllkind {
                                PllKind::BotOut0 => "CMT_PLL2_BOT",
                                PllKind::BotOut1 => {
                                    if grid.rows.len() < 128 {
                                        "CMT_PLL_BOT"
                                    } else {
                                        "CMT_PLL1_BOT"
                                    }
                                }
                                PllKind::BotNoOut => "CMT_PLL3_BOT",
                                PllKind::TopOut0 => "CMT_PLL2_TOP",
                                PllKind::TopOut1 => "CMT_PLL_TOP",
                                PllKind::TopNoOut => "CMT_PLL3_TOP",
                            };
                            let name = format!("{naming}_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        "LL" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let nnode = namer.ngrid.name_node(nloc, "LL", [format!("LL_X{x}Y{y}")]);
                            nnode.add_bel(0, "OCT_CAL_X0Y0".to_string());
                            nnode.add_bel(1, "OCT_CAL_X0Y1".to_string());
                        }
                        "UL" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let nnode = namer.ngrid.name_node(nloc, "UL", [format!("UL_X{x}Y{y}")]);
                            nnode.add_bel(0, "OCT_CAL_X0Y2".to_string());
                            nnode.add_bel(1, "OCT_CAL_X0Y3".to_string());
                            nnode.add_bel(2, "PMV".to_string());
                            nnode.add_bel(3, "DNA_PORT".to_string());
                        }
                        "LR" => {
                            let x = col.to_idx();
                            let y0 = row.to_idx();
                            let y1 = row.to_idx() + 1;
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "LR",
                                [format!("LR_LOWER_X{x}Y{y0}"), format!("LR_UPPER_X{x}Y{y1}")],
                            );
                            nnode.add_bel(0, "OCT_CAL_X1Y0".to_string());
                            nnode.add_bel(1, "ICAP_X0Y0".to_string());
                            nnode.add_bel(2, "SPI_ACCESS".to_string());
                            nnode.add_bel(3, "SUSPEND_SYNC".to_string());
                            nnode.add_bel(4, "POST_CRC_INTERNAL".to_string());
                            nnode.add_bel(5, "STARTUP".to_string());
                            nnode.add_bel(6, "SLAVE_SPI".to_string());
                        }
                        "UR" => {
                            let x = col.to_idx();
                            let y0 = row.to_idx();
                            let y1 = row.to_idx() + 1;
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "UR",
                                [format!("UR_LOWER_X{x}Y{y0}"), format!("UR_UPPER_X{x}Y{y1}")],
                            );
                            nnode.add_bel(0, "OCT_CAL_X1Y1".to_string());
                            nnode.add_bel(1, "BSCAN_X0Y0".to_string());
                            nnode.add_bel(2, "BSCAN_X0Y1".to_string());
                            nnode.add_bel(3, "BSCAN_X0Y2".to_string());
                            nnode.add_bel(4, "BSCAN_X0Y3".to_string());
                        }
                        "GTP" => {
                            let (naming, name, name_buf) = if row < grid.row_clk() {
                                if col < grid.col_clk {
                                    (
                                        "GTPDUAL_BOT",
                                        format!(
                                            "GTPDUAL_BOT_X{x}Y{y}",
                                            x = col.to_idx(),
                                            y = row.to_idx() + 16
                                        ),
                                        format!(
                                            "BRAM_BOT_BTERM_L_X{x}Y{y}",
                                            x = namer.rxlut[col] + 2,
                                            y = namer.rylut[row] - 1
                                        ),
                                    )
                                } else {
                                    (
                                        "GTPDUAL_BOT",
                                        format!(
                                            "GTPDUAL_BOT_X{x}Y{y}",
                                            x = col.to_idx(),
                                            y = row.to_idx() + 16
                                        ),
                                        format!(
                                            "BRAM_BOT_BTERM_R_X{x}Y{y}",
                                            x = namer.rxlut[col] + 2,
                                            y = namer.rylut[row] - 1
                                        ),
                                    )
                                }
                            } else {
                                if col < grid.col_clk {
                                    (
                                        "GTPDUAL_TOP",
                                        format!(
                                            "GTPDUAL_TOP_X{x}Y{y}",
                                            x = col.to_idx(),
                                            y = row.to_idx() - 32
                                        ),
                                        format!(
                                            "BRAM_TOP_TTERM_L_X{x}Y{y}",
                                            x = namer.rxlut[col] + 2,
                                            y = namer.rylut[row] + 1
                                        ),
                                    )
                                } else {
                                    (
                                        "GTPDUAL_TOP",
                                        format!(
                                            "GTPDUAL_TOP_X{x}Y{y}",
                                            x = col.to_idx(),
                                            y = row.to_idx() - 16
                                        ),
                                        format!(
                                            "BRAM_TOP_TTERM_R_X{x}Y{y}",
                                            x = namer.rxlut[col] + 2,
                                            y = namer.rylut[row] + 1
                                        ),
                                    )
                                }
                            };
                            let nnode = namer.ngrid.name_node(nloc, naming, [name, name_buf]);
                            let gx = namer.gtp_grid.xlut[col];
                            let gy = namer.gtp_grid.ylut[row];
                            nnode.add_bel(0, format!("IPAD_X{gx}Y{y}", y = gy * 8 + 2));
                            nnode.add_bel(1, format!("IPAD_X{gx}Y{y}", y = gy * 8));
                            nnode.add_bel(2, format!("IPAD_X{gx}Y{y}", y = gy * 8 + 3));
                            nnode.add_bel(3, format!("IPAD_X{gx}Y{y}", y = gy * 8 + 1));
                            nnode.add_bel(4, format!("IPAD_X{gx}Y{y}", y = gy * 8 + 5));
                            nnode.add_bel(5, format!("IPAD_X{gx}Y{y}", y = gy * 8 + 4));
                            nnode.add_bel(6, format!("IPAD_X{gx}Y{y}", y = gy * 8 + 7));
                            nnode.add_bel(7, format!("IPAD_X{gx}Y{y}", y = gy * 8 + 6));
                            nnode.add_bel(8, format!("OPAD_X{gx}Y{y}", y = gy * 4 + 1));
                            nnode.add_bel(9, format!("OPAD_X{gx}Y{y}", y = gy * 4 + 3));
                            nnode.add_bel(10, format!("OPAD_X{gx}Y{y}", y = gy * 4));
                            nnode.add_bel(11, format!("OPAD_X{gx}Y{y}", y = gy * 4 + 2));
                            nnode
                                .add_bel(12, format!("BUFDS_X{x}Y{y}", x = gx + 1, y = 2 + gy * 2));
                            nnode.add_bel(
                                13,
                                format!("BUFDS_X{x}Y{y}", x = gx + 1, y = 2 + gy * 2 + 1),
                            );
                            nnode.add_bel(14, format!("GTPA1_DUAL_X{gx}Y{gy}"));
                        }
                        "MCB" => {
                            let x = col.to_idx();
                            let mx = if col == grid.col_rio() { 1 } else { 0 };
                            let (my, mcb) = grid
                                .mcbs
                                .iter()
                                .enumerate()
                                .find(|(_, mcb)| mcb.row_mcb == row)
                                .unwrap();
                            let naming = if grid.is_25() { "MCB_L_BOT" } else { "MCB_L" };
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                naming,
                                [
                                    format!("{naming}_X{x}Y{y}", y = row.to_idx() + 6),
                                    format!("MCB_HCLK_X{x}Y{y}", y = row.to_idx() - 1),
                                    format!("MCB_CAP_CLKPN_X{x}Y{y}", y = mcb.iop_clk.to_idx()),
                                    format!("MCB_INT_DQI_X{x}Y{y}", y = mcb.iop_dqs[0].to_idx()),
                                    format!("MCB_INT_DQI_X{x}Y{y}", y = mcb.iop_dqs[1].to_idx()),
                                    format!("MCB_MUI0R_X{x}Y{y}", y = mcb.row_mui[0].to_idx()),
                                    format!("MCB_MUI0W_X{x}Y{y}", y = mcb.row_mui[1].to_idx()),
                                    format!("MCB_MUI1R_X{x}Y{y}", y = mcb.row_mui[2].to_idx()),
                                    format!("MCB_MUI1W_X{x}Y{y}", y = mcb.row_mui[3].to_idx()),
                                    format!("MCB_MUI2_X{x}Y{y}", y = mcb.row_mui[4].to_idx()),
                                    format!("MCB_MUI3_X{x}Y{y}", y = mcb.row_mui[5].to_idx()),
                                    format!("MCB_MUI4_X{x}Y{y}", y = mcb.row_mui[6].to_idx()),
                                    format!("MCB_MUI5_X{x}Y{y}", y = mcb.row_mui[7].to_idx()),
                                ],
                            );
                            nnode.add_bel(0, format!("MCB_X{mx}Y{my}", my = my * 2 + 1));
                            nnode.add_bel(
                                1,
                                format!(
                                    "TIEOFF_X{x}Y{y}",
                                    x = namer.tiexlut[col] + 1,
                                    y = mcb.iop_clk.to_idx() * 2 + 1
                                ),
                            );
                            nnode.add_bel(
                                2,
                                format!(
                                    "TIEOFF_X{x}Y{y}",
                                    x = namer.tiexlut[col] + 1,
                                    y = mcb.iop_dqs[0].to_idx() * 2 + 1
                                ),
                            );
                            nnode.add_bel(
                                3,
                                format!(
                                    "TIEOFF_X{x}Y{y}",
                                    x = namer.tiexlut[col] + 1,
                                    y = mcb.iop_dqs[1].to_idx() * 2 + 1
                                ),
                            );
                        }
                        "PCILOGICSE" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let ry = namer.rylut[row] - 1;
                            if col == grid.col_lio() {
                                let rx = namer.rxlut[col] - 2;
                                let nnode = namer.ngrid.name_node(
                                    nloc,
                                    "PCILOGICSE_L",
                                    [
                                        if grid.is_25() {
                                            format!("REGH_LIOI_INT_BOT25_X{x}Y{y}")
                                        } else {
                                            format!("REGH_LIOI_INT_X{x}Y{y}", y = y - 1)
                                        },
                                        format!("REG_L_X{rx}Y{ry}"),
                                        if grid.is_25() {
                                            format!("REGH_IOI_BOT25_X{x}Y{y}")
                                        } else {
                                            format!("REGH_IOI_X{x}Y{y}", y = y - 1)
                                        },
                                        format!("INT_X{x}Y{y}"),
                                    ],
                                );
                                nnode.add_bel(0, "PCILOGIC_X0Y0".to_string());
                            } else {
                                let rx = namer.rxlut[col] + 3;
                                let nnode = namer.ngrid.name_node(
                                    nloc,
                                    "PCILOGICSE_R",
                                    [
                                        if grid.is_25() {
                                            format!("REGH_RIOI_BOT25_X{x}Y{y}")
                                        } else {
                                            format!("REGH_RIOI_X{x}Y{y}", y = y - 1)
                                        },
                                        format!("REG_R_X{rx}Y{ry}"),
                                        format!("INT_X{x}Y{y}"),
                                    ],
                                );
                                nnode.add_bel(0, "PCILOGIC_X1Y0".to_string());
                            }
                        }
                        "CLKC" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "CLKC",
                                [
                                    format!(
                                        "CLKC_X{x}Y{y}",
                                        y = if grid.is_25() { y } else { y - 1 }
                                    ),
                                    format!(
                                        "REG_C_CMT_X{x}Y{y}",
                                        y = if grid.is_25() { y } else { y - 1 }
                                    ),
                                ],
                            );
                            for i in 0..16 {
                                nnode.add_bel(
                                    i,
                                    format!(
                                        "BUFGMUX_X{x}Y{y}",
                                        x = if (i & 4) != 0 { 3 } else { 2 },
                                        y = i + 1
                                    ),
                                );
                            }
                        }
                        "HCLK_ROW" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "HCLK_ROW",
                                [if row == grid.row_clk() {
                                    format!(
                                        "REG_V_HCLK_BOT25_X{x}Y{y}",
                                        x = col.to_idx(),
                                        y = row.to_idx() - 1
                                    )
                                } else {
                                    format!(
                                        "REG_V_HCLK_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 2,
                                        ry = namer.rylut[row] - 1
                                    )
                                }],
                            );
                            let hy = row.to_idx() / 16;
                            for i in 0..16 {
                                nnode.add_bel(i, format!("BUFH_X0Y{y}", y = 16 + 32 * hy + i));
                            }
                            for i in 0..16 {
                                nnode.add_bel(16 + i, format!("BUFH_X3Y{y}", y = 32 * hy + i));
                            }
                        }
                        "REG_B" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "REG_B",
                                [
                                    format!(
                                        "REG_B_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 1,
                                        ry = namer.rylut[row] - 2
                                    ),
                                    format!(
                                        "REG_B_BTERM_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 2,
                                        ry = namer.rylut[row] - 1
                                    ),
                                    format!(
                                        "IOI_BTERM_BUFPLL_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 4,
                                        ry = namer.rylut[row] - 1
                                    ),
                                    format!(
                                        "IOI_INT_X{x}Y{y}",
                                        x = col.to_idx() + 1,
                                        y = row.to_idx() + 1
                                    ),
                                ],
                            );
                            nnode.add_bel(0, "BUFIO2_X3Y0".to_string());
                            nnode.add_bel(1, "BUFIO2_X3Y1".to_string());
                            nnode.add_bel(2, "BUFIO2_X3Y6".to_string());
                            nnode.add_bel(3, "BUFIO2_X3Y7".to_string());
                            nnode.add_bel(4, "BUFIO2_X1Y0".to_string());
                            nnode.add_bel(5, "BUFIO2_X1Y1".to_string());
                            nnode.add_bel(6, "BUFIO2_X1Y6".to_string());
                            nnode.add_bel(7, "BUFIO2_X1Y7".to_string());
                            nnode.add_bel(8, "BUFIO2FB_X3Y0".to_string());
                            nnode.add_bel(9, "BUFIO2FB_X3Y1".to_string());
                            nnode.add_bel(10, "BUFIO2FB_X3Y6".to_string());
                            nnode.add_bel(11, "BUFIO2FB_X3Y7".to_string());
                            nnode.add_bel(12, "BUFIO2FB_X1Y0".to_string());
                            nnode.add_bel(13, "BUFIO2FB_X1Y1".to_string());
                            nnode.add_bel(14, "BUFIO2FB_X1Y6".to_string());
                            nnode.add_bel(15, "BUFIO2FB_X1Y7".to_string());
                            nnode.add_bel(16, "BUFPLL_X1Y0".to_string());
                            nnode.add_bel(17, "BUFPLL_X1Y1".to_string());
                            nnode.add_bel(18, "BUFPLL_MCB_X1Y5".to_string());
                            nnode.add_bel(
                                19,
                                format!(
                                    "TIEOFF_X{x}Y{y}",
                                    x = namer.tiexlut[col] + 4,
                                    y = row.to_idx() * 2 + 1
                                ),
                            );
                        }
                        "REG_T" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "REG_T",
                                [
                                    format!(
                                        "REG_T_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 1,
                                        ry = namer.rylut[row] + 2
                                    ),
                                    format!(
                                        "REG_T_TTERM_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 2,
                                        ry = namer.rylut[row] + 1
                                    ),
                                    format!(
                                        "IOI_TTERM_BUFPLL_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 4,
                                        ry = namer.rylut[row] + 1
                                    ),
                                    format!(
                                        "IOI_INT_X{x}Y{y}",
                                        x = col.to_idx() + 1,
                                        y = row.to_idx()
                                    ),
                                ],
                            );
                            nnode.add_bel(0, "BUFIO2_X2Y28".to_string());
                            nnode.add_bel(1, "BUFIO2_X2Y29".to_string());
                            nnode.add_bel(2, "BUFIO2_X2Y26".to_string());
                            nnode.add_bel(3, "BUFIO2_X2Y27".to_string());
                            nnode.add_bel(4, "BUFIO2_X4Y28".to_string());
                            nnode.add_bel(5, "BUFIO2_X4Y29".to_string());
                            nnode.add_bel(6, "BUFIO2_X4Y26".to_string());
                            nnode.add_bel(7, "BUFIO2_X4Y27".to_string());
                            nnode.add_bel(8, "BUFIO2FB_X2Y28".to_string());
                            nnode.add_bel(9, "BUFIO2FB_X2Y29".to_string());
                            nnode.add_bel(10, "BUFIO2FB_X2Y26".to_string());
                            nnode.add_bel(11, "BUFIO2FB_X2Y27".to_string());
                            nnode.add_bel(12, "BUFIO2FB_X4Y28".to_string());
                            nnode.add_bel(13, "BUFIO2FB_X4Y29".to_string());
                            nnode.add_bel(14, "BUFIO2FB_X4Y26".to_string());
                            nnode.add_bel(15, "BUFIO2FB_X4Y27".to_string());
                            nnode.add_bel(16, "BUFPLL_X1Y5".to_string());
                            nnode.add_bel(17, "BUFPLL_X1Y4".to_string());
                            nnode.add_bel(18, "BUFPLL_MCB_X1Y9".to_string());
                            nnode.add_bel(
                                19,
                                format!(
                                    "TIEOFF_X{x}Y{y}",
                                    x = namer.tiexlut[col] + 1,
                                    y = row.to_idx() * 2 + 1
                                ),
                            );
                        }
                        "REG_L" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "REG_L",
                                [
                                    format!(
                                        "REG_L_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] - 2,
                                        ry = namer.rylut[row] - 1
                                    ),
                                    format!(
                                        "REGH_IOI_LTERM_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] - 1,
                                        ry = namer.rylut[row] - 1
                                    ),
                                    format!("INT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx()),
                                    format!("INT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() + 1),
                                ],
                            );
                            nnode.add_bel(0, "BUFIO2_X1Y8".to_string());
                            nnode.add_bel(1, "BUFIO2_X1Y9".to_string());
                            nnode.add_bel(2, "BUFIO2_X1Y14".to_string());
                            nnode.add_bel(3, "BUFIO2_X1Y15".to_string());
                            nnode.add_bel(4, "BUFIO2_X0Y16".to_string());
                            nnode.add_bel(5, "BUFIO2_X0Y17".to_string());
                            nnode.add_bel(6, "BUFIO2_X0Y22".to_string());
                            nnode.add_bel(7, "BUFIO2_X0Y23".to_string());
                            nnode.add_bel(8, "BUFIO2FB_X1Y8".to_string());
                            nnode.add_bel(9, "BUFIO2FB_X1Y9".to_string());
                            nnode.add_bel(10, "BUFIO2FB_X1Y14".to_string());
                            nnode.add_bel(11, "BUFIO2FB_X1Y15".to_string());
                            nnode.add_bel(12, "BUFIO2FB_X0Y16".to_string());
                            nnode.add_bel(13, "BUFIO2FB_X0Y17".to_string());
                            nnode.add_bel(14, "BUFIO2FB_X0Y22".to_string());
                            nnode.add_bel(15, "BUFIO2FB_X0Y23".to_string());
                            nnode.add_bel(16, "BUFPLL_X0Y3".to_string());
                            nnode.add_bel(17, "BUFPLL_X0Y2".to_string());
                            nnode.add_bel(18, "BUFPLL_MCB_X0Y5".to_string());
                            nnode.add_bel(
                                19,
                                format!(
                                    "TIEOFF_X{x}Y{y}",
                                    x = namer.tiexlut[col] + 1,
                                    y = row.to_idx() * 2 - 1
                                ),
                            );
                        }
                        "REG_R" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "REG_R",
                                [
                                    format!(
                                        "REG_R_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 3,
                                        ry = namer.rylut[row] - 1
                                    ),
                                    format!(
                                        "REGH_IOI_RTERM_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 3,
                                        ry = namer.rylut[row] - 1
                                    ),
                                    format!("INT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx()),
                                    format!("INT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() + 1),
                                ],
                            );
                            nnode.add_bel(0, "BUFIO2_X4Y20".to_string());
                            nnode.add_bel(1, "BUFIO2_X4Y21".to_string());
                            nnode.add_bel(2, "BUFIO2_X4Y18".to_string());
                            nnode.add_bel(3, "BUFIO2_X4Y19".to_string());
                            nnode.add_bel(4, "BUFIO2_X3Y12".to_string());
                            nnode.add_bel(5, "BUFIO2_X3Y13".to_string());
                            nnode.add_bel(6, "BUFIO2_X3Y10".to_string());
                            nnode.add_bel(7, "BUFIO2_X3Y11".to_string());
                            nnode.add_bel(8, "BUFIO2FB_X4Y20".to_string());
                            nnode.add_bel(9, "BUFIO2FB_X4Y21".to_string());
                            nnode.add_bel(10, "BUFIO2FB_X4Y18".to_string());
                            nnode.add_bel(11, "BUFIO2FB_X4Y19".to_string());
                            nnode.add_bel(12, "BUFIO2FB_X3Y12".to_string());
                            nnode.add_bel(13, "BUFIO2FB_X3Y13".to_string());
                            nnode.add_bel(14, "BUFIO2FB_X3Y10".to_string());
                            nnode.add_bel(15, "BUFIO2FB_X3Y11".to_string());
                            nnode.add_bel(16, "BUFPLL_X2Y3".to_string());
                            nnode.add_bel(17, "BUFPLL_X2Y2".to_string());
                            nnode.add_bel(18, "BUFPLL_MCB_X2Y5".to_string());
                            nnode.add_bel(
                                19,
                                format!(
                                    "TIEOFF_X{x}Y{y}",
                                    x = namer.tiexlut[col] + 1,
                                    y = row.to_idx() * 2 - 1
                                ),
                            );
                        }
                        "HCLK_V_MIDBUF" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let kind = if row < grid.row_clk() {
                                "REG_V_HCLKBUF_BOT"
                            } else {
                                "REG_V_HCLKBUF_TOP"
                            };
                            namer.ngrid.name_node(
                                nloc,
                                "HCLK_V_MIDBUF",
                                [format!("{kind}_X{x}Y{y}")],
                            );
                        }
                        "CKPIN_V_MIDBUF" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let naming = if row < grid.row_clk() {
                                "REG_V_MIDBUF_BOT"
                            } else {
                                "REG_V_MIDBUF_TOP"
                            };
                            namer
                                .ngrid
                                .name_node(nloc, naming, [format!("{naming}_X{x}Y{y}")]);
                        }
                        "CKPIN_H_MIDBUF" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let lr = if col < grid.col_clk { 'L' } else { 'R' };
                            let kind = match (lr, grid.columns[col].kind) {
                                ('L', ColumnKind::Dsp) => "REGH_DSP_L",
                                ('R', ColumnKind::Dsp | ColumnKind::DspPlus) => "REGH_DSP_R",
                                ('L', ColumnKind::Bram) => "REGH_BRAM_FEEDTHRU_L_GCLK",
                                ('R', ColumnKind::Bram) => "REGH_BRAM_FEEDTHRU_R_GCLK",
                                ('L', ColumnKind::CleXM) => "REGH_CLEXM_INT_GCLKL",
                                ('R', ColumnKind::CleXM | ColumnKind::CleXL) => {
                                    "REGH_CLEXL_INT_CLK"
                                }
                                _ => unreachable!(),
                            };
                            let name = if grid.is_25() {
                                format!("{kind}_X{x}Y{y}")
                            } else {
                                format!("{kind}_X{x}Y{y}", y = y - 1)
                            };
                            namer.ngrid.name_node(nloc, "CKPIN_H_MIDBUF", [name]);
                        }
                        "HCLK" => {
                            let fold = if grid.cols_clk_fold.is_some() {
                                "_FOLD"
                            } else {
                                ""
                            };
                            let naming = if grid.cols_clk_fold.is_some() {
                                "HCLK_FOLD"
                            } else {
                                "HCLK"
                            };

                            let x = col.to_idx();
                            let y = row.to_idx();
                            let mut name = match grid.columns[col].kind {
                                ColumnKind::CleXL | ColumnKind::CleClk => {
                                    format!("HCLK_CLB_XL_INT{fold}_X{x}Y{y}", y = y - 1)
                                }
                                ColumnKind::CleXM => {
                                    format!("HCLK_CLB_XM_INT{fold}_X{x}Y{y}", y = y - 1)
                                }
                                ColumnKind::Bram => {
                                    format!("BRAM_HCLK_FEEDTHRU{fold}_X{x}Y{y}", y = y - 1)
                                }
                                ColumnKind::Dsp | ColumnKind::DspPlus => {
                                    format!("DSP_INT_HCLK_FEEDTHRU{fold}_X{x}Y{y}", y = y - 1)
                                }
                                ColumnKind::Io => {
                                    if col == grid.col_lio() {
                                        format!("HCLK_IOIL_INT{fold}_X{x}Y{y}", y = y - 1)
                                    } else {
                                        format!("HCLK_IOIR_INT{fold}_X{x}Y{y}", y = y - 1)
                                    }
                                }
                            };
                            if let Gts::Single(cl) | Gts::Double(cl, _) | Gts::Quad(cl, _) =
                                grid.gts
                            {
                                if col == cl + 2 && row == grid.row_top() - 24 {
                                    name = format!("HCLK_CLB_XM_INT{fold}_X{x}Y{y}", y = y - 1);
                                }
                                if col == cl + 3 && row == grid.row_top() - 8 {
                                    name = format!("HCLK_CLB_XL_INT{fold}_X{x}Y{y}", y = y - 1);
                                }
                            }
                            if let Gts::Double(_, cr) | Gts::Quad(_, cr) = grid.gts {
                                if col == cr + 6 && row == grid.row_top() - 8 {
                                    name = format!("HCLK_CLB_XL_INT{fold}_X{x}Y{y}", y = y - 1);
                                }
                            }
                            if let Gts::Quad(cl, cr) = grid.gts {
                                if col == cl - 6 && row == grid.row_bio_outer() + 8 {
                                    name = format!("DSP_INT_HCLK_FEEDTHRU{fold}_X{x}Y{y}");
                                }
                                if (col == cl - 5
                                    || col == cl + 3
                                    || col == cl + 4
                                    || col == cr - 3
                                    || col == cr + 6)
                                    && row == grid.row_bio_outer() + 8
                                {
                                    name = format!("HCLK_CLB_XL_INT{fold}_X{x}Y{y}");
                                }
                                if col == cr - 4 && row == grid.row_bio_outer() + 8 {
                                    name = format!("HCLK_CLB_XM_INT{fold}_X{x}Y{y}");
                                }
                            }
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "HCLK_H_MIDBUF" => {
                            let x = col.to_idx();
                            let y = row.to_idx();
                            let rx = namer.rxlut[col];
                            let ry = namer.rylut[row];
                            let mut name = format!("DSP_HCLK_GCLK_FOLD_X{x}Y{y}", y = y - 1);
                            let mut naming = "DSP_HCLK_GCLK_FOLD";
                            if let Gts::Double(_, cr) | Gts::Quad(_, cr) = grid.gts {
                                if col == cr + 6 && row == grid.row_top() - 8 {
                                    name = format!("GTPDUAL_DSP_FEEDTHRU_X{rx}Y{ry}", rx = rx + 1);
                                    naming = "GTPDUAL_DSP_FEEDTHRU";
                                }
                            }
                            if let Gts::Quad(cl, cr) = grid.gts {
                                if col == cl - 6 && row == grid.row_bio_outer() + 8 {
                                    name = format!("DSP_HCLK_GCLK_FOLD_X{x}Y{y}");
                                }
                                if col == cr + 6 && row == grid.row_bio_outer() + 8 {
                                    name = format!("GTPDUAL_DSP_FEEDTHRU_X{x}Y{y}");
                                    naming = "GTPDUAL_DSP_FEEDTHRU";
                                }
                            }
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "CLKPIN_BUF" => {
                            if col == grid.col_lio() {
                                let (_, name) = namer.get_lterm_name(col, row);
                                let naming = if row == grid.row_clk() - 2 {
                                    "CLKPIN_BUF.L.BOT"
                                } else if row == grid.row_clk() - 1 {
                                    "CLKPIN_BUF.L.TOP"
                                } else if row == grid.row_clk() + 2 {
                                    "CLKPIN_BUF.L.BOT"
                                } else if row == grid.row_clk() + 3 {
                                    "CLKPIN_BUF.L.TOP"
                                } else {
                                    unreachable!()
                                };
                                namer.ngrid.name_node(nloc, naming, [name]);
                            } else if col == grid.col_rio() {
                                let (_, name) = namer.get_rterm_name(col, row);
                                let naming = if row == grid.row_clk() - 2 {
                                    "CLKPIN_BUF.R.BOT"
                                } else if row == grid.row_clk() - 1 {
                                    "CLKPIN_BUF.R.TOP"
                                } else if row == grid.row_clk() + 2 {
                                    "CLKPIN_BUF.R.BOT"
                                } else if row == grid.row_clk() + 3 {
                                    "CLKPIN_BUF.R.TOP"
                                } else {
                                    unreachable!()
                                };
                                namer.ngrid.name_node(nloc, naming, [name]);
                            } else if row == grid.row_bio_outer() {
                                let name = namer.get_ioi_bterm_name(col);
                                namer.ngrid.name_node(nloc, "CLKPIN_BUF.B.BOT", [name]);
                            } else if row == grid.row_bio_inner() {
                                let name = namer.get_ioi_bterm_name(col);
                                namer.ngrid.name_node(nloc, "CLKPIN_BUF.B.TOP", [name]);
                            } else if row == grid.row_tio_inner() {
                                let name = namer.get_ioi_tterm_name(col);
                                namer.ngrid.name_node(nloc, "CLKPIN_BUF.T.BOT", [name]);
                            } else if row == grid.row_tio_outer() {
                                let name = namer.get_ioi_tterm_name(col);
                                namer.ngrid.name_node(nloc, "CLKPIN_BUF.T.TOP", [name]);
                            } else {
                                unreachable!()
                            }
                        }
                        "BTIOI_CLK" => {
                            if row < grid.row_clk() {
                                let name = namer.get_ioi_bterm_name(col);
                                namer.ngrid.name_node(nloc, "BIOI_CLK", [name]);
                            } else {
                                let name = namer.get_ioi_tterm_name(col);
                                namer.ngrid.name_node(nloc, "TIOI_CLK", [name]);
                            }
                        }
                        "LRIOI_CLK" => {
                            let name = namer.get_hclk_ioi_name(col, row);
                            if col == grid.col_lio() {
                                let name_term = if row == grid.row_clk() {
                                    format!(
                                        "HCLK_IOI_LTERM_BOT25_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] - 1,
                                        ry = namer.rylut[row] - 2
                                    )
                                } else {
                                    format!(
                                        "HCLK_IOI_LTERM_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] - 1,
                                        ry = namer.rylut[row] - 1
                                    )
                                };
                                namer
                                    .ngrid
                                    .name_node(nloc, "LRIOI_CLK.L", [name, name_term]);
                            } else if col == grid.col_rio() {
                                let name_term = if row == grid.row_clk() {
                                    format!(
                                        "HCLK_IOI_RTERM_BOT25_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 3,
                                        ry = namer.rylut[row] - 2
                                    )
                                } else {
                                    format!(
                                        "HCLK_IOI_RTERM_X{rx}Y{ry}",
                                        rx = namer.rxlut[col] + 3,
                                        ry = namer.rylut[row] - 1
                                    )
                                };
                                namer
                                    .ngrid
                                    .name_node(nloc, "LRIOI_CLK.R", [name, name_term]);
                            } else {
                                unreachable!()
                            }
                        }
                        "PCI_CE_SPLIT" => {
                            let name = namer.get_hclk_ioi_name(col, row);
                            namer.ngrid.name_node(nloc, "PCI_CE_SPLIT", [name]);
                        }
                        "PCI_CE_V_BUF" => {
                            let name = namer.get_hclk_ioi_name(col, row);
                            let naming = if row <= grid.row_clk() {
                                if row < grid.rows_pci_ce_split.0 {
                                    "PCI_CE_V_BUF_DN"
                                } else {
                                    "PCI_CE_V_BUF_UP"
                                }
                            } else {
                                if row < grid.rows_pci_ce_split.1 {
                                    "PCI_CE_V_BUF_DN"
                                } else {
                                    "PCI_CE_V_BUF_UP"
                                }
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "PCI_CE_TRUNK_BUF" => {
                            let name = namer.get_hclk_ioi_name(col, row);
                            let naming = if row <= grid.row_clk() {
                                "PCI_CE_TRUNK_BUF_BOT"
                            } else {
                                "PCI_CE_TRUNK_BUF_TOP"
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "PCI_CE_H_BUF" => match grid.columns[col].kind {
                            ColumnKind::Io => {
                                let rx = namer.rxlut[col];
                                let ry = if row == grid.row_bio_outer() {
                                    namer.rylut[row] - 1
                                } else if row == grid.row_tio_outer() {
                                    namer.rylut[row] + 1
                                } else {
                                    unreachable!()
                                };
                                let name = if col == grid.col_lio() {
                                    format!("IOI_PCI_CE_LEFT_X{rx}Y{ry}")
                                } else if col == grid.col_rio() {
                                    format!("IOI_PCI_CE_RIGHT_X{rx}Y{ry}")
                                } else {
                                    unreachable!()
                                };
                                namer.ngrid.name_node(nloc, "PCI_CE_H_BUF_CNR", [name]);
                            }
                            ColumnKind::Bram => {
                                let rx = namer.rxlut[col];
                                let lr = if col < grid.col_clk { 'L' } else { 'R' };
                                let name = if row == grid.row_bio_outer() {
                                    let ry = namer.rylut[row];
                                    format!(
                                        "BRAM_BOT_BTERM_{lr}_X{rx}Y{ry}",
                                        rx = rx + 2,
                                        ry = ry - 1
                                    )
                                } else if row == grid.row_tio_outer() {
                                    let ry = namer.rylut[row];
                                    format!(
                                        "BRAM_TOP_TTERM_{lr}_X{rx}Y{ry}",
                                        rx = rx + 2,
                                        ry = ry + 1
                                    )
                                } else {
                                    unreachable!()
                                };
                                namer.ngrid.name_node(nloc, "PCI_CE_H_BUF_BRAM", [name]);
                            }
                            ColumnKind::Dsp | ColumnKind::DspPlus => {
                                let rx = namer.rxlut[col];
                                let lr = if col < grid.col_clk { 'L' } else { 'R' };
                                let name = if row == grid.row_bio_outer() {
                                    let ry = namer.rylut[row];
                                    format!(
                                        "DSP_BOT_BTERM_{lr}_X{rx}Y{ry}",
                                        rx = rx + 2,
                                        ry = ry - 1
                                    )
                                } else if row == grid.row_tio_outer() {
                                    let ry = namer.rylut[row];
                                    format!(
                                        "DSP_TOP_TTERM_{lr}_X{rx}Y{ry}",
                                        rx = rx + 2,
                                        ry = ry + 1
                                    )
                                } else {
                                    unreachable!()
                                };
                                namer.ngrid.name_node(nloc, "PCI_CE_H_BUF_DSP", [name]);
                            }
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    }
                }
                for (dir, term) in &die[(col, row)].terms {
                    let Some(term) = term else { continue };
                    let tloc = (die.die, col, row, dir);
                    let kind = egrid.db.terms.key(term.kind);

                    match &kind[..] {
                        "TERM.W" => {
                            let (naming, name) = namer.get_lterm_name(col, row);
                            namer.ngrid.name_term_tile(tloc, naming, name);
                        }
                        "TERM.E" => {
                            let (naming, name) = namer.get_rterm_name(col, row);
                            namer.ngrid.name_term_tile(tloc, naming, name);
                        }
                        "TERM.S" => {
                            let kind = match grid.columns[col].kind {
                                ColumnKind::Io => "CNR_BR_BTERM",
                                ColumnKind::Bram => unreachable!(),
                                ColumnKind::Dsp | ColumnKind::DspPlus => "DSP_INT_BTERM",
                                _ => {
                                    if col == grid.col_clk + 1 {
                                        "IOI_BTERM_BUFPLL"
                                    } else if grid.columns[col].bio == ColumnIoKind::None {
                                        "CLB_INT_BTERM"
                                    } else {
                                        "IOI_BTERM"
                                    }
                                }
                            };
                            let rx = namer.rxlut[col];
                            let ry = namer.rylut[grid.row_bio_outer()] - 1;
                            let name = format!("{kind}_X{rx}Y{ry}");
                            namer.ngrid.name_term_tile(tloc, "TERM.S", name);
                        }
                        "TERM.N" => {
                            let kind = match grid.columns[col].kind {
                                ColumnKind::Io => "CNR_TR_TTERM",
                                ColumnKind::Bram => "RAMB_TOP_TTERM",
                                ColumnKind::Dsp | ColumnKind::DspPlus => "DSP_INT_TTERM",
                                _ => {
                                    if col == grid.col_clk + 1 {
                                        "IOI_TTERM_BUFPLL"
                                    } else {
                                        "IOI_TTERM"
                                    }
                                }
                            };
                            let rx = namer.rxlut[col];
                            let ry = namer.rylut[grid.row_tio_outer()] + 1;
                            let name = format!("{kind}_X{rx}Y{ry}");
                            namer.ngrid.name_term_tile(tloc, "TERM.N", name);
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    let mut pad_cnt = 1;
    for &io in &edev.io {
        let (col, row, bel) = grid.get_io_loc(io);
        let layer = edev
            .egrid
            .find_node_loc(DieId::from_idx(0), (col, row), |node| {
                edev.egrid.db.nodes.key(node.kind) == "IOB"
            })
            .unwrap()
            .0;
        let nnode = namer
            .ngrid
            .nodes
            .get_mut(&(DieId::from_idx(0), col, row, layer))
            .unwrap();
        nnode.add_bel(bel.to_idx(), format!("PAD{pad_cnt}"));
        pad_cnt += 1;
    }

    ExpandedNamedDevice {
        edev,
        ngrid: namer.ngrid,
        grid,
    }
}
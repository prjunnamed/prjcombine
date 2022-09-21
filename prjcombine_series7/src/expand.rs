use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use std::collections::{BTreeSet, HashMap};

use crate::{
    ColumnKind, DisabledPart, ExpandedDevice, ExtraDie, Grid, GridKind, GtKind, IoKind, Pcie2Kind,
    XadcKind,
};

struct DieExpander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    die: ExpandedDieRefMut<'a, 'b>,
    xlut: EntityVec<ColId, usize>,
    rxlut: EntityVec<ColId, usize>,
    tiexlut: EntityVec<ColId, usize>,
    ipxlut: EntityVec<ColId, usize>,
    opxlut: EntityVec<ColId, usize>,
    ylut: EntityVec<RowId, usize>,
    rylut: EntityVec<RowId, usize>,
    tieylut: EntityVec<RowId, usize>,
    dciylut: EntityVec<RowId, usize>,
    ipylut: EntityVec<RowId, usize>,
    opylut: EntityVec<RowId, usize>,
    gtylut: EntityVec<RowId, usize>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    has_slr_d: bool,
    has_slr_u: bool,
    has_gtz_d: bool,
    has_gtz_u: bool,
}

impl<'a, 'b> DieExpander<'a, 'b> {
    fn fill_ylut(&mut self, yb: usize) -> usize {
        let mut y = yb;
        for _ in self.die.rows() {
            self.ylut.push(y);
            y += 1;
        }
        y
    }

    fn fill_rylut(&mut self, ryb: usize) -> usize {
        let mut y = ryb;
        for row in self.die.rows() {
            if row.to_idx() % 25 == 0 {
                y += 1;
            }
            self.rylut.push(y);
            y += 1;
        }
        y + 1
    }

    fn fill_tieylut(&mut self, tyb: usize) -> usize {
        let mut y = tyb;
        for _ in self.die.rows() {
            self.tieylut.push(y);
            y += 1;
        }
        y
    }

    fn fill_dciylut(&mut self, mut dciy: usize) -> usize {
        for row in self.die.rows() {
            self.dciylut.push(dciy);
            if row.to_idx() % 50 == 25
                && self
                    .grid
                    .cols_io
                    .iter()
                    .flatten()
                    .any(|x| x.regs[row.to_idx() / 50] == Some(IoKind::Hpio))
            {
                dciy += 1;
            }
        }
        dciy
    }

    fn fill_ipylut(&mut self, mut ipy: usize, is_7k70t: bool) -> usize {
        for row in self.die.rows() {
            let reg = row.to_idx() / 50;
            self.ipylut.push(ipy);
            if matches!(row.to_idx() % 50, 0 | 11 | 22 | 28 | 39) {
                let mut has_gt = false;
                for gtcol in self.grid.cols_gt.iter().flatten() {
                    if gtcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if let Some((ref lcol, ref rcol)) = self.grid.cols_gtp_mid {
                    if lcol.regs[reg].is_some() || rcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if has_gt {
                    ipy += 6;
                }
            }
            if !is_7k70t && row == RowId::from_idx(self.grid.reg_cfg * 50 + 25) {
                ipy += 6;
            }
        }
        if is_7k70t {
            self.ipylut[RowId::from_idx(self.grid.reg_cfg * 50 + 25)] = ipy + 6;
        }
        ipy
    }

    fn fill_opylut(&mut self, mut opy: usize) -> usize {
        for row in self.die.rows() {
            let reg = row.to_idx() / 50;
            self.opylut.push(opy);
            if matches!(row.to_idx() % 50, 0 | 11 | 28 | 39) {
                let mut has_gt = false;
                for gtcol in self.grid.cols_gt.iter().flatten() {
                    if gtcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if let Some((ref lcol, ref rcol)) = self.grid.cols_gtp_mid {
                    if lcol.regs[reg].is_some() || rcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if has_gt {
                    opy += 2;
                }
            }
        }
        opy
    }

    fn fill_gtylut(&mut self, mut gty: usize) -> usize {
        for row in self.die.rows() {
            let reg = row.to_idx() / 50;
            self.gtylut.push(gty);
            if row.to_idx() % 50 == 0 {
                let mut has_gt = false;
                for gtcol in self.grid.cols_gt.iter().flatten() {
                    if gtcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if let Some((ref lcol, ref rcol)) = self.grid.cols_gtp_mid {
                    if lcol.regs[reg].is_some() || rcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if has_gt {
                    gty += 1;
                }
            }
        }
        gty
    }

    fn fill_xlut(&mut self) {
        let mut x = 0;
        for col in self.grid.columns.ids() {
            self.xlut.push(x);
            if self.grid.regs == 2 && self.grid.has_ps && col.to_idx() < 18 {
                continue;
            }
            if self.grid.regs <= 2 && col < self.grid.col_cfg && col >= self.grid.col_cfg - 6 {
                continue;
            }
            x += 1;
        }
    }

    fn fill_rxlut(&mut self) {
        let mut rx = 0;
        for (col, &kind) in &self.grid.columns {
            if self.grid.has_ps && self.grid.regs == 2 && col.to_idx() == 18 {
                rx -= 19;
            }
            if self.grid.cols_vbrk.contains(&col) {
                rx += 1;
            }
            if kind == ColumnKind::Bram && col.to_idx() == 0 {
                rx += 1;
            }
            self.rxlut.push(rx);
            match kind {
                ColumnKind::ClbLL | ColumnKind::ClbLM => rx += 2,
                ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Clk | ColumnKind::Cfg => rx += 3,
                ColumnKind::Io => {
                    if col == self.die.cols().next().unwrap()
                        || col == self.die.cols().next_back().unwrap()
                    {
                        rx += 5;
                    } else {
                        rx += 4;
                    }
                }
                ColumnKind::Gt | ColumnKind::Cmt => rx += 4,
            }
        }
    }

    fn fill_tiexlut(&mut self) {
        let mut tie_x = 0;
        for (col, &kind) in &self.grid.columns {
            if self.grid.regs == 2 && self.grid.has_ps && col.to_idx() < 18 {
                self.tiexlut.push(tie_x);
                continue;
            }
            if self.grid.regs <= 2 && col < self.grid.col_cfg && col >= self.grid.col_cfg - 6 {
                self.tiexlut.push(tie_x);
                continue;
            }
            let lr = ['L', 'R'][col.to_idx() % 2];
            if lr == 'L' && kind == ColumnKind::Dsp {
                tie_x += 1;
            }
            self.tiexlut.push(tie_x);
            tie_x += 1;
            if lr == 'R' && kind == ColumnKind::Dsp {
                tie_x += 1;
            }
        }
    }

    fn fill_ipxlut(&mut self, has_gtz: bool, is_7k70t: bool) {
        let mut ipx = 0;
        for (col, &kind) in &self.grid.columns {
            self.ipxlut.push(ipx);
            for gtcol in self.grid.cols_gt.iter().flatten() {
                if gtcol.col == col {
                    ipx += 1;
                }
            }
            if let Some((ref lcol, ref rcol)) = self.grid.cols_gtp_mid {
                if lcol.col == col || rcol.col == col {
                    ipx += 1;
                }
            }
            if kind == ColumnKind::Cfg && !is_7k70t {
                ipx += 1;
            }
            if kind == ColumnKind::Clk && has_gtz {
                ipx += 1;
            }
        }
    }

    fn fill_opxlut(&mut self, has_gtz: bool) {
        let mut opx = 0;
        for (col, &kind) in &self.grid.columns {
            self.opxlut.push(opx);
            for gtcol in self.grid.cols_gt.iter().flatten() {
                if gtcol.col == col {
                    opx += 1;
                }
            }
            if let Some((ref lcol, ref rcol)) = self.grid.cols_gtp_mid {
                if lcol.col == col || rcol.col == col {
                    opx += 1;
                }
            }
            if kind == ColumnKind::Clk && has_gtz {
                opx += 1;
            }
        }
    }

    fn fill_int(&mut self) {
        for (col, &kind) in &self.grid.columns {
            let x = self.xlut[col];
            let lr = ['L', 'R'][col.to_idx() % 2];
            for row in self.die.rows() {
                let y = self.ylut[row];
                self.die.fill_tile(
                    (col, row),
                    "INT",
                    &format!("INT.{lr}"),
                    format!("INT_{lr}_X{x}Y{y}"),
                );
                let tie_x = self.tiexlut[col];
                let tie_y = self.tieylut[row];
                self.die[(col, row)].nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Io => {
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("INTF"),
                            &[&format!("IO_INT_INTERFACE_{lr}_X{x}Y{y}")],
                            self.db.get_node_naming(&format!("INTF.{lr}")),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Bram => {
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("INTF.BRAM"),
                            &[&format!("BRAM_INT_INTERFACE_{lr}_X{x}Y{y}")],
                            self.db.get_node_naming(&format!("INTF.{lr}")),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Dsp | ColumnKind::Cmt | ColumnKind::Cfg | ColumnKind::Clk => {
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("INTF"),
                            &[&format!("INT_INTERFACE_{lr}_X{x}Y{y}")],
                            self.db.get_node_naming(&format!("INTF.{lr}")),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gt => (),
                }
            }
        }
    }

    fn fill_cfg(&mut self, is_master: bool) {
        let row_cm = RowId::from_idx(self.grid.reg_cfg * 50);
        let row_cb = row_cm - 50;
        let row_ct = row_cm + 50;
        if self.grid.regs == 1 {
            self.die.nuke_rect(self.grid.col_cfg - 6, row_cb, 6, 50);
            self.int_holes.push(Rect {
                col_l: self.grid.col_cfg - 6,
                col_r: self.grid.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 50,
            });
            self.site_holes.push(Rect {
                col_l: self.grid.col_cfg - 6,
                col_r: self.grid.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 50,
            });
        } else {
            self.die.nuke_rect(self.grid.col_cfg - 6, row_cb, 6, 100);
            self.int_holes.push(Rect {
                col_l: self.grid.col_cfg - 6,
                col_r: self.grid.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 100,
            });
            self.site_holes.push(Rect {
                col_l: self.grid.col_cfg - 6,
                col_r: self.grid.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 100,
            });
            for dx in 0..6 {
                let col = self.grid.col_cfg - 6 + dx;
                if row_cb.to_idx() != 0 {
                    self.die.fill_term_anon((col, row_cb - 1), "TERM.N");
                }
                if row_ct.to_idx() != self.grid.regs * 50 {
                    self.die.fill_term_anon((col, row_ct), "TERM.S");
                }
            }
        }

        let slv = if is_master { "" } else { "_SLAVE" };
        let rx = self.rxlut[self.grid.col_cfg] - 1;
        let name_b = format!("CFG_CENTER_BOT_X{rx}Y{y}", y = self.rylut[row_cb + 10]);
        let name_m = format!("CFG_CENTER_MID{slv}_X{rx}Y{y}", y = self.rylut[row_cb + 30]);
        let name_t = format!("CFG_CENTER_TOP{slv}_X{rx}Y{y}", y = self.rylut[row_cb + 40]);
        let crds: [_; 50] = core::array::from_fn(|dy| (self.grid.col_cfg, row_cb + dy));
        let di = self.die.die.to_idx();
        let node = self.die[(self.grid.col_cfg, row_cb)].add_xnode(
            self.db.get_node("CFG"),
            &[&name_b, &name_m, &name_t],
            self.db.get_node_naming("CFG"),
            &crds,
        );
        node.add_bel(0, format!("BSCAN_X0Y{y}", y = di * 4));
        node.add_bel(1, format!("BSCAN_X0Y{y}", y = di * 4 + 1));
        node.add_bel(2, format!("BSCAN_X0Y{y}", y = di * 4 + 2));
        node.add_bel(3, format!("BSCAN_X0Y{y}", y = di * 4 + 3));
        node.add_bel(4, format!("ICAP_X0Y{y}", y = di * 2));
        node.add_bel(5, format!("ICAP_X0Y{y}", y = di * 2 + 1));
        node.add_bel(6, format!("STARTUP_X0Y{di}"));
        node.add_bel(7, format!("CAPTURE_X0Y{di}"));
        node.add_bel(8, format!("FRAME_ECC_X0Y{di}"));
        node.add_bel(9, format!("USR_ACCESS_X0Y{di}"));
        node.add_bel(10, format!("CFG_IO_ACCESS_X0Y{di}"));
        let pix = if self.grid.col_cfg < self.grid.col_clk {
            0
        } else {
            1
        };
        let piy = if self.grid.reg_cfg < self.grid.reg_clk {
            di * 2
        } else {
            di * 2 + 1
        };
        node.add_bel(11, format!("PMVIOB_X{pix}Y{piy}"));
        node.add_bel(12, format!("DCIRESET_X0Y{di}"));
        node.add_bel(13, format!("DNA_PORT_X0Y{di}"));
        node.add_bel(14, format!("EFUSE_USR_X0Y{di}"));

        if self.grid.regs != 1 {
            let row_m = row_cm + 25;
            let kind = match self.grid.xadc_kind {
                XadcKind::Right => "XADC.R",
                XadcKind::Left => "XADC.L",
                XadcKind::Both => "XADC.LR",
            };
            let suf = match kind {
                "XADC.LR" => "",
                "XADC.L" => "_FUJI2",
                "XADC.R" => "_PELE1",
                _ => unreachable!(),
            };
            let name_b = format!("MONITOR_BOT{suf}{slv}_X{rx}Y{y}", y = self.rylut[row_m]);
            let name_m = format!("MONITOR_MID{suf}_X{rx}Y{y}", y = self.rylut[row_m + 10]);
            let name_t = format!("MONITOR_TOP{suf}_X{rx}Y{y}", y = self.rylut[row_m + 20]);
            let name_bs = format!("CFG_SECURITY_BOT_PELE1_X{rx}Y{y}", y = self.rylut[row_cm]);
            let name_ms = format!(
                "CFG_SECURITY_MID_PELE1_X{rx}Y{y}",
                y = self.rylut[row_cm + 10]
            );
            let name_ts = format!(
                "CFG_SECURITY_TOP_PELE1_X{rx}Y{y}",
                y = self.rylut[row_cm + 20]
            );
            let crds: [_; 25] = core::array::from_fn(|dy| (self.grid.col_cfg, row_m + dy));
            let di = self.die.die.to_idx();
            let mut names = vec![&name_b[..], &name_m[..], &name_t[..]];
            if self.grid.xadc_kind == XadcKind::Right {
                names.extend([&name_bs[..], &name_ms[..], &name_ts[..]]);
            }
            let node = self.die[(self.grid.col_cfg, row_m)].add_xnode(
                self.db.get_node("XADC"),
                &names,
                self.db.get_node_naming(kind),
                &crds,
            );
            node.add_bel(
                0,
                format!(
                    "IPAD_X{x}Y{y}",
                    x = self.ipxlut[self.grid.col_cfg],
                    y = self.ipylut[row_m],
                ),
            );
            node.add_bel(
                1,
                format!(
                    "IPAD_X{x}Y{y}",
                    x = self.ipxlut[self.grid.col_cfg],
                    y = self.ipylut[row_m] + 1,
                ),
            );
            node.add_bel(2, format!("XADC_X0Y{di}"));
        }
    }

    fn fill_ps(&mut self) {
        if self.grid.has_ps {
            let col_l = self.die.cols().next().unwrap();
            let row_t = self.die.rows().next_back().unwrap();
            let row_pb = row_t - 99;
            self.die.nuke_rect(col_l, row_pb, 18, 100);
            self.int_holes.push(Rect {
                col_l,
                col_r: col_l + 18,
                row_b: row_pb,
                row_t: row_pb + 100,
            });
            self.site_holes.push(Rect {
                col_l,
                col_r: col_l + 19,
                row_b: row_pb,
                row_t: row_pb + 100,
            });
            if self.grid.regs != 2 {
                for dx in 0..18 {
                    let col = col_l + dx;
                    self.die.fill_term_anon((col, row_pb - 1), "TERM.N");
                }
            }
            let col = col_l + 18;
            for dy in 0..100 {
                let row = row_pb + dy;
                self.die.fill_term_anon((col, row), "TERM.W");
                let y = self.ylut[row];
                let x = self.xlut[col];
                self.die[(col, row)].add_xnode(
                    self.db.get_node("INTF"),
                    &[&format!("INT_INTERFACE_PSS_L_X{x}Y{y}")],
                    self.db.get_node_naming("INTF.PSS"),
                    &[(col, row)],
                );
            }

            let crds: [_; 100] = core::array::from_fn(|dy| (col, row_pb + dy));
            let rx = self.rxlut[col] - 18;
            let name_pss0 = format!("PSS0_X{rx}Y{y}", y = self.rylut[row_pb + 10]);
            let name_pss1 = format!("PSS1_X{rx}Y{y}", y = self.rylut[row_pb + 30]);
            let name_pss2 = format!("PSS2_X{rx}Y{y}", y = self.rylut[row_pb + 50]);
            let name_pss3 = format!("PSS3_X{rx}Y{y}", y = self.rylut[row_pb + 70]);
            let name_pss4 = format!("PSS4_X{rx}Y{y}", y = self.rylut[row_pb + 90]);
            let node = self.die[(col, row_pb + 50)].add_xnode(
                self.db.get_node("PS"),
                &[&name_pss0, &name_pss1, &name_pss2, &name_pss3, &name_pss4],
                self.db.get_node_naming("PS"),
                &crds,
            );
            node.add_bel(0, "PS7_X0Y0".to_string());
            for i in 1..73 {
                node.add_bel(i, format!("IOPAD_X1Y{i}"));
            }
            for i in 77..135 {
                node.add_bel(i - 4, format!("IOPAD_X1Y{i}"));
            }
        }
    }

    fn fill_pcie2(&mut self, pcie2_y: usize) -> usize {
        let has_pcie2_left = self.grid.pcie2.iter().any(|x| x.kind == Pcie2Kind::Left);
        let mut ply = pcie2_y;
        let mut pry = pcie2_y;
        for pcie2 in &self.grid.pcie2 {
            self.die.nuke_rect(pcie2.col + 1, pcie2.row, 2, 25);
            self.site_holes.push(Rect {
                col_l: pcie2.col,
                col_r: pcie2.col + 4,
                row_b: pcie2.row,
                row_t: pcie2.row + 25,
            });
            self.int_holes.push(Rect {
                col_l: pcie2.col + 1,
                col_r: pcie2.col + 3,
                row_b: pcie2.row,
                row_t: pcie2.row + 25,
            });
            for dx in 1..3 {
                let col = pcie2.col + dx;
                if pcie2.row.to_idx() != 0 {
                    self.die.fill_term_anon((col, pcie2.row - 1), "TERM.N");
                }
                self.die.fill_term_anon((col, pcie2.row + 25), "TERM.S");
            }
            let col_l = pcie2.col;
            let col_r = pcie2.col + 3;
            let xl = self.xlut[col_l];
            let xr = self.xlut[col_r];
            for dy in 0..25 {
                let row = pcie2.row + dy;
                let y = self.ylut[row];
                let tile_l = &mut self.die[(col_l, row)];
                tile_l.nodes.truncate(1);
                tile_l.add_xnode(
                    self.db.get_node("INTF.DELAY"),
                    &[&format!("PCIE_INT_INTERFACE_R_X{xl}Y{y}")],
                    self.db.get_node_naming("INTF.PCIE_R"),
                    &[(col_l, row)],
                );
                let tile_r = &mut self.die[(col_r, row)];
                tile_r.nodes.truncate(1);
                if pcie2.kind == Pcie2Kind::Left {
                    tile_r.add_xnode(
                        self.db.get_node("INTF.DELAY"),
                        &[&format!("PCIE_INT_INTERFACE_LEFT_L_X{xr}Y{y}")],
                        self.db.get_node_naming("INTF.PCIE_LEFT_L"),
                        &[(col_r, row)],
                    );
                } else {
                    tile_r.add_xnode(
                        self.db.get_node("INTF.DELAY"),
                        &[&format!("PCIE_INT_INTERFACE_L_X{xr}Y{y}")],
                        self.db.get_node_naming("INTF.PCIE_L"),
                        &[(col_r, row)],
                    );
                }
            }
            let mut crds = vec![];
            for dy in 0..25 {
                crds.push((pcie2.col, pcie2.row + dy));
            }
            for dy in 0..25 {
                crds.push((pcie2.col + 3, pcie2.row + dy));
            }
            let kind;
            let tkb;
            let tkt;
            let sx;
            let sy;
            match pcie2.kind {
                Pcie2Kind::Left => {
                    tkb = "PCIE_BOT_LEFT";
                    tkt = "PCIE_TOP_LEFT";
                    kind = "PCIE_L";
                    sy = ply;
                    ply += 1;
                    sx = 0;
                }
                Pcie2Kind::Right => {
                    tkb = "PCIE_BOT";
                    tkt = "PCIE_TOP";
                    kind = "PCIE_R";
                    sy = pry;
                    pry += 1;
                    sx = if has_pcie2_left { 1 } else { 0 };
                }
            }
            let x = self.rxlut[pcie2.col] + 2;
            let y = self.rylut[pcie2.row];
            let name_b = format!("{tkb}_X{x}Y{y}", y = y + 10);
            let name_t = format!("{tkt}_X{x}Y{y}", y = y + 20);
            let node = self.die[crds[0]].add_xnode(
                self.db.get_node(kind),
                &[&name_b, &name_t],
                self.db.get_node_naming(kind),
                &crds,
            );
            node.add_bel(0, format!("PCIE_X{sx}Y{sy}"));
        }
        pry
    }

    fn fill_pcie3(&mut self, mut pcie3_y: usize) -> usize {
        for &(bc, br) in &self.grid.pcie3 {
            self.die.nuke_rect(bc + 1, br, 4, 50);
            self.int_holes.push(Rect {
                col_l: bc + 1,
                col_r: bc + 5,
                row_b: br,
                row_t: br + 50,
            });
            self.site_holes.push(Rect {
                col_l: bc,
                col_r: bc + 6,
                row_b: br,
                row_t: br + 50,
            });
            for dx in 1..5 {
                let col = bc + dx;
                self.die.fill_term_anon((col, br - 1), "TERM.N");
                self.die.fill_term_anon((col, br + 50), "TERM.S");
            }
            let col_l = bc;
            let col_r = bc + 5;
            let xl = self.xlut[col_l];
            let xr = self.xlut[col_r];
            for dy in 0..50 {
                let row = br + dy;
                let y = self.ylut[row];
                let tile_l = &mut self.die[(col_l, row)];
                tile_l.nodes.truncate(1);
                tile_l.add_xnode(
                    self.db.get_node("INTF.DELAY"),
                    &[&format!("PCIE3_INT_INTERFACE_R_X{xl}Y{y}")],
                    self.db.get_node_naming("INTF.PCIE3_R"),
                    &[(col_l, row)],
                );
                let tile_r = &mut self.die[(col_r, row)];
                tile_r.nodes.truncate(1);
                tile_r.add_xnode(
                    self.db.get_node("INTF.DELAY"),
                    &[&format!("PCIE3_INT_INTERFACE_L_X{xr}Y{y}")],
                    self.db.get_node_naming("INTF.PCIE3_L"),
                    &[(col_r, row)],
                );
            }
            let mut crds = vec![];
            for dy in 0..50 {
                crds.push((bc, br + dy));
            }
            for dy in 0..50 {
                crds.push((bc + 5, br + dy));
            }
            let x = self.rxlut[bc] + 2;
            let y = self.rylut[br];
            let name_b = format!("PCIE3_BOT_RIGHT_X{x}Y{y}", y = y + 7);
            let name = format!("PCIE3_RIGHT_X{x}Y{y}", y = y + 26);
            let name_t = format!("PCIE3_TOP_RIGHT_X{x}Y{y}", y = y + 43);
            let node = self.die[crds[0]].add_xnode(
                self.db.get_node("PCIE3"),
                &[&name, &name_b, &name_t],
                self.db.get_node_naming("PCIE3"),
                &crds,
            );
            node.add_bel(0, format!("PCIE3_X0Y{pcie3_y}"));
            pcie3_y += 1;
        }
        pcie3_y
    }

    fn fill_gt_mid(&mut self) {
        if let Some((ref lcol, ref rcol)) = self.grid.cols_gtp_mid {
            for (reg, &kind) in lcol.regs.iter().enumerate() {
                let gtx = 0;
                let ipx = self.ipxlut[lcol.col];
                let opx = self.opxlut[lcol.col];
                if let Some(kind) = kind {
                    assert_eq!(kind, GtKind::Gtp);
                    let br = RowId::from_idx(reg * 50);
                    self.die.nuke_rect(lcol.col + 1, br, 18, 50);
                    self.int_holes.push(Rect {
                        col_l: lcol.col + 1,
                        col_r: lcol.col + 19,
                        row_b: br,
                        row_t: br + 50,
                    });
                    self.site_holes.push(Rect {
                        col_l: lcol.col,
                        col_r: lcol.col + 19,
                        row_b: br,
                        row_t: br + 50,
                    });
                    for dx in 1..19 {
                        let col = lcol.col + dx;
                        if br.to_idx() != 0 {
                            self.die.fill_term_anon((col, br - 1), "TERM.N");
                        }
                        if br.to_idx() + 50 != self.grid.regs * 50 {
                            self.die.fill_term_anon((col, br + 50), "TERM.S");
                        }
                    }
                    let col_l = lcol.col;
                    let col_r = lcol.col + 19;
                    let xl = self.xlut[col_l];
                    for dy in 0..50 {
                        let row = br + dy;
                        let y = self.ylut[row];
                        let tile = &mut self.die[(col_l, row)];
                        tile.nodes.truncate(1);
                        tile.add_xnode(
                            self.db.get_node("INTF.DELAY"),
                            &[&format!("GTP_INT_INTERFACE_R_X{xl}Y{y}")],
                            self.db.get_node_naming("INTF.GTP_R"),
                            &[(col_l, row)],
                        );
                        self.die.fill_term_anon((col_l, row), "TERM.E");
                        self.die.fill_term_anon((col_r, row), "TERM.W");
                    }

                    let gty = self.gtylut[br];
                    let sk = match kind {
                        GtKind::Gtp => "GTP",
                        GtKind::Gtx => "GTX",
                        GtKind::Gth => "GTH",
                    };
                    for (i, dy) in [(0, 0), (1, 11), (2, 28), (3, 39)] {
                        let row = br + dy;
                        let name = format!(
                            "{sk}_CHANNEL_{i}_MID_LEFT_X{x}Y{y}",
                            x = self.rxlut[lcol.col] + 14,
                            y = self.rylut[row + 5]
                        );
                        let crds: [_; 11] = core::array::from_fn(|dy| (lcol.col, row + dy));
                        let node = self.die[(lcol.col, row)].add_xnode(
                            self.db.get_node(&format!("{sk}_CHANNEL")),
                            &[&name],
                            self.db
                                .get_node_naming(&format!("{sk}_CHANNEL_{i}_MID_LEFT")),
                            &crds,
                        );
                        let ipy = self.ipylut[row];
                        let opy = self.opylut[row];
                        node.add_bel(0, format!("{sk}E2_CHANNEL_X{gtx}Y{y}", y = gty * 4 + i));
                        node.add_bel(1, format!("IPAD_X{ipx}Y{y}", y = ipy + 1));
                        node.add_bel(2, format!("IPAD_X{ipx}Y{y}", y = ipy));
                        node.add_bel(3, format!("OPAD_X{opx}Y{y}", y = opy + 1));
                        node.add_bel(4, format!("OPAD_X{opx}Y{y}", y = opy));
                    }
                    let row = br + 22;
                    let name = format!(
                        "{sk}_COMMON_MID_LEFT_X{x}Y{y}",
                        x = self.rxlut[lcol.col] + 14,
                        y = self.rylut[row]
                    );
                    let crds: [_; 6] = core::array::from_fn(|dy| (lcol.col, row + dy));
                    let node = self.die[(lcol.col, row + 3)].add_xnode(
                        self.db.get_node(&format!("{sk}_COMMON")),
                        &[&name],
                        self.db.get_node_naming(&format!("{sk}_COMMON_MID_LEFT")),
                        &crds,
                    );
                    let ipy = self.ipylut[row];
                    node.add_bel(0, format!("{sk}E2_COMMON_X{gtx}Y{gty}"));
                    node.add_bel(1, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2));
                    node.add_bel(2, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2 + 1));
                    node.add_bel(3, format!("IPAD_X{ipx}Y{y}", y = ipy - 4));
                    node.add_bel(4, format!("IPAD_X{ipx}Y{y}", y = ipy - 3));
                    node.add_bel(5, format!("IPAD_X{ipx}Y{y}", y = ipy - 2));
                    node.add_bel(6, format!("IPAD_X{ipx}Y{y}", y = ipy - 1));
                }
            }
            for (reg, &kind) in rcol.regs.iter().enumerate() {
                let gtx = 1;
                let ipx = self.ipxlut[rcol.col];
                let opx = self.opxlut[rcol.col];
                if let Some(kind) = kind {
                    assert_eq!(kind, GtKind::Gtp);
                    let br = RowId::from_idx(reg * 50);
                    self.die.nuke_rect(rcol.col - 18, br, 18, 50);
                    self.int_holes.push(Rect {
                        col_l: rcol.col - 18,
                        col_r: rcol.col,
                        row_b: br,
                        row_t: br + 50,
                    });
                    self.site_holes.push(Rect {
                        col_l: rcol.col - 18,
                        col_r: rcol.col + 1,
                        row_b: br,
                        row_t: br + 50,
                    });
                    for dx in 1..19 {
                        let col = rcol.col - 19 + dx;
                        if br.to_idx() != 0 {
                            self.die.fill_term_anon((col, br - 1), "TERM.N");
                        }
                        if br.to_idx() + 50 != self.grid.regs * 50 {
                            self.die.fill_term_anon((col, br + 50), "TERM.S");
                        }
                    }
                    let col_l = rcol.col - 19;
                    let col_r = rcol.col;
                    let xr = self.xlut[col_r];
                    for dy in 0..50 {
                        let row = br + dy;
                        let y = self.ylut[row];
                        let tile = &mut self.die[(col_r, row)];
                        tile.nodes.truncate(1);
                        tile.add_xnode(
                            self.db.get_node("INTF.DELAY"),
                            &[&format!("GTP_INT_INTERFACE_L_X{xr}Y{y}")],
                            self.db.get_node_naming("INTF.GTP_L"),
                            &[(col_r, row)],
                        );
                        self.die.fill_term_anon((col_l, row), "TERM.E");
                        self.die.fill_term_anon((col_r, row), "TERM.W");
                    }

                    let gty = self.gtylut[br];
                    let sk = match kind {
                        GtKind::Gtp => "GTP",
                        GtKind::Gtx => "GTX",
                        GtKind::Gth => "GTH",
                    };
                    for (i, dy) in [(0, 0), (1, 11), (2, 28), (3, 39)] {
                        let row = br + dy;
                        let name = format!(
                            "{sk}_CHANNEL_{i}_MID_RIGHT_X{x}Y{y}",
                            x = self.rxlut[rcol.col] - 18,
                            y = self.rylut[row + 5]
                        );
                        let crds: [_; 11] = core::array::from_fn(|dy| (rcol.col, row + dy));
                        let node = self.die[(rcol.col, row)].add_xnode(
                            self.db.get_node(&format!("{sk}_CHANNEL")),
                            &[&name],
                            self.db
                                .get_node_naming(&format!("{sk}_CHANNEL_{i}_MID_RIGHT")),
                            &crds,
                        );
                        let ipy = self.ipylut[row];
                        let opy = self.opylut[row];
                        node.add_bel(0, format!("{sk}E2_CHANNEL_X{gtx}Y{y}", y = gty * 4 + i));
                        node.add_bel(1, format!("IPAD_X{ipx}Y{y}", y = ipy + 1));
                        node.add_bel(2, format!("IPAD_X{ipx}Y{y}", y = ipy));
                        node.add_bel(3, format!("OPAD_X{opx}Y{y}", y = opy + 1));
                        node.add_bel(4, format!("OPAD_X{opx}Y{y}", y = opy));
                    }
                    let row = br + 22;
                    let name = format!(
                        "{sk}_COMMON_MID_RIGHT_X{x}Y{y}",
                        x = self.rxlut[rcol.col] - 18,
                        y = self.rylut[row]
                    );
                    let crds: [_; 6] = core::array::from_fn(|dy| (rcol.col, row + dy));
                    let node = self.die[(rcol.col, row + 3)].add_xnode(
                        self.db.get_node(&format!("{sk}_COMMON")),
                        &[&name],
                        self.db.get_node_naming(&format!("{sk}_COMMON_MID_RIGHT")),
                        &crds,
                    );
                    let ipy = self.ipylut[row];
                    node.add_bel(0, format!("{sk}E2_COMMON_X{gtx}Y{gty}"));
                    node.add_bel(1, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2));
                    node.add_bel(2, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2 + 1));
                    node.add_bel(3, format!("IPAD_X{ipx}Y{y}", y = ipy - 4));
                    node.add_bel(4, format!("IPAD_X{ipx}Y{y}", y = ipy - 3));
                    node.add_bel(5, format!("IPAD_X{ipx}Y{y}", y = ipy - 2));
                    node.add_bel(6, format!("IPAD_X{ipx}Y{y}", y = ipy - 1));
                }
            }
        }
    }

    fn fill_gt_left(&mut self) {
        if let Some(ref gtcol) = self.grid.cols_gt[0] {
            let gtx = 0;
            let ipx = self.ipxlut[gtcol.col];
            let opx = self.opxlut[gtcol.col];

            for (reg, &kind) in gtcol.regs.iter().enumerate() {
                if let Some(kind) = kind {
                    let br = RowId::from_idx(reg * 50);
                    let x = self.xlut[gtcol.col];
                    for dy in 0..50 {
                        let row = br + dy;
                        let y = self.ylut[row];
                        let t = match kind {
                            GtKind::Gtp => unreachable!(),
                            GtKind::Gtx => "GTX",
                            GtKind::Gth => "GTH",
                        };
                        let tile = &mut self.die[(gtcol.col, row)];
                        tile.nodes.truncate(1);
                        tile.add_xnode(
                            self.db.get_node("INTF.DELAY"),
                            &[&format!("{t}_INT_INTERFACE_L_X{x}Y{y}")],
                            self.db.get_node_naming(&format!("INTF.{t}_L")),
                            &[(gtcol.col, row)],
                        );
                    }
                    let gty = self.gtylut[br];
                    let sk = match kind {
                        GtKind::Gtp => "GTP",
                        GtKind::Gtx => "GTX",
                        GtKind::Gth => "GTH",
                    };
                    for (i, dy) in [(0, 0), (1, 11), (2, 28), (3, 39)] {
                        let row = br + dy;
                        let name = format!(
                            "{sk}_CHANNEL_{i}_X{x}Y{y}",
                            x = self.rxlut[gtcol.col],
                            y = self.rylut[row + 5]
                        );
                        let crds: [_; 11] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                        let node = self.die[(gtcol.col, row)].add_xnode(
                            self.db.get_node(&format!("{sk}_CHANNEL")),
                            &[&name],
                            self.db.get_node_naming(&format!("{sk}_CHANNEL_{i}")),
                            &crds,
                        );
                        let ipy = self.ipylut[row];
                        let opy = self.opylut[row];
                        node.add_bel(0, format!("{sk}E2_CHANNEL_X{gtx}Y{y}", y = gty * 4 + i));
                        node.add_bel(1, format!("IPAD_X{ipx}Y{y}", y = ipy + 1));
                        node.add_bel(2, format!("IPAD_X{ipx}Y{y}", y = ipy));
                        node.add_bel(3, format!("OPAD_X{opx}Y{y}", y = opy + 1));
                        node.add_bel(4, format!("OPAD_X{opx}Y{y}", y = opy));
                    }
                    let row = br + 22;
                    let name = format!(
                        "{sk}_COMMON_X{x}Y{y}",
                        x = self.rxlut[gtcol.col],
                        y = self.rylut[row]
                    );
                    let crds: [_; 6] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                    let node = self.die[(gtcol.col, row + 3)].add_xnode(
                        self.db.get_node(&format!("{sk}_COMMON")),
                        &[&name],
                        self.db.get_node_naming(&format!("{sk}_COMMON")),
                        &crds,
                    );
                    let ipy = self.ipylut[row];
                    node.add_bel(0, format!("{sk}E2_COMMON_X{gtx}Y{gty}"));
                    node.add_bel(1, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2));
                    node.add_bel(2, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2 + 1));
                    node.add_bel(3, format!("IPAD_X{ipx}Y{y}", y = ipy - 4));
                    node.add_bel(4, format!("IPAD_X{ipx}Y{y}", y = ipy - 3));
                    node.add_bel(5, format!("IPAD_X{ipx}Y{y}", y = ipy - 2));
                    node.add_bel(6, format!("IPAD_X{ipx}Y{y}", y = ipy - 1));

                    if br.to_idx() != 0 {
                        let name = format!(
                            "BRKH_GTX_X{x}Y{y}",
                            x = self.rxlut[gtcol.col],
                            y = self.rylut[br] - 1
                        );
                        self.die[(gtcol.col, br)].add_xnode(
                            self.db.get_node("BRKH_GTX"),
                            &[&name],
                            self.db.get_node_naming("BRKH_GTX"),
                            &[],
                        );
                    }
                }
            }
        }
    }

    fn fill_gt_right(&mut self) {
        if let Some(ref gtcol) = self.grid.cols_gt[1] {
            let gtx = if self.grid.cols_gt[0].is_some() { 1 } else { 0 };
            let ipx = self.ipxlut[gtcol.col];
            let opx = self.opxlut[gtcol.col];
            let need_holes = self.grid.columns[gtcol.col] != ColumnKind::Gt;
            for (reg, &kind) in gtcol.regs.iter().enumerate() {
                let br = RowId::from_idx(reg * 50);
                if let Some(kind) = kind {
                    if need_holes {
                        self.die.nuke_rect(gtcol.col + 1, br, 6, 50);
                        self.site_holes.push(Rect {
                            col_l: gtcol.col,
                            col_r: gtcol.col + 7,
                            row_b: br,
                            row_t: br + 50,
                        });
                        self.int_holes.push(Rect {
                            col_l: gtcol.col + 1,
                            col_r: gtcol.col + 7,
                            row_b: br,
                            row_t: br + 50,
                        });
                        if reg != 0 && gtcol.regs[reg - 1].is_none() {
                            for dx in 1..7 {
                                self.die.fill_term_anon((gtcol.col + dx, br - 1), "TERM.N");
                            }
                        }
                        if reg != self.grid.regs - 1 && gtcol.regs[reg + 1].is_none() {
                            for dx in 1..7 {
                                self.die.fill_term_anon((gtcol.col + dx, br + 50), "TERM.S");
                            }
                        }
                        for dy in 0..50 {
                            self.die.fill_term_anon((gtcol.col, br + dy), "TERM.E");
                        }
                    }
                    let x = self.xlut[gtcol.col];
                    for dy in 0..50 {
                        let row = br + dy;
                        let y = self.ylut[row];
                        let t = match kind {
                            GtKind::Gtp => "GTP",
                            GtKind::Gtx => "GTX",
                            GtKind::Gth => "GTH",
                        };
                        let tile = &mut self.die[(gtcol.col, row)];
                        tile.nodes.truncate(1);
                        tile.add_xnode(
                            self.db.get_node("INTF.DELAY"),
                            &[&format!("{t}_INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming(&format!("INTF.{t}")),
                            &[(gtcol.col, row)],
                        );
                    }

                    let gty = self.gtylut[br];
                    let sk = match kind {
                        GtKind::Gtp => "GTP",
                        GtKind::Gtx => "GTX",
                        GtKind::Gth => "GTH",
                    };
                    for (i, dy) in [(0, 0), (1, 11), (2, 28), (3, 39)] {
                        let row = br + dy;
                        let name = format!(
                            "{sk}_CHANNEL_{i}_X{x}Y{y}",
                            x = self.rxlut[gtcol.col] + 4,
                            y = self.rylut[row + 5]
                        );
                        let crds: [_; 11] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                        let node = self.die[(gtcol.col, row)].add_xnode(
                            self.db.get_node(&format!("{sk}_CHANNEL")),
                            &[&name],
                            self.db.get_node_naming(&format!("{sk}_CHANNEL_{i}")),
                            &crds,
                        );
                        let ipy = self.ipylut[row];
                        let opy = self.opylut[row];
                        node.add_bel(0, format!("{sk}E2_CHANNEL_X{gtx}Y{y}", y = gty * 4 + i));
                        node.add_bel(1, format!("IPAD_X{ipx}Y{y}", y = ipy + 1));
                        node.add_bel(2, format!("IPAD_X{ipx}Y{y}", y = ipy));
                        node.add_bel(3, format!("OPAD_X{opx}Y{y}", y = opy + 1));
                        node.add_bel(4, format!("OPAD_X{opx}Y{y}", y = opy));
                    }
                    let row = br + 22;
                    let name = format!(
                        "{sk}_COMMON_X{x}Y{y}",
                        x = self.rxlut[gtcol.col] + 4,
                        y = self.rylut[row]
                    );
                    let crds: [_; 6] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                    let node = self.die[(gtcol.col, row + 3)].add_xnode(
                        self.db.get_node(&format!("{sk}_COMMON")),
                        &[&name],
                        self.db.get_node_naming(&format!("{sk}_COMMON")),
                        &crds,
                    );
                    let ipy = self.ipylut[row];
                    node.add_bel(0, format!("{sk}E2_COMMON_X{gtx}Y{gty}"));
                    node.add_bel(1, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2));
                    node.add_bel(2, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2 + 1));
                    node.add_bel(3, format!("IPAD_X{ipx}Y{y}", y = ipy - 4));
                    node.add_bel(4, format!("IPAD_X{ipx}Y{y}", y = ipy - 3));
                    node.add_bel(5, format!("IPAD_X{ipx}Y{y}", y = ipy - 2));
                    node.add_bel(6, format!("IPAD_X{ipx}Y{y}", y = ipy - 1));
                }
                if br.to_idx() != 0 && (kind.is_some() || gtcol.regs[reg - 1].is_some()) {
                    let name = if gtcol.regs[reg - 1].is_none() {
                        format!(
                            "BRKH_GTX_X{x}Y{y}",
                            x = self.xlut[gtcol.col] + 1,
                            y = self.ylut[br] - 1
                        )
                    } else {
                        format!(
                            "BRKH_GTX_X{x}Y{y}",
                            x = self.rxlut[gtcol.col] + 4,
                            y = self.rylut[br] - 1
                        )
                    };
                    self.die[(gtcol.col, br)].add_xnode(
                        self.db.get_node("BRKH_GTX"),
                        &[&name],
                        self.db.get_node_naming("BRKH_GTX"),
                        &[],
                    );
                }
            }
        }
    }

    fn fill_terms(&mut self) {
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        for col in self.die.cols() {
            if !self.die[(col, row_b)].nodes.is_empty() {
                if self.grid.has_no_tbuturn {
                    self.die.fill_term_anon((col, row_b), "TERM.S.HOLE");
                } else {
                    self.die.fill_term_anon((col, row_b), "TERM.S");
                }
            }
            if !self.die[(col, row_t)].nodes.is_empty() {
                if self.grid.has_no_tbuturn {
                    self.die.fill_term_anon((col, row_t), "TERM.N.HOLE");
                } else {
                    self.die.fill_term_anon((col, row_t), "TERM.N");
                }
            }
        }
        for row in self.die.rows() {
            if !self.die[(col_l, row)].nodes.is_empty() {
                self.die.fill_term_anon((col_l, row), "TERM.W");
            }
            if !self.die[(col_r, row)].nodes.is_empty() {
                self.die.fill_term_anon((col_r, row), "TERM.E");
            }
        }
        for reg in 1..self.grid.regs {
            let row_s = RowId::from_idx(reg * 50 - 1);
            let row_n = RowId::from_idx(reg * 50);
            let term_s = self.db.get_term("BRKH.S");
            let term_n = self.db.get_term("BRKH.N");
            let naming_s = self.db.get_term_naming("BRKH.S");
            let naming_n = self.db.get_term_naming("BRKH.N");
            for col in self.die.cols() {
                if !self.die[(col, row_s)].nodes.is_empty()
                    && !self.die[(col, row_n)].nodes.is_empty()
                {
                    let x = self.xlut[col];
                    let y = self.ylut[row_s];
                    self.die.fill_term_pair_buf(
                        (col, row_s),
                        (col, row_n),
                        term_n,
                        term_s,
                        format!("BRKH_INT_X{x}Y{y}"),
                        naming_s,
                        naming_n,
                    );
                }
            }
        }
    }

    fn fill_clb(&mut self) {
        let mut sx = 0;
        for (col, &cd) in &self.grid.columns {
            let (kind, naming) = match (cd, col.to_idx() % 2) {
                (ColumnKind::ClbLL, 0) => ("CLBLL", "CLBLL_L"),
                (ColumnKind::ClbLL, 1) => ("CLBLL", "CLBLL_R"),
                (ColumnKind::ClbLM, 0) => ("CLBLM", "CLBLM_L"),
                (ColumnKind::ClbLM, 1) => ("CLBLM", "CLBLM_R"),
                _ => continue,
            };
            let mut found = false;
            'a: for row in self.die.rows() {
                let tile = &mut self.die[(col, row)];
                for &hole in &self.site_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let x = self.xlut[col];
                let y = self.ylut[row];
                let sy = self.tieylut[row];
                let name = format!("{naming}_X{x}Y{y}");
                let node = tile.add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(naming),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 1));
                found = true;
            }
            if found {
                sx += 2;
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        let mut bx = 0;
        let mut dx = 0;
        for (col, &cd) in &self.grid.columns {
            let (kind, naming) = match cd {
                ColumnKind::Bram => ("BRAM", ["BRAM_L", "BRAM_R"][col.to_idx() % 2]),
                ColumnKind::Dsp => ("DSP", ["DSP_L", "DSP_R"][col.to_idx() % 2]),
                _ => continue,
            };
            let mut found = false;
            'a: for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                for &hole in &self.site_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                if col.to_idx() == 0
                    && (row.to_idx() < 5 || row.to_idx() >= self.die.rows().len() - 5)
                {
                    continue;
                }
                found = true;
                let x = self.xlut[col];
                let y = self.ylut[row];
                let sy = (self.tieylut[row]) / 5;
                let name = format!("{naming}_X{x}Y{y}");
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(naming),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                if cd == ColumnKind::Bram {
                    node.add_bel(0, format!("RAMB36_X{bx}Y{sy}", sy = sy));
                    node.add_bel(1, format!("RAMB18_X{bx}Y{sy}", sy = sy * 2));
                    node.add_bel(2, format!("RAMB18_X{bx}Y{sy}", sy = sy * 2 + 1));
                } else {
                    node.add_bel(0, format!("DSP48_X{dx}Y{sy}", sy = sy * 2));
                    node.add_bel(1, format!("DSP48_X{dx}Y{sy}", sy = sy * 2 + 1));
                    let tx = if naming == "DSP_L" {
                        self.tiexlut[col] - 1
                    } else {
                        self.tiexlut[col] + 1
                    };
                    let ty = self.tieylut[row];
                    node.add_bel(2, format!("TIEOFF_X{tx}Y{ty}"));
                }
                if kind == "BRAM" && row.to_idx() % 50 == 25 {
                    let hx = if naming == "BRAM_L" {
                        self.rxlut[col]
                    } else {
                        self.rxlut[col] + 2
                    };
                    let hy = self.rylut[row] - 1;
                    let name_h = format!("HCLK_BRAM_X{hx}Y{hy}");
                    let name_1 = format!("{naming}_X{x}Y{y}", y = y + 5);
                    let name_2 = format!("{naming}_X{x}Y{y}", y = y + 10);
                    let coords: Vec<_> = (0..15).map(|dy| (col, row + dy)).collect();
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("PMVBRAM"),
                        &[&name_h, &name, &name_1, &name_2],
                        self.db.get_node_naming("PMVBRAM"),
                        &coords,
                    );
                    node.add_bel(0, format!("PMVBRAM_X{bx}Y{sy}", sy = sy / 10));
                }
            }
            if cd == ColumnKind::Bram {
                'a: for row in self.die.rows() {
                    if row.to_idx() % 50 != 25 {
                        continue;
                    }
                    let mut is_hole_up = false;
                    for &hole in &self.site_holes {
                        if hole.contains(col, row - 1) {
                            continue 'a;
                        }
                        if hole.contains(col, row) {
                            is_hole_up = true;
                        }
                    }
                    if !is_hole_up {
                        continue;
                    }
                    let hx = if naming == "BRAM_L" {
                        self.rxlut[col]
                    } else {
                        self.rxlut[col] + 2
                    };
                    let hy = self.rylut[row] - 1;
                    let name_h = format!("HCLK_BRAM_X{hx}Y{hy}");
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("PMVBRAM_NC"),
                        &[&name_h],
                        self.db.get_node_naming("PMVBRAM_NC"),
                        &[],
                    );
                    node.add_bel(
                        0,
                        format!("PMVBRAM_X{bx}Y{sy}", sy = self.tieylut[row] / 50),
                    );
                }
            }
            if found {
                if cd == ColumnKind::Bram {
                    bx += 1;
                } else {
                    dx += 1;
                }
            }
        }
    }

    fn fill_io(&mut self) {
        let mut iox = 0;
        let mut dcix = 0;
        for iocol in self.grid.cols_io.iter().flatten() {
            let col = iocol.col;
            let is_l = col < self.grid.col_clk;
            let is_term = if is_l {
                col == self.grid.columns.first_id().unwrap()
            } else {
                col == self.grid.columns.last_id().unwrap()
            };
            let mut found = false;
            let mut found_hp = false;
            for row in self.die.rows() {
                if let Some(kind) = iocol.regs[row.to_idx() / 50] {
                    found = true;
                    if kind == IoKind::Hpio {
                        found_hp = true;
                    }
                    let tk = match kind {
                        IoKind::Hpio => {
                            if is_l {
                                "LIOI"
                            } else {
                                "RIOI"
                            }
                        }
                        IoKind::Hrio => {
                            if is_l {
                                "LIOI3"
                            } else {
                                "RIOI3"
                            }
                        }
                    };
                    let iob_tk = match kind {
                        IoKind::Hpio => {
                            if is_l {
                                "LIOB18"
                            } else {
                                "RIOB18"
                            }
                        }
                        IoKind::Hrio => {
                            if is_l {
                                "LIOB33"
                            } else {
                                "RIOB33"
                            }
                        }
                    };
                    let rx = self.rxlut[col]
                        + if is_l {
                            1
                        } else if is_term {
                            3
                        } else {
                            2
                        };
                    let rxiob = if is_l { rx - 1 } else { rx + 1 };

                    if matches!(row.to_idx() % 50, 0 | 49) {
                        let name;
                        let name_iob;
                        if is_term {
                            name = format!(
                                "{tk}_SING_X{x}Y{y}",
                                x = self.xlut[col],
                                y = self.ylut[row]
                            );
                            name_iob = format!(
                                "{iob_tk}_SING_X{x}Y{y}",
                                x = self.xlut[col],
                                y = self.ylut[row]
                            );
                        } else {
                            name = format!("{tk}_SING_X{rx}Y{y}", y = self.rylut[row]);
                            name_iob = format!("{iob_tk}_SING_X{rxiob}Y{y}", y = self.rylut[row]);
                        }
                        let naming = format!("{tk}_SING");
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node(if kind == IoKind::Hpio {
                                "IOS_HP"
                            } else {
                                "IOS_HR"
                            }),
                            &[&name, &name_iob],
                            self.db.get_node_naming(&naming),
                            &[(col, row)],
                        );
                        node.add_bel(0, format!("ILOGIC_X{iox}Y{y}", y = self.tieylut[row]));
                        node.add_bel(1, format!("OLOGIC_X{iox}Y{y}", y = self.tieylut[row]));
                        node.add_bel(2, format!("IDELAY_X{iox}Y{y}", y = self.tieylut[row]));
                        if kind == IoKind::Hpio {
                            node.add_bel(3, format!("ODELAY_X{iox}Y{y}", y = self.tieylut[row]));
                            node.add_bel(4, format!("IOB_X{iox}Y{y}", y = self.tieylut[row]));
                        } else {
                            node.add_bel(3, format!("IOB_X{iox}Y{y}", y = self.tieylut[row]));
                        }
                    } else if row.to_idx() % 2 == 1 {
                        let suf = match row.to_idx() % 50 {
                            7 | 19 | 31 | 43 => "_TBYTESRC",
                            13 | 37 => "_TBYTETERM",
                            _ => "",
                        };
                        let name;
                        let name_iob;
                        if is_term {
                            name = format!(
                                "{tk}{suf}_X{x}Y{y}",
                                x = self.xlut[col],
                                y = self.ylut[row]
                            );
                            name_iob = format!(
                                "{iob_tk}_X{x}Y{y}",
                                x = self.xlut[col],
                                y = self.ylut[row]
                            );
                        } else {
                            name = format!("{tk}{suf}_X{rx}Y{y}", y = self.rylut[row]);
                            name_iob = format!("{iob_tk}_X{rxiob}Y{y}", y = self.rylut[row]);
                        }
                        let naming = format!("{tk}{suf}");
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node(if kind == IoKind::Hpio {
                                "IOP_HP"
                            } else {
                                "IOP_HR"
                            }),
                            &[&name, &name_iob],
                            self.db.get_node_naming(&naming),
                            &[(col, row), (col, row + 1)],
                        );
                        node.add_bel(0, format!("ILOGIC_X{iox}Y{y}", y = self.tieylut[row] + 1));
                        node.add_bel(1, format!("ILOGIC_X{iox}Y{y}", y = self.tieylut[row]));
                        node.add_bel(2, format!("OLOGIC_X{iox}Y{y}", y = self.tieylut[row] + 1));
                        node.add_bel(3, format!("OLOGIC_X{iox}Y{y}", y = self.tieylut[row]));
                        node.add_bel(4, format!("IDELAY_X{iox}Y{y}", y = self.tieylut[row] + 1));
                        node.add_bel(5, format!("IDELAY_X{iox}Y{y}", y = self.tieylut[row]));
                        if kind == IoKind::Hpio {
                            node.add_bel(
                                6,
                                format!("ODELAY_X{iox}Y{y}", y = self.tieylut[row] + 1),
                            );
                            node.add_bel(7, format!("ODELAY_X{iox}Y{y}", y = self.tieylut[row]));
                            node.add_bel(8, format!("IOB_X{iox}Y{y}", y = self.tieylut[row] + 1));
                            node.add_bel(9, format!("IOB_X{iox}Y{y}", y = self.tieylut[row]));
                        } else {
                            node.add_bel(6, format!("IOB_X{iox}Y{y}", y = self.tieylut[row] + 1));
                            node.add_bel(7, format!("IOB_X{iox}Y{y}", y = self.tieylut[row]));
                        }
                    }

                    if row.to_idx() % 50 == 25 {
                        let htk = match kind {
                            IoKind::Hpio => "HCLK_IOI",
                            IoKind::Hrio => "HCLK_IOI3",
                        };
                        let name = format!("{htk}_X{rx}Y{y}", y = self.rylut[row] - 1);
                        let name_b0;
                        let name_b1;
                        let name_t0;
                        let name_t1;
                        if is_term {
                            name_b0 = format!(
                                "{tk}_X{x}Y{y}",
                                x = self.xlut[col],
                                y = self.ylut[row - 4]
                            );
                            name_b1 = format!(
                                "{tk}_X{x}Y{y}",
                                x = self.xlut[col],
                                y = self.ylut[row - 2]
                            );
                            name_t0 =
                                format!("{tk}_X{x}Y{y}", x = self.xlut[col], y = self.ylut[row]);
                            name_t1 = format!(
                                "{tk}_X{x}Y{y}",
                                x = self.xlut[col],
                                y = self.ylut[row + 2]
                            );
                        } else {
                            name_b0 = format!("{tk}_X{rx}Y{y}", y = self.rylut[row - 4]);
                            name_b1 = format!("{tk}_X{rx}Y{y}", y = self.rylut[row - 2]);
                            name_t0 = format!("{tk}_X{rx}Y{y}", y = self.rylut[row]);
                            name_t1 = format!("{tk}_X{rx}Y{y}", y = self.rylut[row + 2]);
                        }
                        let crds: [_; 8] = core::array::from_fn(|dy| (col, row - 4 + dy));
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node(htk),
                            &[&name, &name_b0, &name_b1, &name_t0, &name_t1],
                            self.db.get_node_naming(htk),
                            &crds,
                        );
                        let hy = self.tieylut[row] / 50;
                        for i in 0..4 {
                            node.add_bel(i, format!("BUFIO_X{iox}Y{y}", y = hy * 4 + (i ^ 2)));
                        }
                        for i in 0..4 {
                            node.add_bel(i + 4, format!("BUFR_X{iox}Y{y}", y = hy * 4 + (i ^ 2)));
                        }
                        node.add_bel(8, format!("IDELAYCTRL_X{iox}Y{hy}"));
                        if kind == IoKind::Hpio {
                            node.add_bel(9, format!("DCI_X{dcix}Y{y}", y = self.dciylut[row]));
                        }
                    }
                }
            }
            if found {
                iox += 1;
            }
            if found_hp {
                dcix += 1;
            }
        }
    }

    fn fill_cmt(&mut self) {
        let mut cmtx = 0;
        for (col, &cd) in &self.grid.columns {
            if cd != ColumnKind::Cmt {
                continue;
            }
            let is_l = col.to_idx() % 2 == 0;
            let lr = if is_l { 'L' } else { 'R' };
            let rx = if is_l {
                self.rxlut[col]
            } else {
                self.rxlut[col] + 3
            };
            let mut found = false;
            'a: for reg in 0..self.grid.regs {
                let row = RowId::from_idx(reg * 50 + 25);
                for hole in &self.site_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                found = true;
                let crds: [_; 50] = core::array::from_fn(|dy| (col, row - 25 + dy));
                let name0 = format!("CMT_TOP_{lr}_LOWER_B_X{rx}Y{y}", y = self.rylut[row - 17]);
                let name1 = format!("CMT_TOP_{lr}_LOWER_T_X{rx}Y{y}", y = self.rylut[row - 8]);
                let name2 = format!("CMT_TOP_{lr}_UPPER_B_X{rx}Y{y}", y = self.rylut[row + 4]);
                let name3 = format!("CMT_TOP_{lr}_UPPER_T_X{rx}Y{y}", y = self.rylut[row + 17]);
                let name_h = if is_l {
                    format!("HCLK_CMT_L_X{rx}Y{y}", y = self.rylut[row] - 1)
                } else {
                    format!("HCLK_CMT_X{rx}Y{y}", y = self.rylut[row] - 1)
                };
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("CMT"),
                    &[&name0, &name1, &name2, &name3, &name_h],
                    self.db
                        .get_node_naming(if is_l { "CMT.L" } else { "CMT.R" }),
                    &crds,
                );
                let hy = self.tieylut[row] / 50;
                for i in 0..4 {
                    node.add_bel(i, format!("PHASER_IN_PHY_X{cmtx}Y{y}", y = hy * 4 + i));
                }
                for i in 0..4 {
                    node.add_bel(4 + i, format!("PHASER_OUT_PHY_X{cmtx}Y{y}", y = hy * 4 + i));
                }
                node.add_bel(8, format!("PHASER_REF_X{cmtx}Y{hy}"));
                node.add_bel(9, format!("PHY_CONTROL_X{cmtx}Y{hy}"));
                node.add_bel(10, format!("MMCME2_ADV_X{cmtx}Y{hy}"));
                node.add_bel(11, format!("PLLE2_ADV_X{cmtx}Y{hy}"));
                for i in 0..2 {
                    node.add_bel(12 + i, format!("BUFMRCE_X{cmtx}Y{y}", y = hy * 2 + i));
                }

                for (i, row) in [row - 24, row - 12, row, row + 12].into_iter().enumerate() {
                    let tkn = if is_l { "CMT_FIFO_L" } else { "CMT_FIFO_R" };
                    let crds: [_; 12] = core::array::from_fn(|dy| (col, row + dy));
                    let rx = if is_l {
                        self.rxlut[col] + 1
                    } else {
                        self.rxlut[col] + 2
                    };
                    let name = format!("{tkn}_X{rx}Y{y}", y = self.rylut[row + 6]);
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("CMT_FIFO"),
                        &[&name],
                        self.db.get_node_naming(tkn),
                        &crds,
                    );
                    node.add_bel(0, format!("IN_FIFO_X{cmtx}Y{y}", y = hy * 4 + i));
                    node.add_bel(1, format!("OUT_FIFO_X{cmtx}Y{y}", y = hy * 4 + i));
                }
            }
            if found {
                cmtx += 1;
            }
        }
    }

    fn fill_clk(&mut self, mut bglb_y: usize) -> usize {
        let col = self.grid.col_clk;
        for reg in 0..self.grid.regs {
            let row_h = RowId::from_idx(reg * 50 + 25);
            let ctb_y = self.tieylut[row_h] / 50 * 48;
            let bufh_y = self.tieylut[row_h] / 50 * 12;
            if self.grid.has_slr && reg == 0 {
                let tk = if self.has_gtz_d {
                    "CLK_BALI_REBUF_GTZ_BOT"
                } else {
                    "CLK_BALI_REBUF"
                };
                let row = row_h - 13;
                let name = format!(
                    "{tk}_X{x}Y{y}",
                    x = self.rxlut[col] + 2,
                    y = self.rylut[row],
                );
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("CLK_REBUF"),
                    &[&name],
                    self.db.get_node_naming("CLK_BALI_REBUF"),
                    &[],
                );
                for i in 0..16 {
                    let y = (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3;
                    node.add_bel(i, format!("GCLK_TEST_BUF_X1Y{y}", y = ctb_y + y));
                }
                for i in 0..16 {
                    let y = (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3;
                    if self.has_gtz_d {
                        node.add_bel(16 + i, format!("BUFG_LB_X3Y{y}", y = bglb_y + y));
                    } else {
                        node.add_bel(16 + i, format!("GCLK_TEST_BUF_X3Y{y}", y = ctb_y + y));
                    }
                }
                if self.has_gtz_d {
                    bglb_y += 16;
                }
            } else {
                let row = row_h - 13;
                let name = format!(
                    "CLK_BUFG_REBUF_X{x}Y{y}",
                    x = self.rxlut[col] + 2,
                    y = self.rylut[row],
                );
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("CLK_REBUF"),
                    &[&name],
                    self.db.get_node_naming("CLK_BUFG_REBUF"),
                    &[],
                );
                for i in 0..16 {
                    node.add_bel(i, format!("GCLK_TEST_BUF_X0Y{y}", y = ctb_y + i));
                }
                for i in 0..16 {
                    node.add_bel(16 + i, format!("GCLK_TEST_BUF_X1Y{y}", y = ctb_y + i));
                }
            }

            let tk = if reg < self.grid.reg_clk {
                "CLK_HROW_BOT_R"
            } else {
                "CLK_HROW_TOP_R"
            };
            let name = format!(
                "{tk}_X{x}Y{y}",
                x = self.rxlut[col] + 2,
                y = self.rylut[row_h] - 1,
            );
            let node = self.die[(col, row_h)].add_xnode(
                self.db.get_node("CLK_HROW"),
                &[&name],
                self.db.get_node_naming(tk),
                &[(col, row_h - 1), (col, row_h)],
            );
            for i in 0..32 {
                node.add_bel(
                    i,
                    format!(
                        "GCLK_TEST_BUF_X{x}Y{y}",
                        x = i >> 4,
                        y = ctb_y + 16 + (i & 0xf ^ 0xf)
                    ),
                );
            }
            for i in 0..12 {
                node.add_bel(32 + i, format!("BUFHCE_X0Y{y}", y = bufh_y + i));
            }
            for i in 0..12 {
                node.add_bel(44 + i, format!("BUFHCE_X1Y{y}", y = bufh_y + i));
            }
            node.add_bel(56, format!("GCLK_TEST_BUF_X3Y{y}", y = ctb_y + 17));
            node.add_bel(57, format!("GCLK_TEST_BUF_X3Y{y}", y = ctb_y + 16));

            if self.grid.has_slr && reg == self.grid.regs - 1 {
                let tk = if self.has_gtz_u {
                    "CLK_BALI_REBUF_GTZ_TOP"
                } else {
                    "CLK_BALI_REBUF"
                };
                let row = row_h + 13;
                let name = format!(
                    "{tk}_X{x}Y{y}",
                    x = self.rxlut[col] + 2,
                    y = self.rylut[row],
                );
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("CLK_REBUF"),
                    &[&name],
                    self.db.get_node_naming("CLK_BALI_REBUF"),
                    &[],
                );
                for i in 0..16 {
                    let y = (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3;
                    if self.has_gtz_u {
                        node.add_bel(i, format!("BUFG_LB_X1Y{y}", y = bglb_y + y));
                    } else {
                        node.add_bel(i, format!("GCLK_TEST_BUF_X1Y{y}", y = ctb_y + 32 + y));
                    }
                }
                for i in 0..16 {
                    let y = (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3;
                    node.add_bel(16 + i, format!("GCLK_TEST_BUF_X3Y{y}", y = ctb_y + 32 + y));
                }
                if self.has_gtz_u {
                    bglb_y += 16;
                }
            } else {
                let row = row_h + 11;
                let name = format!(
                    "CLK_BUFG_REBUF_X{x}Y{y}",
                    x = self.rxlut[col] + 2,
                    y = self.rylut[row],
                );
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("CLK_REBUF"),
                    &[&name],
                    self.db.get_node_naming("CLK_BUFG_REBUF"),
                    &[],
                );
                for i in 0..16 {
                    node.add_bel(i, format!("GCLK_TEST_BUF_X0Y{y}", y = ctb_y + 32 + i));
                }
                for i in 0..16 {
                    node.add_bel(16 + i, format!("GCLK_TEST_BUF_X1Y{y}", y = ctb_y + 32 + i));
                }
            }
        }

        let di = self.die.die.to_idx();
        let bg_y = di * 32;
        let row = self.grid.row_bufg() - 4;
        let crds: [_; 4] = core::array::from_fn(|dy| (col, row + dy));
        let name = format!(
            "CLK_BUFG_BOT_R_X{x}Y{y}",
            x = self.rxlut[col] + 2,
            y = self.rylut[row]
        );
        let node = self.die[(col, row)].add_xnode(
            self.db.get_node("CLK_BUFG"),
            &[&name],
            self.db.get_node_naming("CLK_BUFG_BOT_R"),
            &crds,
        );
        for i in 0..16 {
            node.add_bel(i, format!("BUFGCTRL_X0Y{y}", y = bg_y + i));
        }
        if self.grid.reg_clk != self.grid.regs {
            let row = self.grid.row_bufg();
            let crds: [_; 4] = core::array::from_fn(|dy| (col, row + dy));
            let name = format!(
                "CLK_BUFG_TOP_R_X{x}Y{y}",
                x = self.rxlut[col] + 2,
                y = self.rylut[row]
            );
            let node = self.die[(col, row)].add_xnode(
                self.db.get_node("CLK_BUFG"),
                &[&name],
                self.db.get_node_naming("CLK_BUFG_TOP_R"),
                &crds,
            );
            for i in 0..16 {
                node.add_bel(i, format!("BUFGCTRL_X0Y{y}", y = bg_y + 16 + i));
            }
        }

        let pmv_base = if self.grid.regs == 1 { 0 } else { 1 };
        let piox = if self.grid.col_clk < self.grid.col_cfg {
            0
        } else {
            1
        };
        let pioy = if self.grid.reg_clk <= self.grid.reg_cfg {
            0
        } else {
            1
        };
        for (tk, dy, dyi, bname) in [
            (
                "CLK_PMV",
                pmv_base,
                pmv_base + 3,
                format!("PMV_X0Y{y}", y = di * 3),
            ),
            (
                "CLK_PMVIOB",
                17,
                17,
                format!("PMVIOB_X{piox}Y{y}", y = di * 2 + pioy),
            ),
            (
                "CLK_PMV2_SVT",
                32,
                32,
                format!("PMV_X0Y{y}", y = di * 3 + 1),
            ),
            ("CLK_PMV2", 41, 41, format!("PMV_X0Y{y}", y = di * 3 + 2)),
            ("CLK_MTBF2", 45, 45, format!("MTBF2_X0Y{di}")),
        ] {
            let row = self.grid.row_bufg() - 50 + dy;
            let row_int = self.grid.row_bufg() - 50 + dyi;
            let name = format!(
                "{tk}_X{x}Y{y}",
                x = self.rxlut[col] + 2,
                y = self.rylut[row]
            );
            let node = self.die[(col, row_int)].add_xnode(
                self.db.get_node(tk),
                &[&name],
                self.db.get_node_naming(tk),
                &[(col, row_int)],
            );
            node.add_bel(0, bname);
        }

        bglb_y
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            if col.to_idx() % 2 != 0 {
                continue;
            }
            'a: for row in self.die.rows() {
                if row.to_idx() % 50 == 25 {
                    let mut suf = "";
                    if self.grid.has_slr
                        && !(col >= self.grid.col_cfg - 6 && col < self.grid.col_cfg)
                    {
                        if row.to_idx() < 50 {
                            if self.has_slr_d {
                                suf = "_SLV";
                            }
                            if self.has_gtz_d && col.to_idx() < 162 {
                                suf = "_SLV";
                            }
                        }
                        if row.to_idx() >= self.grid.regs * 50 - 50 {
                            if self.has_slr_u {
                                suf = "_SLV";
                            }
                            if self.has_gtz_u && col.to_idx() < 162 {
                                suf = "_SLV";
                            }
                        }
                    }
                    let mut hole_bot = false;
                    let mut hole_top = false;
                    for &hole in &self.int_holes {
                        if hole.contains(col, row) {
                            hole_top = true;
                        }
                        if hole.contains(col, row - 1) {
                            hole_bot = true;
                        }
                    }
                    if hole_bot && hole_top {
                        continue;
                    }
                    if hole_bot {
                        suf = "_BOT_UTURN";
                    }
                    if hole_top {
                        suf = "_TOP_UTURN";
                    }
                    let x = self.rxlut[col + 1] - 1;
                    let y = self.rylut[row] - 1;
                    let name_l = format!("HCLK_L{suf}_X{x}Y{y}");
                    let name_r = format!("HCLK_R{suf}_X{x}Y{y}", x = x + 1);
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("HCLK"),
                        &[&name_l, &name_r],
                        self.db.get_node_naming("HCLK"),
                        &[],
                    );
                }

                for &hole in &self.int_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let x = self.xlut[col];
                let y = self.ylut[row];
                let name_l = format!("INT_L_X{x}Y{y}");
                let name_r = format!("INT_R_X{x}Y{y}", x = x + 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("INT_GCLK"),
                    &[&name_l, &name_r],
                    self.db.get_node_naming("INT_GCLK"),
                    &[(col, row), (col + 1, row)],
                );
            }
        }
    }
}

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &'a Grid>,
    grid_master: DieId,
    extras: &[ExtraDie],
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    egrid.tie_kind = Some("TIEOFF".to_string());
    egrid.tie_pin_gnd = Some("HARD0".to_string());
    egrid.tie_pin_vcc = Some("HARD1".to_string());
    let mut yb = 0;
    let mut ryb = 0;
    let mut tie_yb = 0;
    let mut pcie2_y = 0;
    let mut pcie3_y = 0;
    let mut bglb_y = 0;
    let mut dci_y = 0;
    let mut ipy = 0;
    let mut opy = 0;
    let mut gty = 0;
    if extras.iter().any(|&x| x == ExtraDie::GtzBottom) {
        yb = 1;
        ryb = 2;
        ipy = 6;
        opy = 2;
    }
    for &grid in grids.values() {
        let is_7k70t = grid.kind == GridKind::Kintex && grid.regs == 4 && !grid.has_ps;
        let (did, die) = egrid.add_die(grid.columns.len(), grid.regs * 50);

        let mut de = DieExpander {
            grid,
            db,
            die,
            xlut: EntityVec::new(),
            rxlut: EntityVec::new(),
            tiexlut: EntityVec::new(),
            ipxlut: EntityVec::new(),
            opxlut: EntityVec::new(),
            ylut: EntityVec::new(),
            rylut: EntityVec::new(),
            tieylut: EntityVec::new(),
            dciylut: EntityVec::new(),
            ipylut: EntityVec::new(),
            opylut: EntityVec::new(),
            gtylut: EntityVec::new(),
            site_holes: Vec::new(),
            int_holes: Vec::new(),
            has_slr_d: did != grids.first_id().unwrap(),
            has_slr_u: did != grids.last_id().unwrap(),
            has_gtz_d: did == grids.first_id().unwrap() && extras.contains(&ExtraDie::GtzBottom),
            has_gtz_u: did == grids.last_id().unwrap() && extras.contains(&ExtraDie::GtzTop),
        };

        de.fill_xlut();
        de.fill_rxlut();
        de.fill_tiexlut();
        de.fill_ipxlut(!extras.is_empty(), is_7k70t);
        de.fill_opxlut(!extras.is_empty());
        yb = de.fill_ylut(yb);
        ryb = de.fill_rylut(ryb);
        tie_yb = de.fill_tieylut(tie_yb);
        dci_y = de.fill_dciylut(dci_y);
        ipy = de.fill_ipylut(ipy, is_7k70t);
        opy = de.fill_opylut(opy);
        gty = de.fill_gtylut(gty);
        de.fill_int();
        de.fill_cfg(de.die.die == grid_master);
        de.fill_ps();
        pcie2_y = de.fill_pcie2(pcie2_y);
        pcie3_y = de.fill_pcie3(pcie3_y);
        de.fill_gt_mid();
        de.fill_gt_left();
        de.fill_gt_right();
        de.fill_terms();
        de.die.fill_main_passes();
        de.fill_clb();
        de.fill_bram_dsp();
        de.fill_io();
        de.fill_cmt();
        bglb_y = de.fill_clk(bglb_y);
        de.fill_hclk();
    }

    let lvb6 = db
        .wires
        .iter()
        .find_map(|(k, v)| if v.name == "LVB.6" { Some(k) } else { None })
        .unwrap();
    let mut xdie_wires = HashMap::new();
    for i in 1..grids.len() {
        let dieid_s = DieId::from_idx(i - 1);
        let dieid_n = DieId::from_idx(i);
        let die_s = egrid.die(dieid_s);
        let die_n = egrid.die(dieid_n);
        for col in die_s.cols() {
            for dy in 0..49 {
                let row_s = die_s.rows().next_back().unwrap() - 49 + dy;
                let row_n = die_n.rows().next().unwrap() + 1 + dy;
                if !die_s[(col, row_s)].nodes.is_empty() && !die_n[(col, row_n)].nodes.is_empty() {
                    xdie_wires.insert((dieid_n, (col, row_n), lvb6), (dieid_s, (col, row_s), lvb6));
                }
            }
        }
    }
    egrid.xdie_wires = xdie_wires;

    ExpandedDevice {
        grids: grids.clone(),
        grid_master,
        egrid,
        extras: extras.to_vec(),
        disabled: disabled.clone(),
    }
}

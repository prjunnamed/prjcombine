#![allow(clippy::bool_to_int_with_if)]
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::{
    ColumnKind, DieFrameGeom, DisabledPart, ExpandedDevice, ExtraDie, Grid, Gt, GtKind, Gtz,
    GtzLoc, Io, IoCoord, IoDiffKind, IoKind, IoVrKind, Pcie2Kind, PsIo, PsPin, RegId, SharedCfgPin,
    SysMon, TileIobId,
};

struct DieExpander<'a, 'b, 'c> {
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
    bankylut: EntityVec<RegId, u32>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    has_slr_d: bool,
    has_slr_u: bool,
    has_gtz_d: bool,
    has_gtz_u: bool,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
    col_clk: ColId,
    col_lio: Option<ColId>,
    col_rio: Option<ColId>,
    io: &'c mut Vec<Io>,
    gt: &'c mut Vec<Gt>,
    sysmon: &'c mut Vec<SysMon>,
}

impl DieExpander<'_, '_, '_> {
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
                    .any(|x| x.regs[self.grid.row_to_reg(row)] == Some(IoKind::Hpio))
            {
                dciy += 1;
            }
        }
        dciy
    }

    fn fill_ipylut(&mut self, mut ipy: usize, is_7k70t: bool) -> usize {
        for row in self.die.rows() {
            let reg = self.grid.row_to_reg(row);
            self.ipylut.push(ipy);
            if matches!(row.to_idx() % 50, 0 | 11 | 22 | 28 | 39) {
                let mut has_gt = false;
                for gtcol in self.grid.cols_gt.iter() {
                    if gtcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if has_gt {
                    ipy += 6;
                }
            }
            if !is_7k70t && row == self.grid.row_reg_hclk(self.grid.reg_cfg) {
                ipy += 6;
            }
        }
        if is_7k70t {
            self.ipylut[self.grid.row_reg_hclk(self.grid.reg_cfg)] = ipy + 6;
        }
        ipy
    }

    fn fill_opylut(&mut self, mut opy: usize) -> usize {
        for row in self.die.rows() {
            let reg = self.grid.row_to_reg(row);
            self.opylut.push(opy);
            if matches!(row.to_idx() % 50, 0 | 11 | 28 | 39) {
                let mut has_gt = false;
                for gtcol in self.grid.cols_gt.iter() {
                    if gtcol.regs[reg].is_some() {
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
            let reg = self.grid.row_to_reg(row);
            self.gtylut.push(gty);
            if row.to_idx() % 50 == 0 {
                let mut has_gt = false;
                for gtcol in self.grid.cols_gt.iter() {
                    if gtcol.regs[reg].is_some() {
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

    fn fill_bankylut(&mut self, mut bank: u32) -> u32 {
        for _ in self.grid.regs() {
            self.bankylut.push(bank);
            bank += 1
        }
        bank
    }

    fn fill_xlut(&mut self) {
        let mut x = 0;
        for col in self.grid.columns.ids() {
            self.xlut.push(x);
            if self.grid.regs == 2 && self.grid.has_ps && col.to_idx() < 18 {
                continue;
            }
            if self.grid.regs <= 2 && col < self.col_cfg && col >= self.col_cfg - 6 {
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
            if self.grid.regs <= 2 && col < self.col_cfg && col >= self.col_cfg - 6 {
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
            for gtcol in self.grid.cols_gt.iter() {
                if gtcol.col == col {
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
            for gtcol in self.grid.cols_gt.iter() {
                if gtcol.col == col {
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
        let row_cm = self.grid.row_reg_bot(self.grid.reg_cfg);
        let row_cb = row_cm - 50;
        let row_ct = row_cm + 50;
        if self.grid.regs == 1 {
            self.die.nuke_rect(self.col_cfg - 6, row_cb, 6, 50);
            self.int_holes.push(Rect {
                col_l: self.col_cfg - 6,
                col_r: self.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 50,
            });
            self.site_holes.push(Rect {
                col_l: self.col_cfg - 6,
                col_r: self.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 50,
            });
        } else {
            self.die.nuke_rect(self.col_cfg - 6, row_cb, 6, 100);
            self.int_holes.push(Rect {
                col_l: self.col_cfg - 6,
                col_r: self.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 100,
            });
            self.site_holes.push(Rect {
                col_l: self.col_cfg - 6,
                col_r: self.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 100,
            });
            for dx in 0..6 {
                let col = self.col_cfg - 6 + dx;
                if row_cb.to_idx() != 0 {
                    self.die.fill_term_anon((col, row_cb - 1), "TERM.N");
                }
                if row_ct.to_idx() != self.grid.regs * 50 {
                    self.die.fill_term_anon((col, row_ct), "TERM.S");
                }
            }
        }

        let slv = if is_master { "" } else { "_SLAVE" };
        let rx = self.rxlut[self.col_cfg] - 1;
        let name_b = format!("CFG_CENTER_BOT_X{rx}Y{y}", y = self.rylut[row_cb + 10]);
        let name_m = format!("CFG_CENTER_MID{slv}_X{rx}Y{y}", y = self.rylut[row_cb + 30]);
        let name_t = format!("CFG_CENTER_TOP{slv}_X{rx}Y{y}", y = self.rylut[row_cb + 40]);
        let crds: [_; 50] = core::array::from_fn(|dy| (self.col_cfg, row_cb + dy));
        let di = self.die.die.to_idx();
        let node = self.die[(self.col_cfg, row_cb)].add_xnode(
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
        let pix = if self.col_cfg < self.col_clk { 0 } else { 1 };
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
            #[derive(Copy, Clone, Eq, PartialEq)]
            enum XadcIoLoc {
                Left,
                Right,
                LR,
            }
            let row_m = row_cm + 25;
            let io_loc = if self.grid.has_ps {
                XadcIoLoc::Right
            } else if let Some(col_rio) = self.col_rio {
                if self.grid.get_col_io(col_rio).unwrap().regs[self.grid.reg_cfg].is_some() {
                    XadcIoLoc::LR
                } else {
                    XadcIoLoc::Left
                }
            } else {
                XadcIoLoc::Left
            };
            let vaux = match io_loc {
                XadcIoLoc::Left => [
                    Some((self.col_lio, 47)),
                    Some((self.col_lio, 43)),
                    Some((self.col_lio, 39)),
                    Some((self.col_lio, 33)),
                    Some((self.col_lio, 29)),
                    Some((self.col_lio, 25)),
                    None,
                    None,
                    Some((self.col_lio, 45)),
                    Some((self.col_lio, 41)),
                    Some((self.col_lio, 35)),
                    Some((self.col_lio, 31)),
                    Some((self.col_lio, 27)),
                    None,
                    None,
                    None,
                ],
                XadcIoLoc::Right => [
                    Some((self.col_rio, 47)),
                    Some((self.col_rio, 43)),
                    Some((self.col_rio, 35)),
                    Some((self.col_rio, 31)),
                    Some((self.col_rio, 21)),
                    Some((self.col_rio, 15)),
                    Some((self.col_rio, 9)),
                    Some((self.col_rio, 5)),
                    Some((self.col_rio, 45)),
                    Some((self.col_rio, 39)),
                    Some((self.col_rio, 33)),
                    Some((self.col_rio, 29)),
                    Some((self.col_rio, 19)),
                    Some((self.col_rio, 13)),
                    Some((self.col_rio, 7)),
                    Some((self.col_rio, 1)),
                ],
                XadcIoLoc::LR => [
                    Some((self.col_lio, 47)),
                    Some((self.col_lio, 43)),
                    Some((self.col_lio, 35)),
                    Some((self.col_lio, 31)),
                    Some((self.col_rio, 47)),
                    Some((self.col_rio, 43)),
                    Some((self.col_rio, 35)),
                    Some((self.col_rio, 31)),
                    Some((self.col_lio, 45)),
                    Some((self.col_lio, 39)),
                    Some((self.col_lio, 33)),
                    Some((self.col_lio, 29)),
                    Some((self.col_rio, 45)),
                    Some((self.col_rio, 39)),
                    Some((self.col_rio, 33)),
                    Some((self.col_rio, 29)),
                ],
            };
            let sysmon = SysMon {
                die: self.die.die,
                col: self.col_cfg,
                row: row_m,
                bank: 0,
                pad_vp: format!(
                    "IPAD_X{x}Y{y}",
                    x = self.ipxlut[self.col_cfg],
                    y = self.ipylut[row_m],
                ),
                pad_vn: format!(
                    "IPAD_X{x}Y{y}",
                    x = self.ipxlut[self.col_cfg],
                    y = self.ipylut[row_m] + 1,
                ),
                vaux: vaux
                    .into_iter()
                    .map(|x| {
                        x.map(|(col, dy)| {
                            let col = col.unwrap();
                            let row = row_cm + dy;
                            (
                                IoCoord {
                                    die: self.die.die,
                                    col,
                                    row,
                                    iob: TileIobId::from_idx(0),
                                },
                                IoCoord {
                                    die: self.die.die,
                                    col,
                                    row,
                                    iob: TileIobId::from_idx(1),
                                },
                            )
                        })
                    })
                    .collect(),
            };
            let kind = match io_loc {
                XadcIoLoc::Right => "XADC.R",
                XadcIoLoc::Left => "XADC.L",
                XadcIoLoc::LR => "XADC.LR",
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
            let crds: [_; 25] = core::array::from_fn(|dy| (self.col_cfg, row_m + dy));
            let di = self.die.die.to_idx();
            let mut names = vec![&name_b[..], &name_m[..], &name_t[..]];
            if io_loc == XadcIoLoc::Right {
                names.extend([&name_bs[..], &name_ms[..], &name_ts[..]]);
            }
            let node = self.die[(self.col_cfg, row_m)].add_xnode(
                self.db.get_node("XADC"),
                &names,
                self.db.get_node_naming(kind),
                &crds,
            );
            node.add_bel(0, sysmon.pad_vp.clone());
            node.add_bel(1, sysmon.pad_vn.clone());
            node.add_bel(2, format!("XADC_X0Y{di}"));
            self.sysmon.push(sysmon);
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
        let has_pcie2_left = self
            .grid
            .holes_pcie2
            .iter()
            .any(|x| x.kind == Pcie2Kind::Left);
        let mut ply = pcie2_y;
        let mut pry = pcie2_y;
        for pcie2 in &self.grid.holes_pcie2 {
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
                    sx = usize::from(has_pcie2_left);
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
        for &(bc, br) in &self.grid.holes_pcie3 {
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

    fn fill_gt(&mut self) {
        for (gtx, gtcol) in self.grid.cols_gt.iter().enumerate() {
            let is_l = gtcol.col < self.col_clk;
            let is_m = if is_l {
                gtcol.col.to_idx() != 0
            } else {
                self.grid.columns.len() - gtcol.col.to_idx() > 7
            };
            let ipx = self.ipxlut[gtcol.col];
            let opx = self.opxlut[gtcol.col];
            for (reg, &kind) in &gtcol.regs {
                let br = self.grid.row_reg_bot(reg);
                if let Some(kind) = kind {
                    let sk = match kind {
                        GtKind::Gtp => "GTP",
                        GtKind::Gtx => "GTX",
                        GtKind::Gth => "GTH",
                    };
                    let x = self.xlut[gtcol.col];
                    if is_m {
                        assert_eq!(kind, GtKind::Gtp);
                        if is_l {
                            self.die.nuke_rect(gtcol.col + 1, br, 18, 50);
                            self.int_holes.push(Rect {
                                col_l: gtcol.col + 1,
                                col_r: gtcol.col + 19,
                                row_b: br,
                                row_t: br + 50,
                            });
                            self.site_holes.push(Rect {
                                col_l: gtcol.col,
                                col_r: gtcol.col + 19,
                                row_b: br,
                                row_t: br + 50,
                            });
                            for dx in 1..19 {
                                let col = gtcol.col + dx;
                                if br.to_idx() != 0 {
                                    self.die.fill_term_anon((col, br - 1), "TERM.N");
                                }
                                if br.to_idx() + 50 != self.grid.regs * 50 {
                                    self.die.fill_term_anon((col, br + 50), "TERM.S");
                                }
                            }
                            let col_l = gtcol.col;
                            let col_r = gtcol.col + 19;
                            for dy in 0..50 {
                                let row = br + dy;
                                let y = self.ylut[row];
                                let tile = &mut self.die[(col_l, row)];
                                tile.nodes.truncate(1);
                                tile.add_xnode(
                                    self.db.get_node("INTF.DELAY"),
                                    &[&format!("GTP_INT_INTERFACE_R_X{x}Y{y}")],
                                    self.db.get_node_naming("INTF.GTP_R"),
                                    &[(col_l, row)],
                                );
                                self.die.fill_term_anon((col_l, row), "TERM.E");
                                self.die.fill_term_anon((col_r, row), "TERM.W");
                            }
                        } else {
                            self.die.nuke_rect(gtcol.col - 18, br, 18, 50);
                            self.int_holes.push(Rect {
                                col_l: gtcol.col - 18,
                                col_r: gtcol.col,
                                row_b: br,
                                row_t: br + 50,
                            });
                            self.site_holes.push(Rect {
                                col_l: gtcol.col - 18,
                                col_r: gtcol.col + 1,
                                row_b: br,
                                row_t: br + 50,
                            });
                            for dx in 1..19 {
                                let col = gtcol.col - 19 + dx;
                                if br.to_idx() != 0 {
                                    self.die.fill_term_anon((col, br - 1), "TERM.N");
                                }
                                if br.to_idx() + 50 != self.grid.regs * 50 {
                                    self.die.fill_term_anon((col, br + 50), "TERM.S");
                                }
                            }
                            let col_l = gtcol.col - 19;
                            let col_r = gtcol.col;
                            for dy in 0..50 {
                                let row = br + dy;
                                let y = self.ylut[row];
                                let tile = &mut self.die[(col_r, row)];
                                tile.nodes.truncate(1);
                                tile.add_xnode(
                                    self.db.get_node("INTF.DELAY"),
                                    &[&format!("GTP_INT_INTERFACE_L_X{x}Y{y}")],
                                    self.db.get_node_naming("INTF.GTP_L"),
                                    &[(col_r, row)],
                                );
                                self.die.fill_term_anon((col_l, row), "TERM.E");
                                self.die.fill_term_anon((col_r, row), "TERM.W");
                            }
                        }
                    } else if is_l {
                        for dy in 0..50 {
                            let row = br + dy;
                            let y = self.ylut[row];
                            let tile = &mut self.die[(gtcol.col, row)];
                            tile.nodes.truncate(1);
                            tile.add_xnode(
                                self.db.get_node("INTF.DELAY"),
                                &[&format!("{sk}_INT_INTERFACE_L_X{x}Y{y}")],
                                self.db.get_node_naming(&format!("INTF.{sk}_L")),
                                &[(gtcol.col, row)],
                            );
                        }
                    } else {
                        if gtcol.col != self.grid.columns.last_id().unwrap() {
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
                            if reg.to_idx() != 0 && gtcol.regs[reg - 1].is_none() {
                                for dx in 1..7 {
                                    self.die.fill_term_anon((gtcol.col + dx, br - 1), "TERM.N");
                                }
                            }
                            if reg.to_idx() != self.grid.regs - 1 && gtcol.regs[reg + 1].is_none() {
                                for dx in 1..7 {
                                    self.die.fill_term_anon((gtcol.col + dx, br + 50), "TERM.S");
                                }
                            }
                            for dy in 0..50 {
                                self.die.fill_term_anon((gtcol.col, br + dy), "TERM.E");
                            }
                        }
                        for dy in 0..50 {
                            let row = br + dy;
                            let y = self.ylut[row];
                            let tile = &mut self.die[(gtcol.col, row)];
                            tile.nodes.truncate(1);
                            tile.add_xnode(
                                self.db.get_node("INTF.DELAY"),
                                &[&format!("{sk}_INT_INTERFACE_X{x}Y{y}")],
                                self.db.get_node_naming(&format!("INTF.{sk}")),
                                &[(gtcol.col, row)],
                            );
                        }
                    }
                    let gty = self.gtylut[br];
                    let bank = if kind == GtKind::Gtp {
                        if self.grid.has_ps {
                            112
                        } else {
                            (if reg.to_idx() == 0 { 13 } else { 16 })
                                + if is_m && !is_l { 100 } else { 200 }
                        }
                    } else {
                        self.bankylut[reg] + if is_l { 200 } else { 100 }
                    };
                    let mut gt = Gt {
                        die: self.die.die,
                        col: gtcol.col,
                        row: br,
                        bank,
                        kind,
                        pads_clk: vec![],
                        pads_rx: vec![],
                        pads_tx: vec![],
                    };
                    let rx = if is_m {
                        if is_l {
                            self.rxlut[gtcol.col] + 14
                        } else {
                            self.rxlut[gtcol.col] - 18
                        }
                    } else {
                        if is_l {
                            self.rxlut[gtcol.col]
                        } else {
                            self.rxlut[gtcol.col] + 4
                        }
                    };
                    let nsuf = if is_m {
                        if is_l {
                            "_MID_LEFT"
                        } else {
                            "_MID_RIGHT"
                        }
                    } else {
                        ""
                    };
                    for (i, dy) in [(0, 0), (1, 11), (2, 28), (3, 39)] {
                        let row = br + dy;
                        let ry = self.rylut[row + 5];
                        let name = format!("{sk}_CHANNEL_{i}{nsuf}_X{rx}Y{ry}");
                        let crds: [_; 11] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                        let node = self.die[(gtcol.col, row)].add_xnode(
                            self.db.get_node(&format!("{sk}_CHANNEL")),
                            &[&name],
                            self.db.get_node_naming(&format!("{sk}_CHANNEL_{i}{nsuf}")),
                            &crds,
                        );
                        let ipy = self.ipylut[row];
                        let opy = self.opylut[row];
                        node.add_bel(0, format!("{sk}E2_CHANNEL_X{gtx}Y{y}", y = gty * 4 + i));
                        gt.pads_rx.push((
                            format!("IPAD_X{ipx}Y{y}", y = ipy + 1),
                            format!("IPAD_X{ipx}Y{y}", y = ipy),
                        ));
                        gt.pads_tx.push((
                            format!("OPAD_X{opx}Y{y}", y = opy + 1),
                            format!("OPAD_X{opx}Y{y}", y = opy),
                        ));
                        node.add_bel(1, gt.pads_rx[i].0.clone());
                        node.add_bel(2, gt.pads_rx[i].1.clone());
                        node.add_bel(3, gt.pads_tx[i].0.clone());
                        node.add_bel(4, gt.pads_tx[i].1.clone());
                    }
                    let row = br + 22;
                    let ry = self.rylut[row];
                    let name = format!("{sk}_COMMON{nsuf}_X{rx}Y{ry}",);
                    let crds: [_; 6] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                    let node = self.die[(gtcol.col, row + 3)].add_xnode(
                        self.db.get_node(&format!("{sk}_COMMON")),
                        &[&name],
                        self.db.get_node_naming(&format!("{sk}_COMMON{nsuf}")),
                        &crds,
                    );

                    let ipy = self.ipylut[row];
                    node.add_bel(0, format!("{sk}E2_COMMON_X{gtx}Y{gty}"));
                    node.add_bel(1, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2));
                    node.add_bel(2, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2 + 1));
                    gt.pads_clk = vec![
                        (
                            format!("IPAD_X{ipx}Y{y}", y = ipy - 4),
                            format!("IPAD_X{ipx}Y{y}", y = ipy - 3),
                        ),
                        (
                            format!("IPAD_X{ipx}Y{y}", y = ipy - 2),
                            format!("IPAD_X{ipx}Y{y}", y = ipy - 1),
                        ),
                    ];
                    node.add_bel(3, gt.pads_clk[0].0.clone());
                    node.add_bel(4, gt.pads_clk[0].1.clone());
                    node.add_bel(5, gt.pads_clk[1].0.clone());
                    node.add_bel(6, gt.pads_clk[1].1.clone());

                    self.gt.push(gt);
                }
                if br.to_idx() != 0 && (kind.is_some() || gtcol.regs[reg - 1].is_some()) && !is_m {
                    let name = if gtcol.regs[reg - 1].is_none() {
                        format!(
                            "BRKH_GTX_X{x}Y{y}",
                            x = self.xlut[gtcol.col] + 1,
                            y = self.ylut[br] - 1
                        )
                    } else {
                        format!(
                            "BRKH_GTX_X{x}Y{y}",
                            x = self.rxlut[gtcol.col] + if is_l { 0 } else { 4 },
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
        for iocol in self.grid.cols_io.iter() {
            let col = iocol.col;
            let is_l = col < self.col_clk;
            let is_term = if is_l {
                col == self.grid.columns.first_id().unwrap()
            } else {
                col == self.grid.columns.last_id().unwrap()
            };
            let mut found = false;
            let mut found_hp = false;
            for row in self.die.rows() {
                let reg = self.grid.row_to_reg(row);
                if let Some(kind) = iocol.regs[reg] {
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
                    let bank = self.bankylut[reg] + if is_l { 0 } else { 20 };
                    let biob = (row.to_idx() % 50) as u32;

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
                        let iob_name = format!("IOB_X{iox}Y{y}", y = self.tieylut[row]);
                        if kind == IoKind::Hpio {
                            node.add_bel(3, format!("ODELAY_X{iox}Y{y}", y = self.tieylut[row]));
                            node.add_bel(4, iob_name.clone());
                        } else {
                            node.add_bel(3, iob_name.clone());
                        }
                        self.io.push(Io {
                            crd: IoCoord {
                                die: self.die.die,
                                col,
                                row,
                                iob: TileIobId::from_idx(0),
                            },
                            name: iob_name,
                            bank,
                            biob,
                            pkgid: match biob {
                                0 => 25,
                                49 => 0,
                                _ => unreachable!(),
                            },
                            byte: None,
                            kind,
                            diff: IoDiffKind::None,
                            is_lc: false,
                            is_gc: false,
                            is_srcc: false,
                            is_mrcc: false,
                            is_dqs: false,
                            is_vref: false,
                            vr: if kind == IoKind::Hpio {
                                match biob {
                                    0 => IoVrKind::VrP,
                                    49 => IoVrKind::VrN,
                                    _ => unreachable!(),
                                }
                            } else {
                                IoVrKind::None
                            },
                        });
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
                        let iob_name_p = format!("IOB_X{iox}Y{y}", y = self.tieylut[row] + 1);
                        let iob_name_n = format!("IOB_X{iox}Y{y}", y = self.tieylut[row]);
                        if kind == IoKind::Hpio {
                            node.add_bel(
                                6,
                                format!("ODELAY_X{iox}Y{y}", y = self.tieylut[row] + 1),
                            );
                            node.add_bel(7, format!("ODELAY_X{iox}Y{y}", y = self.tieylut[row]));
                            node.add_bel(8, iob_name_p.clone());
                            node.add_bel(9, iob_name_n.clone());
                        } else {
                            node.add_bel(6, iob_name_p.clone());
                            node.add_bel(7, iob_name_n.clone());
                        }
                        let pkgid = (50 - biob) / 2;
                        let crd_p = IoCoord {
                            die: self.die.die,
                            col,
                            row,
                            iob: TileIobId::from_idx(0),
                        };
                        let crd_n = IoCoord {
                            die: self.die.die,
                            col,
                            row,
                            iob: TileIobId::from_idx(1),
                        };
                        let is_srcc = matches!(biob, 21 | 27);
                        let is_mrcc = matches!(biob, 23 | 25);
                        let is_dqs = matches!(biob, 7 | 19 | 31 | 43);
                        let is_vref = matches!(biob, 11 | 37);
                        let byte = Some((pkgid - 1) / 6);
                        self.io.extend([
                            Io {
                                crd: crd_p,
                                name: iob_name_p,
                                bank,
                                biob: biob + 1,
                                pkgid,
                                byte,
                                kind,
                                diff: IoDiffKind::P(crd_n),
                                is_lc: false,
                                is_gc: false,
                                is_srcc,
                                is_mrcc,
                                is_dqs,
                                is_vref: false,
                                vr: IoVrKind::None,
                            },
                            Io {
                                crd: crd_n,
                                name: iob_name_n,
                                bank,
                                biob,
                                pkgid,
                                byte,
                                kind,
                                diff: IoDiffKind::N(crd_p),
                                is_lc: false,
                                is_gc: false,
                                is_srcc,
                                is_mrcc,
                                is_dqs,
                                is_vref,
                                vr: IoVrKind::None,
                            },
                        ]);
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
            'a: for reg in self.grid.regs() {
                let row = self.grid.row_reg_hclk(reg);
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
        let col = self.col_clk;
        for reg in self.grid.regs() {
            let row_h = self.grid.row_reg_hclk(reg);
            let ctb_y = self.tieylut[row_h] / 50 * 48;
            let bufh_y = self.tieylut[row_h] / 50 * 12;
            if self.grid.has_slr && reg.to_idx() == 0 {
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

            if self.grid.has_slr && reg.to_idx() == self.grid.regs - 1 {
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
        if self.grid.reg_clk.to_idx() != self.grid.regs {
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
        let piox = if self.col_clk < self.col_cfg { 0 } else { 1 };
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
                    if self.grid.has_slr && !(col >= self.col_cfg - 6 && col < self.col_cfg) {
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

    fn fill_frame_info(&mut self) {
        let mut regs: Vec<_> = self.grid.regs().collect();
        regs.sort_by_key(|&reg| {
            let rreg = reg - self.grid.reg_cfg;
            (rreg < 0, rreg.abs())
        });
        for _ in 0..self.grid.regs {
            self.frames.col_frame.push(EntityVec::new());
            self.frames.bram_frame.push(EntityPartVec::new());
        }
        for &reg in &regs {
            for (col, cd) in &self.grid.columns {
                self.frames.col_frame[reg].push(self.frame_info.len());
                if let Some(gtcol) = self.grid.get_col_gt(col) {
                    if gtcol.col != self.grid.columns.last_id().unwrap()
                        && gtcol.regs[reg].is_some()
                    {
                        for minor in 0..32 {
                            self.frame_info.push(FrameInfo {
                                addr: FrameAddr {
                                    typ: 0,
                                    region: (reg - self.grid.reg_cfg) as i32,
                                    major: col.to_idx() as u32,
                                    minor,
                                },
                            });
                        }
                        break;
                    }
                }
                let width = match cd {
                    ColumnKind::ClbLL => 36,
                    ColumnKind::ClbLM => 36,
                    ColumnKind::Bram => 28,
                    ColumnKind::Dsp => 28,
                    ColumnKind::Io => 42,
                    ColumnKind::Cmt => 30,
                    ColumnKind::Cfg => 30,
                    ColumnKind::Clk => 30,
                    ColumnKind::Gt => 32,
                };
                for minor in 0..width {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: (reg - self.grid.reg_cfg) as i32,
                            major: col.to_idx() as u32,
                            minor,
                        },
                    });
                }
            }
        }
        for &reg in &regs {
            let mut major = 0;
            for (col, &cd) in &self.grid.columns {
                if cd != ColumnKind::Bram {
                    continue;
                }
                self.frames.bram_frame[reg].insert(col, self.frame_info.len());
                for minor in 0..128 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 1,
                            region: (reg - self.grid.reg_cfg) as i32,
                            major,
                            minor,
                        },
                    });
                }
                major += 1;
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
    let mgrid = &grids[grid_master];
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
    let mut bank = (15
        - mgrid.reg_cfg.to_idx()
        - grids
            .iter()
            .filter_map(|(die, grid)| {
                if die < grid_master {
                    Some(grid.regs)
                } else {
                    None
                }
            })
            .sum::<usize>()) as u32;
    if extras.iter().any(|&x| x == ExtraDie::Gtz(GtzLoc::Bottom)) {
        yb = 1;
        ryb = 2;
        ipy = 6;
        opy = 2;
    }
    let mut frames = EntityVec::new();
    let mut die_bs_geom = EntityVec::new();

    let col_cfg = mgrid
        .columns
        .iter()
        .find_map(|(col, &cd)| {
            if cd == ColumnKind::Cfg {
                Some(col)
            } else {
                None
            }
        })
        .unwrap();
    let col_clk = mgrid
        .columns
        .iter()
        .find_map(|(col, &cd)| {
            if cd == ColumnKind::Clk {
                Some(col)
            } else {
                None
            }
        })
        .unwrap();
    let col_lio = mgrid.columns.iter().find_map(|(col, &cd)| {
        if cd == ColumnKind::Io && col < col_cfg {
            Some(col)
        } else {
            None
        }
    });
    let col_rio = mgrid.columns.iter().find_map(|(col, &cd)| {
        if cd == ColumnKind::Io && col > col_cfg {
            Some(col)
        } else {
            None
        }
    });
    let mut col_mgt = None;
    let mut col_lgt = None;
    let mut col_rgt = None;
    if mgrid.cols_gt.len() == 2 && mgrid.cols_gt[0].col.to_idx() != 0 {
        col_mgt = Some((mgrid.cols_gt[0].col, mgrid.cols_gt[1].col));
    } else {
        col_lgt = mgrid.cols_gt.iter().find_map(|gtcol| {
            if gtcol.col < col_cfg {
                Some(gtcol.col)
            } else {
                None
            }
        });
        col_rgt = mgrid.cols_gt.iter().find_map(|gtcol| {
            if gtcol.col > col_cfg {
                Some(gtcol.col)
            } else {
                None
            }
        });
    }

    let mut io = vec![];
    let mut gt = vec![];
    let mut sysmon = vec![];

    let mut is_7k70t = false;
    if let Some(rgt) = col_rgt {
        let gtcol = mgrid.get_col_gt(rgt).unwrap();
        if rgt == mgrid.columns.last_id().unwrap() - 6
            && gtcol.regs.values().any(|&y| y == Some(GtKind::Gtx))
            && mgrid.regs == 4
            && !mgrid.has_ps
        {
            is_7k70t = true;
        }
    }
    let has_gtz_bot = extras.contains(&ExtraDie::Gtz(GtzLoc::Bottom));
    let has_gtz_top = extras.contains(&ExtraDie::Gtz(GtzLoc::Top));
    for &grid in grids.values() {
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
            bankylut: EntityVec::new(),
            site_holes: Vec::new(),
            int_holes: Vec::new(),
            has_slr_d: did != grids.first_id().unwrap(),
            has_slr_u: did != grids.last_id().unwrap(),
            has_gtz_d: did == grids.first_id().unwrap() && has_gtz_bot,
            has_gtz_u: did == grids.last_id().unwrap() && has_gtz_top,
            frame_info: vec![],
            frames: DieFrameGeom {
                col_frame: EntityVec::new(),
                bram_frame: EntityVec::new(),
                spine_frame: EntityVec::new(),
            },
            col_cfg,
            col_clk,
            col_lio,
            col_rio,
            io: &mut io,
            gt: &mut gt,
            sysmon: &mut sysmon,
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
        bank = de.fill_bankylut(bank);
        de.fill_int();
        de.fill_cfg(de.die.die == grid_master);
        de.fill_ps();
        pcie2_y = de.fill_pcie2(pcie2_y);
        pcie3_y = de.fill_pcie3(pcie3_y);
        de.fill_gt();
        de.fill_terms();
        de.die.fill_main_passes();
        de.fill_clb();
        de.fill_bram_dsp();
        de.fill_io();
        de.fill_cmt();
        bglb_y = de.fill_clk(bglb_y);
        de.fill_hclk();
        de.fill_frame_info();

        frames.push(de.frames);
        die_bs_geom.push(DieBitstreamGeom {
            frame_len: 50 * 64 + 32,
            frame_info: de.frame_info,
            bram_cols: 0,
            bram_regs: 0,
            iob_frame_len: 0,
        });
    }

    for (die, &grid) in grids {
        if grid.has_no_tbuturn {
            let (w, _) = db.wires.iter().find(|(_, w)| w.name == "LVB.6").unwrap();
            for col in grid.columns.ids() {
                for i in 0..6 {
                    let row = RowId::from_idx(i);
                    egrid.blackhole_wires.insert((die, (col, row), w));
                }
                for i in 0..6 {
                    let row = RowId::from_idx(grid.regs * 50 - 6 + i);
                    egrid.blackhole_wires.insert((die, (col, row), w));
                }
            }
        }
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

    let mut die_order = vec![];
    die_order.push(grid_master);
    for die in grids.ids() {
        if die != grid_master {
            die_order.push(die);
        }
    }

    let bs_geom = BitstreamGeom {
        kind: DeviceKind::Series7,
        die: die_bs_geom,
        die_order,
    };

    let mut cfg_io = BTreeMap::new();
    let mut ps_io = BTreeMap::new();
    if mgrid.has_ps {
        cfg_io.insert(
            SharedCfgPin::PudcB,
            IoCoord {
                die: grid_master,
                col: col_rio.unwrap(),
                row: mgrid.row_reg_bot(mgrid.reg_cfg) - 50 + 43,
                iob: TileIobId::from_idx(0),
            },
        );
        let mut pins = vec![
            (1, 502, PsPin::DdrWeB),
            (2, 502, PsPin::DdrVrN),
            (3, 502, PsPin::DdrVrP),
            (4, 502, PsPin::DdrA(0)),
            (5, 502, PsPin::DdrA(1)),
            (6, 502, PsPin::DdrA(2)),
            (7, 502, PsPin::DdrA(3)),
            (8, 502, PsPin::DdrA(4)),
            (9, 502, PsPin::DdrA(5)),
            (10, 502, PsPin::DdrA(6)),
            (11, 502, PsPin::DdrA(7)),
            (12, 502, PsPin::DdrA(8)),
            (13, 502, PsPin::DdrA(9)),
            (14, 502, PsPin::DdrA(10)),
            (15, 502, PsPin::DdrA(11)),
            (16, 502, PsPin::DdrA(12)),
            (17, 502, PsPin::DdrA(14)),
            (18, 502, PsPin::DdrA(13)),
            (19, 502, PsPin::DdrBa(0)),
            (20, 502, PsPin::DdrBa(1)),
            (21, 502, PsPin::DdrBa(2)),
            (22, 502, PsPin::DdrCasB),
            (23, 502, PsPin::DdrCke),
            (24, 502, PsPin::DdrCkN),
            (25, 502, PsPin::DdrCkP),
            (26, 500, PsPin::Clk),
            (27, 502, PsPin::DdrCsB),
        ];
        pins.extend((0..4).map(|i| (28 + i, 502, PsPin::DdrDm(i))));
        pins.extend((0..32).map(|i| (32 + i, 502, PsPin::DdrDq(i))));
        pins.extend((0..4).map(|i| (64 + i, 502, PsPin::DdrDqsN(i))));
        pins.extend((0..4).map(|i| (68 + i, 502, PsPin::DdrDqsP(i))));
        pins.push((72, 502, PsPin::DdrDrstB));

        pins.extend((0..16).map(|i| (77 + i, 500, PsPin::Mio(i))));
        pins.extend((16..54).map(|i| (77 + i, 501, PsPin::Mio(i))));
        pins.extend([
            (131, 502, PsPin::DdrOdt),
            (132, 500, PsPin::PorB),
            (133, 502, PsPin::DdrRasB),
            (134, 501, PsPin::SrstB),
        ]);
        for (y, bank, pin) in pins {
            ps_io.insert(
                pin,
                PsIo {
                    bank,
                    name: format!("IOPAD_X1Y{y}"),
                },
            );
        }
    } else {
        cfg_io.extend(
            [
                (1, 1, SharedCfgPin::Data(16)),
                (1, 0, SharedCfgPin::Data(17)),
                (3, 1, SharedCfgPin::Data(18)),
                (3, 0, SharedCfgPin::Data(19)),
                (5, 1, SharedCfgPin::Data(20)),
                (5, 0, SharedCfgPin::Data(21)),
                (7, 1, SharedCfgPin::Data(22)),
                (9, 1, SharedCfgPin::Data(23)),
                (9, 0, SharedCfgPin::Data(24)),
                (11, 1, SharedCfgPin::Data(25)),
                (11, 0, SharedCfgPin::Data(26)),
                (13, 1, SharedCfgPin::Data(27)),
                (13, 0, SharedCfgPin::Data(28)),
                (15, 1, SharedCfgPin::Data(29)),
                (15, 0, SharedCfgPin::Data(30)),
                (17, 1, SharedCfgPin::Data(31)),
                (17, 0, SharedCfgPin::CsiB),
                (19, 1, SharedCfgPin::CsoB),
                (19, 0, SharedCfgPin::RdWrB),
                (29, 1, SharedCfgPin::Data(15)),
                (29, 0, SharedCfgPin::Data(14)),
                (31, 1, SharedCfgPin::Data(13)),
                (33, 1, SharedCfgPin::Data(12)),
                (33, 0, SharedCfgPin::Data(11)),
                (35, 1, SharedCfgPin::Data(10)),
                (35, 0, SharedCfgPin::Data(9)),
                (37, 1, SharedCfgPin::Data(8)),
                (37, 0, SharedCfgPin::FcsB),
                (39, 1, SharedCfgPin::Data(7)),
                (39, 0, SharedCfgPin::Data(6)),
                (41, 1, SharedCfgPin::Data(5)),
                (41, 0, SharedCfgPin::Data(4)),
                (43, 1, SharedCfgPin::EmCclk),
                (43, 0, SharedCfgPin::PudcB),
                (45, 1, SharedCfgPin::Data(3)),
                (45, 0, SharedCfgPin::Data(2)),
                (47, 1, SharedCfgPin::Data(1)),
                (47, 0, SharedCfgPin::Data(0)),
                (51, 1, SharedCfgPin::Rs(0)),
                (51, 0, SharedCfgPin::Rs(1)),
                (53, 1, SharedCfgPin::FweB),
                (53, 0, SharedCfgPin::FoeB),
                (55, 1, SharedCfgPin::Addr(16)),
                (55, 0, SharedCfgPin::Addr(17)),
                (57, 1, SharedCfgPin::Addr(18)),
                (59, 1, SharedCfgPin::Addr(19)),
                (59, 0, SharedCfgPin::Addr(20)),
                (61, 1, SharedCfgPin::Addr(21)),
                (61, 0, SharedCfgPin::Addr(22)),
                (63, 1, SharedCfgPin::Addr(23)),
                (63, 0, SharedCfgPin::Addr(24)),
                (65, 1, SharedCfgPin::Addr(25)),
                (65, 0, SharedCfgPin::Addr(26)),
                (67, 1, SharedCfgPin::Addr(27)),
                (67, 0, SharedCfgPin::Addr(28)),
                (69, 1, SharedCfgPin::AdvB),
            ]
            .into_iter()
            .map(|(dy, iob, pin)| {
                (
                    pin,
                    IoCoord {
                        die: grid_master,
                        col: col_lio.unwrap(),
                        row: mgrid.row_reg_bot(mgrid.reg_cfg) - 50 + dy,
                        iob: TileIobId::from_idx(iob),
                    },
                )
            }),
        );
    }

    let mut gtz = vec![];
    if has_gtz_bot {
        let ipy = 0;
        let opy = 0;
        gtz.push(Gtz {
            loc: GtzLoc::Bottom,
            bank: 400,
            pads_rx: (0..8)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 5 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 4 + 2 * i),
                    )
                })
                .collect(),
            pads_tx: (0..8)
                .map(|i| {
                    (
                        format!("OPAD_X1Y{}", opy + 1 + 2 * i),
                        format!("OPAD_X1Y{}", opy + 2 * i),
                    )
                })
                .collect(),
            pads_clk: (0..2)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 1 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 2 * i),
                    )
                })
                .collect(),
        });
    }
    if has_gtz_top {
        let ipy = if has_gtz_bot { 20 } else { 0 };
        let opy = if has_gtz_bot { 16 } else { 0 };
        gtz.push(Gtz {
            loc: GtzLoc::Bottom,
            bank: 300,
            pads_rx: (0..8)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 5 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 4 + 2 * i),
                    )
                })
                .collect(),
            pads_tx: (0..8)
                .map(|i| {
                    (
                        format!("OPAD_X1Y{}", opy + 1 + 2 * i),
                        format!("OPAD_X1Y{}", opy + 2 * i),
                    )
                })
                .collect(),
            pads_clk: (0..2)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 1 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 2 * i),
                    )
                })
                .collect(),
        });
    }

    ExpandedDevice {
        kind: mgrid.kind,
        grids: grids.clone(),
        grid_master,
        egrid,
        extras: extras.to_vec(),
        disabled: disabled.clone(),
        bs_geom,
        frames,
        col_cfg,
        col_clk,
        col_lio,
        col_rio,
        col_lcio: None,
        col_rcio: None,
        col_lgt,
        col_rgt,
        col_mgt,
        row_dcmiob: None,
        row_iobdcm: None,
        io,
        gt,
        gtz,
        sysmon,
        cfg_io,
        ps_io,
    }
}

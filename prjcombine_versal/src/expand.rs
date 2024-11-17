use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityVec};

use crate::expanded::ExpandedDevice;
use crate::grid::{
    ColumnKind, CpmKind, DisabledPart, Grid, HardRowKind, Interposer, PsKind, RightKind,
};

struct DieInfo {
    col_cfrm: ColId,
    ps_height: usize,
}

struct Expander<'a> {
    grids: EntityVec<DieId, &'a Grid>,
    disabled: BTreeSet<DisabledPart>,
    egrid: ExpandedGrid<'a>,
    die: EntityVec<DieId, DieInfo>,
}

impl Expander<'_> {
    fn fill_die(&mut self) {
        for (_, &grid) in &self.grids {
            self.egrid.add_die(grid.columns.len(), grid.regs * 48);
            let ps_height = match (grid.ps, grid.cpm) {
                (PsKind::Ps9, CpmKind::None) => 48 * 2,
                (PsKind::Ps9, CpmKind::Cpm4) => 48 * 3,
                (PsKind::Ps9, CpmKind::Cpm5) => 48 * 6,
                (PsKind::PsX, CpmKind::Cpm5N) => 48 * 9,
                _ => unreachable!(),
            };
            self.die.push(DieInfo {
                col_cfrm: grid
                    .columns
                    .iter()
                    .find(|(_, cd)| cd.l == ColumnKind::Cfrm)
                    .unwrap()
                    .0,
                ps_height,
            });
        }
    }

    fn fill_int(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
            let mut die = self.egrid.die_mut(dieid);

            let col_l = die.cols().next().unwrap();
            let col_r = die.cols().next_back().unwrap();
            let row_b = die.rows().next().unwrap();
            let row_t = die.rows().next_back().unwrap();
            let ps_width = di.col_cfrm.to_idx();
            for col in grid.columns.ids() {
                if self.disabled.contains(&DisabledPart::Column(dieid, col)) {
                    continue;
                }
                for row in die.rows() {
                    let reg = grid.row_to_reg(row);
                    if self.disabled.contains(&DisabledPart::Region(dieid, reg)) {
                        continue;
                    }
                    if col < di.col_cfrm && row.to_idx() < di.ps_height {
                        continue;
                    }
                    die.fill_tile((col, row), "INT");
                    if row.to_idx() % 48 == 0 && grid.is_reg_top(reg) {
                        die.add_xnode((col, row), "RCLK", &[(col, row)]);
                    }
                }
            }

            if di.ps_height != grid.regs * 48 {
                let row_t = RowId::from_idx(di.ps_height);
                for dx in 0..ps_width {
                    let col = ColId::from_idx(dx);
                    die.fill_term((col, row_t), "TERM.S");
                }
            }
            for dy in 0..di.ps_height {
                let row = RowId::from_idx(dy);
                die.fill_term((di.col_cfrm, row), "TERM.W");
            }

            for col in die.cols() {
                if !die[(col, row_b)].nodes.is_empty() {
                    die.fill_term((col, row_b), "TERM.S");
                }
                if !die[(col, row_t)].nodes.is_empty() {
                    die.fill_term((col, row_t), "TERM.N");
                }
            }
            for row in die.rows() {
                if !die[(col_l, row)].nodes.is_empty() {
                    die.fill_term((col_l, row), "TERM.W");
                }
                if !die[(col_r, row)].nodes.is_empty() {
                    die.fill_term((col_r, row), "TERM.E");
                }
            }
        }
    }

    fn fill_cle_bc(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);

            let row_b = die.rows().next().unwrap();
            let row_t = die.rows().next_back().unwrap();
            for (col, &cd) in &grid.columns {
                if matches!(cd.r, ColumnKind::Cle(_)) {
                    for row in die.rows() {
                        if die[(col, row)].nodes.is_empty() {
                            continue;
                        }
                        let has_bli_r = if row < row_b + 4 {
                            cd.has_bli_bot_r
                        } else if row > row_t - 4 {
                            cd.has_bli_top_r
                        } else {
                            false
                        };
                        die.add_xnode((col, row), "CLE_BC", &[(col, row), (col + 1, row)]);
                        if has_bli_r {
                            die.fill_term_pair(
                                (col, row),
                                (col + 1, row),
                                "CLE.BLI.E",
                                "CLE.BLI.W",
                            );
                        } else {
                            die.fill_term_pair((col, row), (col + 1, row), "CLE.E", "CLE.W");
                        }
                        let reg = grid.row_to_reg(row);
                        if row.to_idx() % 48 == 0 && grid.is_reg_top(reg) {
                            if reg.to_idx() % 2 == 1 {
                                die.add_xnode(
                                    (col + 1, row),
                                    "RCLK_CLE",
                                    &[(col + 1, row), (col + 1, row - 1)],
                                )
                            } else {
                                die.add_xnode((col + 1, row), "RCLK_CLE.HALF", &[(col + 1, row)])
                            };
                        }
                    }
                }
            }
        }
    }

    fn fill_intf(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
            let mut die = self.egrid.die_mut(dieid);

            let row_b = die.rows().next().unwrap();
            let row_t = die.rows().next_back().unwrap();
            for (col, &cd) in &grid.columns {
                for row in die.rows() {
                    if die[(col, row)].nodes.is_empty() {
                        continue;
                    }
                    if !matches!(cd.l, ColumnKind::Cle(_) | ColumnKind::None) {
                        let kind = match cd.l {
                            ColumnKind::Gt => "INTF.W.TERM.GT",
                            ColumnKind::Cfrm => {
                                if row.to_idx() < di.ps_height {
                                    "INTF.W.TERM.PSS"
                                } else {
                                    "INTF.W.PSS"
                                }
                            }
                            ColumnKind::Hard => {
                                let ch = grid.get_col_hard(col).unwrap();
                                match ch.regs[grid.row_to_reg(row)] {
                                    HardRowKind::Hdio => "INTF.W.HDIO",
                                    _ => "INTF.W.HB",
                                }
                            }
                            _ => "INTF.W",
                        };
                        die.add_xnode((col, row), kind, &[(col, row)]);
                    } else if matches!(cd.l, ColumnKind::Cle(_))
                        && cd.has_bli_bot_l
                        && row < row_b + 4
                    {
                        let idx = row - row_b;
                        die.add_xnode(
                            (col, row),
                            &format!("INTF.BLI_CLE.BOT.W.{idx}"),
                            &[(col, row)],
                        );
                    } else if matches!(cd.l, ColumnKind::Cle(_))
                        && cd.has_bli_top_l
                        && row > row_t - 4
                    {
                        let idx = row - (row_t - 3);
                        die.add_xnode(
                            (col, row),
                            &format!("INTF.BLI_CLE.TOP.W.{idx}"),
                            &[(col, row)],
                        );
                    }
                    if !matches!(cd.r, ColumnKind::Cle(_) | ColumnKind::None) {
                        let kind = match cd.r {
                            ColumnKind::Gt => "INTF.E.TERM.GT",
                            ColumnKind::Hard => {
                                let ch = grid.get_col_hard(col + 1).unwrap();
                                match ch.regs[grid.row_to_reg(row)] {
                                    HardRowKind::Hdio => "INTF.E.HDIO",
                                    _ => "INTF.E.HB",
                                }
                            }
                            _ => "INTF.E",
                        };
                        die.add_xnode((col, row), kind, &[(col, row)]);
                    } else if matches!(cd.r, ColumnKind::Cle(_))
                        && cd.has_bli_bot_r
                        && row < row_b + 4
                    {
                        let idx = row - row_b;
                        die.add_xnode(
                            (col, row),
                            &format!("INTF.BLI_CLE.BOT.E.{idx}"),
                            &[(col, row)],
                        );
                    } else if matches!(cd.r, ColumnKind::Cle(_))
                        && cd.has_bli_top_r
                        && row > row_t - 4
                    {
                        let idx = row - (row_t - 3);
                        die.add_xnode(
                            (col, row),
                            &format!("INTF.BLI_CLE.TOP.E.{idx}"),
                            &[(col, row)],
                        );
                    }
                    let reg = grid.row_to_reg(row);
                    if row.to_idx() % 48 == 0 && grid.is_reg_top(reg) {
                        if !matches!(cd.l, ColumnKind::Cle(_) | ColumnKind::None) {
                            if reg.to_idx() % 2 == 1 {
                                die.add_xnode(
                                    (col, row),
                                    "RCLK_INTF.W",
                                    &[(col, row), (col, row - 1)],
                                );
                            } else {
                                die.add_xnode((col, row), "RCLK_INTF.W.HALF", &[(col, row)]);
                            }
                            if matches!(
                                cd.l,
                                ColumnKind::Dsp | ColumnKind::Bram(_) | ColumnKind::Uram
                            ) {
                                die.add_xnode((col, row), "RCLK_DFX.W", &[(col, row)]);
                            }
                            if cd.l == ColumnKind::Hard {
                                let hc = grid.get_col_hard(col).unwrap();
                                if hc.regs[reg] == HardRowKind::Hdio {
                                    die.add_xnode((col, row), "RCLK_HDIO", &[]);
                                } else if reg.to_idx() % 2 != 0
                                    && hc.regs[reg - 1] == HardRowKind::Hdio
                                {
                                    die.add_xnode((col, row), "RCLK_HB_HDIO", &[]);
                                }
                            }
                        }
                        if !matches!(cd.r, ColumnKind::Cle(_) | ColumnKind::None)
                            && !(matches!(cd.r, ColumnKind::Gt)
                                && matches!(grid.right, RightKind::Cidb))
                        {
                            if reg.to_idx() % 2 == 1 {
                                die.add_xnode(
                                    (col, row),
                                    "RCLK_INTF.E",
                                    &[(col, row), (col, row - 1)],
                                )
                            } else {
                                die.add_xnode((col, row), "RCLK_INTF.E.HALF", &[(col, row)])
                            };
                            if matches!(cd.r, ColumnKind::Bram(_) | ColumnKind::Uram) {
                                die.add_xnode((col, row), "RCLK_DFX.E", &[(col, row)]);
                            }
                        }
                    }
                }
            }
        }
    }

    fn fill_cle(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &grid.columns {
                if !matches!(cd.r, ColumnKind::Cle(_)) {
                    continue;
                }
                for row in die.rows() {
                    if cd.has_bli_bot_r && row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_top_r && row.to_idx() >= die.rows().len() - 4 {
                        continue;
                    }
                    let tile = &mut die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    die.add_xnode((col, row), "CLE_R", &[(col, row)]);
                    die.add_xnode((col + 1, row), "CLE_L", &[(col + 1, row)]);
                }
            }
        }
    }

    fn fill_dsp(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &grid.columns {
                if cd.r != ColumnKind::Dsp {
                    continue;
                }
                for row in die.rows() {
                    if row.to_idx() % 2 != 0 {
                        continue;
                    }
                    if cd.has_bli_bot_r && row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_top_r && row.to_idx() >= die.rows().len() - 4 {
                        continue;
                    }
                    let tile = &mut die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    die.add_xnode(
                        (col, row),
                        "DSP",
                        &[
                            (col, row),
                            (col, row + 1),
                            (col + 1, row),
                            (col + 1, row + 1),
                        ],
                    );
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &grid.columns {
                for (kind, ck, has_bli_bot, has_bli_top) in [
                    ("BRAM_L", cd.l, cd.has_bli_bot_l, cd.has_bli_top_l),
                    ("BRAM_R", cd.r, cd.has_bli_bot_r, cd.has_bli_top_r),
                ] {
                    if !matches!(ck, ColumnKind::Bram(_)) {
                        continue;
                    }
                    for row in die.rows() {
                        if row.to_idx() % 4 != 0 {
                            continue;
                        }
                        if has_bli_bot && row.to_idx() < 4 {
                            continue;
                        }
                        if has_bli_top && row.to_idx() >= die.rows().len() - 4 {
                            continue;
                        }
                        let tile = &mut die[(col, row)];
                        if tile.nodes.is_empty() {
                            continue;
                        }
                        die.add_xnode(
                            (col, row),
                            kind,
                            &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                        );
                    }
                }
            }
        }
    }

    fn fill_uram(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &grid.columns {
                if cd.l != ColumnKind::Uram {
                    continue;
                }
                for row in die.rows() {
                    if row.to_idx() % 4 != 0 {
                        continue;
                    }
                    if cd.has_bli_bot_l && row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_top_l && row.to_idx() >= die.rows().len() - 4 {
                        continue;
                    }
                    if die[(col, row)].nodes.is_empty() {
                        continue;
                    }
                    let reg = grid.row_to_reg(row);
                    die.add_xnode(
                        (col, row),
                        if grid.is_reg_top(reg) && row.to_idx() % 48 == 44 {
                            "URAM_DELAY"
                        } else {
                            "URAM"
                        },
                        &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                    );
                }
            }
        }
    }

    fn fill_hard(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);
            for hc in &grid.cols_hard {
                for reg in grid.regs() {
                    if self
                        .disabled
                        .contains(&DisabledPart::HardIp(die.die, hc.col, reg))
                    {
                        continue;
                    }
                    if self.disabled.contains(&DisabledPart::Region(die.die, reg)) {
                        continue;
                    }
                    let kind = hc.regs[reg];
                    let (nk, is_high) = match kind {
                        HardRowKind::None => continue,
                        HardRowKind::DcmacT | HardRowKind::IlknT | HardRowKind::HscT => continue,
                        HardRowKind::Hdio => ("HDIO", false),
                        HardRowKind::CpmExt => {
                            // XXX
                            continue;
                        }
                        HardRowKind::Pcie4 => ("PCIE4", false),
                        HardRowKind::Pcie5 => ("PCIE5", false),
                        HardRowKind::Mrmac => ("MRMAC", false),
                        HardRowKind::DcmacB => ("DCMAC", true),
                        HardRowKind::IlknB => ("ILKN", true),
                        HardRowKind::HscB => ("HSC", true),
                    };
                    let row = grid.row_reg_bot(reg);
                    let mut crd = vec![];
                    let height = if is_high { 96 } else { 48 };
                    for i in 0..height {
                        crd.push((hc.col - 1, row + i));
                    }
                    for i in 0..height {
                        crd.push((hc.col, row + i));
                    }
                    die.add_xnode((hc.col, row), nk, &crd);
                }
            }
        }
    }

    fn fill_vnoc(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);
            for (col, cd) in &grid.columns {
                if !matches!(cd.l, ColumnKind::VNoc | ColumnKind::VNoc2) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Column(die.die, col)) {
                    continue;
                }
                for reg in grid.regs() {
                    if self.disabled.contains(&DisabledPart::Region(die.die, reg)) {
                        continue;
                    }
                    let row = grid.row_reg_bot(reg);
                    let mut crd = vec![];
                    for i in 0..48 {
                        crd.push((col - 1, row + i));
                    }
                    for i in 0..48 {
                        crd.push((col, row + i));
                    }
                    if cd.l == ColumnKind::VNoc {
                        die.add_xnode((col, row), "VNOC", &crd);
                    } else {
                        die.add_xnode((col, row), "VNOC2", &crd);
                    }
                    if grid.is_reg_top(reg) {
                        die.add_xnode((col, row), "MISR", &crd);
                    } else {
                        die.add_xnode((col, row), "SYSMON_SAT.VNOC", &crd);
                    }
                }
            }
        }
    }

    fn fill_clkroot(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);

            for col in die.cols() {
                for row in die.rows() {
                    let crow = RowId::from_idx(
                        if grid.regs % 2 == 1 && row.to_idx() >= (grid.regs - 1) * 48 {
                            row.to_idx() / 48 * 48
                        } else if row.to_idx() % 96 < 48 {
                            row.to_idx() / 96 * 96 + 47
                        } else {
                            row.to_idx() / 96 * 96 + 48
                        },
                    );
                    die[(col, row)].clkroot = (col, crow);
                }
            }
        }
    }
}

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &'a Grid>,
    interposer: &'a Interposer,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut expander = Expander {
        grids: grids.clone(),
        disabled: disabled.clone(),
        egrid: ExpandedGrid::new(db),
        die: EntityVec::new(),
    };
    expander.fill_die();
    expander.fill_int();
    expander.fill_cle_bc();
    expander.fill_intf();
    for dieid in expander.grids.ids() {
        expander.egrid.die_mut(dieid).fill_main_passes();
    }
    expander.fill_cle();
    expander.fill_dsp();
    expander.fill_bram();
    expander.fill_uram();
    expander.fill_hard();
    expander.fill_vnoc();
    expander.fill_clkroot();
    expander.egrid.finish();

    let col_cfrm = expander.die.map_values(|die| die.col_cfrm);

    ExpandedDevice {
        grids: expander.grids,
        interposer,
        egrid: expander.egrid,
        disabled: expander.disabled,
        col_cfrm,
    }
}

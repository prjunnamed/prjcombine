use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use std::collections::{BTreeSet, HashMap};
use unnamed_entity::{EntityBitVec, EntityId, EntityIds, EntityVec};

use crate::expanded::{ExpandedDevice, SllConns, UbumpId};
use crate::grid::{
    CleKind, ColumnKind, DisabledPart, Grid, GtRowKind, HardRowKind, Interposer, RightKind,
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
            self.egrid
                .add_die(grid.columns.len(), grid.regs * Grid::ROWS_PER_REG);
            self.die.push(DieInfo {
                col_cfrm: grid
                    .columns
                    .iter()
                    .find(|(_, cd)| cd.l == ColumnKind::Cfrm)
                    .unwrap()
                    .0,
                ps_height: grid.get_ps_height(),
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
                    if row.to_idx() % Grid::ROWS_PER_REG == 0 && grid.is_reg_top(reg) {
                        die.add_xnode((col, row), "RCLK", &[(col, row)]);
                    }
                }
            }

            if di.ps_height != grid.regs * Grid::ROWS_PER_REG {
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
                        let kind = match cd.r {
                            ColumnKind::Cle(CleKind::Plain) => "CLE_BC",
                            ColumnKind::Cle(CleKind::Sll) => {
                                if has_bli_r {
                                    "CLE_BC"
                                } else {
                                    "CLE_BC.SLL"
                                }
                            }
                            ColumnKind::Cle(CleKind::Sll2) => {
                                if has_bli_r {
                                    "CLE_BC"
                                } else {
                                    "CLE_BC.SLL2"
                                }
                            }
                            _ => unreachable!(),
                        };
                        die.add_xnode((col, row), kind, &[(col, row), (col + 1, row)]);
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
                        if row.to_idx() % Grid::ROWS_PER_REG == 0 && grid.is_reg_top(reg) {
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
                    if row.to_idx() % Grid::ROWS_PER_REG == 0 && grid.is_reg_top(reg) {
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
                    die.add_xnode(
                        (col, row),
                        if grid.is_vr { "CLE_R.VR" } else { "CLE_R" },
                        &[(col, row), (col + 1, row)],
                    );
                    die.add_xnode(
                        (col + 1, row),
                        if grid.is_vr { "CLE_L.VR" } else { "CLE_L" },
                        &[(col + 1, row)],
                    );
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
                        if grid.is_reg_top(reg) && row.to_idx() % Grid::ROWS_PER_REG == 44 {
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
                        HardRowKind::SdfecA => ("SDFEC", false),
                        HardRowKind::DfeCfcB => ("DFE_CFC_BOT", false),
                        HardRowKind::DfeCfcT => ("DFE_CFC_TOP", false),
                    };
                    let row = grid.row_reg_bot(reg);
                    let mut crd = vec![];
                    let height = if is_high {
                        Grid::ROWS_PER_REG * 2
                    } else {
                        Grid::ROWS_PER_REG
                    };
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
                if !matches!(
                    cd.l,
                    ColumnKind::VNoc | ColumnKind::VNoc2 | ColumnKind::VNoc4
                ) {
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
                    for i in 0..Grid::ROWS_PER_REG {
                        crd.push((col - 1, row + i));
                    }
                    for i in 0..Grid::ROWS_PER_REG {
                        crd.push((col, row + i));
                    }
                    match cd.l {
                        ColumnKind::VNoc => {
                            die.add_xnode((col, row), "VNOC", &crd);
                        }
                        ColumnKind::VNoc2 => {
                            die.add_xnode((col, row), "VNOC2", &crd);
                        }
                        ColumnKind::VNoc4 => {
                            die.add_xnode((col, row), "VNOC4", &crd);
                        }
                        _ => unreachable!(),
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

    fn fill_lgt(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);
            let col = die.cols().next().unwrap();
            let ps_height = grid.get_ps_height();
            for reg in grid.regs() {
                let row = grid.row_reg_bot(reg);
                if row.to_idx() < ps_height {
                    continue;
                }
                let crds: [_; Grid::ROWS_PER_REG] = core::array::from_fn(|dy| (col, row + dy));
                die.add_xnode(crds[0], "SYSMON_SAT.LGT", &crds);
                die.add_xnode(crds[0], "DPLL.LGT", &crds);
                // TODO: actual GT
            }
        }
    }

    fn fill_rgt(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);
            let col = die.cols().next_back().unwrap();
            match grid.right {
                RightKind::Gt(ref regs) => {
                    for (reg, &kind) in regs {
                        let row = grid.row_reg_bot(reg);
                        let crds: [_; Grid::ROWS_PER_REG] =
                            core::array::from_fn(|dy| (col, row + dy));
                        match kind {
                            GtRowKind::None => (),
                            GtRowKind::Gty => {
                                // TODO
                            }
                            GtRowKind::Gtyp => {
                                // TODO
                            }
                            GtRowKind::Gtm => {
                                // TODO
                            }
                            GtRowKind::RfAdc => {
                                // TODO
                            }
                            GtRowKind::RfDac => {
                                // TODO
                            }
                            GtRowKind::Xram => unreachable!(),
                            GtRowKind::Vdu => {
                                die.add_xnode(crds[0], "VDU.E", &crds);
                            }
                            GtRowKind::BfrB => {
                                die.add_xnode(crds[0], "BFR_B.E", &crds);
                            }
                            GtRowKind::Isp2 => {
                                // TODO
                            }
                            GtRowKind::Vcu2B => {
                                // TODO
                            }
                            GtRowKind::Vcu2T => {
                                // handled in bottom tile
                            }
                        }
                    }
                }
                RightKind::HNicX => {
                    // TODO
                }
                _ => continue,
            }
            for reg in grid.regs() {
                let row = grid.row_reg_bot(reg);
                let crds: [_; Grid::ROWS_PER_REG] = core::array::from_fn(|dy| (col, row + dy));
                die.add_xnode(crds[0], "SYSMON_SAT.RGT", &crds);
                die.add_xnode(crds[0], "DPLL.RGT", &crds);
            }
        }
    }

    fn fill_clkroot(&mut self) {
        for (dieid, grid) in &self.grids {
            let mut die = self.egrid.die_mut(dieid);

            for col in die.cols() {
                for row in die.rows() {
                    let reg = grid.row_to_reg(row);
                    let crow = if grid.is_reg_top(reg) {
                        grid.row_reg_hclk(reg)
                    } else {
                        grid.row_reg_hclk(reg) - 1
                    };
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
    expander.fill_lgt();
    expander.fill_rgt();
    expander.fill_clkroot();
    expander.egrid.finish();

    let col_cfrm = expander.die.map_values(|die| die.col_cfrm);

    let mut sll = HashMap::new();
    match interposer.kind {
        crate::grid::InterposerKind::Single => (),
        crate::grid::InterposerKind::Column => {
            fill_sll_column(interposer, grids, &mut sll);
        }
        crate::grid::InterposerKind::MirrorSquare => {
            fill_sll_mirror_square(interposer, grids, &mut sll);
        }
    }

    ExpandedDevice {
        grids: expander.grids,
        interposer,
        egrid: expander.egrid,
        disabled: expander.disabled,
        col_cfrm,
        sll,
    }
}

fn fill_sll_column(
    interposer: &Interposer,
    grids: &EntityVec<DieId, &Grid>,
    sll: &mut HashMap<(DieId, ColId, RowId), SllConns>,
) {
    let mut curse_queue = vec![];
    for (die, cols) in &interposer.sll_columns {
        let grid = grids[die];
        let all_rows = grid.rows();
        let has_link_bot = die != grids.first_id().unwrap();
        let has_link_top = die != grids.last_id().unwrap();
        for (cidx, &col) in cols.iter().enumerate() {
            let start = if grid.columns[col].has_bli_bot_l {
                assert!(!has_link_bot);
                4
            } else {
                0
            };
            let end = if grid.columns[col].has_bli_top_l {
                assert!(!has_link_top);
                all_rows.len() - 4
            } else {
                all_rows.len()
            };
            let rows: EntityIds<RowId> = EntityIds::new_range(start, end);
            for row in rows {
                let mut conns = SllConns {
                    conns: (0..6).map(|_| None).collect(),
                    cursed: EntityBitVec::repeat(false, 6),
                };
                if has_link_bot && row.to_idx() < start + 75 {
                    let odie = die - 1;
                    let orow = RowId::from_idx(grids[odie].rows().len() - 75 + row.to_idx());
                    let ocol = interposer.sll_columns[odie][cidx];
                    for (bump, obump) in [(0, 0), (1, 1), (3, 2), (4, 4), (5, 5)] {
                        let bump = UbumpId::from_idx(bump);
                        let obump = UbumpId::from_idx(obump);
                        conns.conns[bump] = Some((odie, ocol, orow, obump));
                    }
                    let bump = UbumpId::from_idx(2);
                    let obump = UbumpId::from_idx(3);
                    let orow = row + 75;
                    conns.conns[bump] = Some((die, col, orow, obump));
                } else if has_link_top && row.to_idx() >= end - 75 {
                    let odie = die + 1;
                    let orow = RowId::from_idx(row.to_idx() - (end - 75));
                    let ocol = interposer.sll_columns[odie][cidx];
                    for (bump, obump) in [(0, 0), (1, 1), (2, 3), (4, 4), (5, 5)] {
                        let bump = UbumpId::from_idx(bump);
                        let obump = UbumpId::from_idx(obump);
                        conns.conns[bump] = Some((odie, ocol, orow, obump));
                    }
                    let bump = UbumpId::from_idx(3);
                    let obump = UbumpId::from_idx(2);
                    let orow = row - 75;
                    conns.conns[bump] = Some((die, col, orow, obump));
                } else {
                    if row.to_idx() < start + 75 {
                        let bump = UbumpId::from_idx(3);
                        let triad = (row.to_idx() - start) / 3;
                        let sub = (row.to_idx() - start) % 3;
                        let orow = RowId::from_idx(start + (24 - triad) * 3 + sub);
                        if orow != row {
                            conns.conns[bump] = Some((die, col, orow, bump));
                        }
                    } else {
                        let bump = UbumpId::from_idx(3);
                        let obump = UbumpId::from_idx(2);
                        let orow = row - 75;
                        conns.conns[bump] = Some((die, col, orow, obump));
                    }
                    if row.to_idx() >= end - 75 {
                        let bump = UbumpId::from_idx(2);
                        let triad = (row.to_idx() - (end - 75)) / 3;
                        let sub = (row.to_idx() - (end - 75)) % 3;
                        let orow = RowId::from_idx(end - 75 + (24 - triad) * 3 + sub);
                        if orow != row {
                            conns.conns[bump] = Some((die, col, orow, bump));
                        }
                    } else {
                        let bump = UbumpId::from_idx(2);
                        let obump = UbumpId::from_idx(3);
                        let orow = row + 75;
                        conns.conns[bump] = Some((die, col, orow, obump));
                    }
                    if cidx < 10 {
                        for bump in [1, 5] {
                            let bump = UbumpId::from_idx(bump);
                            let ocol = cols[9 - cidx];
                            conns.conns[bump] = Some((die, ocol, row, bump));
                        }
                    } else {
                        for (bump, obump) in [(1, 0), (5, 4)] {
                            let bump = UbumpId::from_idx(bump);
                            let obump = UbumpId::from_idx(obump);
                            let ocol = cols[cidx - 10];
                            conns.conns[bump] = Some((die, ocol, row, obump));
                        }
                    }
                    if cidx >= cols.len() - 10 {
                        for bump in [0, 4] {
                            let bump = UbumpId::from_idx(bump);
                            let ocol = cols[cols.len() - 10 + (9 - (cidx - (cols.len() - 10)))];
                            conns.conns[bump] = Some((die, ocol, row, bump));
                        }
                    } else {
                        for (bump, obump) in [(0, 1), (4, 5)] {
                            let bump = UbumpId::from_idx(bump);
                            let obump = UbumpId::from_idx(obump);
                            let ocol = cols[cidx + 10];
                            conns.conns[bump] = Some((die, ocol, row, obump));
                        }
                    }
                }
                sll.insert((die, col, row), conns);
            }
            curse_queue.push((die, col, RowId::from_idx(start)));
            if has_link_top {
                curse_queue.push((die, col, RowId::from_idx(end - 75)));
            }
            if has_link_bot {
                curse_queue.push((die, col, RowId::from_idx(start + 75)));
            }
        }
    }
    for (die, col, row) in curse_queue {
        let conns = sll.get_mut(&(die, col, row)).unwrap();
        for mut val in conns.cursed.values_mut() {
            *val = true;
        }
        for (odie, ocol, orow, ubump) in conns.conns.clone().into_values().flatten() {
            let conns = sll.get_mut(&(odie, ocol, orow)).unwrap();
            conns.cursed.set(ubump, true);
        }
    }
}

fn fill_sll_mirror_square(
    interposer: &Interposer,
    grids: &EntityVec<DieId, &Grid>,
    sll: &mut HashMap<(DieId, ColId, RowId), SllConns>,
) {
    let mut curse_queue = vec![];
    for (die, cols) in &interposer.sll_columns {
        let grid = grids[die];
        let all_rows = grid.rows();
        let col_cfrm = grid
            .columns
            .iter()
            .find(|(_, c)| c.l == ColumnKind::Cfrm)
            .unwrap()
            .0;
        let ps_height = grid.get_ps_height();
        let cidx_ps = cols.binary_search(&col_cfrm).unwrap_err();
        for (cidx, &col) in cols.iter().enumerate() {
            let start = if col < col_cfrm {
                ps_height
            } else if grid.columns[col].has_bli_bot_l {
                4
            } else {
                0
            };
            let end = all_rows.len();
            let rows: EntityIds<RowId> = EntityIds::new_range(start, end);
            for row in rows.clone() {
                let cidx_l = if row.to_idx() < ps_height { cidx_ps } else { 0 };
                let mut conns = SllConns {
                    conns: (0..6).map(|_| None).collect(),
                    cursed: EntityBitVec::repeat(false, 6),
                };
                if row == RowId::from_idx(end - 63) {
                    // do nothing
                } else if cidx < cols.len() - 10 {
                    if row < RowId::from_idx(end - 63) {
                        if row.to_idx() < start + 75 {
                            let bump = UbumpId::from_idx(3);
                            let triad = (row.to_idx() - start) / 3;
                            let sub = (row.to_idx() - start) % 3;
                            let orow = RowId::from_idx(start + (24 - triad) * 3 + sub);
                            if orow != row {
                                conns.conns[bump] = Some((die, col, orow, bump));
                            }
                        } else {
                            let bump = UbumpId::from_idx(3);
                            let obump = UbumpId::from_idx(2);
                            let orow = row - 75;
                            conns.conns[bump] = Some((die, col, orow, obump));
                        }
                        if row.to_idx() >= end - 63 - 75 {
                            // nothing
                        } else {
                            let bump = UbumpId::from_idx(2);
                            let obump = UbumpId::from_idx(3);
                            let orow = row + 75;
                            conns.conns[bump] = Some((die, col, orow, obump));
                        }
                        if cidx < cidx_l + 10 {
                            for bump in [1, 5] {
                                let bump = UbumpId::from_idx(bump);
                                let ocol = cols[cidx_l + 9 - (cidx - cidx_l)];
                                conns.conns[bump] = Some((die, ocol, row, bump));
                            }
                        } else {
                            for (bump, obump) in [(1, 0), (5, 4)] {
                                let bump = UbumpId::from_idx(bump);
                                let obump = UbumpId::from_idx(obump);
                                let ocol = cols[cidx - 10];
                                conns.conns[bump] = Some((die, ocol, row, obump));
                            }
                        }
                        for (bump, obump) in [(0, 1), (4, 5)] {
                            let bump = UbumpId::from_idx(bump);
                            let obump = UbumpId::from_idx(obump);
                            let ocol = cols[cidx + 10];
                            conns.conns[bump] = Some((die, ocol, row, obump));
                        }
                    } else {
                        for bump in 0..6 {
                            let bump = UbumpId::from_idx(bump);
                            let odie = DieId::from_idx(die.to_idx() ^ 1);
                            let orow = RowId::from_idx(end - 62 + (end - 1 - row.to_idx()));
                            conns.conns[bump] = Some((odie, col, orow, bump));
                        }
                    }
                } else {
                    if !(46..50).contains(&(row.to_idx() % (Grid::ROWS_PER_REG * 2))) {
                        for bump in [0, 2, 4] {
                            let bump = UbumpId::from_idx(bump);
                            let odie = DieId::from_idx(die.to_idx() ^ 3);
                            let ocol = cols[cols.len() - 10 + (cols.len() - 1 - cidx)];
                            conns.conns[bump] = Some((odie, ocol, row, bump));
                        }
                    }
                    if row.to_idx() % (Grid::ROWS_PER_REG * 2) == 50 {
                        for bump in [0, 2, 4] {
                            let bump = UbumpId::from_idx(bump);
                            curse_queue.push((die, col, row, bump));
                        }
                    }
                    if row < RowId::from_idx(end - 63) {
                        for (bump, obump) in [(1, 0), (5, 4)] {
                            let bump = UbumpId::from_idx(bump);
                            let obump = UbumpId::from_idx(obump);
                            let ocol = cols[cidx - 10];
                            conns.conns[bump] = Some((die, ocol, row, obump));
                        }
                        if row.to_idx() % (Grid::ROWS_PER_REG * 2) == 50 {
                            for bump in [1, 5] {
                                let bump = UbumpId::from_idx(bump);
                                curse_queue.push((die, col, row, bump));
                            }
                        }
                    }
                }
                sll.insert((die, col, row), conns);
            }
            for bump in 0..6 {
                let bump = UbumpId::from_idx(bump);
                if cidx < cols.len() - 10 || bump.to_idx() != 3 {
                    curse_queue.push((die, col, RowId::from_idx(start), bump));
                }
                if cidx < cols.len() - 10 || matches!(bump.to_idx(), 0 | 2 | 4) {
                    curse_queue.push((die, col, RowId::from_idx(end - 63), bump));
                }
            }
            if cidx < 10 {
                let row = RowId::from_idx(ps_height);
                for bump in [1, 5] {
                    let bump = UbumpId::from_idx(bump);
                    curse_queue.push((die, col, row, bump));
                }
            }
        }
    }
    for (die, col, row, bump) in curse_queue {
        let conns = sll.get_mut(&(die, col, row)).unwrap();
        conns.cursed.set(bump, true);
        if let Some((odie, ocol, orow, obump)) = conns.conns[bump] {
            let conns = sll.get_mut(&(odie, ocol, orow)).unwrap();
            conns.cursed.set(obump, true);
        }
    }
}

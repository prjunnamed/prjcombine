use enum_map::EnumMap;
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use std::collections::BTreeSet;
use unnamed_entity::{entity_id, EntityId, EntityPartVec, EntityVec};

use crate::expanded::ExpandedDevice;
use crate::grid::{ColSide, ColumnKind, CpmKind, DisabledPart, Grid, HardRowKind, PsKind, RegId};
use crate::naming::{DeviceNaming, DieNaming};

entity_id! {
    id EColId u32, delta;
}

pub const BUFDIV_LEAF_SWZ_A: [u32; 32] = [
    3, 2, 1, 0, 8, 9, 10, 11, 19, 18, 17, 16, 24, 25, 26, 27, 4, 5, 6, 7, 15, 14, 13, 12, 20, 21,
    22, 23, 31, 30, 29, 28,
];

pub const BUFDIV_LEAF_SWZ_B: [u32; 32] = [
    7, 6, 5, 4, 12, 13, 14, 15, 23, 22, 21, 20, 28, 29, 30, 31, 0, 1, 2, 3, 11, 10, 9, 8, 16, 17,
    18, 19, 27, 26, 25, 24,
];

pub const BUFDIV_LEAF_SWZ_AH: [u32; 32] = [
    35, 34, 33, 32, 40, 41, 42, 43, 51, 50, 49, 48, 56, 57, 58, 59, 36, 37, 38, 39, 47, 46, 45, 44,
    52, 53, 54, 55, 63, 62, 61, 60,
];

pub const BUFDIV_LEAF_SWZ_BH: [u32; 32] = [
    39, 38, 37, 36, 44, 45, 46, 47, 55, 54, 53, 52, 60, 61, 62, 63, 32, 33, 34, 35, 43, 42, 41, 40,
    48, 49, 50, 51, 59, 58, 57, 56,
];

struct DieInfo<'a> {
    ecol2col: EntityPartVec<EColId, ColId>,
    col2ecol: EntityVec<ColId, EColId>,
    xlut: EntityVec<ColId, u32>,
    ylut: EntityVec<RowId, u32>,
    cleylut: EntityPartVec<RowId, u32>,
    dspylut: EntityPartVec<RowId, u32>,
    bramylut: EntityPartVec<RowId, u32>,
    uramylut: EntityPartVec<RowId, u32>,
    uramdylut: EntityPartVec<RowId, u32>,
    hardylut: EnumMap<HardRowKind, EntityPartVec<RegId, u32>>,
    irixlut: EnumMap<ColSide, EntityPartVec<ColId, u32>>,
    iriylut: EntityVec<RowId, u32>,
    rclkxlut: EnumMap<ColSide, EntityPartVec<ColId, u32>>,
    rclkylut: EntityPartVec<RegId, u32>,
    dfxylut: EntityPartVec<RegId, u32>,
    vnocylut: EntityPartVec<RegId, u32>,
    naming: &'a DieNaming,
    col_cfrm: ColId,
    ps_height: usize,
}

struct Expander<'a> {
    db: &'a IntDb,
    grids: EntityVec<DieId, &'a Grid>,
    disabled: BTreeSet<DisabledPart>,
    naming: &'a DeviceNaming,
    egrid: ExpandedGrid<'a>,
    die: EntityVec<DieId, DieInfo<'a>>,
    ecol_cfrm: EColId,
    ecols: EntityVec<EColId, ()>,
    clexlut: EntityPartVec<EColId, u32>,
    dspxlut: EntityPartVec<EColId, u32>,
    bramxlut: EnumMap<ColSide, EntityPartVec<EColId, u32>>,
    uramxlut: EntityPartVec<EColId, u32>,
    hardxlut: EnumMap<HardRowKind, EntityPartVec<EColId, u32>>,
    dfxxlut: EnumMap<ColSide, EntityPartVec<EColId, u32>>,
    vnocxlut: EntityPartVec<EColId, u32>,
}

impl Expander<'_> {
    fn fill_die(&mut self) {
        for (dieid, &grid) in &self.grids {
            self.egrid.add_die(grid.columns.len(), grid.regs * 48);
            let ps_height = match (grid.ps, grid.cpm) {
                (PsKind::Ps9, CpmKind::None) => 48 * 2,
                (PsKind::Ps9, CpmKind::Cpm4) => 48 * 3,
                (PsKind::Ps9, CpmKind::Cpm5) => 48 * 6,
                (PsKind::PsX, CpmKind::Cpm5N) => 48 * 9,
                _ => unreachable!(),
            };
            self.die.push(DieInfo {
                naming: &self.naming.die[dieid],
                ecol2col: Default::default(),
                col2ecol: Default::default(),
                xlut: Default::default(),
                ylut: Default::default(),
                cleylut: Default::default(),
                dspylut: Default::default(),
                bramylut: Default::default(),
                uramylut: Default::default(),
                uramdylut: Default::default(),
                hardylut: Default::default(),
                irixlut: Default::default(),
                iriylut: Default::default(),
                rclkxlut: Default::default(),
                rclkylut: Default::default(),
                dfxylut: Default::default(),
                vnocylut: Default::default(),
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

    fn fill_ecol(&mut self) {
        self.ecol_cfrm = EColId::from_idx(
            self.die
                .values()
                .map(|x| x.col_cfrm.to_idx())
                .max()
                .unwrap(),
        );
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            let mut ecol = EColId::from_idx(0);
            for col in grid.columns.ids() {
                if col == di.col_cfrm {
                    ecol = self.ecol_cfrm;
                }
                di.col2ecol.push(ecol);
                di.ecol2col.insert(ecol, col);
                di.xlut.push(ecol.to_idx() as u32);
                ecol += 1;
            }
            while self.ecols.len() < ecol.to_idx() {
                self.ecols.push(());
            }
        }
    }

    fn fill_ylut(&mut self) {
        let mut y = 0;
        for (dieid, di) in &mut self.die {
            let die = self.egrid.die(dieid);
            for _ in die.rows() {
                di.ylut.push(y);
                y += 1;
            }
        }
    }

    fn fill_clexlut(&mut self) {
        let mut clex = 0;
        for ecol in self.ecols.ids() {
            let mut has_cle = false;
            for (dieid, grid) in &self.grids {
                let di = &self.die[dieid];
                if let Some(&col) = di.ecol2col.get(ecol) {
                    if matches!(grid.columns[col].r, ColumnKind::Cle | ColumnKind::CleLaguna) {
                        has_cle = true;
                    }
                }
            }
            if has_cle {
                self.clexlut.insert(ecol, clex);
                clex += 1;
            }
        }
    }

    fn fill_cleylut(&mut self) {
        let mut cley = 0;
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            let die = self.egrid.die(dieid);
            let has_cle_bot = grid.columns.iter().any(|(col, cd)| {
                col >= di.col_cfrm
                    && matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna)
                    && !cd.has_bli_bot_r
            });
            let has_cle_top = grid.columns.values().any(|cd| {
                matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna) && !cd.has_bli_top_r
            });
            for row in die.rows() {
                if row.to_idx() < 4 && !has_cle_bot {
                    continue;
                }
                if row.to_idx() >= die.rows().len() - 4 && !has_cle_top {
                    continue;
                }
                di.cleylut.insert(row, cley);
                cley += 1;
            }
        }
    }

    fn fill_dspxlut(&mut self) {
        let mut dspx = 0;
        for ecol in self.ecols.ids() {
            let mut has_dsp = false;
            for (dieid, grid) in &self.grids {
                let di = &self.die[dieid];
                if let Some(&col) = di.ecol2col.get(ecol) {
                    if grid.columns[col].r == ColumnKind::Dsp {
                        has_dsp = true;
                    }
                }
            }
            if has_dsp {
                self.dspxlut.insert(ecol, dspx);
                dspx += 1;
            }
        }
    }

    fn fill_dspylut(&mut self) {
        let mut dspy = 0;
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            let die = self.egrid.die(dieid);
            let has_dsp_bot = grid.columns.iter().any(|(col, cd)| {
                col >= di.col_cfrm && cd.r == ColumnKind::Dsp && !cd.has_bli_bot_r
            });
            let has_dsp_top = grid
                .columns
                .values()
                .any(|cd| cd.r == ColumnKind::Dsp && !cd.has_bli_top_r);
            for row in die.rows() {
                if row.to_idx() % 2 != 0 {
                    continue;
                }
                if row.to_idx() < 4 && !has_dsp_bot {
                    continue;
                }
                if row.to_idx() >= die.rows().len() - 4 && !has_dsp_top {
                    continue;
                }
                di.dspylut.insert(row, dspy);
                dspy += 1;
            }
        }
    }

    fn fill_bramxlut(&mut self) {
        let mut bramx = 0;
        for ecol in self.ecols.ids() {
            let mut has_bram_l = false;
            let mut has_bram_r = false;
            for (dieid, grid) in &self.grids {
                let di = &self.die[dieid];
                if let Some(&col) = di.ecol2col.get(ecol) {
                    if matches!(
                        grid.columns[col].l,
                        ColumnKind::Bram | ColumnKind::BramClkBuf
                    ) {
                        has_bram_l = true;
                    }
                    if matches!(
                        grid.columns[col].r,
                        ColumnKind::Bram | ColumnKind::BramClkBuf
                    ) {
                        has_bram_r = true;
                    }
                }
            }
            if has_bram_l {
                self.bramxlut[ColSide::Left].insert(ecol, bramx);
                bramx += 1;
            }
            if has_bram_r {
                self.bramxlut[ColSide::Right].insert(ecol, bramx);
                bramx += 1;
            }
        }
    }

    fn fill_bramylut(&mut self) {
        let mut bramy = 0;
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            let die = self.egrid.die(dieid);
            let has_bram_bot = grid.columns.iter().any(|(col, cd)| {
                col >= di.col_cfrm
                    && ((matches!(cd.l, ColumnKind::Bram | ColumnKind::BramClkBuf)
                        && !cd.has_bli_bot_l)
                        || (matches!(cd.r, ColumnKind::Bram | ColumnKind::BramClkBuf)
                            && !cd.has_bli_bot_r))
            });
            let has_bram_top = grid.columns.values().any(|cd| {
                (matches!(cd.l, ColumnKind::Bram | ColumnKind::BramClkBuf) && !cd.has_bli_top_l)
                    || (matches!(cd.r, ColumnKind::Bram | ColumnKind::BramClkBuf)
                        && !cd.has_bli_top_r)
            });
            for row in die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                if row.to_idx() < 4 && !has_bram_bot {
                    continue;
                }
                if row.to_idx() >= die.rows().len() - 4 && !has_bram_top {
                    continue;
                }
                di.bramylut.insert(row, bramy);
                bramy += 1;
            }
        }
    }

    fn fill_uramxlut(&mut self) {
        let mut uramx = 0;
        for ecol in self.ecols.ids() {
            let mut has_uram = false;
            for (dieid, grid) in &self.grids {
                let di = &self.die[dieid];
                if let Some(&col) = di.ecol2col.get(ecol) {
                    if grid.columns[col].l == ColumnKind::Uram {
                        has_uram = true;
                    }
                }
            }
            if has_uram {
                self.uramxlut.insert(ecol, uramx);
                uramx += 1;
            }
        }
    }

    fn fill_uramylut(&mut self) {
        let mut uramy = 0;
        let mut uramdy = 0;
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            let die = self.egrid.die(dieid);
            let has_uram_bot = grid.columns.iter().any(|(col, cd)| {
                col >= di.col_cfrm && cd.l == ColumnKind::Uram && !cd.has_bli_bot_l
            });
            let has_uram_top = grid
                .columns
                .values()
                .any(|cd| cd.l == ColumnKind::Uram && !cd.has_bli_top_l);
            for row in die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                if row.to_idx() < 4 && !has_uram_bot {
                    continue;
                }
                if row.to_idx() >= die.rows().len() - 4 && !has_uram_top {
                    continue;
                }
                di.uramylut.insert(row, uramy);
                uramy += 1;
                let reg = grid.row_to_reg(row);
                if grid.is_reg_top(reg) && row.to_idx() % 48 == 44 {
                    di.uramdylut.insert(row, uramdy);
                    uramdy += 1;
                }
            }
        }
    }

    fn fill_hardxlut(&mut self) {
        let mut hardx = EnumMap::default();
        for ecol in self.ecols.ids() {
            let mut has_hard = EnumMap::default();
            for (dieid, grid) in &self.grids {
                let di = &self.die[dieid];
                if let Some(&col) = di.ecol2col.get(ecol) {
                    if grid.columns[col].l == ColumnKind::Hard {
                        let hc = grid.get_col_hard(col).unwrap();
                        for &t in hc.regs.values() {
                            has_hard[t] = true;
                        }
                    }
                }
            }
            for (k, v) in has_hard {
                if v {
                    self.hardxlut[k].insert(ecol, hardx[k]);
                    hardx[k] += 1;
                }
            }
        }
    }

    fn fill_hardylut(&mut self) {
        let mut hardy = EnumMap::default();
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            for reg in grid.regs() {
                let mut has_hard = EnumMap::default();
                for hc in &grid.cols_hard {
                    has_hard[hc.regs[reg]] = true;
                }
                for (k, v) in has_hard {
                    if v {
                        di.hardylut[k].insert(reg, hardy[k]);
                        hardy[k] += 1;
                    }
                }
            }
        }
    }

    fn fill_irixlut(&mut self) {
        let mut irix = self.die.map_values(|_| 0);
        for ecol in self.ecols.ids() {
            if ecol == self.ecol_cfrm {
                let irix_max = irix.values().copied().max().unwrap();
                irix = irix.map_values(|_| irix_max);
            }
            for (dieid, grid) in &self.grids {
                let di = &mut self.die[dieid];
                if let Some(&col) = di.ecol2col.get(ecol) {
                    let cd = &grid.columns[col];
                    let mut has_iri_l = false;
                    let mut has_iri_r = false;
                    if matches!(cd.l, ColumnKind::Cle | ColumnKind::CleLaguna) {
                        if cd.has_bli_bot_l || cd.has_bli_top_l {
                            has_iri_l = true;
                        }
                    } else if cd.l != ColumnKind::None {
                        has_iri_l = true;
                    }
                    if matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna) {
                        if cd.has_bli_bot_r || cd.has_bli_top_r {
                            has_iri_r = true;
                        }
                    } else if cd.r != ColumnKind::None {
                        has_iri_r = true;
                    }
                    if has_iri_l {
                        di.irixlut[ColSide::Left].insert(col, irix[dieid]);
                        irix[dieid] += 1;
                    }
                    if has_iri_r {
                        di.irixlut[ColSide::Right].insert(col, irix[dieid]);
                        irix[dieid] += 1;
                    }
                }
            }
        }
    }

    fn fill_iriylut(&mut self) {
        let mut iriy = 0;
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            let die = self.egrid.die(dieid);
            let has_bli_bot = grid
                .columns
                .values()
                .any(|cd| cd.has_bli_bot_r || cd.has_bli_bot_l);
            let has_bli_top = grid
                .columns
                .values()
                .any(|cd| cd.has_bli_top_r || cd.has_bli_top_l);
            for row in die.rows() {
                di.iriylut.push(iriy);
                if (row.to_idx() == 0 && has_bli_bot)
                    || (row.to_idx() == die.rows().len() - 4 && has_bli_top)
                {
                    iriy += 16;
                } else {
                    iriy += 4;
                }
            }
        }
    }

    fn fill_rclkxlut(&mut self) {
        let mut rclkx = self.die.map_values(|_| 0);
        let mut dfxx = 0;
        for ecol in self.ecols.ids() {
            if ecol == self.ecol_cfrm {
                let rclkx_max = rclkx.values().copied().max().unwrap();
                rclkx = rclkx.map_values(|_| rclkx_max);
            }
            let mut has_dfx_l = false;
            let mut has_dfx_r = false;
            for (dieid, grid) in &self.grids {
                let di = &mut self.die[dieid];
                if let Some(&col) = di.ecol2col.get(ecol) {
                    let mut has_rclk_l = false;
                    let mut has_rclk_r = false;
                    match grid.columns[col].l {
                        ColumnKind::Cle
                        | ColumnKind::CleLaguna
                        | ColumnKind::Gt
                        | ColumnKind::Cfrm
                        | ColumnKind::VNoc
                        | ColumnKind::VNoc2 => {
                            has_rclk_l = true;
                        }
                        ColumnKind::Bram | ColumnKind::BramClkBuf | ColumnKind::Uram => {
                            has_rclk_l = true;
                            has_dfx_l = true;
                        }
                        ColumnKind::Dsp => {
                            has_dfx_l = true;
                        }
                        ColumnKind::Hard => {
                            let hc = grid.get_col_hard(col).unwrap();
                            if hc.regs.values().any(|&x| {
                                matches!(
                                    x,
                                    HardRowKind::DcmacB | HardRowKind::HscB | HardRowKind::IlknB
                                )
                            }) {
                                has_rclk_l = true;
                            }
                        }
                        ColumnKind::None => (),
                    }
                    match grid.columns[col].r {
                        ColumnKind::Hard
                        | ColumnKind::Gt
                        | ColumnKind::Cfrm
                        | ColumnKind::VNoc
                        | ColumnKind::VNoc2
                        | ColumnKind::Dsp => {
                            has_rclk_r = true;
                        }
                        ColumnKind::Bram | ColumnKind::BramClkBuf | ColumnKind::Uram => {
                            has_rclk_r = true;
                            has_dfx_r = true;
                        }
                        ColumnKind::Cle | ColumnKind::CleLaguna | ColumnKind::None => (),
                    }
                    if has_rclk_l {
                        di.rclkxlut[ColSide::Left].insert(col, rclkx[dieid]);
                        rclkx[dieid] += 1;
                    }
                    if has_rclk_r {
                        di.rclkxlut[ColSide::Right].insert(col, rclkx[dieid]);
                        rclkx[dieid] += 1;
                    }
                }
            }
            if has_dfx_l {
                self.dfxxlut[ColSide::Left].insert(ecol, dfxx);
                dfxx += 1;
            }
            if has_dfx_r {
                self.dfxxlut[ColSide::Right].insert(ecol, dfxx);
                dfxx += 1;
            }
        }
    }

    fn fill_rclkylut(&mut self) {
        let mut rclky = 0;
        let mut dfxy = 0;
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            let has_dsp = grid.columns.values().any(|cd| cd.l == ColumnKind::Dsp);
            for reg in grid.regs() {
                if grid.is_reg_top(reg) {
                    di.rclkylut.insert(reg, rclky);
                    di.dfxylut.insert(reg, dfxy);
                    if has_dsp {
                        rclky += 64;
                    } else {
                        rclky += 32;
                    }
                    dfxy += 1;
                }
            }
        }
    }

    fn fill_vnocxlut(&mut self) {
        let mut vnocx = 0;
        for ecol in self.ecols.ids() {
            let mut has_vnoc = false;
            for (dieid, grid) in &self.grids {
                let di = &mut self.die[dieid];
                if let Some(&col) = di.ecol2col.get(ecol) {
                    match grid.columns[col].l {
                        ColumnKind::VNoc | ColumnKind::VNoc2 => has_vnoc = true,
                        _ => (),
                    }
                }
            }
            if has_vnoc {
                self.vnocxlut.insert(ecol, vnocx);
                vnocx += 1;
            }
        }
    }

    fn fill_vnocylut(&mut self) {
        let mut vnocy = 0;
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            for reg in grid.regs() {
                di.vnocylut.insert(reg, vnocy);
                vnocy += 1;
            }
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
                let x = di.xlut[col];
                for row in die.rows() {
                    let reg = grid.row_to_reg(row);
                    if self.disabled.contains(&DisabledPart::Region(dieid, reg)) {
                        continue;
                    }
                    if col < di.col_cfrm && row.to_idx() < di.ps_height {
                        continue;
                    }
                    let y = di.ylut[row];
                    die.fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                    if row.to_idx() % 48 == 0 && grid.is_reg_top(reg) {
                        let lr = if col < di.col_cfrm { 'L' } else { 'R' };
                        let yy = if reg.to_idx() % 2 == 1 { y - 1 } else { y };
                        let name = format!("RCLK_INT_{lr}_FT_X{x}Y{yy}");
                        die.add_xnode(
                            (col, row),
                            self.db.get_node("RCLK"),
                            &[&name],
                            self.db.get_node_naming("RCLK"),
                            &[(col, row)],
                        );
                    }
                }
            }

            if di.ps_height != grid.regs * 48 {
                let row_t = RowId::from_idx(di.ps_height);
                for dx in 0..ps_width {
                    let col = ColId::from_idx(dx);
                    die.fill_term_anon((col, row_t), "TERM.S");
                }
            }
            for dy in 0..di.ps_height {
                let row = RowId::from_idx(dy);
                die.fill_term_anon((di.col_cfrm, row), "TERM.W");
            }

            for col in die.cols() {
                if !die[(col, row_b)].nodes.is_empty() {
                    die.fill_term_anon((col, row_b), "TERM.S");
                }
                if !die[(col, row_t)].nodes.is_empty() {
                    die.fill_term_anon((col, row_t), "TERM.N");
                }
            }
            for row in die.rows() {
                if !die[(col_l, row)].nodes.is_empty() {
                    die.fill_term_anon((col_l, row), "TERM.W");
                }
                if !die[(col_r, row)].nodes.is_empty() {
                    die.fill_term_anon((col_r, row), "TERM.E");
                }
            }
        }
    }

    fn fill_cle_bc(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
            let mut die = self.egrid.die_mut(dieid);

            let row_b = die.rows().next().unwrap();
            let row_t = die.rows().next_back().unwrap();
            let cle_e = self.db.get_term("CLE.E");
            let cle_w = self.db.get_term("CLE.W");
            let cle_bli_e = self.db.get_term("CLE.BLI.E");
            let cle_bli_w = self.db.get_term("CLE.BLI.W");
            let mut cle_x_bump_prev = false;
            for (col, &cd) in &grid.columns {
                let x = di.xlut[col];
                let cle_x_bump_cur;
                if matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna)
                    && col >= di.col_cfrm
                    && grid.cols_vbrk.contains(&(col + 1))
                {
                    cle_x_bump_cur = true;
                    cle_x_bump_prev = false;
                } else {
                    cle_x_bump_cur = false;
                }
                if matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna) {
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
                        let tk = if (cd.r == ColumnKind::CleLaguna) && !has_bli_r {
                            "SLL"
                        } else {
                            "CLE_BC_CORE"
                        };
                        let y = di.ylut[row];
                        let tile;
                        if cle_x_bump_cur {
                            tile = format!("{tk}_X{xx}Y{y}", xx = x + 1);
                        } else if cle_x_bump_prev {
                            tile = format!("{tk}_1_X{x}Y{y}");
                        } else {
                            tile = format!("{tk}_X{x}Y{y}");
                        }
                        die.add_xnode(
                            (col, row),
                            self.db.get_node("CLE_BC"),
                            &[&tile],
                            self.db.get_node_naming("CLE_BC"),
                            &[(col, row), (col + 1, row)],
                        );
                        if has_bli_r {
                            die.fill_term_pair_anon(
                                (col, row),
                                (col + 1, row),
                                cle_bli_e,
                                cle_bli_w,
                            );
                        } else {
                            die.fill_term_pair_anon((col, row), (col + 1, row), cle_e, cle_w);
                        }
                        let reg = grid.row_to_reg(row);
                        if row.to_idx() % 48 == 0 && grid.is_reg_top(reg) {
                            let is_lag = cd.r == ColumnKind::CleLaguna;
                            let yy = if reg.to_idx() % 2 == 1 { y - 1 } else { y };
                            let kind = if is_lag {
                                "RCLK_CLE_LAG_CORE"
                            } else {
                                "RCLK_CLE_CORE"
                            };
                            let name = format!("{kind}_X{x}Y{yy}");
                            let node = if reg.to_idx() % 2 == 1 {
                                die.add_xnode(
                                    (col + 1, row),
                                    self.db.get_node("RCLK_CLE"),
                                    &[&name],
                                    self.db.get_node_naming(if is_lag {
                                        "RCLK_CLE.LAG"
                                    } else {
                                        "RCLK_CLE"
                                    }),
                                    &[(col + 1, row), (col + 1, row - 1)],
                                )
                            } else {
                                die.add_xnode(
                                    (col + 1, row),
                                    self.db.get_node("RCLK_CLE.HALF"),
                                    &[&name],
                                    self.db.get_node_naming(if is_lag {
                                        "RCLK_CLE.HALF.LAG"
                                    } else {
                                        "RCLK_CLE.HALF"
                                    }),
                                    &[(col + 1, row)],
                                )
                            };
                            let swz = if is_lag {
                                BUFDIV_LEAF_SWZ_B
                            } else {
                                BUFDIV_LEAF_SWZ_A
                            };
                            let sx = di.rclkxlut[ColSide::Left][col + 1];
                            let sy = di.rclkylut[reg];
                            for (i, dy) in swz.into_iter().enumerate() {
                                node.add_bel(i, format!("BUFDIV_LEAF_X{sx}Y{sy}", sy = sy + dy));
                            }
                        }
                    }
                }
                cle_x_bump_prev = cle_x_bump_cur;
            }
        }
    }

    fn fill_intf(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &grid.columns {
                let x = di.xlut[col];
                for row in die.rows() {
                    let reg = grid.row_to_reg(row);
                    if die[(col, row)].nodes.is_empty() {
                        continue;
                    }
                    let y = di.ylut[row];
                    let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                    let ocf = if col < di.col_cfrm { "LOCF" } else { "ROCF" };
                    if !matches!(
                        cd.l,
                        ColumnKind::Cle | ColumnKind::CleLaguna | ColumnKind::None
                    ) {
                        let kind;
                        let tile;
                        match cd.l {
                            ColumnKind::Gt => {
                                kind = "INTF.W.TERM.GT";
                                tile = format!("INTF_GT_{bt}L_TILE_X{x}Y{y}");
                            }
                            ColumnKind::Cfrm => {
                                if row.to_idx() < di.ps_height {
                                    kind = "INTF.W.TERM.PSS";
                                    tile = format!("INTF_PSS_{bt}L_TILE_X{x}Y{y}");
                                } else {
                                    kind = "INTF.W.PSS";
                                    tile = format!("INTF_CFRM_{bt}L_TILE_X{x}Y{y}");
                                }
                            }
                            ColumnKind::Hard => {
                                let ch = grid.get_col_hard(col).unwrap();
                                match ch.regs[grid.row_to_reg(row)] {
                                    HardRowKind::Hdio => {
                                        kind = "INTF.W.HDIO";
                                        tile = format!("INTF_HDIO_{ocf}_{bt}L_TILE_X{x}Y{y}");
                                    }
                                    _ => {
                                        kind = "INTF.W.HB";
                                        tile = format!("INTF_HB_{ocf}_{bt}L_TILE_X{x}Y{y}");
                                    }
                                }
                            }
                            _ => {
                                kind = "INTF.W";
                                tile = format!("INTF_{ocf}_{bt}L_TILE_X{x}Y{y}");
                            }
                        }
                        let node = die.add_xnode(
                            (col, row),
                            self.db.get_node(kind),
                            &[&tile],
                            self.db.get_node_naming(kind),
                            &[(col, row)],
                        );
                        for i in 0..4 {
                            node.iri_names.push(format!(
                                "IRI_QUAD_X{ix}Y{iy}",
                                ix = di.irixlut[ColSide::Left][col],
                                iy = di.iriylut[row] + i
                            ));
                        }
                    }
                    if !matches!(
                        cd.r,
                        ColumnKind::Cle | ColumnKind::CleLaguna | ColumnKind::None
                    ) {
                        let kind;
                        let tile;
                        match cd.r {
                            ColumnKind::Gt => {
                                kind = "INTF.E.TERM.GT";
                                tile = format!("INTF_GT_{bt}R_TILE_X{x}Y{y}");
                            }
                            ColumnKind::Hard => {
                                let ch = grid.get_col_hard(col + 1).unwrap();
                                match ch.regs[grid.row_to_reg(row)] {
                                    HardRowKind::Hdio => {
                                        kind = "INTF.E.HDIO";
                                        tile = format!("INTF_HDIO_{ocf}_{bt}R_TILE_X{x}Y{y}");
                                    }
                                    _ => {
                                        kind = "INTF.E.HB";
                                        tile = format!("INTF_HB_{ocf}_{bt}R_TILE_X{x}Y{y}");
                                    }
                                }
                            }
                            _ => {
                                kind = "INTF.E";
                                tile = format!("INTF_{ocf}_{bt}R_TILE_X{x}Y{y}");
                            }
                        }
                        let node = die.add_xnode(
                            (col, row),
                            self.db.get_node(kind),
                            &[&tile],
                            self.db.get_node_naming(kind),
                            &[(col, row)],
                        );
                        for i in 0..4 {
                            node.iri_names.push(format!(
                                "IRI_QUAD_X{ix}Y{iy}",
                                ix = di.irixlut[ColSide::Right][col],
                                iy = di.iriylut[row] + i
                            ));
                        }
                    }
                    let reg = grid.row_to_reg(row);
                    if row.to_idx() % 48 == 0 && grid.is_reg_top(reg) {
                        let yy = if reg.to_idx() % 2 == 1 { y - 1 } else { y };
                        if !matches!(
                            cd.l,
                            ColumnKind::Cle | ColumnKind::CleLaguna | ColumnKind::None
                        ) {
                            let kind;
                            let tile;
                            let swz;
                            let mut has_dfx = false;
                            let mut is_rclk_hdio = false;
                            let mut is_rclk_hb_hdio = false;
                            let wide;
                            match cd.l {
                                ColumnKind::Dsp => {
                                    kind = "DSP";
                                    tile = format!("RCLK_DSP_CORE_X{x}Y{yy}", x = x - 1);
                                    swz = BUFDIV_LEAF_SWZ_AH;
                                    has_dfx = true;
                                    wide = true;
                                }
                                ColumnKind::Bram => {
                                    kind = "BRAM";
                                    tile = format!("RCLK_BRAM_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_A;
                                    has_dfx = true;
                                    wide = false;
                                }
                                ColumnKind::Uram => {
                                    kind = "URAM";
                                    tile = format!("RCLK_URAM_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_A;
                                    has_dfx = true;
                                    wide = false;
                                }
                                ColumnKind::Gt => {
                                    kind = "GT";
                                    tile = format!("RCLK_INTF_TERM_LEFT_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_A;
                                    wide = false;
                                }
                                ColumnKind::Cfrm => {
                                    kind = "CFRM";
                                    tile = format!("RCLK_INTF_OPT_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_A;
                                    wide = false;
                                }
                                ColumnKind::VNoc | ColumnKind::VNoc2 => {
                                    kind = "VNOC";
                                    tile = format!("RCLK_INTF_L_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_B;
                                    wide = false;
                                }
                                ColumnKind::Hard => {
                                    let hc = grid.get_col_hard(col).unwrap();
                                    if reg.to_idx() % 2 == 0 {
                                        if hc.regs[reg] == HardRowKind::Hdio {
                                            kind = "HDIO";
                                            tile = format!("RCLK_HDIO_CORE_X{x}Y{yy}", x = x - 1);
                                            swz = BUFDIV_LEAF_SWZ_AH;
                                            wide = true;
                                            is_rclk_hdio = true;
                                        } else {
                                            kind = "HB";
                                            tile = format!("RCLK_HB_CORE_X{x}Y{yy}", x = x - 1);
                                            swz = BUFDIV_LEAF_SWZ_AH;
                                            wide = true;
                                        }
                                    } else {
                                        if hc.regs[reg] == HardRowKind::Hdio {
                                            kind = "HDIO";
                                            tile = format!("RCLK_HDIO_CORE_X{x}Y{yy}", x = x - 1);
                                            swz = BUFDIV_LEAF_SWZ_AH;
                                            wide = true;
                                            is_rclk_hdio = true;
                                        } else if hc.regs[reg - 1] == HardRowKind::Hdio {
                                            kind = "HB_HDIO";
                                            tile =
                                                format!("RCLK_HB_HDIO_CORE_X{x}Y{yy}", x = x - 1);
                                            swz = BUFDIV_LEAF_SWZ_BH;
                                            wide = true;
                                            is_rclk_hb_hdio = true;
                                        } else if matches!(
                                            hc.regs[reg - 1],
                                            HardRowKind::DcmacB
                                                | HardRowKind::HscB
                                                | HardRowKind::IlknB
                                        ) {
                                            kind = "HB_FULL";
                                            tile = format!("RCLK_HB_FULL_R_CORE_X{x}Y{yy}");
                                            swz = BUFDIV_LEAF_SWZ_B;
                                            wide = false;
                                        } else {
                                            kind = "HB";
                                            tile = format!("RCLK_HB_CORE_X{x}Y{yy}", x = x - 1);
                                            swz = BUFDIV_LEAF_SWZ_AH;
                                            wide = true;
                                        }
                                    }
                                }
                                _ => unreachable!(),
                            }
                            let node = if reg.to_idx() % 2 == 1 {
                                die.add_xnode(
                                    (col, row),
                                    self.db.get_node("RCLK_INTF.W"),
                                    &[&tile],
                                    self.db.get_node_naming(&format!("RCLK_INTF.W.{kind}")),
                                    &[(col, row), (col, row - 1)],
                                )
                            } else {
                                die.add_xnode(
                                    (col, row),
                                    self.db.get_node("RCLK_INTF.W.HALF"),
                                    &[&tile],
                                    self.db.get_node_naming(&format!("RCLK_INTF.W.HALF.{kind}")),
                                    &[(col, row)],
                                )
                            };
                            let sx = if wide {
                                di.rclkxlut[ColSide::Right][col - 1]
                            } else {
                                di.rclkxlut[ColSide::Left][col]
                            };
                            let sy = di.rclkylut[reg];
                            for (i, dy) in swz.into_iter().enumerate() {
                                node.add_bel(i, format!("BUFDIV_LEAF_X{sx}Y{sy}", sy = sy + dy));
                            }
                            if has_dfx {
                                let node = die.add_xnode(
                                    (col, row),
                                    self.db.get_node("RCLK_DFX.W"),
                                    &[&tile],
                                    self.db.get_node_naming(&format!("RCLK_DFX.W.{kind}")),
                                    &[(col, row)],
                                );
                                let sx = self.dfxxlut[ColSide::Left][di.col2ecol[col]];
                                let sy = di.dfxylut[reg];
                                node.add_bel(0, format!("RCLK_X{sx}Y{sy}"));
                            }
                            if is_rclk_hdio {
                                die.add_xnode(
                                    (col, row),
                                    self.db.get_node("RCLK_HDIO"),
                                    &[&tile],
                                    self.db.get_node_naming("RCLK_HDIO"),
                                    &[],
                                );
                            }
                            if is_rclk_hb_hdio {
                                die.add_xnode(
                                    (col, row),
                                    self.db.get_node("RCLK_HB_HDIO"),
                                    &[&tile],
                                    self.db.get_node_naming("RCLK_HB_HDIO"),
                                    &[],
                                );
                            }
                        }
                        if !matches!(
                            cd.r,
                            ColumnKind::Cle | ColumnKind::CleLaguna | ColumnKind::None
                        ) {
                            let kind;
                            let tile;
                            let swz;
                            let mut has_dfx = false;
                            match cd.r {
                                ColumnKind::Dsp => {
                                    kind = "DSP";
                                    tile = format!("RCLK_DSP_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_A;
                                }
                                ColumnKind::Bram => {
                                    kind = "BRAM";
                                    tile = format!("RCLK_BRAM_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_A;
                                    has_dfx = true;
                                }
                                ColumnKind::BramClkBuf => {
                                    kind = "BRAM.CLKBUF";
                                    tile = format!("RCLK_BRAM_CLKBUF_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_A;
                                    has_dfx = true;
                                }
                                ColumnKind::Uram => {
                                    kind = "URAM";
                                    tile = format!("RCLK_URAM_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_A;
                                    has_dfx = true;
                                }
                                ColumnKind::Gt => {
                                    if reg.to_idx() == 1
                                        && grid.regs_gt_right.is_none()
                                        && grid.cpm == CpmKind::None
                                    {
                                        kind = "GT.ALT";
                                        tile = format!("RCLK_INTF_TERM2_RIGHT_CORE_X{x}Y{yy}");
                                        swz = BUFDIV_LEAF_SWZ_B;
                                    } else {
                                        kind = "GT";
                                        tile = format!("RCLK_INTF_TERM_RIGHT_CORE_X{x}Y{yy}");
                                        swz = BUFDIV_LEAF_SWZ_A;
                                    }
                                }
                                ColumnKind::VNoc | ColumnKind::VNoc2 => {
                                    kind = "VNOC";
                                    tile = format!("RCLK_INTF_R_CORE_X{x}Y{yy}");
                                    swz = BUFDIV_LEAF_SWZ_B;
                                }
                                ColumnKind::Hard => {
                                    let hc = grid.get_col_hard(col + 1).unwrap();
                                    if reg.to_idx() % 2 == 0 {
                                        if hc.regs[reg] == HardRowKind::Hdio {
                                            kind = "HDIO";
                                            tile = format!("RCLK_HDIO_CORE_X{x}Y{yy}");
                                            swz = BUFDIV_LEAF_SWZ_A;
                                        } else {
                                            kind = "HB";
                                            tile = format!("RCLK_HB_CORE_X{x}Y{yy}");
                                            swz = BUFDIV_LEAF_SWZ_A;
                                        }
                                    } else {
                                        if hc.regs[reg] == HardRowKind::Hdio {
                                            kind = "HDIO";
                                            tile = format!("RCLK_HDIO_CORE_X{x}Y{yy}");
                                            swz = BUFDIV_LEAF_SWZ_A;
                                        } else if hc.regs[reg - 1] == HardRowKind::Hdio {
                                            kind = "HB_HDIO";
                                            tile = format!("RCLK_HB_HDIO_CORE_X{x}Y{yy}");
                                            swz = BUFDIV_LEAF_SWZ_B;
                                        } else if matches!(
                                            hc.regs[reg - 1],
                                            HardRowKind::DcmacB
                                                | HardRowKind::HscB
                                                | HardRowKind::IlknB
                                        ) {
                                            kind = "HB_FULL";
                                            tile = format!("RCLK_HB_FULL_L_CORE_X{x}Y{yy}");
                                            swz = BUFDIV_LEAF_SWZ_B;
                                        } else {
                                            kind = "HB";
                                            tile = format!("RCLK_HB_CORE_X{x}Y{yy}");
                                            swz = BUFDIV_LEAF_SWZ_A;
                                        }
                                    }
                                }
                                _ => unreachable!(),
                            }
                            let node = if reg.to_idx() % 2 == 1 {
                                die.add_xnode(
                                    (col, row),
                                    self.db.get_node("RCLK_INTF.E"),
                                    &[&tile],
                                    self.db.get_node_naming(&format!("RCLK_INTF.E.{kind}")),
                                    &[(col, row), (col, row - 1)],
                                )
                            } else {
                                die.add_xnode(
                                    (col, row),
                                    self.db.get_node("RCLK_INTF.E.HALF"),
                                    &[&tile],
                                    self.db.get_node_naming(&format!("RCLK_INTF.E.HALF.{kind}")),
                                    &[(col, row)],
                                )
                            };
                            let sx = di.rclkxlut[ColSide::Right][col];
                            let sy = di.rclkylut[reg];
                            for (i, dy) in swz.into_iter().enumerate() {
                                node.add_bel(i, format!("BUFDIV_LEAF_X{sx}Y{sy}", sy = sy + dy));
                            }
                            if has_dfx {
                                let node = die.add_xnode(
                                    (col, row),
                                    self.db.get_node("RCLK_DFX.E"),
                                    &[&tile],
                                    self.db.get_node_naming(&format!("RCLK_DFX.E.{kind}")),
                                    &[(col, row)],
                                );
                                let sx = self.dfxxlut[ColSide::Right][di.col2ecol[col]];
                                let sy = di.dfxylut[reg];
                                node.add_bel(0, format!("RCLK_X{sx}Y{sy}"));
                            }
                        }
                    }
                }
            }
        }
    }

    fn fill_cle(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &grid.columns {
                if !matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna) {
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
                    let x = di.xlut[col];
                    let y = di.ylut[row];
                    let name = format!("CLE_W_CORE_X{x}Y{y}");
                    let node = die.add_xnode(
                        (col, row),
                        self.db.get_node("CLE_R"),
                        &[&name],
                        self.db.get_node_naming("CLE_R"),
                        &[(col, row)],
                    );
                    let sx = self.clexlut[di.col2ecol[col]] * 4;
                    let sy = di.cleylut[row];
                    node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                    node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 1));
                    let name = format!("CLE_E_CORE_X{x}Y{y}", x = x + 1);
                    let node = die.add_xnode(
                        (col + 1, row),
                        self.db.get_node("CLE_L"),
                        &[&name],
                        self.db.get_node_naming("CLE_L"),
                        &[(col + 1, row)],
                    );
                    node.add_bel(0, format!("SLICE_X{sx}Y{sy}", sx = sx + 2));
                    node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 3));
                }
            }
        }
    }

    fn fill_dsp(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
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
                    let x = di.xlut[col];
                    let y = di.ylut[row];
                    let ocf = if col < di.col_cfrm { "LOCF" } else { "ROCF" };
                    let reg = grid.row_to_reg(row);
                    let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                    let name = format!("DSP_{ocf}_{bt}_TILE_X{x}Y{y}");
                    let naming = if self.naming.is_dsp_v2 {
                        "DSP.V2"
                    } else {
                        "DSP.V1"
                    };
                    let node = die.add_xnode(
                        (col, row),
                        self.db.get_node("DSP"),
                        &[&name],
                        self.db.get_node_naming(naming),
                        &[
                            (col, row),
                            (col, row + 1),
                            (col + 1, row),
                            (col + 1, row + 1),
                        ],
                    );
                    let sx = self.dspxlut[di.col2ecol[col]];
                    let sy = di.dspylut[row];
                    node.add_bel(0, format!("DSP_X{sx}Y{sy}", sx = sx * 2));
                    node.add_bel(1, format!("DSP_X{sx}Y{sy}", sx = sx * 2 + 1));
                    node.add_bel(2, format!("DSP58_CPLX_X{sx}Y{sy}"));
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &grid.columns {
                for (side, lr, kind, ck, has_bli_bot, has_bli_top) in [
                    (
                        ColSide::Left,
                        'L',
                        "BRAM_L",
                        cd.l,
                        cd.has_bli_bot_l,
                        cd.has_bli_top_l,
                    ),
                    (
                        ColSide::Right,
                        'R',
                        "BRAM_R",
                        cd.r,
                        cd.has_bli_bot_r,
                        cd.has_bli_top_r,
                    ),
                ] {
                    if !matches!(ck, ColumnKind::Bram | ColumnKind::BramClkBuf) {
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
                        let x = di.xlut[col];
                        let y = di.ylut[row];
                        let ocf = if col < di.col_cfrm { "LOCF" } else { "ROCF" };
                        let reg = grid.row_to_reg(row);
                        let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                        let name = format!("BRAM_{ocf}_{bt}{lr}_TILE_X{x}Y{y}");
                        let node = die.add_xnode(
                            (col, row),
                            self.db.get_node(kind),
                            &[&name],
                            self.db.get_node_naming(kind),
                            &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                        );
                        let sx = self.bramxlut[side][di.col2ecol[col]];
                        let sy = di.bramylut[row];
                        node.add_bel(0, format!("RAMB36_X{sx}Y{sy}"));
                        node.add_bel(1, format!("RAMB18_X{sx}Y{sy}", sy = sy * 2));
                        node.add_bel(2, format!("RAMB18_X{sx}Y{sy}", sy = sy * 2 + 1));
                    }
                }
            }
        }
    }

    fn fill_uram(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
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
                    let x = di.xlut[col];
                    let y = di.ylut[row];
                    let ocf = if col < di.col_cfrm { "LOCF" } else { "ROCF" };
                    let reg = grid.row_to_reg(row);
                    let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                    let is_delay = grid.is_reg_top(reg) && row.to_idx() % 48 == 44;
                    let delay = if is_delay { "_DELAY" } else { "" };
                    let name = format!("URAM{delay}_{ocf}_{bt}L_TILE_X{x}Y{y}");
                    let kind = if is_delay { "URAM_DELAY" } else { "URAM" };
                    let node = die.add_xnode(
                        (col, row),
                        self.db.get_node(kind),
                        &[&name],
                        self.db.get_node_naming(kind),
                        &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                    );
                    let sx = self.uramxlut[di.col2ecol[col]];
                    let sy = di.uramylut[row];
                    node.add_bel(0, format!("URAM288_X{sx}Y{sy}"));
                    if is_delay {
                        let dy = di.uramdylut[row];
                        node.add_bel(1, format!("URAM_CAS_DLY_X{sx}Y{dy}"));
                    }
                }
            }
        }
    }

    fn fill_hard(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
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
                    let (nk, tk, bk, is_high) = match kind {
                        HardRowKind::None => continue,
                        HardRowKind::DcmacT | HardRowKind::IlknT | HardRowKind::HscT => continue,
                        HardRowKind::Hdio => (
                            "HDIO",
                            if grid.is_reg_top(reg) {
                                "HDIO_TILE"
                            } else {
                                "HDIO_BOT_TILE"
                            },
                            "",
                            false,
                        ),
                        HardRowKind::CpmExt => {
                            // XXX
                            continue;
                        }
                        HardRowKind::Pcie4 => (
                            "PCIE4",
                            if grid.is_reg_top(reg) {
                                "PCIEB_TOP_TILE"
                            } else {
                                "PCIEB_BOT_TILE"
                            },
                            "PCIE40",
                            false,
                        ),
                        HardRowKind::Pcie5 => (
                            "PCIE5",
                            if grid.is_reg_top(reg) {
                                "PCIEB5_TOP_TILE"
                            } else {
                                "PCIEB5_BOT_TILE"
                            },
                            "PCIE50",
                            false,
                        ),
                        HardRowKind::Mrmac => (
                            "MRMAC",
                            if grid.is_reg_top(reg) {
                                "MRMAC_TOP_TILE"
                            } else {
                                "MRMAC_BOT_TILE"
                            },
                            "MRMAC",
                            false,
                        ),
                        HardRowKind::DcmacB => ("DCMAC", "DCMAC_TILE", "DCMAC", true),
                        HardRowKind::IlknB => ("ILKN", "ILKN_TILE", "ILKNF", true),
                        HardRowKind::HscB => ("HSC", "HSC_TILE", "HSC", true),
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
                    let x = di.xlut[hc.col - 1];
                    let y = di.ylut[row];
                    let name = format!("{tk}_X{x}Y{y}");
                    let node = die.add_xnode(
                        (hc.col, row),
                        self.db.get_node(nk),
                        &[&name],
                        self.db.get_node_naming(nk),
                        &crd,
                    );
                    let sx = self.hardxlut[kind][di.col2ecol[hc.col]];
                    let sy = di.hardylut[kind][reg];
                    if nk == "HDIO" {
                        let naming = &di.naming.hdio[&(hc.col, reg)];
                        for i in 0..11 {
                            node.add_bel(i, format!("HDIOLOGIC_X{sx}Y{y}", y = sy * 11 + i as u32));
                        }
                        for i in 0..11 {
                            node.add_bel(
                                11 + i,
                                format!(
                                    "IOB_X{x}Y{y}",
                                    x = naming.iob_xy.0,
                                    y = naming.iob_xy.1 + i as u32
                                ),
                            );
                        }
                        for i in 0..4 {
                            node.add_bel(
                                22 + i,
                                format!("BUFGCE_HDIO_X{sx}Y{y}", y = sy * 4 + i as u32),
                            );
                        }
                        node.add_bel(
                            26,
                            format!("DPLL_X{x}Y{y}", x = naming.dpll_xy.0, y = naming.dpll_xy.1),
                        );
                        node.add_bel(27, format!("HDIO_BIAS_X{sx}Y{sy}"));
                        node.add_bel(28, format!("RPI_HD_APB_X{sx}Y{sy}"));
                        node.add_bel(29, format!("HDLOGIC_APB_X{sx}Y{sy}"));
                    } else {
                        node.add_bel(0, format!("{bk}_X{sx}Y{sy}"));
                    }
                }
            }
        }
    }

    fn fill_vnoc(&mut self) {
        for (dieid, grid) in &self.grids {
            let di = &self.die[dieid];
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
                    let x = di.xlut[col - 1];
                    let y = di.ylut[row];
                    let sx = self.vnocxlut[di.col2ecol[col]];
                    let sy = di.vnocylut[reg];
                    if cd.l == ColumnKind::VNoc {
                        let name_nsu = format!("NOC_NSU512_TOP_X{x}Y{y}", y = y + 7);
                        let name_nps_a = format!("NOC_NPS_VNOC_TOP_X{x}Y{y}", y = y + 15);
                        let name_nps_b = format!("NOC_NPS_VNOC_TOP_X{x}Y{y}", y = y + 23);
                        let name_nmu = format!("NOC_NMU512_TOP_X{x}Y{y}", y = y + 31);
                        let node = die.add_xnode(
                            (col, row),
                            self.db.get_node("VNOC"),
                            &[&name_nsu, &name_nps_a, &name_nps_b, &name_nmu],
                            self.db.get_node_naming("VNOC"),
                            &crd,
                        );
                        node.add_bel(0, format!("NOC_NSU512_X{sx}Y{sy}"));
                        node.add_bel(1, format!("NOC_NPS_VNOC_X{sx}Y{sy}", sy = sy * 2));
                        node.add_bel(2, format!("NOC_NPS_VNOC_X{sx}Y{sy}", sy = sy * 2 + 1));
                        node.add_bel(3, format!("NOC_NMU512_X{sx}Y{sy}"));
                    } else {
                        let name_nsu = format!("NOC2_NSU512_VNOC_TILE_X{x}Y{y}", y = y + 7);
                        let name_nps_a = format!("NOC2_NPS5555_TOP_X{x}Y{y}", y = y + 11);
                        let name_nps_b = format!("NOC2_NPS5555_TOP_X{x}Y{y}", y = y + 14);
                        let name_nmu = format!("NOC2_NMU512_VNOC_TILE_X{x}Y{y}", y = y + 16);
                        let name_scan = format!("NOC2_SCAN_TOP_X{x}Y{y}", y = y + 7);
                        let node = die.add_xnode(
                            (col, row),
                            self.db.get_node("VNOC2"),
                            &[&name_nsu, &name_nps_a, &name_nps_b, &name_nmu, &name_scan],
                            self.db.get_node_naming("VNOC2"),
                            &crd,
                        );
                        let naming = &di.naming.vnoc2[&(col, reg)];
                        node.add_bel(
                            0,
                            format!(
                                "NOC2_NSU512_X{sx}Y{sy}",
                                sx = naming.nsu_xy.0,
                                sy = naming.nsu_xy.1
                            ),
                        );
                        node.add_bel(
                            1,
                            format!(
                                "NOC2_NPS5555_X{sx}Y{sy}",
                                sx = naming.nps_xy.0,
                                sy = naming.nps_xy.1
                            ),
                        );
                        node.add_bel(
                            2,
                            format!(
                                "NOC2_NPS5555_X{sx}Y{sy}",
                                sx = naming.nps_xy.0,
                                sy = naming.nps_xy.1 + 1
                            ),
                        );
                        node.add_bel(
                            3,
                            format!(
                                "NOC2_NMU512_X{sx}Y{sy}",
                                sx = naming.nmu_xy.0,
                                sy = naming.nmu_xy.1
                            ),
                        );
                        node.add_bel(
                            4,
                            format!(
                                "NOC2_SCAN_X{sx}Y{sy}",
                                sx = naming.scan_xy.0,
                                sy = naming.scan_xy.1
                            ),
                        );
                    }
                    if grid.is_reg_top(reg) {
                        let yy = if reg.to_idx() % 2 == 0 { y } else { y - 1 };
                        let name = format!("MISR_TILE_X{x}Y{yy}", x = x + 1);
                        let node = die.add_xnode(
                            (col, row),
                            self.db.get_node("MISR"),
                            &[&name],
                            self.db.get_node_naming("MISR"),
                            &crd,
                        );
                        let sy = di.dfxylut[reg];
                        node.add_bel(0, format!("MISR_X{sx}Y{sy}"));
                    } else {
                        let name = format!("AMS_SAT_VNOC_TILE_X{x}Y{y}", y = y + 39);
                        let node = die.add_xnode(
                            (col, row),
                            self.db.get_node("SYSMON_SAT.VNOC"),
                            &[&name],
                            self.db.get_node_naming("SYSMON_SAT.VNOC"),
                            &crd,
                        );
                        let (sx, sy) = di.naming.sysmon_sat_vnoc[&(col, reg)];
                        node.add_bel(0, format!("SYSMON_SAT_X{sx}Y{sy}"));
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
    disabled: &BTreeSet<DisabledPart>,
    naming: &'a DeviceNaming,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut expander = Expander {
        db,
        grids: grids.clone(),
        disabled: disabled.clone(),
        naming,
        egrid: ExpandedGrid::new(db),
        die: EntityVec::new(),
        ecol_cfrm: EColId::from_idx(0),
        ecols: EntityVec::new(),
        clexlut: Default::default(),
        dspxlut: Default::default(),
        bramxlut: Default::default(),
        uramxlut: Default::default(),
        hardxlut: Default::default(),
        dfxxlut: Default::default(),
        vnocxlut: Default::default(),
    };
    expander.fill_die();
    expander.fill_ecol();
    expander.fill_ylut();
    expander.fill_clexlut();
    expander.fill_cleylut();
    expander.fill_dspxlut();
    expander.fill_dspylut();
    expander.fill_bramxlut();
    expander.fill_bramylut();
    expander.fill_uramxlut();
    expander.fill_uramylut();
    expander.fill_hardxlut();
    expander.fill_hardylut();
    expander.fill_irixlut();
    expander.fill_iriylut();
    expander.fill_rclkxlut();
    expander.fill_rclkylut();
    expander.fill_vnocxlut();
    expander.fill_vnocylut();
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

    ExpandedDevice {
        grids: expander.grids,
        egrid: expander.egrid,
        disabled: expander.disabled,
    }
}

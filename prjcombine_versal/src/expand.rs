use enum_map::EnumMap;
use prjcombine_entity::{entity_id, EntityId, EntityPartVec, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use std::collections::BTreeSet;

use crate::expanded::ExpandedDevice;
use crate::grid::{ColSide, ColumnKind, CpmKind, DisabledPart, Grid, HardRowKind, PsKind};

entity_id! {
    id EColId u32, delta;
}

struct DieInfo {
    ecol2col: EntityPartVec<EColId, ColId>,
    col2ecol: EntityVec<ColId, EColId>,
    xlut: EntityVec<ColId, u32>,
    ylut: EntityVec<RowId, u32>,
    cleylut: EntityPartVec<RowId, u32>,
    iriylut: EntityVec<RowId, u32>,
    irixlut: EnumMap<ColSide, EntityPartVec<ColId, u32>>,
}

struct Expander<'a> {
    db: &'a IntDb,
    grids: EntityVec<DieId, &'a Grid>,
    disabled: BTreeSet<DisabledPart>,
    egrid: ExpandedGrid<'a>,
    die: EntityVec<DieId, DieInfo>,
    ecol_cfrm: EColId,
    ecols: EntityVec<EColId, ()>,
    clexlut: EntityPartVec<EColId, u32>,
}

impl Expander<'_> {
    fn fill_die(&mut self) {
        for grid in self.grids.values().copied() {
            self.egrid.add_die(grid.columns.len(), grid.regs * 48);
            self.die.push(DieInfo {
                ecol2col: Default::default(),
                col2ecol: Default::default(),
                xlut: Default::default(),
                ylut: Default::default(),
                cleylut: Default::default(),
                iriylut: Default::default(),
                irixlut: Default::default(),
            });
        }
    }

    fn fill_ecol(&mut self) {
        self.ecol_cfrm = EColId::from_idx(
            self.grids
                .values()
                .map(|x| x.col_cfrm.to_idx())
                .max()
                .unwrap(),
        );
        for (dieid, grid) in &self.grids {
            let di = &mut self.die[dieid];
            let mut ecol = EColId::from_idx(0);
            for col in grid.columns.ids() {
                if col == grid.col_cfrm {
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
                col >= grid.col_cfrm && matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna) && !cd.has_bli_bot_r
            });
            let has_cle_top = grid
                .columns
                .values()
                .any(|cd| matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna) && !cd.has_bli_top_r);
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
            let has_bli_bot = grid.columns.values().any(|cd| {
                cd.has_bli_bot_r || cd.has_bli_bot_l
            });
            let has_bli_top = grid.columns.values().any(|cd| {
                cd.has_bli_top_r || cd.has_bli_top_l
            });
            for row in die.rows() {
                di.iriylut.push(iriy);
                if (row.to_idx() == 0 && has_bli_bot) ||
                (row.to_idx() == die.rows().len() - 4 && has_bli_top) {
                    iriy += 16;
                } else {
                    iriy += 4;
                }
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
            let cle_e = self.db.get_term("CLE.E");
            let cle_w = self.db.get_term("CLE.W");
            let cle_bli_e = self.db.get_term("CLE.BLI.E");
            let cle_bli_w = self.db.get_term("CLE.BLI.W");
            let ps_height = match (grid.ps, grid.cpm) {
                (PsKind::Ps9, CpmKind::None) => 48 * 2,
                (PsKind::Ps9, CpmKind::Cpm4) => 48 * 3,
                (PsKind::Ps9, CpmKind::Cpm5) => 48 * 6,
                (PsKind::PsX, CpmKind::Cpm5N) => 48 * 9,
                _ => unreachable!(),
            };
            let ps_width = grid.col_cfrm.to_idx();
            let mut cle_x_bump_prev = false;
            for (col, &cd) in &grid.columns {
                if self.disabled.contains(&DisabledPart::Column(dieid, col)) {
                    continue;
                }
                let x = di.xlut[col];
                let cle_x_bump_cur;
                if matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna)
                    && col >= grid.col_cfrm
                    && grid.cols_vbrk.contains(&(col + 1))
                {
                    cle_x_bump_cur = true;
                    cle_x_bump_prev = false;
                } else {
                    cle_x_bump_cur = false;
                }
                for row in die.rows() {
                    let reg = grid.row_to_reg(row);
                    if self.disabled.contains(&DisabledPart::Region(dieid, reg)) {
                        continue;
                    }
                    let y = di.ylut[row];
                    die.fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                    let bt = if reg.to_idx() == grid.regs - 1 || reg.to_idx() % 2 == 1 {
                        'T'
                    } else {
                        'B'
                    };
                    if row.to_idx() % 48 == 0 && bt == 'T' {
                        let lr = if col < grid.col_cfrm { 'L' } else { 'R' };
                        let yy = if reg.to_idx() % 2 == 1 { y - 1 } else { y };
                        let name = format!("RCLK_INT_{lr}_FT_X{x}Y{yy}");
                        die[(col, row)].add_xnode(
                            self.db.get_node("RCLK"),
                            &[&name],
                            self.db.get_node_naming("RCLK"),
                            &[(col, row)],
                        );
                    }
                    let has_bli_r = if row < row_b + 4 {
                        cd.has_bli_bot_r
                    } else if row > row_t - 4 {
                        cd.has_bli_top_r
                    } else {
                        false
                    };
                    if matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna) {
                        let tk = if (cd.r == ColumnKind::CleLaguna) && !has_bli_r {
                            "SLL"
                        } else {
                            "CLE_BC_CORE"
                        };
                        let tile;
                        if cle_x_bump_cur {
                            tile = format!("{tk}_X{xx}Y{y}", xx = x + 1);
                        } else if cle_x_bump_prev {
                            tile = format!("{tk}_1_X{x}Y{y}");
                        } else {
                            tile = format!("{tk}_X{x}Y{y}");
                        }
                        die[(col, row)].add_xnode(
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
                    }
                    if !matches!(
                        cd.l,
                        ColumnKind::Cle | ColumnKind::CleLaguna | ColumnKind::None
                    ) {
                        let kind;
                        let tile;
                        let ocf = if col < grid.col_cfrm { "LOCF" } else { "ROCF" };
                        match cd.l {
                            ColumnKind::Gt => {
                                kind = "INTF.W.TERM.GT";
                                tile = format!("INTF_GT_{bt}L_TILE_X{x}Y{y}");
                            }
                            ColumnKind::Cfrm => {
                                if row.to_idx() < ps_height {
                                    kind = "INTF.W.TERM.PSS";
                                    tile = format!("INTF_PSS_{bt}L_TILE_X{x}Y{y}");
                                } else {
                                    kind = "INTF.W.PSS";
                                    tile = format!("INTF_CFRM_{bt}L_TILE_X{x}Y{y}");
                                }
                            }
                            ColumnKind::Hard => {
                                let ch = grid
                                    .cols_hard
                                    .iter()
                                    .flatten()
                                    .find(|x| x.col == col)
                                    .unwrap();
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
                        let node = die[(col, row)].add_xnode(
                            self.db.get_node(kind),
                            &[&tile],
                            self.db.get_node_naming(kind),
                            &[(col, row)],
                        );
                        for i in 0..4 {
                            node.iri_names.push(format!("IRI_QUAD_X{ix}Y{iy}", ix = di.irixlut[ColSide::Left][col], iy = di.iriylut[row] + i));
                        }
                    }
                    if !matches!(
                        cd.r,
                        ColumnKind::Cle | ColumnKind::CleLaguna | ColumnKind::None
                    ) {
                        let kind;
                        let tile;
                        let ocf = if col < grid.col_cfrm { "LOCF" } else { "ROCF" };
                        match cd.r {
                            ColumnKind::Gt => {
                                kind = "INTF.E.TERM.GT";
                                tile = format!("INTF_GT_{bt}R_TILE_X{x}Y{y}");
                            }
                            ColumnKind::Hard => {
                                let ch = grid
                                    .cols_hard
                                    .iter()
                                    .flatten()
                                    .find(|x| x.col == col + 1)
                                    .unwrap();
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
                        let node = die[(col, row)].add_xnode(
                            self.db.get_node(kind),
                            &[&tile],
                            self.db.get_node_naming(kind),
                            &[(col, row)],
                        );
                        for i in 0..4 {
                            node.iri_names.push(format!("IRI_QUAD_X{ix}Y{iy}", ix = di.irixlut[ColSide::Right][col], iy = di.iriylut[row] + i));
                        }
                    }
                }
                cle_x_bump_prev = cle_x_bump_cur;
            }

            die.nuke_rect(ColId::from_idx(0), RowId::from_idx(0), ps_width, ps_height);
            if ps_height != grid.regs * 48 {
                let row_t = RowId::from_idx(ps_height);
                for dx in 0..ps_width {
                    let col = ColId::from_idx(dx);
                    die.fill_term_anon((col, row_t), "TERM.S");
                }
            }
            for dy in 0..ps_height {
                let row = RowId::from_idx(dy);
                die.fill_term_anon((grid.col_cfrm, row), "TERM.W");
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

            die.fill_main_passes();
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
                    let node = tile.add_xnode(
                        self.db.get_node("CLE_R"),
                        &[&name],
                        self.db.get_node_naming("CLE_R"),
                        &[(col, row)],
                    );
                    let sx = self.clexlut[di.col2ecol[col]] * 4;
                    let sy = di.cleylut[row];
                    node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                    node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 1));
                    let tile = &mut die[(col + 1, row)];
                    let name = format!("CLE_E_CORE_X{x}Y{y}", x = x + 1);
                    let node = tile.add_xnode(
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
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut expander = Expander {
        db,
        grids: grids.clone(),
        disabled: disabled.clone(),
        egrid: ExpandedGrid::new(db),
        die: EntityVec::new(),
        ecol_cfrm: EColId::from_idx(0),
        ecols: EntityVec::new(),
        clexlut: EntityPartVec::new(),
    };
    expander.fill_die();
    expander.fill_ecol();
    expander.fill_ylut();
    expander.fill_clexlut();
    expander.fill_cleylut();
    expander.fill_irixlut();
    expander.fill_iriylut();
    expander.fill_int();
    expander.fill_cle();
    expander.fill_clkroot();

    ExpandedDevice {
        grids: expander.grids,
        egrid: expander.egrid,
        disabled: expander.disabled,
    }
}

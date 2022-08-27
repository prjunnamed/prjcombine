use crate::{eint, int, ColId, DisabledPart, RowId, SlrId};
use prjcombine_entity::{EntityId, EntityVec};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_cpipe: BTreeSet<ColId>,
    pub cols_hard: [Option<HardColumn>; 3],
    pub col_cfrm: ColId,
    pub regs: usize,
    pub regs_gt_left: Vec<GtRowKind>,
    pub regs_gt_right: Option<Vec<GtRowKind>>,
    pub cpm: CpmKind,
    pub top: TopKind,
    pub bottom: BotKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub l: ColumnKind,
    pub r: ColumnKind,
    pub has_bli_bot_l: bool,
    pub has_bli_top_l: bool,
    pub has_bli_bot_r: bool,
    pub has_bli_top_r: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Cle,
    CleLaguna,
    Bram,
    BramClkBuf,
    Uram,
    Dsp,
    Hard,
    Gt,
    Cfrm,
    VNoc,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CpmKind {
    None,
    Cpm4,
    Cpm5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HardRowKind {
    None,
    Hdio,
    Pcie4,
    Pcie5,
    Mrmac,
    IlknB,
    IlknT,
    DcmacB,
    DcmacT,
    HscB,
    HscT,
    CpmExt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: Vec<HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GtRowKind {
    None,
    Gty,
    Gtyp,
    Gtm,
    Xram,
    Vdu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BotKind {
    Xpio(usize),
    Ssit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TopKind {
    Xpio(usize),
    Ssit,
    Me,
    Ai(usize, usize),
    AiMl(usize, usize, usize),
    Hbm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum NocEndpoint {
    // tile idx, switch idx, port idx
    BotNps(usize, usize, usize),
    TopNps(usize, usize, usize),
    Ncrb(usize, usize, usize),
    // column, region, switch idx, port idx
    VNocNps(ColId, usize, usize, usize),
    VNocEnd(ColId, usize, usize),
    Pmc(usize),
    Me(usize, usize),
    // tile idx, port idx
    BotDmc(usize, usize),
    TopDmc(usize, usize),
}

pub fn expand_grid<'a>(
    grids: &EntityVec<SlrId, &Grid>,
    _grid_master: SlrId,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a int::IntDb,
) -> eint::ExpandedGrid<'a> {
    let mut egrid = eint::ExpandedGrid::new(db);
    let mut yb = 0;
    let mut syb = 0;
    let x_cfrm = grids.values().map(|x| x.col_cfrm.to_idx()).max().unwrap();
    let sx_cfrm = grids
        .values()
        .map(|grid| {
            grid.columns
                .iter()
                .filter(|&(col, &cd)| col < grid.col_cfrm && cd.r == ColumnKind::Cle)
                .count()
                * 4
        })
        .max()
        .unwrap();
    for (slrid, grid) in grids {
        let (_, mut slr) = egrid.add_slr(grid.columns.len(), grid.regs * 48);
        let col_l = slr.cols().next().unwrap();
        let col_r = slr.cols().next_back().unwrap();
        let row_b = slr.rows().next().unwrap();
        let row_t = slr.rows().next_back().unwrap();
        let xlut: EntityVec<ColId, usize> = slr
            .cols()
            .map(|x| {
                if x < grid.col_cfrm {
                    x.to_idx()
                } else {
                    x.to_idx() - grid.col_cfrm.to_idx() + x_cfrm
                }
            })
            .collect();
        let cle_e = db.get_term("CLE.E");
        let cle_w = db.get_term("CLE.W");
        let cle_bli_e = db.get_term("CLE.BLI.E");
        let cle_bli_w = db.get_term("CLE.BLI.W");
        let ps_height = match grid.cpm {
            CpmKind::Cpm4 => 48 * 3,
            CpmKind::Cpm5 => 48 * 6,
            CpmKind::None => 48 * 2,
        };
        let ps_width = grid.col_cfrm.to_idx();
        let mut cle_x_bump_prev = false;
        for (col, &cd) in &grid.columns {
            if disabled.contains(&DisabledPart::VersalColumn(slrid, col)) {
                continue;
            }
            let x = xlut[col];
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
            for row in slr.rows() {
                let reg = row.to_idx() / 48;
                let y = yb + row.to_idx();
                slr.fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                let bt = if reg == grid.regs - 1 || reg % 2 == 1 {
                    'T'
                } else {
                    'B'
                };
                if row.to_idx() % 48 == 0 && bt == 'T' {
                    let lr = if col < grid.col_cfrm { 'L' } else { 'R' };
                    let yy = if reg % 2 == 1 { y - 1 } else { y };
                    let name = format!("RCLK_INT_{lr}_FT_X{x}Y{yy}");
                    slr[(col, row)].add_xnode(
                        db.get_node("RCLK"),
                        &[&name],
                        db.get_node_naming("RCLK"),
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
                    slr[(col, row)].add_xnode(
                        db.get_node("CLE_BC"),
                        &[&tile],
                        db.get_node_naming("CLE_BC"),
                        &[(col, row), (col + 1, row)],
                    );
                    if has_bli_r {
                        slr.fill_term_pair_anon((col, row), (col + 1, row), cle_bli_e, cle_bli_w);
                    } else {
                        slr.fill_term_pair_anon((col, row), (col + 1, row), cle_e, cle_w);
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
                            kind = "INTF.E.TERM";
                            tile = format!("INTF_GT_{bt}R_TILE_X{x}Y{y}");
                        }
                        ColumnKind::Hard => {
                            kind = "INTF.E.HB";
                            let ch = grid
                                .cols_hard
                                .iter()
                                .flatten()
                                .find(|x| x.col == col + 1)
                                .unwrap();
                            match ch.regs[row.to_idx() / 48] {
                                HardRowKind::Hdio => {
                                    tile = format!("INTF_HDIO_{ocf}_{bt}R_TILE_X{x}Y{y}");
                                }
                                _ => {
                                    tile = format!("INTF_HB_{ocf}_{bt}R_TILE_X{x}Y{y}");
                                }
                            }
                        }
                        _ => {
                            kind = "INTF.E";
                            tile = format!("INTF_{ocf}_{bt}R_TILE_X{x}Y{y}");
                        }
                    }
                    slr[(col, row)].add_xnode(
                        db.get_node(kind),
                        &[&tile],
                        db.get_node_naming(kind),
                        &[(col, row)],
                    );
                }
                if !matches!(
                    cd.l,
                    ColumnKind::Cle | ColumnKind::CleLaguna | ColumnKind::None
                ) {
                    let kind;
                    let tile;
                    let bt = if reg == grid.regs - 1 || reg % 2 == 1 {
                        'T'
                    } else {
                        'B'
                    };
                    let ocf = if col < grid.col_cfrm { "LOCF" } else { "ROCF" };
                    match cd.l {
                        ColumnKind::Gt => {
                            kind = "INTF.W.TERM";
                            tile = format!("INTF_GT_{bt}L_TILE_X{x}Y{y}");
                        }
                        ColumnKind::Cfrm => {
                            if row.to_idx() < ps_height {
                                kind = "INTF.W.TERM";
                                tile = format!("INTF_PSS_{bt}L_TILE_X{x}Y{y}");
                            } else {
                                kind = "INTF.W";
                                tile = format!("INTF_CFRM_{bt}L_TILE_X{x}Y{y}");
                            }
                        }
                        ColumnKind::Hard => {
                            kind = "INTF.W.HB";
                            let ch = grid
                                .cols_hard
                                .iter()
                                .flatten()
                                .find(|x| x.col == col)
                                .unwrap();
                            match ch.regs[row.to_idx() / 48] {
                                HardRowKind::Hdio => {
                                    tile = format!("INTF_HDIO_{ocf}_{bt}L_TILE_X{x}Y{y}");
                                }
                                _ => {
                                    tile = format!("INTF_HB_{ocf}_{bt}L_TILE_X{x}Y{y}");
                                }
                            }
                        }
                        _ => {
                            kind = "INTF.W";
                            tile = format!("INTF_{ocf}_{bt}L_TILE_X{x}Y{y}");
                        }
                    }
                    slr[(col, row)].add_xnode(
                        db.get_node(kind),
                        &[&tile],
                        db.get_node_naming(kind),
                        &[(col, row)],
                    );
                }
            }
            cle_x_bump_prev = cle_x_bump_cur;
        }

        slr.nuke_rect(ColId(0), RowId(0), ps_width, ps_height);
        if ps_height != grid.regs * 48 {
            let row_t = RowId::from_idx(ps_height);
            for dx in 0..ps_width {
                let col = ColId::from_idx(dx);
                slr.fill_term_anon((col, row_t), "TERM.S");
            }
        }
        for dy in 0..ps_height {
            let row = RowId::from_idx(dy);
            slr.fill_term_anon((grid.col_cfrm, row), "TERM.W");
        }

        for col in slr.cols() {
            if !slr[(col, row_b)].nodes.is_empty() {
                slr.fill_term_anon((col, row_b), "TERM.S");
            }
            if !slr[(col, row_t)].nodes.is_empty() {
                slr.fill_term_anon((col, row_t), "TERM.N");
            }
        }
        for row in slr.rows() {
            if !slr[(col_l, row)].nodes.is_empty() {
                slr.fill_term_anon((col_l, row), "TERM.W");
            }
            if !slr[(col_r, row)].nodes.is_empty() {
                slr.fill_term_anon((col_r, row), "TERM.E");
            }
        }

        slr.fill_main_passes();

        for col in slr.cols() {
            for row in slr.rows() {
                let crow = RowId::from_idx(
                    if grid.regs % 2 == 1 && row.to_idx() >= (grid.regs - 1) * 48 {
                        row.to_idx() / 48 * 48
                    } else if row.to_idx() % 96 < 48 {
                        row.to_idx() / 96 * 96 + 47
                    } else {
                        row.to_idx() / 96 * 96 + 48
                    },
                );
                slr[(col, row)].clkroot = (col, crow);
            }
        }

        let mut dsy = 4;
        for (col, &cd) in &grid.columns {
            if col >= grid.col_cfrm
                && matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna)
                && !cd.has_bli_bot_r
            {
                dsy = 0;
            }
        }

        let mut sx = 0;
        for (col, &cd) in &grid.columns {
            if col == grid.col_cfrm {
                sx = sx_cfrm;
            }
            if !matches!(cd.r, ColumnKind::Cle | ColumnKind::CleLaguna) {
                continue;
            }
            for row in slr.rows() {
                if cd.has_bli_bot_r && row.to_idx() < 4 {
                    continue;
                }
                if cd.has_bli_top_r && row.to_idx() >= slr.rows().len() - 4 {
                    continue;
                }
                let tile = &mut slr[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = xlut[col];
                let y = yb + row.to_idx();
                let name = format!("CLE_W_CORE_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node("CLE_R"),
                    &[&name],
                    db.get_node_naming("CLE_R"),
                    &[(col, row)],
                );
                let sy = syb + row.to_idx() - dsy;
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 1));
                let tile = &mut slr[(col + 1, row)];
                let name = format!("CLE_E_CORE_X{x}Y{y}", x = x + 1);
                let node = tile.add_xnode(
                    db.get_node("CLE_L"),
                    &[&name],
                    db.get_node_naming("CLE_L"),
                    &[(col + 1, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}", sx = sx + 2));
                node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 3));
            }
            sx += 4;
        }

        yb += slr.rows().len();
        syb += slr.rows().len() - dsy;
    }

    egrid
}

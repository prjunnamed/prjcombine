use prjcombine_int::grid::{ColId, DieId, RowId};
use prjcombine_rawdump::{Coord, Part};
use prjcombine_virtex4::grid::{
    ColumnKind, DisabledPart, Grid, GridKind, GtColumn, GtKind, Interposer, IoColumn, IoKind,
    Pcie2, Pcie2Kind, RegId,
};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityVec};

use prjcombine_rdgrid::{extract_int_slr_column, find_row, find_rows, ExtraCol, IntGrid};

fn make_columns(int: &IntGrid) -> EntityVec<ColId, ColumnKind> {
    let mut res: EntityVec<ColId, Option<ColumnKind>> = int.cols.map_values(|_| None);
    *res.first_mut().unwrap() = Some(ColumnKind::Gt);
    *res.last_mut().unwrap() = Some(ColumnKind::Gt);
    for c in int.find_columns(&["CLBLL_L"]) {
        res[int.lookup_column(c + 1)] = Some(ColumnKind::ClbLL);
    }
    for c in int.find_columns(&["CLBLM_L"]) {
        res[int.lookup_column(c + 1)] = Some(ColumnKind::ClbLM);
    }
    for c in int.find_columns(&["CLBLL_R"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLL);
    }
    for c in int.find_columns(&["CLBLM_R"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLM);
    }
    for c in int.find_columns(&["BRAM_L"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Bram);
    }
    for c in int.find_columns(&["BRAM_R"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Bram);
    }
    for c in int.find_columns(&["DSP_L"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Dsp);
    }
    for c in int.find_columns(&["DSP_R"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Dsp);
    }
    for c in int.find_columns(&["RIOI", "RIOI3"]) {
        res[int.lookup_column_inter(c) - 1] = Some(ColumnKind::Io);
    }
    for c in int.find_columns(&["LIOI", "LIOI3"]) {
        res[int.lookup_column_inter(c)] = Some(ColumnKind::Io);
    }
    for c in int.find_columns(&["CMT_FIFO_R"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Cmt);
    }
    for c in int.find_columns(&["CMT_FIFO_L"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Cmt);
    }
    for c in int.find_columns(&["VFRAME"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Cfg);
    }
    for c in int.find_columns(&["CLK_HROW_BOT_R"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Clk);
    }
    for c in int.find_columns(&["CFG_CENTER_BOT"]) {
        for d in [-10, -9, -6, -5, -2, -1] {
            res[int.lookup_column(c + d)] = Some(ColumnKind::ClbLL);
        }
    }
    for c in int.find_columns(&["INT_INTERFACE_PSS_L"]) {
        for (d, kind) in [
            (-46, ColumnKind::Io),
            (-45, ColumnKind::Cmt),
            (-39, ColumnKind::ClbLM),
            (-38, ColumnKind::ClbLM),
            (-35, ColumnKind::ClbLM),
            (-34, ColumnKind::ClbLM),
            (-29, ColumnKind::Bram),
            (-28, ColumnKind::ClbLM),
            (-25, ColumnKind::ClbLM),
            (-24, ColumnKind::Dsp),
            (-19, ColumnKind::ClbLM),
            (-18, ColumnKind::ClbLM),
            (-15, ColumnKind::ClbLM),
            (-14, ColumnKind::ClbLM),
            (-9, ColumnKind::Dsp),
            (-8, ColumnKind::ClbLM),
            (-5, ColumnKind::ClbLM),
            (-4, ColumnKind::Bram),
            (1, ColumnKind::ClbLL),
        ] {
            res[int.lookup_column(c + d)] = Some(kind);
        }
    }
    res.map_values(|x| x.unwrap())
}

fn get_cols_vbrk(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["VBRK"]) {
        res.insert(int.lookup_column_inter(c));
    }
    for c in int.find_columns(&["INT_INTERFACE_PSS_L"]) {
        res.insert(int.lookup_column_inter(c - 41));
        res.insert(int.lookup_column_inter(c - 32));
        res.insert(int.lookup_column_inter(c - 21));
        res.insert(int.lookup_column_inter(c - 12));
        res.insert(int.lookup_column_inter(c - 1));
    }
    res
}

fn get_holes_pcie2(int: &IntGrid) -> Vec<Pcie2> {
    let mut res = Vec::new();
    for (x, y) in int.find_tiles(&["PCIE_BOT"]) {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 10);
        assert_eq!(row.to_idx() % 50, 0);
        res.push(Pcie2 {
            kind: Pcie2Kind::Right,
            col,
            row,
        });
    }
    for (x, y) in int.find_tiles(&["PCIE_BOT_LEFT"]) {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 10);
        assert_eq!(row.to_idx() % 50, 0);
        res.push(Pcie2 {
            kind: Pcie2Kind::Left,
            col,
            row,
        });
    }
    res
}

fn get_holes_pcie3(int: &IntGrid) -> Vec<(ColId, RowId)> {
    let mut res = vec![];
    for (x, y) in int.find_tiles(&["PCIE3_BOT_RIGHT"]) {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 7);
        assert_eq!(row.to_idx() % 50, 25);
        res.push((col, row));
    }
    res
}

fn get_cols_io(int: &IntGrid) -> Vec<IoColumn> {
    let mut res = vec![];
    if let Some(x) = int.find_column(&["LIOI", "LIOI3"]) {
        let col = int.lookup_column_inter(x);
        let mut regs = EntityVec::new();
        for i in 0..(int.rows.len() / 50) {
            let c = Coord {
                x: x as u16,
                y: int.rows[RowId::from_idx(i * 50 + 1)] as u16,
            };
            let kind = match &int.rd.tile_kinds.key(int.rd.tiles[&c].kind)[..] {
                "LIOI" => Some(IoKind::Hpio),
                "LIOI3" => Some(IoKind::Hrio),
                "PCIE_NULL" | "NULL" => None,
                _ => unreachable!(),
            };
            regs.push(kind);
        }
        res.push(IoColumn { col, regs });
    }
    if let Some(x) = int.find_column(&["RIOI", "RIOI3"]) {
        let col = int.lookup_column_inter(x) - 1;
        let mut regs = EntityVec::new();
        for i in 0..(int.rows.len() / 50) {
            let c = Coord {
                x: x as u16,
                y: int.rows[RowId::from_idx(i * 50 + 1)] as u16,
            };
            let kind = match &int.rd.tile_kinds.key(int.rd.tiles[&c].kind)[..] {
                "RIOI" => Some(IoKind::Hpio),
                "RIOI3" => Some(IoKind::Hrio),
                "NULL" => None,
                _ => unreachable!(),
            };
            regs.push(kind);
        }
        res.push(IoColumn { col, regs });
    }
    res
}

fn get_cols_gt(int: &IntGrid, columns: &EntityVec<ColId, ColumnKind>) -> Vec<GtColumn> {
    let mut res = vec![];
    if *columns.first().unwrap() == ColumnKind::Gt {
        let mut regs = EntityVec::new();
        for i in 0..(int.rows.len() / 50) {
            let c = Coord {
                x: 0,
                y: int.rows[RowId::from_idx(i * 50 + 5)] as u16,
            };
            let kind = match &int.rd.tile_kinds.key(int.rd.tiles[&c].kind)[..] {
                "GTH_CHANNEL_0" => Some(GtKind::Gth),
                "GTX_CHANNEL_0" => Some(GtKind::Gtx),
                _ => unreachable!(),
            };
            regs.push(kind);
        }
        res.push(GtColumn {
            col: columns.first_id().unwrap(),
            is_middle: false,
            regs,
        });
    }
    {
        let mut lcol = None;
        let mut regs = vec![None; int.rows.len() / 50];
        for (x, y) in int.find_tiles(&["GTP_CHANNEL_0_MID_LEFT"]) {
            lcol = Some(int.lookup_column(x - 14));
            let row = int.lookup_row(y - 5);
            assert_eq!(row.to_idx() % 50, 0);
            regs[row.to_idx() / 50] = Some(GtKind::Gtp);
        }
        if let Some(col) = lcol {
            res.push(GtColumn {
                col,
                is_middle: true,
                regs: regs.into_iter().collect(),
            });
        }
    }
    {
        let mut rcol = None;
        let mut regs = vec![None; int.rows.len() / 50];
        for (x, y) in int.find_tiles(&["GTP_CHANNEL_0_MID_RIGHT"]) {
            rcol = Some(int.lookup_column(x + 19));
            let row = int.lookup_row(y - 5);
            assert_eq!(row.to_idx() % 50, 0);
            regs[row.to_idx() / 50] = Some(GtKind::Gtp);
        }
        if let Some(col) = rcol {
            res.push(GtColumn {
                col,
                is_middle: true,
                regs: regs.into_iter().collect(),
            });
        }
    }
    {
        let col = if *columns.last().unwrap() == ColumnKind::Gt {
            columns.last_id().unwrap()
        } else {
            columns.last_id().unwrap() - 6
        };
        let x = int.cols[col] + 4;
        let mut regs = EntityVec::new();
        for i in 0..(int.rows.len() / 50) {
            let c = Coord {
                x: x as u16,
                y: int.rows[RowId::from_idx(i * 50 + 5)] as u16,
            };
            let kind = match &int.rd.tile_kinds.key(int.rd.tiles[&c].kind)[..] {
                "GTH_CHANNEL_0" => Some(GtKind::Gth),
                "GTX_CHANNEL_0" => Some(GtKind::Gtx),
                "GTP_CHANNEL_0" => Some(GtKind::Gtp),
                _ => None,
            };
            regs.push(kind);
        }
        if regs.values().any(|&x| x.is_some()) {
            res.push(GtColumn {
                col,
                is_middle: false,
                regs,
            });
        }
    }
    res
}

pub fn make_grids(rd: &Part) -> (EntityVec<DieId, Grid>, Interposer, BTreeSet<DisabledPart>) {
    let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["B_TERM_INT_SLV"])
        .into_iter()
        .map(|x| x as u16)
        .collect();
    rows_slr_split.insert(0);
    rows_slr_split.insert(rd.height);
    if rows_slr_split.contains(&2) {
        rows_slr_split.remove(&0);
    }
    if rows_slr_split.contains(&(rd.height - 2)) {
        rows_slr_split.remove(&rd.height);
    }
    let rows_slr_split: Vec<_> = rows_slr_split.iter().collect();
    let mut grids = EntityVec::new();
    let mut primary = None;
    for w in rows_slr_split.windows(2) {
        let int = extract_int_slr_column(
            rd,
            &[
                "INT_L",
                "INT_R",
                "INT_L_SLV",
                "INT_L_SLV_FLY",
                "INT_R_SLV",
                "INT_R_SLV_FLY",
            ],
            &[
                ExtraCol {
                    tts: &["CFG_CENTER_BOT"],
                    dx: &[-10, -9, -6, -5, -2, -1],
                },
                ExtraCol {
                    tts: &["INT_INTERFACE_PSS_L"],
                    dx: &[
                        -46, -45, -39, -38, -35, -34, -29, -28, -25, -24, -19, -18, -15, -14, -9,
                        -8, -5, -4,
                    ],
                },
            ],
            *w[0],
            *w[1],
        );
        let columns = make_columns(&int);
        let cols_vbrk = get_cols_vbrk(&int);
        let has_no_tbuturn = !int.find_rows(&["T_TERM_INT_NOUTURN"]).is_empty();
        let row_cfg: RowId = int.lookup_row(int.find_row(&["CFG_CENTER_BOT"]).unwrap() - 10) + 50;
        let row_clk: RowId = int.lookup_row(int.find_row(&["CLK_BUFG_BOT_R"]).unwrap()) + 4;
        let has_ps = !int.find_columns(&["INT_INTERFACE_PSS_L"]).is_empty();
        let has_slr = !int.find_columns(&["INT_L_SLV"]).is_empty();
        assert_eq!(row_cfg.to_idx() % 50, 0);
        assert_eq!(row_clk.to_idx() % 50, 0);
        assert_eq!(int.rows.len() % 50, 0);
        let slr = grids.push(Grid {
            kind: GridKind::Virtex7,
            columns: columns.clone(),
            cols_vbrk: cols_vbrk.clone(),
            cols_mgt_buf: BTreeSet::new(),
            cols_qbuf: None,
            col_hard: None,
            cols_io: get_cols_io(&int),
            cols_gt: get_cols_gt(&int, &columns),
            regs: int.rows.len() / 50,
            reg_cfg: RegId::from_idx(row_cfg.to_idx() / 50),
            reg_clk: RegId::from_idx(row_clk.to_idx() / 50),
            rows_cfg: vec![],
            holes_ppc: vec![],
            holes_pcie2: get_holes_pcie2(&int),
            holes_pcie3: get_holes_pcie3(&int),
            has_bram_fx: false,
            has_ps,
            has_slr,
            has_no_tbuturn,
        });
        if int.find_row(&["CFG_CENTER_MID"]).is_some() {
            primary = Some(slr);
        }
    }
    let primary = primary.unwrap();
    let interposer = Interposer {
        primary,
        gtz_bot: find_row(rd, &["GTZ_BOT"]).is_some(),
        gtz_top: find_row(rd, &["GTZ_TOP"]).is_some(),
    };
    let mut disabled = BTreeSet::new();
    if (rd.part.starts_with("xc7s") || rd.part.starts_with("xa7s"))
        && grids.values().any(|x| !x.holes_pcie2.is_empty())
    {
        disabled.insert(DisabledPart::Gtp);
    }
    (grids, interposer, disabled)
}

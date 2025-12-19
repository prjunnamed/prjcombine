use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::{ColId, RowId};
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_virtex4::chip::{Chip, ChipKind, ColumnKind, GtColumn, GtKind, HardColumn, RegId};
use std::collections::BTreeSet;

use prjcombine_re_xilinx_rd2db_grid::{
    IntGrid, extract_int, find_column, find_columns, find_row, find_rows,
};

fn make_columns(rd: &Part, int: &IntGrid) -> EntityVec<ColId, ColumnKind> {
    let mut res: EntityVec<ColId, Option<ColumnKind>> = int.cols.map_values(|_| None);
    for c in find_columns(rd, &["CLBLL"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLL);
    }
    for c in find_columns(rd, &["CLBLM"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLM);
    }
    for c in find_columns(rd, &["BRAM", "PCIE_BRAM"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["IOI"]) {
        res[int.lookup_column_inter(c) - 1] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["CFG_CENTER"]) {
        res[int.lookup_column_inter(c) - 1] = Some(ColumnKind::Cfg);
    }
    for c in find_columns(rd, &["GT3", "GTX"]) {
        res[int.lookup_column(c - 3)] = Some(ColumnKind::Gt);
    }
    for c in find_columns(rd, &["GTX_LEFT"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Gt);
    }
    res.map_values(|x| x.unwrap())
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["CFG_VBRK"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_mgt_buf(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["HCLK_BRAM_MGT", "HCLK_BRAM_MGT_LEFT"]) {
        res.insert(int.lookup_column(c - 2));
    }
    res
}

fn get_col_hard(rd: &Part, int: &IntGrid) -> Option<HardColumn> {
    let col = int.lookup_column(find_column(rd, &["EMAC", "PCIE_B"])? - 2);
    let rows_emac = find_rows(rd, &["EMAC"])
        .into_iter()
        .map(|r| int.lookup_row(r))
        .collect();
    let rows_pcie = find_rows(rd, &["PCIE_B"])
        .into_iter()
        .map(|r| int.lookup_row(r) - 10)
        .collect();
    Some(HardColumn {
        col,
        rows_emac,
        rows_pcie,
    })
}

fn get_reg_cfg(rd: &Part, int: &IntGrid) -> RegId {
    RegId::from_idx(
        int.lookup_row_inter(find_row(rd, &["CFG_CENTER"]).unwrap())
            .to_idx()
            / 20,
    )
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(ColId, RowId)> {
    let mut res = Vec::new();
    for tile in rd.tiles_by_kind_name("PPC_B") {
        let x = int.lookup_column((tile.x - 11) as i32);
        let y = int.lookup_row((tile.y - 10) as i32);
        assert_eq!(y.to_idx() % 20, 0);
        res.push((x, y));
    }
    res.sort();
    res
}

fn get_cols_gt(rd: &Part, int: &IntGrid, cols: &EntityVec<ColId, ColumnKind>) -> Vec<GtColumn> {
    let cols_gtp: BTreeSet<_> = find_columns(rd, &["GT3"])
        .into_iter()
        .map(|c| int.lookup_column(c - 3))
        .collect();
    cols.iter()
        .filter_map(|(col, &cd)| {
            if cd == ColumnKind::Gt {
                Some(GtColumn {
                    col,
                    is_middle: false,
                    regs: (0..(int.rows.len() / 20))
                        .map(|_| {
                            Some(if cols_gtp.contains(&col) {
                                GtKind::Gtp
                            } else {
                                GtKind::Gtx
                            })
                        })
                        .collect(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn make_grid(rd: &Part) -> Chip {
    let int = extract_int(rd, &["INT"], &[]);
    let columns = make_columns(rd, &int);
    let cols_gt = get_cols_gt(rd, &int, &columns);
    let reg_cfg = get_reg_cfg(rd, &int);
    Chip {
        kind: ChipKind::Virtex5,
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_mgt_buf: get_cols_mgt_buf(rd, &int),
        cols_qbuf: None,
        cols_io: vec![],
        cols_gt,
        col_hard: get_col_hard(rd, &int),
        regs: (int.rows.len() / 20),
        reg_cfg,
        reg_clk: reg_cfg,
        rows_cfg: vec![],
        holes_ppc: get_holes_ppc(rd, &int),
        holes_pcie2: vec![],
        holes_pcie3: vec![],
        has_bram_fx: false,
        has_ps: false,
        has_slr: false,
        has_no_tbuturn: true,
    }
}

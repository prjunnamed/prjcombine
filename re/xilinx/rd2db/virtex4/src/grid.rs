use prjcombine_entity::{EntityId, EntityVec};

use prjcombine_interconnect::grid::{ColId, RowId};
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_virtex4::chip::{CfgRowKind, Chip, ChipKind, ColumnKind, GtColumn, GtKind, RegId};
use std::collections::BTreeSet;

use prjcombine_re_xilinx_rd2db_grid::{IntGrid, extract_int, find_columns, find_row, find_rows};

fn make_columns(rd: &Part, int: &IntGrid) -> EntityVec<ColId, ColumnKind> {
    let mut res: EntityVec<ColId, Option<ColumnKind>> = int.cols.map_values(|_| None);
    for c in find_columns(rd, &["CLB"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLM);
    }
    for c in find_columns(rd, &["BRAM"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["IOIS_LC", "IOIS_LC_L"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["DCM"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Cfg);
    }
    for c in find_columns(rd, &["MGT_AR"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Gt);
    }
    for c in find_columns(rd, &["MGT_AL"]) {
        res[int.lookup_column(c + 1)] = Some(ColumnKind::Gt);
    }
    res.map_values(|x| x.unwrap())
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["CFG_VBRK_FRAME"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_reg_cfg(rd: &Part, int: &IntGrid) -> RegId {
    RegId::from_idx(
        int.lookup_row_inter(find_row(rd, &["CFG_CENTER"]).unwrap())
            .to_idx()
            / 16,
    )
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(ColId, RowId)> {
    let mut res = Vec::new();
    for tile in rd.tiles_by_kind_name("PB") {
        let x = int.lookup_column((tile.x - 1) as i32);
        let y = int.lookup_row((tile.y - 4) as i32);
        assert_eq!(y.to_idx() % 16, 12);
        res.push((x, y));
    }
    res.sort();
    res
}

fn get_has_bram_fx(rd: &Part) -> bool {
    !find_columns(rd, &["HCLK_BRAM_FX"]).is_empty()
}

fn get_cols_gt(int: &IntGrid, cols: &EntityVec<ColId, ColumnKind>) -> Vec<GtColumn> {
    cols.iter()
        .filter_map(|(col, &cd)| {
            if cd == ColumnKind::Gt {
                Some(GtColumn {
                    col,
                    is_middle: false,
                    regs: (0..(int.rows.len() / 16))
                        .map(|_| Some(GtKind::Gtp))
                        .collect(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn get_rows_cfg(rd: &Part, int: &IntGrid) -> Vec<(RowId, CfgRowKind)> {
    let mut res = vec![];
    for y in find_rows(rd, &["DCM", "DCM_BOT"]) {
        let row = int.lookup_row(y);
        res.push((row, CfgRowKind::Dcm));
    }
    for y in find_rows(rd, &["CCM"]) {
        let row = int.lookup_row(y);
        res.push((row, CfgRowKind::Ccm));
    }
    for y in find_rows(rd, &["SYS_MON"]) {
        let row = int.lookup_row(y);
        res.push((row, CfgRowKind::Sysmon));
    }
    res.sort_by_key(|&(x, _)| x);
    res
}

pub fn make_grid(rd: &Part) -> Chip {
    let int = extract_int(rd, &["INT", "INT_SO"], &[]);
    let columns = make_columns(rd, &int);
    let cols_gt = get_cols_gt(&int, &columns);
    let reg_cfg = get_reg_cfg(rd, &int);
    Chip {
        kind: ChipKind::Virtex4,
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_mgt_buf: BTreeSet::new(),
        cols_qbuf: None,
        cols_io: vec![],
        cols_gt,
        col_hard: None,
        regs: int.rows.len() / 16,
        reg_cfg,
        reg_clk: reg_cfg,
        rows_cfg: get_rows_cfg(rd, &int),
        holes_ppc: get_holes_ppc(rd, &int),
        holes_pcie2: vec![],
        holes_pcie3: vec![],
        has_bram_fx: get_has_bram_fx(rd),
        has_ps: false,
        has_slr: false,
        has_no_tbuturn: false,
    }
}

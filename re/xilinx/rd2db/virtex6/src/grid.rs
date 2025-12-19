use prjcombine_interconnect::grid::ColId;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_virtex4::chip::{
    Chip, ChipKind, ColumnKind, DisabledPart, GtColumn, GtKind, HardColumn, RegId,
};
use std::collections::BTreeSet;
use prjcombine_entity::{EntityId, EntityVec};

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
    for c in find_columns(rd, &["BRAM"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["RIOI"]) {
        res[int.lookup_column_inter(c) - 1] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["LIOI"]) {
        res[int.lookup_column_inter(c)] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["CMT_TOP"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Cfg);
    }
    for c in find_columns(rd, &["GTX"]) {
        res[int.lookup_column(c - 3)] = Some(ColumnKind::Gt);
    }
    for c in find_columns(rd, &["GTX_LEFT"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Gt);
    }
    res.map_values(|x| x.unwrap())
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["VBRK"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_mgt_buf(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["HCLK_CLBLM_MGT", "HCLK_CLBLM_MGT_LEFT"]) {
        res.insert(int.lookup_column(c - 1));
    }
    res
}

fn get_col_hard(rd: &Part, int: &IntGrid) -> Option<HardColumn> {
    let col = int.lookup_column(find_column(rd, &["EMAC"])? - 2);
    let rows_emac = find_rows(rd, &["EMAC", "EMAC_DUMMY"])
        .into_iter()
        .map(|r| int.lookup_row(r))
        .collect();
    let rows_pcie = find_rows(rd, &["PCIE", "PCIE_DUMMY"])
        .into_iter()
        .map(|r| int.lookup_row(r) - 10)
        .collect();
    Some(HardColumn {
        col,
        rows_emac,
        rows_pcie,
    })
}

fn get_cols_qbuf(rd: &Part, int: &IntGrid) -> (ColId, ColId) {
    (
        int.lookup_column(find_column(rd, &["HCLK_QBUF_L"]).unwrap()),
        int.lookup_column(find_column(rd, &["HCLK_QBUF_R"]).unwrap()),
    )
}

fn get_reg_cfg(rd: &Part, int: &IntGrid) -> RegId {
    RegId::from_idx(
        int.lookup_row(find_row(rd, &["CFG_CENTER_2"]).unwrap() - 10)
            .to_idx()
            / 40,
    )
}

fn get_cols_gt(rd: &Part, int: &IntGrid, cols: &EntityVec<ColId, ColumnKind>) -> Vec<GtColumn> {
    let reg_gth_start = if let Some(r) = find_rows(rd, &["GTH_BOT"]).into_iter().min() {
        int.lookup_row(r - 10).to_idx() / 40
    } else {
        int.rows.len() / 40
    };
    cols.iter()
        .filter_map(|(col, &cd)| {
            if cd == ColumnKind::Gt {
                Some(GtColumn {
                    col,
                    is_middle: false,
                    regs: (0..(int.rows.len() / 40))
                        .map(|reg| {
                            Some(if reg >= reg_gth_start {
                                GtKind::Gth
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

pub fn make_grid(rd: &Part) -> (Chip, BTreeSet<DisabledPart>) {
    let mut disabled = BTreeSet::new();
    let int = extract_int(rd, &["INT"], &[]);
    let columns = make_columns(rd, &int);
    let cols_gt = get_cols_gt(rd, &int, &columns);
    if rd.part.contains("vcx") {
        disabled.insert(DisabledPart::SysMon);
    }
    for r in find_rows(rd, &["EMAC_DUMMY"]) {
        disabled.insert(DisabledPart::Emac(int.lookup_row(r)));
    }
    for r in find_rows(rd, &["GTX_DUMMY"]) {
        disabled.insert(DisabledPart::GtxRow(RegId::from_idx(
            int.lookup_row(r).to_idx() / 40,
        )));
    }
    let reg_cfg = get_reg_cfg(rd, &int);
    let grid = Chip {
        kind: ChipKind::Virtex6,
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_mgt_buf: get_cols_mgt_buf(rd, &int),
        cols_qbuf: Some(get_cols_qbuf(rd, &int)),
        col_hard: get_col_hard(rd, &int),
        cols_io: vec![],
        cols_gt,
        regs: int.rows.len() / 40,
        reg_cfg,
        reg_clk: reg_cfg,
        rows_cfg: vec![],
        holes_ppc: vec![],
        holes_pcie2: vec![],
        holes_pcie3: vec![],
        has_bram_fx: false,
        has_ps: false,
        has_slr: false,
        has_no_tbuturn: true,
    };
    (grid, disabled)
}

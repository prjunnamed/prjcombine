use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::ColId;
use prjcombine_rawdump::Part;
use prjcombine_virtex6::{ColumnKind, DisabledPart, Grid, HardColumn};
use std::collections::BTreeSet;

use prjcombine_rdgrid::{extract_int, find_column, find_columns, find_row, find_rows, IntGrid};

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
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Cmt);
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

fn get_cols_io(rd: &Part, int: &IntGrid) -> [Option<ColId>; 4] {
    let mut res = [None; 4];
    let lc: Vec<_> = find_columns(rd, &["LIOI"])
        .into_iter()
        .map(|x| int.lookup_column_inter(x))
        .collect();
    match lc[..] {
        [il] => {
            res[1] = Some(il);
        }
        [ol, il] => {
            res[0] = Some(ol);
            res[1] = Some(il);
        }
        _ => unreachable!(),
    }
    let rc: Vec<_> = find_columns(rd, &["RIOI"])
        .into_iter()
        .map(|x| int.lookup_column_inter(x) - 1)
        .collect();
    match rc[..] {
        [ir] => {
            res[2] = Some(ir);
        }
        [ir, or] => {
            res[2] = Some(ir);
            res[3] = Some(or);
        }
        _ => unreachable!(),
    }
    res
}

fn get_cols_qbuf(rd: &Part, int: &IntGrid) -> (ColId, ColId) {
    (
        int.lookup_column(find_column(rd, &["HCLK_QBUF_L"]).unwrap()),
        int.lookup_column(find_column(rd, &["HCLK_QBUF_R"]).unwrap()),
    )
}

fn get_col_cfg(rd: &Part, int: &IntGrid) -> ColId {
    int.lookup_column(find_column(rd, &["CFG_CENTER_0"]).unwrap() + 2)
}

fn get_reg_cfg(rd: &Part, int: &IntGrid) -> usize {
    int.lookup_row(find_row(rd, &["CFG_CENTER_2"]).unwrap() - 10)
        .to_idx()
        / 40
}

fn get_reg_gth_start(rd: &Part, int: &IntGrid) -> usize {
    if let Some(r) = find_rows(rd, &["GTH_BOT"]).into_iter().min() {
        int.lookup_row(r - 10).to_idx() / 40
    } else {
        int.rows.len() / 40
    }
}

pub fn make_grid(rd: &Part) -> (Grid, BTreeSet<DisabledPart>) {
    let mut disabled = BTreeSet::new();
    let int = extract_int(rd, &["INT"], &[]);
    let columns = make_columns(rd, &int);
    if rd.part.contains("vcx") {
        disabled.insert(DisabledPart::SysMon);
    }
    for r in find_rows(rd, &["EMAC_DUMMY"]) {
        disabled.insert(DisabledPart::Emac(int.lookup_row(r)));
    }
    for r in find_rows(rd, &["GTX_DUMMY"]) {
        disabled.insert(DisabledPart::GtxRow(int.lookup_row(r).to_idx() / 40));
    }
    let grid = Grid {
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_mgt_buf: get_cols_mgt_buf(rd, &int),
        col_cfg: get_col_cfg(rd, &int),
        cols_qbuf: get_cols_qbuf(rd, &int),
        col_hard: get_col_hard(rd, &int),
        cols_io: get_cols_io(rd, &int),
        regs: int.rows.len() / 40,
        reg_cfg: get_reg_cfg(rd, &int),
        reg_gth_start: get_reg_gth_start(rd, &int),
    };
    (grid, disabled)
}

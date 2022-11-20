use prjcombine_entity::{EntityId, EntityVec};

use prjcombine_int::grid::{ColId, RowId};
use prjcombine_rawdump::Part;
use prjcombine_virtex4::{ColumnKind, Grid, RegId};
use std::collections::BTreeSet;

use prjcombine_rdgrid::{extract_int, find_columns, find_row, find_rows, IntGrid};

fn make_columns(rd: &Part, int: &IntGrid) -> EntityVec<ColId, ColumnKind> {
    let mut res: EntityVec<ColId, Option<ColumnKind>> = int.cols.map_values(|_| None);
    for c in find_columns(rd, &["CLB"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Clb);
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

fn get_cols_io(columns: &EntityVec<ColId, ColumnKind>) -> [ColId; 3] {
    let v: Vec<_> = columns
        .iter()
        .filter_map(|(k, &v)| if v == ColumnKind::Io { Some(k) } else { None })
        .collect();
    v.try_into().unwrap()
}

fn get_reg_cfg(rd: &Part, int: &IntGrid) -> RegId {
    RegId::from_idx(
        int.lookup_row_inter(find_row(rd, &["CFG_CENTER"]).unwrap())
            .to_idx()
            / 16,
    )
}

fn get_regs_cfg_io(rd: &Part, int: &IntGrid, reg_cfg: RegId) -> usize {
    let d2i = int
        .lookup_row_inter(find_row(rd, &["HCLK_DCMIOB"]).unwrap())
        .to_idx();
    let i2d = int
        .lookup_row_inter(find_row(rd, &["HCLK_IOBDCM"]).unwrap())
        .to_idx();
    assert_eq!(i2d - reg_cfg.to_idx() * 16, reg_cfg.to_idx() * 16 - d2i);
    (i2d - reg_cfg.to_idx() * 16 - 8) / 16
}

fn get_ccm(rd: &Part) -> usize {
    find_rows(rd, &["CCM"]).len() / 2
}

fn get_has_sysmons(rd: &Part) -> (bool, bool) {
    let sysmons = find_rows(rd, &["SYS_MON"]);
    (
        sysmons.contains(&1),
        sysmons.contains(&((rd.height - 9) as i32)),
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

pub fn make_grid(rd: &Part) -> Grid {
    let int = extract_int(rd, &["INT", "INT_SO"], &[]);
    let columns = make_columns(rd, &int);
    let cols_io = get_cols_io(&columns);
    let (has_bot_sysmon, has_top_sysmon) = get_has_sysmons(rd);
    let reg_cfg = get_reg_cfg(rd, &int);
    Grid {
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_io,
        regs: int.rows.len() / 16,
        has_bot_sysmon,
        has_top_sysmon,
        regs_cfg_io: get_regs_cfg_io(rd, &int, reg_cfg),
        ccm: get_ccm(rd),
        reg_cfg,
        holes_ppc: get_holes_ppc(rd, &int),
        has_bram_fx: get_has_bram_fx(rd),
    }
}

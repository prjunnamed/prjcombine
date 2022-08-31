use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::{ColId, DieId};
use prjcombine_rawdump::Part;
use prjcombine_versal::{
    BotKind, Column, ColumnKind, CpmKind, DisabledPart, Grid, GtRowKind, HardColumn, HardRowKind,
    TopKind,
};
use std::collections::{BTreeMap, BTreeSet};

use prjcombine_rdgrid::{extract_int_slr, find_rows, IntGrid};

fn make_columns(int: &IntGrid) -> (EntityVec<ColId, Column>, ColId, [Option<HardColumn>; 3]) {
    let mut res = int.cols.map_values(|_| Column {
        l: ColumnKind::None,
        r: ColumnKind::None,
        has_bli_bot_l: false,
        has_bli_bot_r: false,
        has_bli_top_l: false,
        has_bli_top_r: false,
    });

    for (tkn, kind) in [
        ("CLE_W_CORE", ColumnKind::Cle),
        ("DSP_ROCF_B_TILE", ColumnKind::Dsp),
        ("DSP_ROCF_T_TILE", ColumnKind::Dsp),
        ("NOC_NSU512_TOP", ColumnKind::VNoc),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c].l = kind;
            res[c - 1].r = kind;
        }
    }
    for (tkn, kind) in [
        ("BRAM_LOCF_TR_TILE", ColumnKind::Bram),
        ("BRAM_LOCF_BR_TILE", ColumnKind::Bram),
        ("BRAM_ROCF_TR_TILE", ColumnKind::Bram),
        ("BRAM_ROCF_BR_TILE", ColumnKind::Bram),
        ("INTF_GT_TR_TILE", ColumnKind::Gt),
        ("INTF_GT_BR_TILE", ColumnKind::Gt),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c - 1].r = kind;
        }
    }
    for (tkn, kind) in [
        ("BRAM_ROCF_TL_TILE", ColumnKind::Bram),
        ("BRAM_ROCF_BL_TILE", ColumnKind::Bram),
        ("URAM_LOCF_TL_TILE", ColumnKind::Uram),
        ("URAM_LOCF_BL_TILE", ColumnKind::Uram),
        ("URAM_ROCF_TL_TILE", ColumnKind::Uram),
        ("URAM_ROCF_BL_TILE", ColumnKind::Uram),
        ("INTF_GT_TL_TILE", ColumnKind::Gt),
        ("INTF_GT_BL_TILE", ColumnKind::Gt),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c].l = kind;
        }
    }
    for c in int.find_columns(&["SLL"]) {
        let c = int.lookup_column_inter(c);
        assert_eq!(res[c].l, ColumnKind::Cle);
        assert_eq!(res[c - 1].r, ColumnKind::Cle);
        res[c].l = ColumnKind::CleLaguna;
        res[c - 1].r = ColumnKind::CleLaguna;
    }
    for c in int.find_columns(&["RCLK_BRAM_CLKBUF_CORE"]) {
        let c = int.lookup_column_inter(c);
        assert_eq!(res[c - 1].r, ColumnKind::Bram);
        res[c - 1].r = ColumnKind::BramClkBuf;
    }

    for c in int.find_columns(&[
        "BLI_CLE_TOP_CORE",
        "BLI_DSP_ROCF_TR_TILE",
        "BLI_BRAM_LOCF_TR_TILE",
        "BLI_BRAM_ROCF_TR_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c - 1].has_bli_top_r = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_TOP_CORE_MY",
        "BLI_DSP_ROCF_TL_TILE",
        "BLI_BRAM_ROCF_TL_TILE",
        "BLI_URAM_LOCF_TL_TILE",
        "BLI_URAM_ROCF_TL_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c].has_bli_top_l = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_BOT_CORE",
        "BLI_DSP_ROCF_BR_TILE",
        "BLI_BRAM_ROCF_BR_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c - 1].has_bli_bot_r = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_BOT_CORE_MY",
        "BLI_DSP_ROCF_BL_TILE",
        "BLI_BRAM_ROCF_BL_TILE",
        "BLI_URAM_ROCF_BL_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c].has_bli_bot_l = true;
    }

    let col_cfrm = int.lookup_column_inter(int.find_column(&["CFRM_PMC_TILE"]).unwrap());
    res[col_cfrm].l = ColumnKind::Cfrm;

    let mut hard_cells = BTreeMap::new();
    for (tt, kind) in [
        ("HDIO_TILE", HardRowKind::Hdio),
        ("HDIO_BOT_TILE", HardRowKind::Hdio),
        ("PCIEB_TOP_TILE", HardRowKind::Pcie4),
        ("PCIEB_BOT_TILE", HardRowKind::Pcie4),
        ("PCIEB5_TOP_TILE", HardRowKind::Pcie5),
        ("PCIEB5_BOT_TILE", HardRowKind::Pcie5),
        ("MRMAC_TOP_TILE", HardRowKind::Mrmac),
        ("MRMAC_BOT_TILE", HardRowKind::Mrmac),
        ("CPM_EXT_TILE", HardRowKind::CpmExt),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x);
            let row = int.lookup_row(y).to_idx() / 48;
            hard_cells.insert((col, row), kind);
        }
    }
    for (tt, kind_b, kind_t) in [
        ("ILKN_TILE", HardRowKind::IlknB, HardRowKind::IlknT),
        ("DCMAC_TILE", HardRowKind::DcmacB, HardRowKind::DcmacT),
        ("HSC_TILE", HardRowKind::HscB, HardRowKind::HscT),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x);
            let row = int.lookup_row(y).to_idx() / 48;
            hard_cells.insert((col, row), kind_b);
            hard_cells.insert((col, row + 1), kind_t);
        }
    }
    let mut cols_hard = Vec::new();
    let cols: BTreeSet<ColId> = hard_cells.keys().map(|&(c, _)| c).collect();
    for col in cols {
        res[col].l = ColumnKind::Hard;
        res[col - 1].r = ColumnKind::Hard;
        let mut regs = Vec::new();
        for _ in 0..(int.rows.len() / 48) {
            regs.push(HardRowKind::None);
        }
        for (&(c, r), &kind) in hard_cells.iter() {
            if c == col {
                assert_eq!(regs[r], HardRowKind::None);
                regs[r] = kind;
            }
        }
        cols_hard.push(HardColumn { col, regs });
    }
    let cols_hard = match cols_hard.len() {
        1 => {
            let [col_l]: [_; 1] = cols_hard.try_into().unwrap();
            [Some(col_l), None, None]
        }
        2 => {
            let [col_l, col_r]: [_; 2] = cols_hard.try_into().unwrap();
            [Some(col_l), None, Some(col_r)]
        }
        3 => {
            let [col_l, col_m, col_r]: [_; 3] = cols_hard.try_into().unwrap();
            [Some(col_l), Some(col_m), Some(col_r)]
        }
        _ => unreachable!(),
    };
    (res, col_cfrm, cols_hard)
}

fn get_cols_vbrk(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["CBRK_LOCF_TOP_TILE", "CBRK_TOP_TILE"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_cpipe(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["CPIPE_TOP_TILE"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_rows_gt_left(int: &IntGrid) -> Vec<GtRowKind> {
    let mut res = vec![GtRowKind::None; int.rows.len() / 48];
    for (tkn, kind) in [
        ("GTY_QUAD_SINGLE_MY", GtRowKind::Gty),
        ("GTYP_QUAD_SINGLE_MY", GtRowKind::Gtyp),
        ("GTM_QUAD_SINGLE_MY", GtRowKind::Gtm),
        ("XRAM_CORE", GtRowKind::Xram),
    ] {
        for row in int.find_rows(&[tkn]) {
            let row = int.lookup_row(row);
            res[row.to_idx() / 48] = kind;
        }
    }
    res
}

fn get_rows_gt_right(int: &IntGrid) -> Option<Vec<GtRowKind>> {
    let mut res = vec![GtRowKind::None; int.rows.len() / 48];
    for (tkn, kind) in [
        ("GTY_QUAD_SINGLE", GtRowKind::Gty),
        ("GTYP_QUAD_SINGLE", GtRowKind::Gtyp),
        ("GTM_QUAD_SINGLE", GtRowKind::Gtm),
        ("VDU_CORE_MY", GtRowKind::Vdu),
    ] {
        for row in int.find_rows(&[tkn]) {
            let row = int.lookup_row(row);
            res[row.to_idx() / 48] = kind;
        }
    }
    if res.iter().any(|&x| x != GtRowKind::None) {
        Some(res)
    } else {
        None
    }
}

pub fn make_grids(rd: &Part) -> (EntityVec<DieId, Grid>, DieId, BTreeSet<DisabledPart>) {
    let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["NOC_TNOC_BRIDGE_BOT_CORE"])
        .into_iter()
        .map(|r| r as u16)
        .collect();
    rows_slr_split.insert(0);
    rows_slr_split.insert(rd.height);
    let rows_slr_split: Vec<_> = rows_slr_split.iter().collect();
    let mut grids = EntityVec::new();
    for w in rows_slr_split.windows(2) {
        let int = extract_int_slr(rd, &["INT"], &[], *w[0], *w[1]);
        let (columns, col_cfrm, cols_hard) = make_columns(&int);
        let cpm = if !int.find_tiles(&["CPM_G5_TILE"]).is_empty() {
            CpmKind::Cpm5
        } else if !int.find_tiles(&["CPM_CORE"]).is_empty() {
            CpmKind::Cpm4
        } else {
            CpmKind::None
        };
        assert_eq!(int.rows.len() % 48, 0);
        grids.push(Grid {
            columns,
            cols_vbrk: get_cols_vbrk(&int),
            cols_cpipe: get_cols_cpipe(&int),
            cols_hard,
            col_cfrm,
            regs: int.rows.len() / 48,
            regs_gt_left: get_rows_gt_left(&int),
            regs_gt_right: get_rows_gt_right(&int),
            cpm,
            top: TopKind::Me,      // XXX
            bottom: BotKind::Ssit, // XXX
        });
    }
    let mut disabled = BTreeSet::new();
    if rd.part.contains("vc1502") {
        let s0 = DieId::from_idx(0);
        assert_eq!(grids[s0].regs, 7);
        let col_hard_r = grids[s0].cols_hard[2].as_mut().unwrap();
        for (reg, kind) in [(0, HardRowKind::Mrmac), (6, HardRowKind::Hdio)] {
            assert_eq!(col_hard_r.regs[reg], HardRowKind::None);
            col_hard_r.regs[reg] = kind;
            disabled.insert(DisabledPart::HardIp(s0, col_hard_r.col, reg));
        }
        let regs_gt_r = grids[s0].regs_gt_right.as_mut().unwrap();
        for reg in [0, 1, 6] {
            assert_eq!(regs_gt_r[reg], GtRowKind::None);
            regs_gt_r[reg] = GtRowKind::Gty;
            disabled.insert(DisabledPart::GtRight(s0, reg));
        }
    }
    if rd.part.contains("vm1302") {
        let s0 = DieId::from_idx(0);
        assert_eq!(grids[s0].regs, 9);
        assert_eq!(grids[s0].columns.len(), 38);
        while grids[s0].columns.len() != 61 {
            grids[s0].columns.push(Column {
                l: ColumnKind::None,
                r: ColumnKind::None,
                has_bli_bot_l: false,
                has_bli_top_l: false,
                has_bli_bot_r: false,
                has_bli_top_r: false,
            });
        }
        for i in [
            36, 37, 38, 40, 41, 43, 44, 45, 47, 48, 49, 51, 52, 53, 55, 56, 58, 59,
        ] {
            let col = ColId::from_idx(i);
            grids[s0].columns[col].r = ColumnKind::Cle;
            grids[s0].columns[col + 1].l = ColumnKind::Cle;
            grids[s0].columns[col].has_bli_bot_r = true;
            grids[s0].columns[col].has_bli_top_r = true;
            grids[s0].columns[col + 1].has_bli_bot_l = true;
            grids[s0].columns[col + 1].has_bli_top_l = true;
        }
        for i in [39, 54] {
            let col = ColId::from_idx(i);
            grids[s0].columns[col].r = ColumnKind::Dsp;
            grids[s0].columns[col + 1].l = ColumnKind::Dsp;
            grids[s0].columns[col].has_bli_bot_r = true;
            grids[s0].columns[col].has_bli_top_r = true;
            grids[s0].columns[col + 1].has_bli_bot_l = true;
            grids[s0].columns[col + 1].has_bli_top_l = true;
        }
        for i in [36, 43, 58] {
            let col = ColId::from_idx(i);
            grids[s0].columns[col].l = ColumnKind::Bram;
        }
        for i in [42, 50, 57] {
            let col = ColId::from_idx(i);
            grids[s0].columns[col].r = ColumnKind::Bram;
        }
        let col = ColId::from_idx(51);
        grids[s0].columns[col].l = ColumnKind::Uram;
        grids[s0].columns[col].has_bli_top_l = true;
        grids[s0].columns[col - 1].has_bli_top_r = true;
        let col = ColId::from_idx(46);
        grids[s0].columns[col].r = ColumnKind::VNoc;
        grids[s0].columns[col + 1].l = ColumnKind::VNoc;
        let col = ColId::from_idx(60);
        grids[s0].columns[col].r = ColumnKind::Gt;
        for i in [37, 41, 46, 48, 53, 57, 59] {
            grids[s0].cols_vbrk.insert(ColId::from_idx(i));
        }
        for i in [43, 51] {
            grids[s0].cols_cpipe.insert(ColId::from_idx(i));
        }
        for i in 36..61 {
            disabled.insert(DisabledPart::Column(s0, ColId::from_idx(i)));
        }
    }
    (grids, DieId::from_idx(0), disabled)
}

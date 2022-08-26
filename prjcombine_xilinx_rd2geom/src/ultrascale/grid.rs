use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_rawdump::{NodeId, Part};
use prjcombine_xilinx_geom::ultrascale::{
    self, ColSide, Column, ColumnKindLeft, ColumnKindRight, GridKind, HardColumn, HardRowKind,
    IoColumn, IoRowKind, Ps,
};
use prjcombine_xilinx_geom::{ColId, DisabledPart, SlrId};
use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::grid::{extract_int_slr, find_rows, IntGrid};

fn make_columns(int: &IntGrid) -> EntityVec<ColId, Column> {
    let mut res: EntityVec<ColId, (Option<ColumnKindLeft>, Option<ColumnKindRight>)> =
        int.cols.map_values(|_| (None, None));
    for (tkn, delta, kind) in [
        ("CLEL_L", 1, ColumnKindLeft::CleL),
        ("CLE_M", 1, ColumnKindLeft::CleM),
        ("CLE_M_R", 1, ColumnKindLeft::CleM),
        ("CLEM", 1, ColumnKindLeft::CleM),
        ("CLEM_R", 1, ColumnKindLeft::CleM),
        ("INT_INTF_LEFT_TERM_PSS", 1, ColumnKindLeft::CleM),
        ("BRAM", 2, ColumnKindLeft::Bram),
        ("URAM_URAM_FT", 2, ColumnKindLeft::Uram),
        ("INT_INT_INTERFACE_GT_LEFT_FT", 1, ColumnKindLeft::Gt),
        ("INT_INTF_L_TERM_GT", 1, ColumnKindLeft::Gt),
        ("INT_INT_INTERFACE_XIPHY_FT", 1, ColumnKindLeft::Io),
        ("INT_INTF_LEFT_TERM_IO_FT", 1, ColumnKindLeft::Io),
        ("INT_INTF_L_IO", 1, ColumnKindLeft::Io),
    ] {
        for c in int.find_columns(&[tkn]) {
            res[int.lookup_column(c + delta)].0 = Some(kind);
        }
    }
    for (tkn, delta, kind) in [
        ("CLEL_R", 1, ColumnKindRight::CleL),
        ("DSP", 2, ColumnKindRight::Dsp),
        ("URAM_URAM_FT", 2, ColumnKindRight::Uram),
        ("INT_INTERFACE_GT_R", 1, ColumnKindRight::Gt),
        ("INT_INTF_R_TERM_GT", 1, ColumnKindRight::Gt),
        ("INT_INTF_RIGHT_TERM_IO", 1, ColumnKindRight::Io),
    ] {
        for c in int.find_columns(&[tkn]) {
            res[int.lookup_column(c - delta)].1 = Some(kind);
        }
    }
    for c in int.find_columns(&[
        // Ultrascale
        "CFG_CFG",
        "PCIE",
        "CMAC_CMAC_FT",
        "ILMAC_ILMAC_FT",
        // Ultrascale+
        "CFG_CONFIG",
        "PCIE4_PCIE4_FT",
        "PCIE4C_PCIE4C_FT",
        "CMAC",
        "ILKN_ILKN_FT",
        "HDIO_BOT_RIGHT",
        "DFE_DFE_TILEA_FT",
        "DFE_DFE_TILEG_FT",
    ]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::Hard);
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::Hard);
    }
    for c in int.find_columns(&["FE_FE_FT"]) {
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::Sdfec);
    }
    for c in int.find_columns(&["DFE_DFE_TILEB_FT"]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::DfeB);
    }
    for c in int.find_columns(&["DFE_DFE_TILEC_FT"]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::DfeC);
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::DfeC);
    }
    for c in int.find_columns(&["DFE_DFE_TILED_FT"]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::DfeDF);
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::DfeDF);
    }
    for c in int.find_columns(&["DFE_DFE_TILEE_FT"]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::DfeE);
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::DfeE);
    }
    for c in int.find_columns(&["RCLK_CLEM_CLKBUF_L"]) {
        let c = int.lookup_column(c + 1);
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM));
        res[c].0 = Some(ColumnKindLeft::CleMClkBuf);
    }
    for c in int.find_columns(&["LAGUNA_TILE"]) {
        let c = int.lookup_column(c + 1);
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM));
        res[c].0 = Some(ColumnKindLeft::CleMLaguna);
    }
    for c in int.find_columns(&["LAG_LAG"]) {
        let c = int.lookup_column(c + 2);
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM));
        res[c].0 = Some(ColumnKindLeft::CleMLaguna);
    }
    for c in int.find_columns(&["RCLK_CLEL_R_DCG10_R"]) {
        let c = int.lookup_column(c - 1);
        assert_eq!(res[c].1, Some(ColumnKindRight::CleL));
        res[c].1 = Some(ColumnKindRight::CleLDcg10);
    }
    for (tkn, kind) in [
        ("RCLK_RCLK_BRAM_L_AUXCLMP_FT", ColumnKindLeft::BramAuxClmp),
        ("RCLK_RCLK_BRAM_L_BRAMCLMP_FT", ColumnKindLeft::BramBramClmp),
        ("RCLK_BRAM_INTF_TD_L", ColumnKindLeft::BramTd),
        ("RCLK_BRAM_INTF_TD_R", ColumnKindLeft::BramTd),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column(c + 2);
            assert_eq!(res[c].0, Some(ColumnKindLeft::Bram));
            res[c].0 = Some(kind);
        }
    }
    for c in int.find_columns(&["RCLK_DSP_CLKBUF_L"]) {
        let c = int.lookup_column(c - 2);
        assert_eq!(res[c].1, Some(ColumnKindRight::Dsp));
        res[c].1 = Some(ColumnKindRight::DspClkBuf);
    }
    for c in int.find_columns(&["RCLK_DSP_INTF_CLKBUF_L"]) {
        let c = int.lookup_column(c - 1);
        assert_eq!(res[c].1, Some(ColumnKindRight::Dsp));
        res[c].1 = Some(ColumnKindRight::DspClkBuf);
    }
    for (i, &(l, r)) in res.iter() {
        if l.is_none() {
            println!("FAILED TO DETERMINE COLUMN {}.L", i.to_idx());
        }
        if r.is_none() {
            println!("FAILED TO DETERMINE COLUMN {}.R", i.to_idx());
        }
    }
    res.into_map_values(|(l, r)| Column {
        l: l.unwrap(),
        r: r.unwrap(),
    })
}

fn get_cols_vbrk(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["CFRM_CBRK_L", "CFRM_CBRK_R"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_fsr_gap(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["FSR_GAP"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_hard(int: &IntGrid) -> Vec<HardColumn> {
    let mut vp_aux0: HashSet<NodeId> = HashSet::new();
    if let Some((_, tk)) = int.rd.tile_kinds.get("AMS") {
        for (i, &v) in tk.conn_wires.iter() {
            if &int.rd.wires[v] == "AMS_AMS_CORE_0_VP_AUX0" {
                for crd in &tk.tiles {
                    let tile = &int.rd.tiles[crd];
                    if let Some(&n) = tile.conn_wires.get(i) {
                        vp_aux0.insert(n);
                    }
                }
            }
        }
    }
    let mut cells = BTreeMap::new();
    for (tt, kind) in [
        // Ultrascale
        ("CFG_CFG", HardRowKind::Cfg),
        ("CFGIO_IOB", HardRowKind::Ams),
        ("PCIE", HardRowKind::Pcie),
        ("CMAC_CMAC_FT", HardRowKind::Cmac),
        ("ILMAC_ILMAC_FT", HardRowKind::Ilkn),
        // Ultrascale+
        ("CFG_CONFIG", HardRowKind::Cfg),
        ("CFGIO_IOB20", HardRowKind::Ams),
        ("PCIE4_PCIE4_FT", HardRowKind::Pcie),
        ("PCIE4C_PCIE4C_FT", HardRowKind::PciePlus),
        ("CMAC", HardRowKind::Cmac),
        ("ILKN_ILKN_FT", HardRowKind::Ilkn),
        ("DFE_DFE_TILEA_FT", HardRowKind::DfeA),
        ("DFE_DFE_TILEG_FT", HardRowKind::DfeG),
        ("HDIO_BOT_RIGHT", HardRowKind::Hdio),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x);
            let row = int.lookup_row(y).to_idx() / 60;
            cells.insert((col, row), kind);
        }
    }
    if let Some((_, tk)) = int.rd.tile_kinds.get("HDIO_TOP_RIGHT") {
        for (i, &v) in tk.conn_wires.iter() {
            if &int.rd.wires[v] == "HDIO_IOBPAIR_53_SWITCH_OUT" {
                for crd in &tk.tiles {
                    if !(int.slr_start..int.slr_end).contains(&crd.y) {
                        continue;
                    }
                    let col = int.lookup_column_inter(crd.x as i32);
                    let row = int.lookup_row(crd.y as i32).to_idx() / 60;
                    let tile = &int.rd.tiles[crd];
                    if let Some(&n) = tile.conn_wires.get(i) {
                        if vp_aux0.contains(&n) {
                            cells.insert((col, row), HardRowKind::HdioAms);
                        }
                    }
                }
            }
        }
    }
    let cols: BTreeSet<ColId> = cells.keys().map(|&(c, _)| c).collect();
    let mut res = Vec::new();
    for col in cols {
        let mut regs = Vec::new();
        for _ in 0..(int.rows.len() / 60) {
            regs.push(HardRowKind::None);
        }
        for (&(c, r), &kind) in cells.iter() {
            if c == col {
                assert_eq!(regs[r], HardRowKind::None);
                regs[r] = kind;
            }
        }
        res.push(HardColumn { col, regs });
    }
    res
}

fn get_cols_io(int: &IntGrid) -> Vec<IoColumn> {
    let mut cells = BTreeMap::new();
    for (tt, kind) in [
        // Ultrascale
        ("HPIO_L", IoRowKind::Hpio),
        ("HRIO_L", IoRowKind::Hrio),
        ("GTH_QUAD_LEFT_FT", IoRowKind::Gth),
        ("GTY_QUAD_LEFT_FT", IoRowKind::Gty),
        // Ultrascale+
        // [reuse HPIO_L]
        ("GTH_QUAD_LEFT", IoRowKind::Gth),
        ("GTY_L", IoRowKind::Gty),
        ("GTM_DUAL_LEFT_FT", IoRowKind::Gtm),
        ("GTFY_QUAD_LEFT_FT", IoRowKind::Gtf),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x);
            let row = int.lookup_row(y).to_idx() / 60;
            cells.insert((col, ColSide::Left, row), kind);
        }
    }
    for (tt, kind) in [
        // Ultrascale
        ("GTH_R", IoRowKind::Gth),
        // Ultrascale+
        ("HPIO_RIGHT", IoRowKind::Hpio),
        ("GTH_QUAD_RIGHT", IoRowKind::Gth),
        ("GTY_R", IoRowKind::Gty),
        ("GTM_DUAL_RIGHT_FT", IoRowKind::Gtm),
        ("GTFY_QUAD_RIGHT_FT", IoRowKind::Gtf),
        ("HSADC_HSADC_RIGHT_FT", IoRowKind::HsAdc),
        ("HSDAC_HSDAC_RIGHT_FT", IoRowKind::HsDac),
        ("RFADC_RFADC_RIGHT_FT", IoRowKind::RfAdc),
        ("RFDAC_RFDAC_RIGHT_FT", IoRowKind::RfDac),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x) - 1;
            let row = int.lookup_row(y).to_idx() / 60;
            cells.insert((col, ColSide::Right, row), kind);
        }
    }
    let cols: BTreeSet<(ColId, ColSide)> = cells.keys().map(|&(c, s, _)| (c, s)).collect();
    let mut res = Vec::new();
    for (col, side) in cols {
        let mut regs = Vec::new();
        for _ in 0..(int.rows.len() / 60) {
            regs.push(IoRowKind::None);
        }
        for (&(c, s, r), &kind) in cells.iter() {
            if c == col && side == s {
                assert_eq!(regs[r], IoRowKind::None);
                regs[r] = kind;
            }
        }
        res.push(IoColumn { col, side, regs });
    }
    res
}

fn get_ps(int: &IntGrid) -> Option<Ps> {
    let col = int.lookup_column(int.find_column(&["INT_INTF_LEFT_TERM_PSS"])? + 1);
    Some(Ps {
        col,
        has_vcu: int.find_column(&["VCU_VCU_FT"]).is_some(),
    })
}

pub fn make_grids(
    rd: &Part,
) -> (
    EntityVec<SlrId, ultrascale::Grid>,
    SlrId,
    BTreeSet<DisabledPart>,
) {
    let is_plus = rd.family == "ultrascaleplus";
    let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["INT_TERM_T"])
        .into_iter()
        .map(|r| (r + 1) as u16)
        .collect();
    rows_slr_split.insert(0);
    rows_slr_split.insert(rd.height);
    let rows_slr_split: Vec<_> = rows_slr_split.iter().collect();
    let kind = if is_plus {
        GridKind::UltrascalePlus
    } else {
        GridKind::Ultrascale
    };
    let mut grids = EntityVec::new();
    for w in rows_slr_split.windows(2) {
        let int = extract_int_slr(rd, &["INT"], &[], *w[0], *w[1]);
        let columns = make_columns(&int);
        let cols_vbrk = get_cols_vbrk(&int);
        let cols_fsr_gap = get_cols_fsr_gap(&int);
        let cols_hard = get_cols_hard(&int);
        let cols_io = get_cols_io(&int);
        let is_alt_cfg = is_plus
            && int
                .find_tiles(&[
                    "CFG_M12BUF_CTR_RIGHT_CFG_OLY_BOT_L_FT",
                    "CFG_M12BUF_CTR_RIGHT_CFG_OLY_DK_BOT_L_FT",
                ])
                .is_empty();

        let (col_hard, col_cfg) = match cols_hard.len() {
            1 => {
                let [col_cfg]: [_; 1] = cols_hard.try_into().unwrap();
                (None, col_cfg)
            }
            2 => {
                let [col_hard, col_cfg]: [_; 2] = cols_hard.try_into().unwrap();
                (Some(col_hard), col_cfg)
            }
            _ => unreachable!(),
        };
        assert_eq!(int.rows.len() % 60, 0);
        grids.push(ultrascale::Grid {
            kind,
            columns,
            cols_vbrk,
            cols_fsr_gap,
            col_cfg,
            col_hard,
            cols_io,
            regs: int.rows.len() / 60,
            ps: get_ps(&int),
            has_hbm: int.find_column(&["HBM_DMAH_FT"]).is_some(),
            is_dmc: int.find_column(&["FSR_DMC_TARGET_FT"]).is_some(),
            is_alt_cfg,
        });
    }
    let mut disabled = BTreeSet::new();
    let tterms = find_rows(rd, &["INT_TERM_T"]);
    if !tterms.contains(&(rd.height as i32 - 1)) {
        if rd.part.contains("ku025") {
            let s0 = SlrId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 3);
            assert_eq!(grids[s0].col_hard, None);
            assert_eq!(grids[s0].cols_io.len(), 3);
            grids[s0].regs = 5;
            grids[s0].col_cfg.regs.push(HardRowKind::Pcie);
            grids[s0].col_cfg.regs.push(HardRowKind::Pcie);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[2].regs.push(IoRowKind::Gth);
            grids[s0].cols_io[2].regs.push(IoRowKind::Gth);
            disabled.insert(DisabledPart::Region(s0, 3));
            disabled.insert(DisabledPart::Region(s0, 4));
        } else if rd.part.contains("ku085") {
            let s0 = SlrId::from_idx(0);
            let s1 = SlrId::from_idx(1);
            assert_eq!(grids.len(), 2);
            assert_eq!(grids[s0].regs, 5);
            assert_eq!(grids[s1].regs, 4);
            assert_eq!(grids[s1].col_hard, None);
            assert_eq!(grids[s1].cols_io.len(), 4);
            grids[s1].regs = 5;
            grids[s1].col_cfg.regs.push(HardRowKind::Pcie);
            grids[s1].cols_io[0].regs.push(IoRowKind::Gth);
            grids[s1].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s1].cols_io[2].regs.push(IoRowKind::Hpio);
            grids[s1].cols_io[3].regs.push(IoRowKind::Gth);
            assert_eq!(grids[s0], grids[s1]);
            disabled.insert(DisabledPart::Region(s1, 4));
        } else if rd.part.contains("zu25dr") {
            let s0 = SlrId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 6);
            assert_eq!(grids[s0].cols_io.len(), 3);
            grids[s0].regs = 8;
            grids[s0].col_cfg.regs.push(HardRowKind::Hdio);
            grids[s0].col_cfg.regs.push(HardRowKind::Hdio);
            grids[s0]
                .col_hard
                .as_mut()
                .unwrap()
                .regs
                .push(HardRowKind::Cmac);
            grids[s0]
                .col_hard
                .as_mut()
                .unwrap()
                .regs
                .push(HardRowKind::Pcie);
            grids[s0].cols_io[0].regs.push(IoRowKind::Gty);
            grids[s0].cols_io[0].regs.push(IoRowKind::Gty);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[2].regs.push(IoRowKind::HsDac);
            grids[s0].cols_io[2].regs.push(IoRowKind::HsDac);
            disabled.insert(DisabledPart::Region(s0, 6));
            disabled.insert(DisabledPart::Region(s0, 7));
        } else if rd.part.contains("ku19p") {
            let s0 = SlrId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 9);
            assert_eq!(grids[s0].cols_io.len(), 2);
            assert_eq!(grids[s0].col_hard, None);
            grids[s0].regs = 11;
            grids[s0].col_cfg.regs.insert(0, HardRowKind::PciePlus);
            grids[s0].col_cfg.regs.push(HardRowKind::Cmac);
            grids[s0].cols_io[0].regs.insert(0, IoRowKind::Hpio);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.insert(0, IoRowKind::Gty);
            grids[s0].cols_io[1].regs.push(IoRowKind::Gtm);
            disabled.insert(DisabledPart::Region(s0, 0));
            disabled.insert(DisabledPart::Region(s0, 10));
        } else {
            println!("UNKNOWN CUT TOP {}", rd.part);
        }
    }
    let bterms = find_rows(rd, &["INT_TERM_B"]);
    if !bterms.contains(&0)
        && !grids.first().unwrap().has_hbm
        && grids.first().unwrap().ps.is_none()
    {
        if rd.part.contains("vu160") {
            let s0 = SlrId::from_idx(0);
            let s1 = SlrId::from_idx(1);
            let s2 = SlrId::from_idx(2);
            assert_eq!(grids.len(), 3);
            assert_eq!(grids[s0].regs, 4);
            assert_eq!(grids[s1].regs, 5);
            assert_eq!(grids[s2].regs, 5);
            assert_eq!(grids[s0].cols_io.len(), 4);
            grids[s0].regs = 5;
            grids[s0].col_cfg.regs.insert(0, HardRowKind::Pcie);
            grids[s0]
                .col_hard
                .as_mut()
                .unwrap()
                .regs
                .insert(0, HardRowKind::Ilkn);
            grids[s0].cols_io[0].regs.insert(0, IoRowKind::Gty);
            grids[s0].cols_io[1].regs.insert(0, IoRowKind::Hpio);
            grids[s0].cols_io[2].regs.insert(0, IoRowKind::Hrio);
            grids[s0].cols_io[3].regs.insert(0, IoRowKind::Gth);
            assert_eq!(grids[s0], grids[s1]);
            disabled.insert(DisabledPart::Region(s0, 0));
        } else if rd.part.contains("ku19p") {
            // fixed above
        } else {
            println!("UNKNOWN CUT BOTTOM {}", rd.part);
        }
    }
    let mut grid_master = None;
    for pins in rd.packages.values() {
        for pin in pins {
            if pin.func == "VP" {
                if is_plus {
                    grid_master = Some(
                        pin.pad
                            .as_ref()
                            .unwrap()
                            .strip_prefix("SYSMONE4_X0Y")
                            .unwrap()
                            .parse()
                            .unwrap(),
                    );
                } else {
                    grid_master = Some(
                        pin.pad
                            .as_ref()
                            .unwrap()
                            .strip_prefix("SYSMONE1_X0Y")
                            .unwrap()
                            .parse()
                            .unwrap(),
                    );
                }
            }
        }
    }
    let grid_master = SlrId::from_idx(grid_master.unwrap());
    if grids.first().unwrap().ps.is_some() {
        let mut found = false;
        for pins in rd.packages.values() {
            for pin in pins {
                if pin.pad.as_ref().filter(|x| x.starts_with("PS8")).is_some() {
                    found = true;
                }
            }
        }
        if !found {
            disabled.insert(DisabledPart::Ps);
        }
    }
    (grids, grid_master, disabled)
}

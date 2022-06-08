use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write;
use prjcombine_xilinx_rawdump::{Part, PkgPin, NodeOrClass};
use prjcombine_xilinx_geom::{self as geom, CfgPin, Bond, BondPin, GtPin, GtRegionPin, SysMonPin, DisabledPart, PsPin, HbmPin, AdcPin, DacPin};
use prjcombine_xilinx_geom::ultrascale::{self, GridKind, ColumnKind, IoColumn, IoRowKind, HardColumn, HardRowKind, Ps, IoKind, Gt};

use itertools::Itertools;

use crate::grid::{extract_int, find_column, find_columns, find_rows, find_tiles, IntGrid, PreDevice, make_device_multi};

fn make_columns(rd: &Part, int: &IntGrid) -> Vec<ColumnKind> {
    let mut res: Vec<Option<ColumnKind>> = Vec::new();
    for _ in 0..int.cols.len() {
        res.push(None);
        res.push(None);
    }
    for c in find_columns(rd, &["CLEL_L"]) {
        res[int.lookup_column(c + 1) as usize * 2] = Some(ColumnKind::CleL);
    }
    for c in find_columns(rd, &["CLEL_R"]) {
        res[int.lookup_column(c - 1) as usize * 2 + 1] = Some(ColumnKind::CleL);
    }
    for c in find_columns(rd, &["CLE_M", "CLE_M_R", "CLEM", "CLEM_R", "INT_INTF_LEFT_TERM_PSS"]) {
        res[int.lookup_column(c + 1) as usize * 2] = Some(ColumnKind::CleM);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 2) as usize * 2 + 1] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["BRAM"]) {
        res[int.lookup_column(c + 2) as usize * 2] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["URAM_URAM_FT"]) {
        res[int.lookup_column(c - 2) as usize * 2 + 1] = Some(ColumnKind::Uram);
        res[int.lookup_column(c + 2) as usize * 2] = Some(ColumnKind::Uram);
    }
    for c in find_columns(rd, &["INT_INT_INTERFACE_GT_LEFT_FT", "INT_INTF_L_TERM_GT"]) {
        res[int.lookup_column(c + 1) as usize * 2] = Some(ColumnKind::Gt);
    }
    for c in find_columns(rd, &["INT_INTERFACE_GT_R", "INT_INTF_R_TERM_GT"]) {
        res[int.lookup_column(c - 1) as usize * 2 + 1] = Some(ColumnKind::Gt);
    }
    for c in find_columns(rd, &["INT_INT_INTERFACE_XIPHY_FT", "INT_INTF_LEFT_TERM_IO_FT", "INT_INTF_L_IO"]) {
        res[int.lookup_column(c + 1) as usize * 2] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["INT_INTF_RIGHT_TERM_IO"]) {
        res[int.lookup_column(c - 1) as usize * 2 + 1] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &[
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
        res[int.lookup_column_inter(c) as usize * 2 - 1] = Some(ColumnKind::Hard);
        res[int.lookup_column_inter(c) as usize * 2] = Some(ColumnKind::Hard);
    }
    for c in find_columns(rd, &["FE_FE_FT"]) {
        res[int.lookup_column_inter(c) as usize * 2] = Some(ColumnKind::Sdfec);
    }
    for c in find_columns(rd, &["DFE_DFE_TILEB_FT"]) {
        res[int.lookup_column_inter(c) as usize * 2 - 1] = Some(ColumnKind::DfeB);
    }
    for c in find_columns(rd, &["DFE_DFE_TILEC_FT"]) {
        res[int.lookup_column_inter(c) as usize * 2 - 1] = Some(ColumnKind::DfeC);
        res[int.lookup_column_inter(c) as usize * 2] = Some(ColumnKind::DfeC);
    }
    for c in find_columns(rd, &["DFE_DFE_TILED_FT"]) {
        res[int.lookup_column_inter(c) as usize * 2 - 1] = Some(ColumnKind::DfeDF);
        res[int.lookup_column_inter(c) as usize * 2] = Some(ColumnKind::DfeDF);
    }
    for c in find_columns(rd, &["DFE_DFE_TILEE_FT"]) {
        res[int.lookup_column_inter(c) as usize * 2 - 1] = Some(ColumnKind::DfeE);
        res[int.lookup_column_inter(c) as usize * 2] = Some(ColumnKind::DfeE);
    }
    for c in find_columns(rd, &["RCLK_CLEM_CLKBUF_L"]) {
        let c = int.lookup_column(c + 1) as usize * 2;
        assert_eq!(res[c], Some(ColumnKind::CleM));
        res[c] = Some(ColumnKind::CleMClkBuf);
    }
    for c in find_columns(rd, &["LAGUNA_TILE"]) {
        let c = int.lookup_column(c + 1) as usize * 2;
        assert_eq!(res[c], Some(ColumnKind::CleM));
        res[c] = Some(ColumnKind::CleMLaguna);
    }
    for c in find_columns(rd, &["LAG_LAG"]) {
        let c = int.lookup_column(c + 2) as usize * 2;
        assert_eq!(res[c], Some(ColumnKind::CleM));
        res[c] = Some(ColumnKind::CleMLaguna);
    }
    for c in find_columns(rd, &["RCLK_CLEL_R_DCG10_R"]) {
        let c = int.lookup_column(c - 1) as usize * 2 + 1;
        assert_eq!(res[c], Some(ColumnKind::CleL));
        res[c] = Some(ColumnKind::CleLDcg10);
    }
    for c in find_columns(rd, &["RCLK_RCLK_BRAM_L_AUXCLMP_FT"]) {
        let c = int.lookup_column(c + 2) as usize * 2;
        assert_eq!(res[c], Some(ColumnKind::Bram));
        res[c] = Some(ColumnKind::BramAuxClmp);
    }
    for c in find_columns(rd, &["RCLK_RCLK_BRAM_L_BRAMCLMP_FT"]) {
        let c = int.lookup_column(c + 2) as usize * 2;
        assert_eq!(res[c], Some(ColumnKind::Bram));
        res[c] = Some(ColumnKind::BramBramClmp);
    }
    for c in find_columns(rd, &["RCLK_BRAM_INTF_TD_L", "RCLK_BRAM_INTF_TD_R"]) {
        let c = int.lookup_column(c + 2) as usize * 2;
        assert_eq!(res[c], Some(ColumnKind::Bram));
        res[c] = Some(ColumnKind::BramTd);
    }
    for c in find_columns(rd, &["RCLK_DSP_CLKBUF_L"]) {
        let c = int.lookup_column(c - 2) as usize * 2 + 1;
        assert_eq!(res[c], Some(ColumnKind::Dsp));
        res[c] = Some(ColumnKind::DspClkBuf);
    }
    for c in find_columns(rd, &["RCLK_DSP_INTF_CLKBUF_L"]) {
        let c = int.lookup_column(c - 1) as usize * 2 + 1;
        assert_eq!(res[c], Some(ColumnKind::Dsp));
        res[c] = Some(ColumnKind::DspClkBuf);
    }
    for (i, x) in res.iter().enumerate() {
        if x.is_none() {
            println!("FAILED TO DETERMINE COLUMN {}.{}", i / 2, ['L', 'R'][i % 2]);
        }
    }
    res.into_iter().map(|x| x.unwrap()).collect()
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<u32> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["CFRM_CBRK_L", "CFRM_CBRK_R"]) {
        res.insert(int.lookup_column_inter(c) * 2);
    }
    res
}

fn get_cols_fsr_gap(rd: &Part, int: &IntGrid) -> BTreeSet<u32> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["FSR_GAP"]) {
        res.insert(int.lookup_column_inter(c) * 2);
    }
    res
}

fn get_cols_hard(rd: &Part, int: &IntGrid) -> Vec<HardColumn> {
    let mut vp_aux0: HashSet<u32> = HashSet::new();
    if let Some(tk) = rd.tile_kinds.get("AMS") {
        for (i, v) in tk.conn_wires.iter().enumerate() {
            if rd.print_wire(*v) == "AMS_AMS_CORE_0_VP_AUX0" {
                for crd in &tk.tiles {
                    let tile = &rd.tiles[crd];
                    if let NodeOrClass::Node(n) = tile.get_conn_wire(i) {
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
        for (x, y) in find_tiles(rd, &[tt]).into_iter().sorted() {
            let col = int.lookup_column_inter(x) * 2;
            let row = int.lookup_row(y) / 60;
            cells.insert((col, row), kind);
        }
    }
    if let Some(tk) = rd.tile_kinds.get("HDIO_TOP_RIGHT") {
        for (i, v) in tk.conn_wires.iter().enumerate() {
            if rd.print_wire(*v) == "HDIO_IOBPAIR_53_SWITCH_OUT" {
                for crd in &tk.tiles {
                    let col = int.lookup_column_inter(crd.x as i32) * 2;
                    let row = int.lookup_row(crd.y as i32) / 60;
                    let tile = &rd.tiles[crd];
                    if let NodeOrClass::Node(n) = tile.get_conn_wire(i) {
                        if vp_aux0.contains(&n) {
                            cells.insert((col, row), HardRowKind::HdioAms);
                        }
                    }
                }
            }
        }
    }
    let cols: BTreeSet<u32> = cells.keys().map(|&(c, _)| c).collect();
    let mut res = Vec::new();
    for col in cols {
        let mut rows = Vec::new();
        for _ in 0..(int.rows.len() / 60) {
            rows.push(HardRowKind::None);
        }
        for (&(c, r), &kind) in cells.iter() {
            if c == col {
                assert_eq!(rows[r as usize], HardRowKind::None);
                rows[r as usize] = kind;
            }
        }
        res.push(HardColumn {
            col,
            rows,
        });
    }
    res
}

fn get_cols_io(rd: &Part, int: &IntGrid) -> Vec<IoColumn> {
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
    ] {
        for (x, y) in find_tiles(rd, &[tt]).into_iter().sorted() {
            let col = int.lookup_column_inter(x) * 2;
            let row = int.lookup_row(y) / 60;
            cells.insert((col, row), kind);
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
        ("HSADC_HSADC_RIGHT_FT", IoRowKind::HsAdc),
        ("HSDAC_HSDAC_RIGHT_FT", IoRowKind::HsDac),
        ("RFADC_RFADC_RIGHT_FT", IoRowKind::RfAdc),
        ("RFDAC_RFDAC_RIGHT_FT", IoRowKind::RfDac),
    ] {
        for (x, y) in find_tiles(rd, &[tt]).into_iter().sorted() {
            let col = int.lookup_column_inter(x) * 2 - 1;
            let row = int.lookup_row(y) / 60;
            cells.insert((col, row), kind);
        }
    }
    let cols: BTreeSet<u32> = cells.keys().map(|&(c, _)| c).collect();
    let mut res = Vec::new();
    for col in cols {
        let mut rows = Vec::new();
        for _ in 0..(int.rows.len() / 60) {
            rows.push(IoRowKind::None);
        }
        for (&(c, r), &kind) in cells.iter() {
            if c == col {
                assert_eq!(rows[r as usize], IoRowKind::None);
                rows[r as usize] = kind;
            }
        }
        res.push(IoColumn {
            col,
            rows,
        });
    }
    res
}

fn get_ps(rd: &Part, int: &IntGrid) -> Option<Ps> {
    let col = int.lookup_column(find_column(rd, &["INT_INTF_LEFT_TERM_PSS"])? + 1) * 2;
    Some(Ps {
        col,
        has_vcu: find_column(rd, &["VCU_VCU_FT"]).is_some(),
    })
}

fn make_grids(rd: &Part) -> (Vec<ultrascale::Grid>, usize, BTreeSet<DisabledPart>) {
    let is_plus = rd.family == "ultrascaleplus";
    let int = extract_int(rd, &["INT"], &[]);
    let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["INT_TERM_T"]).into_iter().map(|r| int.lookup_row_inter(r)).collect();
    rows_slr_split.remove(&0);
    rows_slr_split.insert(int.rows.len() as u32);
    let kind = if is_plus { GridKind::UltrascalePlus } else { GridKind::Ultrascale };
    let columns = make_columns(rd, &int);
    let cols_vbrk = get_cols_vbrk(rd, &int);
    let cols_fsr_gap = get_cols_fsr_gap(rd, &int);
    let cols_hard = get_cols_hard(rd, &int);
    let cols_io = get_cols_io(rd, &int);
    let is_alt_cfg = is_plus && find_tiles(rd, &["CFG_M12BUF_CTR_RIGHT_CFG_OLY_BOT_L_FT", "CFG_M12BUF_CTR_RIGHT_CFG_OLY_DK_BOT_L_FT"]).is_empty();

    let mut grids = Vec::new();
    let mut row_start = 0;
    for (i, row_end) in rows_slr_split.into_iter().enumerate() {
        let reg_start = row_start as usize / 60;
        let reg_end = row_end as usize / 60;
        let (col_hard, col_cfg) = if cols_hard.len() < 2 {
            (None, HardColumn {
                col: cols_hard[0].col,
                rows: cols_hard[0].rows[reg_start..reg_end].to_vec(),
            })
        } else {
            (Some(HardColumn {
                col: cols_hard[0].col,
                rows: cols_hard[0].rows[reg_start..reg_end].to_vec(),
            }), HardColumn {
                col: cols_hard[1].col,
                rows: cols_hard[1].rows[reg_start..reg_end].to_vec(),
            })
        };
        assert_eq!(row_end % 60, 0);
        grids.push(ultrascale::Grid {
            kind,
            columns: columns.clone(),
            cols_vbrk: cols_vbrk.clone(),
            cols_fsr_gap: cols_fsr_gap.clone(),
            col_cfg,
            col_hard,
            cols_io: cols_io.iter().map(|c| IoColumn {
                col: c.col,
                rows: c.rows[reg_start..reg_end].to_vec(),
            }).collect(),
            rows: (row_end - row_start) / 60,
            ps: get_ps(rd, &int),
            has_hbm: i == 0 && find_column(rd, &["HBM_DMAH_FT"]).is_some(),
            is_dmc: find_column(rd, &["FSR_DMC_TARGET_FT"]).is_some(),
            is_alt_cfg,
        });
        row_start = row_end;
    }
    let mut disabled = BTreeSet::new();
    let tterms = find_rows(rd, &["INT_TERM_T"]);
    if !tterms.contains(&(rd.height as i32 - 1)) {
        if rd.part.contains("ku025") {
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[0].rows, 3);
            assert_eq!(grids[0].col_hard, None);
            assert_eq!(grids[0].cols_io.len(), 3);
            grids[0].rows = 5;
            grids[0].col_cfg.rows.push(HardRowKind::Pcie);
            grids[0].col_cfg.rows.push(HardRowKind::Pcie);
            grids[0].cols_io[0].rows.push(IoRowKind::Hpio);
            grids[0].cols_io[0].rows.push(IoRowKind::Hpio);
            grids[0].cols_io[1].rows.push(IoRowKind::Hpio);
            grids[0].cols_io[1].rows.push(IoRowKind::Hpio);
            grids[0].cols_io[2].rows.push(IoRowKind::Gth);
            grids[0].cols_io[2].rows.push(IoRowKind::Gth);
            disabled.insert(DisabledPart::Region(3));
            disabled.insert(DisabledPart::Region(4));
        } else if rd.part.contains("ku085") {
            assert_eq!(grids.len(), 2);
            assert_eq!(grids[0].rows, 5);
            assert_eq!(grids[1].rows, 4);
            assert_eq!(grids[1].col_hard, None);
            assert_eq!(grids[1].cols_io.len(), 4);
            grids[1].rows = 5;
            grids[1].col_cfg.rows.push(HardRowKind::Pcie);
            grids[1].cols_io[0].rows.push(IoRowKind::Gth);
            grids[1].cols_io[1].rows.push(IoRowKind::Hpio);
            grids[1].cols_io[2].rows.push(IoRowKind::Hpio);
            grids[1].cols_io[3].rows.push(IoRowKind::Gth);
            assert_eq!(grids[0], grids[1]);
            disabled.insert(DisabledPart::Region(9));
        } else if rd.part.contains("zu25dr") {
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[0].rows, 6);
            assert_eq!(grids[0].cols_io.len(), 3);
            grids[0].rows = 8;
            grids[0].col_cfg.rows.push(HardRowKind::Hdio);
            grids[0].col_cfg.rows.push(HardRowKind::Hdio);
            grids[0].col_hard.as_mut().unwrap().rows.push(HardRowKind::Cmac);
            grids[0].col_hard.as_mut().unwrap().rows.push(HardRowKind::Pcie);
            grids[0].cols_io[0].rows.push(IoRowKind::Gty);
            grids[0].cols_io[0].rows.push(IoRowKind::Gty);
            grids[0].cols_io[1].rows.push(IoRowKind::Hpio);
            grids[0].cols_io[1].rows.push(IoRowKind::Hpio);
            grids[0].cols_io[2].rows.push(IoRowKind::HsDac);
            grids[0].cols_io[2].rows.push(IoRowKind::HsDac);
            disabled.insert(DisabledPart::Region(6));
            disabled.insert(DisabledPart::Region(7));
        } else if rd.part.contains("ku19p") {
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[0].rows, 9);
            assert_eq!(grids[0].cols_io.len(), 2);
            assert_eq!(grids[0].col_hard, None);
            grids[0].rows = 11;
            grids[0].col_cfg.rows.insert(0, HardRowKind::PciePlus);
            grids[0].col_cfg.rows.push(HardRowKind::Cmac);
            grids[0].cols_io[0].rows.insert(0, IoRowKind::Hpio);
            grids[0].cols_io[0].rows.push(IoRowKind::Hpio);
            grids[0].cols_io[1].rows.insert(0, IoRowKind::Gty);
            grids[0].cols_io[1].rows.push(IoRowKind::Gtm);
            disabled.insert(DisabledPart::Region(0));
            disabled.insert(DisabledPart::Region(10));
        } else {
            println!("UNKNOWN CUT TOP {}", rd.part);
        }
    }
    let bterms = find_rows(rd, &["INT_TERM_B"]);
    if !bterms.contains(&0) && !grids[0].has_hbm && grids[0].ps.is_none() {
        if rd.part.contains("vu160") {
            assert_eq!(grids.len(), 3);
            assert_eq!(grids[0].rows, 4);
            assert_eq!(grids[1].rows, 5);
            assert_eq!(grids[2].rows, 5);
            assert_eq!(grids[0].cols_io.len(), 4);
            grids[0].rows = 5;
            grids[0].col_cfg.rows.insert(0, HardRowKind::Pcie);
            grids[0].col_hard.as_mut().unwrap().rows.insert(0, HardRowKind::Ilkn);
            grids[0].cols_io[0].rows.insert(0, IoRowKind::Gty);
            grids[0].cols_io[1].rows.insert(0, IoRowKind::Hpio);
            grids[0].cols_io[2].rows.insert(0, IoRowKind::Hrio);
            grids[0].cols_io[3].rows.insert(0, IoRowKind::Gth);
            assert_eq!(grids[0], grids[1]);
            disabled.insert(DisabledPart::Region(0));
        } else if rd.part.contains("ku19p") {
            // fixed above
        } else {
            println!("UNKNOWN CUT BOTTOM {}", rd.part);
        }
    }
    let mut grid_master = None;
    for (_, pins) in &rd.packages {
        for pin in pins {
            if pin.func == "VP" {
                if is_plus {
                    grid_master = Some(pin.pad.as_ref().unwrap().strip_prefix("SYSMONE4_X0Y").unwrap().parse().unwrap());
                } else {
                    grid_master = Some(pin.pad.as_ref().unwrap().strip_prefix("SYSMONE1_X0Y").unwrap().parse().unwrap());
                }
            }
        }
    }
    let grid_master = grid_master.unwrap();
    if grids[0].ps.is_some() {
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

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn lookup_nonpad_pin(rd: &Part, pin: &PkgPin) -> Option<BondPin> {
    match &pin.func[..] {
        "NC" => return Some(BondPin::Nc),
        "GND" => return Some(BondPin::Gnd),
        "VCCINT" => return Some(BondPin::VccInt),
        "VCCAUX" => return Some(BondPin::VccAux),
        "VCCAUX_HPIO" => return Some(BondPin::VccAuxHpio),
        "VCCAUX_HDIO" => return Some(BondPin::VccAuxHdio),
        "VCCBRAM" => return Some(BondPin::VccBram),
        "VCCINT_IO" => return Some(BondPin::VccIntIo),
        "VCCAUX_IO" => return Some(BondPin::VccAuxIo(0)),
        "VBATT" => return Some(BondPin::VccBatt),
        "D00_MOSI_0" => return Some(BondPin::Cfg(CfgPin::Data(0))),
        "D01_DIN_0" => return Some(BondPin::Cfg(CfgPin::Data(1))),
        "D02_0" => return Some(BondPin::Cfg(CfgPin::Data(2))),
        "D03_0" => return Some(BondPin::Cfg(CfgPin::Data(3))),
        "RDWR_FCS_B_0" => return Some(BondPin::Cfg(CfgPin::RdWrB)),
        "TCK_0" => return Some(BondPin::Cfg(CfgPin::Tck)),
        "TDI_0" => return Some(BondPin::Cfg(CfgPin::Tdi)),
        "TDO_0" => return Some(BondPin::Cfg(CfgPin::Tdo)),
        "TMS_0" => return Some(BondPin::Cfg(CfgPin::Tms)),
        "CCLK_0" => return Some(BondPin::Cfg(CfgPin::Cclk)),
        "PUDC_B_0" | "PUDC_B" => return Some(BondPin::Cfg(CfgPin::HswapEn)),
        "POR_OVERRIDE" => return Some(BondPin::Cfg(CfgPin::PorOverride)),
        "DONE_0" => return Some(BondPin::Cfg(CfgPin::Done)),
        "PROGRAM_B_0" => return Some(BondPin::Cfg(CfgPin::ProgB)),
        "INIT_B_0" => return Some(BondPin::Cfg(CfgPin::InitB)),
        "M0_0" => return Some(BondPin::Cfg(CfgPin::M0)),
        "M1_0" => return Some(BondPin::Cfg(CfgPin::M1)),
        "M2_0" => return Some(BondPin::Cfg(CfgPin::M2)),
        "CFGBVS_0" => return Some(BondPin::Cfg(CfgPin::CfgBvs)),
        "DXN" => return Some(BondPin::Dxn),
        "DXP" => return Some(BondPin::Dxp),
        "GNDADC" => return Some(BondPin::SysMonByBank(0, SysMonPin::AVss)),
        "VCCADC" => return Some(BondPin::SysMonByBank(0, SysMonPin::AVdd)),
        "VREFP" => return Some(BondPin::SysMonByBank(0, SysMonPin::VRefP)),
        "VREFN" => return Some(BondPin::SysMonByBank(0, SysMonPin::VRefN)),
        "GND_PSADC" => return Some(BondPin::SysMonByBank(1, SysMonPin::AVss)),
        "VCC_PSADC" => return Some(BondPin::SysMonByBank(1, SysMonPin::AVdd)),
        "GND_SENSE" => return Some(BondPin::GndSense),
        "VCCINT_SENSE" => return Some(BondPin::VccIntSense),
        "VCCO_PSIO0_500" => return Some(BondPin::VccO(500)),
        "VCCO_PSIO1_501" => return Some(BondPin::VccO(501)),
        "VCCO_PSIO2_502" => return Some(BondPin::VccO(502)),
        "VCCO_PSIO3_503" => return Some(BondPin::VccO(503)),
        "VCCO_PSDDR_504" => return Some(BondPin::VccO(504)),
        "VCC_PSAUX" => return Some(BondPin::VccPsAux),
        "VCC_PSINTLP" => return Some(BondPin::VccPsIntLp),
        "VCC_PSINTFP" => return Some(BondPin::VccPsIntFp),
        "VCC_PSINTFP_DDR" => return Some(BondPin::VccPsIntFpDdr),
        "VCC_PSPLL" => return Some(BondPin::VccPsPll),
        "VCC_PSDDR_PLL" => return Some(BondPin::VccPsDdrPll),
        "VCC_PSBATT" => return Some(BondPin::VccPsBatt),
        "VCCINT_VCU" => return Some(BondPin::VccIntVcu),
        "PS_MGTRAVCC" => return Some(BondPin::GtByBank(505, GtPin::AVcc, 0)),
        "PS_MGTRAVTT" => return Some(BondPin::GtByBank(505, GtPin::AVtt, 0)),
        "VCCSDFEC" => return Some(BondPin::VccSdfec),
        "VCCINT_AMS" => return Some(BondPin::VccIntAms),
        "DAC_GND" => return Some(BondPin::DacGnd),
        "DAC_SUB_GND" => return Some(BondPin::DacSubGnd),
        "DAC_AVCC" => return Some(BondPin::DacAVcc),
        "DAC_AVCCAUX" => return Some(BondPin::DacAVccAux),
        "DAC_AVTT" => return Some(BondPin::DacAVtt),
        "ADC_GND" => return Some(BondPin::DacGnd),
        "ADC_SUB_GND" => return Some(BondPin::DacSubGnd),
        "ADC_AVCC" => return Some(BondPin::DacAVcc),
        "ADC_AVCCAUX" => return Some(BondPin::DacAVccAux),
        "RSVD" => if let Some(bank) = pin.vcco_bank {
            return Some(BondPin::Hbm(bank, HbmPin::Rsvd))
        } else {
            // disabled DACs
            if rd.part.contains("zu25dr") {
                return Some(BondPin::Rsvd)
            }
        }
        "RSVDGND" => if let Some(bank) = pin.vcco_bank {
            if bank == 0 {
                return Some(BondPin::Cfg(CfgPin::CfgBvs))
            } else {
                return Some(BondPin::Hbm(bank, HbmPin::RsvdGnd))
            }
        } else {
            for p in ["zu2cg", "zu2eg", "zu3cg", "zu3eg", "zu4cg", "zu4eg", "zu5cg", "zu5eg", "zu7cg", "zu7eg"] {
                if rd.part.contains(p) {
                    return Some(BondPin::VccIntVcu)
                }
            }
            // disabled DACs
            if rd.part.contains("zu25dr") {
                return Some(BondPin::RsvdGnd)
            }
            // disabled GT VCCINT
            if rd.part.contains("ku19p") {
                return Some(BondPin::RsvdGnd)
            }
        }
        _ => (),
    }
    if let Some(b) = pin.func.strip_prefix("VCCO_") {
        return Some(BondPin::VccO(b.parse().ok()?))
    }
    if let Some(b) = pin.func.strip_prefix("VREF_") {
        return Some(BondPin::IoVref(b.parse().ok()?, 0))
    }
    if let Some(b) = pin.func.strip_prefix("VCC_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::Vcc))
    }
    if let Some(b) = pin.func.strip_prefix("VCCAUX_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::VccAux))
    }
    if let Some(b) = pin.func.strip_prefix("VCC_IO_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::VccIo))
    }
    if let Some(b) = pin.func.strip_prefix("VCM01_") {
        return Some(BondPin::AdcByBank(b.parse().ok()?, AdcPin::VCm, 0))
    }
    if let Some(b) = pin.func.strip_prefix("VCM23_") {
        return Some(BondPin::AdcByBank(b.parse().ok()?, AdcPin::VCm, 2))
    }
    if let Some(b) = pin.func.strip_prefix("ADC_REXT_") {
        return Some(BondPin::AdcByBank(b.parse().ok()?, AdcPin::RExt, 0))
    }
    if let Some(b) = pin.func.strip_prefix("DAC_REXT_") {
        return Some(BondPin::DacByBank(b.parse().ok()?, DacPin::RExt, 0))
    }
    for (suf, region) in [
        ("", 0),
        ("_L", 2),
        ("_R", 3),
        ("_LS", 4),
        ("_RS", 5),
        ("_LLC", 6),
        ("_RLC", 7),
        ("_LC", 8),
        ("_RC", 9),
        ("_LUC", 10),
        ("_RUC", 11),
        ("_LN", 12),
        ("_RN", 13),
    ] {
        if let Some(f) = pin.func.strip_suffix(suf) {
            match f {
                "MGTAVTT" => return Some(BondPin::GtByRegion(region, GtRegionPin::AVtt)),
                "MGTAVCC" => return Some(BondPin::GtByRegion(region, GtRegionPin::AVcc)),
                "MGTVCCAUX" => return Some(BondPin::GtByRegion(region, GtRegionPin::VccAux)),
                "MGTRREF" => return Some(BondPin::GtByBank(pin.vcco_bank.unwrap(), GtPin::RRef, 0)),
                "MGTAVTTRCAL" => return Some(BondPin::GtByBank(pin.vcco_bank.unwrap(), GtPin::AVttRCal, 0)),
                "VCCINT_GT" => return Some(BondPin::GtByRegion(region, GtRegionPin::VccInt)),
                _ => (),
            }
        }
    }
    None
}

fn lookup_gt_pin(gt_lookup: &HashMap<(IoRowKind, u32, u32), Gt>, pad: &str, func: &str) -> Option<BondPin> {
    if let Some(p) = pad.strip_prefix("HSADC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::HsAdc, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "ADC_VIN0_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 0)),
            "ADC_VIN0_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 0)),
            "ADC_VIN1_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 1)),
            "ADC_VIN1_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 1)),
            "ADC_VIN2_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 2)),
            "ADC_VIN2_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 2)),
            "ADC_VIN3_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 3)),
            "ADC_VIN3_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 3)),
            "ADC_VIN_I01_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairP, 0)),
            "ADC_VIN_I01_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairN, 0)),
            "ADC_VIN_I23_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairP, 2)),
            "ADC_VIN_I23_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairN, 2)),
            "ADC_CLK_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::ClkP, 0)),
            "ADC_CLK_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::ClkN, 0)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("RFADC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::RfAdc, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "ADC_VIN0_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 0)),
            "ADC_VIN0_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 0)),
            "ADC_VIN1_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 1)),
            "ADC_VIN1_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 1)),
            "ADC_VIN2_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 2)),
            "ADC_VIN2_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 2)),
            "ADC_VIN3_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 3)),
            "ADC_VIN3_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 3)),
            "ADC_VIN_I01_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairP, 0)),
            "ADC_VIN_I01_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairN, 0)),
            "ADC_VIN_I23_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairP, 2)),
            "ADC_VIN_I23_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairN, 2)),
            "ADC_CLK_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::ClkP, 0)),
            "ADC_CLK_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::ClkN, 0)),
            "ADC_PLL_TEST_OUT_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::PllTestOutP, 0)),
            "ADC_PLL_TEST_OUT_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::PllTestOutN, 0)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("HSDAC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::HsDac, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "DAC_VOUT0_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 0)),
            "DAC_VOUT0_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 0)),
            "DAC_VOUT1_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 1)),
            "DAC_VOUT1_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 1)),
            "DAC_VOUT2_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 2)),
            "DAC_VOUT2_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 2)),
            "DAC_VOUT3_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 3)),
            "DAC_VOUT3_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 3)),
            "DAC_CLK_P" => Some(BondPin::DacByBank(gt.bank, DacPin::ClkP, 0)),
            "DAC_CLK_N" => Some(BondPin::DacByBank(gt.bank, DacPin::ClkN, 0)),
            "SYSREF_P" => Some(BondPin::DacByBank(gt.bank, DacPin::SysRefP, 0)),
            "SYSREF_N" => Some(BondPin::DacByBank(gt.bank, DacPin::SysRefN, 0)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("RFDAC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::RfDac, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "DAC_VOUT0_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 0)),
            "DAC_VOUT0_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 0)),
            "DAC_VOUT1_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 1)),
            "DAC_VOUT1_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 1)),
            "DAC_VOUT2_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 2)),
            "DAC_VOUT2_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 2)),
            "DAC_VOUT3_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 3)),
            "DAC_VOUT3_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 3)),
            "DAC_CLK_P" => Some(BondPin::DacByBank(gt.bank, DacPin::ClkP, 0)),
            "DAC_CLK_N" => Some(BondPin::DacByBank(gt.bank, DacPin::ClkN, 0)),
            "SYSREF_P" => Some(BondPin::DacByBank(gt.bank, DacPin::SysRefP, 0)),
            "SYSREF_N" => Some(BondPin::DacByBank(gt.bank, DacPin::SysRefN, 0)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("GTM_DUAL_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::Gtm, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "MGTMRXP0" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, 0)),
            "MGTMRXN0" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, 0)),
            "MGTMTXP0" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, 0)),
            "MGTMTXN0" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, 0)),
            "MGTMRXP1" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, 1)),
            "MGTMRXN1" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, 1)),
            "MGTMTXP1" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, 1)),
            "MGTMTXN1" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, 1)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("GTM_REFCLK_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::Gtm, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "MGTREFCLKP" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkP, 0)),
            "MGTREFCLKN" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkN, 0)),
            _ => None,
        }
    } else {
        let p;
        let kind;
        if let Some(x) = pad.strip_prefix("GTHE3_") {
            p = x;
            kind = IoRowKind::Gth;
        } else if let Some(x) = pad.strip_prefix("GTHE4_") {
            p = x;
            kind = IoRowKind::Gth;
        } else if let Some(x) = pad.strip_prefix("GTYE3_") {
            p = x;
            kind = IoRowKind::Gty;
        } else if let Some(x) = pad.strip_prefix("GTYE4_") {
            p = x;
            kind = IoRowKind::Gty;
        } else {
            return None
        }
        if let Some(p) = p.strip_prefix("COMMON_X") {
            let py = p.find('Y')?;
            let gx: u32 = p[..py].parse().ok()?;
            let gy: u32 = p[py+1..].parse().ok()?;
            let gt = gt_lookup.get(&(kind, gx, gy))?;
            let suf = format!("_{}", gt.bank);
            let f = func.strip_suffix(&suf)?;
            match f {
                "MGTREFCLK0P" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkP, 0)),
                "MGTREFCLK0N" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkN, 0)),
                "MGTREFCLK1P" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkP, 1)),
                "MGTREFCLK1N" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkN, 1)),
                _ => None,
            }
        } else if let Some(p) = p.strip_prefix("CHANNEL_X") {
            let py = p.find('Y')?;
            let gx: u32 = p[..py].parse().ok()?;
            let y: u32 = p[py+1..].parse().ok()?;
            let bel = y % 4;
            let gy = y / 4;
            let gt = gt_lookup.get(&(kind, gx, gy))?;
            let suf = format!("{}_{}", bel, gt.bank);
            let f = func.strip_suffix(&suf)?;
            match f {
                "MGTHRXP" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, bel)),
                "MGTHRXN" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, bel)),
                "MGTHTXP" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, bel)),
                "MGTHTXN" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, bel)),
                "MGTYRXP" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, bel)),
                "MGTYRXN" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, bel)),
                "MGTYTXP" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, bel)),
                "MGTYTXN" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, bel)),
                _ => None,
            }
        } else {
            None
        }
    }
}

fn make_bond(rd: &Part, pkg: &str, grids: &[ultrascale::Grid], grid_master: usize, disabled: &BTreeSet<DisabledPart>, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = ultrascale::get_io(grids, grid_master, disabled)
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = ultrascale::get_gt(grids, grid_master, disabled)
        .into_iter()
        .map(|gt| ((gt.kind, gt.gx, gt.gy), gt))
        .collect();
    let is_zynq = grids[0].ps.is_some() && !disabled.contains(&DisabledPart::Ps);
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                if pin.vcco_bank.unwrap() != io.bank {
                    if pin.vcco_bank != Some(64) && !matches!(io.bank, 84 | 94) {
                        println!("wrong bank pad {pkg} {pad} {io:?} got {f} exp {b}", f=pin.func, b=io.bank);
                    }
                }
                let old = io_banks.insert(io.bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                let mut exp_func = format!("IO");
                if io.kind == IoKind::Hdio {
                    write!(exp_func, "_L{}{}", 1 + io.bel / 2, ['P', 'N'][io.bel as usize % 2]).unwrap();
                } else {
                    let group = io.bel / 13;
                    if io.bel % 13 != 12 {
                        write!(exp_func, "_L{}{}", 1 + group * 6 + io.bel % 13 / 2, ['P', 'N'][io.bel as usize % 13 % 2]).unwrap();
                    }
                    write!(exp_func, "_T{}{}_N{}", group, if io.bel % 13 < 6 {'L'} else {'U'}, io.bel % 13).unwrap();
                }
                if io.is_gc() {
                    if io.kind == IoKind::Hdio {
                        exp_func += "_HDGC";
                    } else {
                        exp_func += "_GC";
                    }
                }
                if io.is_dbc() {
                    exp_func += "_DBC";
                }
                if io.is_qbc() {
                    exp_func += "_QBC";
                }
                if io.is_vrp() {
                    exp_func += "_VRP";
                }
                if let Some(sm) = io.sm_pair() {
                    if io.kind == IoKind::Hdio {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.bel as usize % 2]).unwrap();
                    } else {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.bel as usize % 13 % 2]).unwrap();
                    }
                }
                match io.get_cfg() {
                    Some(CfgPin::Data(d)) => if !is_zynq {
                        if d >= 16 {
                            write!(exp_func, "_A{:02}", d - 16).unwrap();
                        }
                        write!(exp_func, "_D{d:02}").unwrap();
                    }
                    Some(CfgPin::Addr(a)) => if !is_zynq {
                        write!(exp_func, "_A{a}").unwrap();
                    }
                    Some(CfgPin::Rs(a)) => if !is_zynq {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(CfgPin::UserCclk) => if !is_zynq {exp_func += "_EMCCLK"},
                    Some(CfgPin::Dout) => if !is_zynq {exp_func += "_DOUT_CSO_B"},
                    Some(CfgPin::FweB) => if !is_zynq {exp_func += "_FWE_FCS2_B"},
                    Some(CfgPin::FoeB) => if !is_zynq {exp_func += "_FOE_B"},
                    Some(CfgPin::CsiB) => if !is_zynq {exp_func += "_CSI_ADV_B"},
                    Some(CfgPin::PerstN0) => exp_func += "_PERSTN0",
                    Some(CfgPin::PerstN1) => exp_func += "_PERSTN1",
                    Some(CfgPin::SmbAlert) => exp_func += "_SMBALERT",
                    Some(CfgPin::I2cSclk) => exp_func += "_I2C_SCLK",
                    Some(CfgPin::I2cSda) => exp_func += if grids[0].kind == GridKind::Ultrascale {"_I2C_SDA"} else {"_PERSTN1_I2C_SDA"},
                    None => (),
                    _ => unreachable!(),
                }
                write!(exp_func, "_{}", io_banks[&io.bank]).unwrap();
                if exp_func != pin.func {
                    println!("pad {pkg} {pad} {io:?} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::IoByBank(io.bank, io.bel)
            } else if pad.starts_with("GT") || pad.starts_with("RF") || pad.starts_with("HS") {
                if let Some(pin) = lookup_gt_pin(&gt_lookup, pad, &pin.func) {
                    pin
                } else {
                    println!("weird gt iopad {pkg} {p} {pad} {f}", f=pin.func, p=rd.part);
                    continue
                }
            } else if pad.starts_with("SYSMON") {
                let exp_site = match grids[0].kind {
                    GridKind::Ultrascale => format!("SYSMONE1_X0Y{}", grid_master),
                    GridKind::UltrascalePlus => format!("SYSMONE4_X0Y{}", grid_master),
                };
                if exp_site != *pad {
                    println!("weird sysmon iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                }
                match &pin.func[..] {
                    "VP" => BondPin::SysMonByBank(grid_master as u32, SysMonPin::VP),
                    "VN" => BondPin::SysMonByBank(grid_master as u32, SysMonPin::VN),
                    _ => {
                        println!("weird sysmon iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                        continue
                    }
                }
            } else if pad == "PS8_X0Y0" {
                let pos = pin.func.rfind('_').unwrap();
                let bank: u32 = pin.func[pos+1..].parse().unwrap();
                if bank == 505 {
                    let (gtpin, bel) = match &pin.func[..pos] {
                        "PS_MGTRREF" => (GtPin::RRef, 0),
                        "PS_MGTREFCLK0P" => (GtPin::ClkP, 0),
                        "PS_MGTREFCLK0N" => (GtPin::ClkN, 0),
                        "PS_MGTREFCLK1P" => (GtPin::ClkP, 1),
                        "PS_MGTREFCLK1N" => (GtPin::ClkN, 1),
                        "PS_MGTREFCLK2P" => (GtPin::ClkP, 2),
                        "PS_MGTREFCLK2N" => (GtPin::ClkN, 2),
                        "PS_MGTREFCLK3P" => (GtPin::ClkP, 3),
                        "PS_MGTREFCLK3N" => (GtPin::ClkN, 3),
                        x => if let Some((n, b)) = split_num(x) {
                            match n {
                                "PS_MGTRTXP" => (GtPin::TxP, b),
                                "PS_MGTRTXN" => (GtPin::TxN, b),
                                "PS_MGTRRXP" => (GtPin::RxP, b),
                                "PS_MGTRRXN" => (GtPin::RxN, b),
                                _ => {
                                    println!("weird ps8 iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                                    continue;
                                }
                            }
                        } else {
                            println!("weird ps8 iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                            continue;
                        }
                    };
                    BondPin::GtByBank(bank, gtpin, bel)
                } else {
                    let pspin = match &pin.func[..pos] {
                        "PS_DONE" => PsPin::Done,
                        "PS_PROG_B" => PsPin::ProgB,
                        "PS_INIT_B" => PsPin::InitB,
                        "PS_ERROR_OUT" => PsPin::ErrorOut,
                        "PS_ERROR_STATUS" => PsPin::ErrorStatus,
                        "PS_PADI" => PsPin::PadI,
                        "PS_PADO" => PsPin::PadO,
                        "PS_POR_B" => PsPin::PorB,
                        "PS_SRST_B" => PsPin::SrstB,
                        "PS_REF_CLK" => PsPin::Clk,
                        "PS_JTAG_TDO" => PsPin::JtagTdo,
                        "PS_JTAG_TDI" => PsPin::JtagTdi,
                        "PS_JTAG_TCK" => PsPin::JtagTck,
                        "PS_JTAG_TMS" => PsPin::JtagTms,
                        "PS_DDR_ACT_N" => PsPin::DdrActN,
                        "PS_DDR_ALERT_N" => PsPin::DdrAlertN,
                        "PS_DDR_PARITY" => PsPin::DdrParity,
                        "PS_DDR_RAM_RST_N" => PsPin::DdrDrstB,
                        "PS_DDR_ZQ" => PsPin::DdrZq,
                        x => if let Some((n, b)) = split_num(x) {
                            match n {
                                "PS_MIO" => PsPin::Mio(b),
                                "PS_MODE" => PsPin::Mode(b),
                                "PS_DDR_DQ" => PsPin::DdrDq(b),
                                "PS_DDR_DM" => PsPin::DdrDm(b),
                                "PS_DDR_DQS_P" => PsPin::DdrDqsP(b),
                                "PS_DDR_DQS_N" => PsPin::DdrDqsN(b),
                                "PS_DDR_A" => PsPin::DdrA(b),
                                "PS_DDR_BA" => PsPin::DdrBa(b),
                                "PS_DDR_BG" => PsPin::DdrBg(b),
                                "PS_DDR_CKE" => PsPin::DdrCke(b),
                                "PS_DDR_ODT" => PsPin::DdrOdt(b),
                                "PS_DDR_CS_N" => PsPin::DdrCsB(b),
                                "PS_DDR_CK" => PsPin::DdrCkP(b),
                                "PS_DDR_CK_N" => PsPin::DdrCkN(b),
                                _ => {
                                    println!("weird ps8 iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                                    continue;
                                }
                            }
                        } else {
                            println!("weird ps8 iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                            continue;
                        }
                    };
                    BondPin::IoPs(bank, pspin)
                }
            } else {
                println!("unk iopad {pad} {f}", f=pin.func);
                continue;
            }
        } else {
            if let Some(p) = lookup_nonpad_pin(rd, pin) {
                p
            } else {
                println!("UNK FUNC {} {} {:?}", pkg, pin.func, pin);
                continue;
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks: Default::default(),
    }
}

pub fn ingest(rd: &Part) -> PreDevice {
    let (grids, grid_master, disabled) = make_grids(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(rd, pkg, &grids, grid_master, &disabled, pins),
        ));
    }
    let grids = grids.into_iter().map(|x| geom::Grid::Ultrascale(x)).collect();
    make_device_multi(rd, grids, grid_master, Vec::new(), bonds, disabled)
}

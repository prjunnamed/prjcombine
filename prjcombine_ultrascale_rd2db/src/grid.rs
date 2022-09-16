use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::{ColId, DieId};
use prjcombine_rawdump::{Coord, NodeId, Part, TkSiteSlot};
use prjcombine_ultrascale::{
    BramKind, CleLKind, CleMKind, ColSide, Column, ColumnKindLeft, ColumnKindRight, DisabledPart,
    DspKind, Grid, GridKind, HardColumn, HardRowKind, HdioIobId, IoColumn, IoRowKind, Ps, RegId,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};

use prjcombine_rdgrid::{extract_int_slr, find_rows, IntGrid};

fn make_columns(int: &IntGrid) -> EntityVec<ColId, Column> {
    let mut res: EntityVec<ColId, (Option<ColumnKindLeft>, Option<ColumnKindRight>)> =
        int.cols.map_values(|_| (None, None));
    for (tkn, delta, kind) in [
        ("CLEL_L", 1, ColumnKindLeft::CleL),
        ("CLE_M", 1, ColumnKindLeft::CleM(CleMKind::Plain)),
        ("CLE_M_R", 1, ColumnKindLeft::CleM(CleMKind::Plain)),
        ("CLEM", 1, ColumnKindLeft::CleM(CleMKind::Plain)),
        ("CLEM_R", 1, ColumnKindLeft::CleM(CleMKind::Plain)),
        (
            "INT_INTF_LEFT_TERM_PSS",
            1,
            ColumnKindLeft::CleM(CleMKind::Plain),
        ),
        ("BRAM", 2, ColumnKindLeft::Bram(BramKind::Plain)),
        ("URAM_URAM_FT", 2, ColumnKindLeft::Uram),
        ("INT_INT_INTERFACE_GT_LEFT_FT", 1, ColumnKindLeft::Gt(0)),
        ("INT_INTF_L_TERM_GT", 1, ColumnKindLeft::Gt(0)),
        ("INT_INT_INTERFACE_XIPHY_FT", 1, ColumnKindLeft::Io(0)),
        ("INT_INTF_LEFT_TERM_IO_FT", 1, ColumnKindLeft::Io(0)),
        ("INT_INTF_L_IO", 1, ColumnKindLeft::Io(0)),
    ] {
        for c in int.find_columns(&[tkn]) {
            res[int.lookup_column(c + delta)].0 = Some(kind);
        }
    }
    for (tkn, delta, kind) in [
        ("CLEL_R", 1, ColumnKindRight::CleL(CleLKind::Plain)),
        ("DSP", 2, ColumnKindRight::Dsp(DspKind::Plain)),
        ("URAM_URAM_FT", 2, ColumnKindRight::Uram),
        ("INT_INTERFACE_GT_R", 1, ColumnKindRight::Gt(0)),
        ("INT_INTF_R_TERM_GT", 1, ColumnKindRight::Gt(0)),
        ("INT_INTF_RIGHT_TERM_IO", 1, ColumnKindRight::Io(0)),
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
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::Hard(0));
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::Hard(0));
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
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM(CleMKind::Plain)));
        res[c].0 = Some(ColumnKindLeft::CleM(CleMKind::ClkBuf));
    }
    for c in int.find_columns(&["LAGUNA_TILE"]) {
        let c = int.lookup_column(c + 1);
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM(CleMKind::Plain)));
        res[c].0 = Some(ColumnKindLeft::CleM(CleMKind::Laguna));
    }
    for c in int.find_columns(&["LAG_LAG"]) {
        let c = int.lookup_column(c + 2);
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM(CleMKind::Plain)));
        res[c].0 = Some(ColumnKindLeft::CleM(CleMKind::Laguna));
    }
    for c in int.find_columns(&["RCLK_CLEL_R_DCG10_R"]) {
        let c = int.lookup_column(c - 1);
        assert_eq!(res[c].1, Some(ColumnKindRight::CleL(CleLKind::Plain)));
        res[c].1 = Some(ColumnKindRight::CleL(CleLKind::Dcg10));
    }
    for (tkn, kind) in [
        ("RCLK_RCLK_BRAM_L_AUXCLMP_FT", BramKind::AuxClmp),
        ("RCLK_RCLK_BRAM_L_BRAMCLMP_FT", BramKind::BramClmp),
        ("RCLK_BRAM_INTF_TD_L", BramKind::Td),
        ("RCLK_BRAM_INTF_TD_R", BramKind::Td),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column(c + 2);
            assert_eq!(res[c].0, Some(ColumnKindLeft::Bram(BramKind::Plain)));
            res[c].0 = Some(ColumnKindLeft::Bram(kind));
        }
    }
    for c in int.find_columns(&["RCLK_BRAM_L"]) {
        let c = int.lookup_column(c + 2);
        match res[c].0 {
            Some(ColumnKindLeft::Bram(BramKind::BramClmp)) => {
                res[c].0 = Some(ColumnKindLeft::Bram(BramKind::BramClmpMaybe))
            }
            Some(ColumnKindLeft::Bram(BramKind::AuxClmp)) => {
                res[c].0 = Some(ColumnKindLeft::Bram(BramKind::AuxClmpMaybe))
            }
            Some(ColumnKindLeft::Bram(BramKind::Plain)) => (),
            _ => unreachable!(),
        }
    }
    for c in int.find_columns(&["RCLK_DSP_CLKBUF_L"]) {
        let c = int.lookup_column(c - 2);
        assert_eq!(res[c].1, Some(ColumnKindRight::Dsp(DspKind::Plain)));
        res[c].1 = Some(ColumnKindRight::Dsp(DspKind::ClkBuf));
    }
    for c in int.find_columns(&["RCLK_DSP_INTF_CLKBUF_L"]) {
        let c = int.lookup_column(c - 1);
        assert_eq!(res[c].1, Some(ColumnKindRight::Dsp(DspKind::Plain)));
        res[c].1 = Some(ColumnKindRight::Dsp(DspKind::ClkBuf));
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

fn get_cols_hard(
    int: &IntGrid,
    dieid: DieId,
    disabled: &mut BTreeSet<DisabledPart>,
) -> Vec<HardColumn> {
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
            let reg = RegId::from_idx(int.lookup_row(y).to_idx() / 60);
            cells.insert((col, reg), kind);
            let crd = Coord {
                x: x as u16,
                y: y as u16,
            };
            let tile = &int.rd.tiles[&crd];
            if tile.sites.iter().next().is_none() && !tt.starts_with("DFE") {
                disabled.insert(DisabledPart::HardIp(dieid, col, reg));
            }
            if tt == "HDIO_BOT_RIGHT" {
                let sk = int.rd.slot_kinds.get("IOB").unwrap();
                let tk = &int.rd.tile_kinds[tile.kind];
                for i in 0..6 {
                    let slot = TkSiteSlot::Xy(sk, 0, 2 * i as u8);
                    let sid = tk.sites.get(&slot).unwrap().0;
                    if !tile.sites.contains_id(sid) {
                        disabled.insert(DisabledPart::HdioIob(
                            dieid,
                            col,
                            reg,
                            HdioIobId::from_idx(i),
                        ));
                    }
                }
                let tile = &int.rd.tiles[&crd.delta(0, 31)];
                let tk = &int.rd.tile_kinds[tile.kind];
                for i in 0..6 {
                    let slot = TkSiteSlot::Xy(sk, 0, 2 * i as u8);
                    let sid = tk.sites.get(&slot).unwrap().0;
                    if !tile.sites.contains_id(sid) {
                        disabled.insert(DisabledPart::HdioIob(
                            dieid,
                            col,
                            reg,
                            HdioIobId::from_idx(i + 6),
                        ));
                    }
                }
            }
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
                    let reg = RegId::from_idx(int.lookup_row(crd.y as i32).to_idx() / 60);
                    let tile = &int.rd.tiles[crd];
                    if let Some(&n) = tile.conn_wires.get(i) {
                        if vp_aux0.contains(&n) {
                            cells.insert((col, reg), HardRowKind::HdioAms);
                        }
                    }
                }
            }
        }
    }
    let cols: BTreeSet<ColId> = cells.keys().map(|&(c, _)| c).collect();
    let mut res = Vec::new();
    for col in cols {
        let mut regs = EntityVec::new();
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
            let reg = RegId::from_idx(int.lookup_row(y).to_idx() / 60);
            cells.insert((col, ColSide::Left, reg), kind);
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
            let reg = RegId::from_idx(int.lookup_row(y).to_idx() / 60);
            cells.insert((col, ColSide::Right, reg), kind);
        }
    }
    let cols: BTreeSet<(ColId, ColSide)> = cells.keys().map(|&(c, s, _)| (c, s)).collect();
    let mut res = Vec::new();
    for (col, side) in cols {
        let mut regs = EntityVec::new();
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

fn prepend_reg<T: Copy>(v: &mut EntityVec<RegId, T>, x: T) {
    *v = core::iter::once(x).chain(v.values().copied()).collect();
}

pub fn make_grids(rd: &Part) -> (EntityVec<DieId, Grid>, DieId, BTreeSet<DisabledPart>) {
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
    let mut disabled = BTreeSet::new();
    let mut dieid = DieId::from_idx(0);
    for w in rows_slr_split.windows(2) {
        let int = extract_int_slr(rd, &["INT"], &[], *w[0], *w[1]);
        let mut columns = make_columns(&int);
        let cols_vbrk = get_cols_vbrk(&int);
        let cols_fsr_gap = get_cols_fsr_gap(&int);
        let cols_hard = get_cols_hard(&int, dieid, &mut disabled);
        let cols_io = get_cols_io(&int);
        for (i, hc) in cols_hard.iter().enumerate() {
            assert_eq!(columns[hc.col - 1].r, ColumnKindRight::Hard(0));
            assert_eq!(columns[hc.col].l, ColumnKindLeft::Hard(0));
            columns[hc.col - 1].r = ColumnKindRight::Hard(i);
            columns[hc.col].l = ColumnKindLeft::Hard(i);
        }
        for (i, ioc) in cols_io.iter().enumerate() {
            match ioc.side {
                ColSide::Left => match columns[ioc.col].l {
                    ColumnKindLeft::Io(ref mut ci) | ColumnKindLeft::Gt(ref mut ci) => *ci = i,
                    _ => unreachable!(),
                },
                ColSide::Right => match columns[ioc.col].r {
                    ColumnKindRight::Io(ref mut ci) | ColumnKindRight::Gt(ref mut ci) => *ci = i,
                    _ => unreachable!(),
                },
            }
        }
        let is_alt_cfg = is_plus
            && int
                .find_tiles(&[
                    "CFG_M12BUF_CTR_RIGHT_CFG_OLY_BOT_L_FT",
                    "CFG_M12BUF_CTR_RIGHT_CFG_OLY_DK_BOT_L_FT",
                ])
                .is_empty();

        assert_eq!(int.rows.len() % 60, 0);
        grids.push(Grid {
            kind,
            columns,
            cols_vbrk,
            cols_fsr_gap,
            cols_hard,
            cols_io,
            regs: int.rows.len() / 60,
            ps: get_ps(&int),
            has_hbm: int.find_column(&["HBM_DMAH_FT"]).is_some(),
            is_dmc: int.find_column(&["FSR_DMC_TARGET_FT"]).is_some(),
            is_alt_cfg,
        });
        dieid += 1;
    }
    let tterms = find_rows(rd, &["INT_TERM_T"]);
    if !tterms.contains(&(rd.height as i32 - 1)) {
        if rd.part.contains("ku025") {
            let s0 = DieId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 3);
            assert_eq!(grids[s0].cols_hard.len(), 1);
            assert_eq!(grids[s0].cols_io.len(), 3);
            grids[s0].regs = 5;
            grids[s0].cols_hard[0].regs.push(HardRowKind::Pcie);
            grids[s0].cols_hard[0].regs.push(HardRowKind::Pcie);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[2].regs.push(IoRowKind::Gth);
            grids[s0].cols_io[2].regs.push(IoRowKind::Gth);
            disabled.insert(DisabledPart::Region(s0, RegId::from_idx(3)));
            disabled.insert(DisabledPart::Region(s0, RegId::from_idx(4)));
        } else if rd.part.contains("ku085") {
            let s0 = DieId::from_idx(0);
            let s1 = DieId::from_idx(1);
            assert_eq!(grids.len(), 2);
            assert_eq!(grids[s0].regs, 5);
            assert_eq!(grids[s1].regs, 4);
            assert_eq!(grids[s1].cols_hard.len(), 1);
            assert_eq!(grids[s1].cols_io.len(), 4);
            grids[s1].regs = 5;
            grids[s1].cols_hard[0].regs.push(HardRowKind::Pcie);
            grids[s1].cols_io[0].regs.push(IoRowKind::Gth);
            grids[s1].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s1].cols_io[2].regs.push(IoRowKind::Hpio);
            grids[s1].cols_io[3].regs.push(IoRowKind::Gth);
            assert_eq!(grids[s0], grids[s1]);
            disabled.insert(DisabledPart::Region(s1, RegId::from_idx(4)));
        } else if rd.part.contains("zu25dr") {
            let s0 = DieId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 6);
            assert_eq!(grids[s0].cols_io.len(), 3);
            grids[s0].regs = 8;
            grids[s0].cols_hard[0].regs.push(HardRowKind::Cmac);
            grids[s0].cols_hard[0].regs.push(HardRowKind::Pcie);
            grids[s0].cols_hard[1].regs.push(HardRowKind::Hdio);
            grids[s0].cols_hard[1].regs.push(HardRowKind::Hdio);
            grids[s0].cols_io[0].regs.push(IoRowKind::Gty);
            grids[s0].cols_io[0].regs.push(IoRowKind::Gty);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[2].regs.push(IoRowKind::HsDac);
            grids[s0].cols_io[2].regs.push(IoRowKind::HsDac);
            disabled.insert(DisabledPart::TopRow(s0, RegId::from_idx(5)));
            disabled.insert(DisabledPart::Region(s0, RegId::from_idx(6)));
            disabled.insert(DisabledPart::Region(s0, RegId::from_idx(7)));
        } else if rd.part.contains("ku19p") {
            let s0 = DieId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 9);
            assert_eq!(grids[s0].cols_io.len(), 2);
            assert_eq!(grids[s0].cols_hard.len(), 1);
            grids[s0].regs = 11;
            prepend_reg(&mut grids[s0].cols_hard[0].regs, HardRowKind::PciePlus);
            grids[s0].cols_hard[0].regs.push(HardRowKind::Cmac);
            prepend_reg(&mut grids[s0].cols_io[0].regs, IoRowKind::Hpio);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            prepend_reg(&mut grids[s0].cols_io[1].regs, IoRowKind::Gty);
            grids[s0].cols_io[1].regs.push(IoRowKind::Gtm);
            disabled.insert(DisabledPart::Region(s0, RegId::from_idx(0)));
            disabled.insert(DisabledPart::Region(s0, RegId::from_idx(10)));
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
            let s0 = DieId::from_idx(0);
            let s1 = DieId::from_idx(1);
            let s2 = DieId::from_idx(2);
            assert_eq!(grids.len(), 3);
            assert_eq!(grids[s0].regs, 4);
            assert_eq!(grids[s1].regs, 5);
            assert_eq!(grids[s2].regs, 5);
            assert_eq!(grids[s0].cols_io.len(), 4);
            grids[s0].regs = 5;
            prepend_reg(&mut grids[s0].cols_hard[0].regs, HardRowKind::Ilkn);
            prepend_reg(&mut grids[s0].cols_hard[1].regs, HardRowKind::Pcie);
            prepend_reg(&mut grids[s0].cols_io[0].regs, IoRowKind::Gty);
            prepend_reg(&mut grids[s0].cols_io[1].regs, IoRowKind::Hpio);
            prepend_reg(&mut grids[s0].cols_io[2].regs, IoRowKind::Hrio);
            prepend_reg(&mut grids[s0].cols_io[3].regs, IoRowKind::Gth);
            assert_eq!(grids[s0], grids[s1]);
            disabled.insert(DisabledPart::Region(s0, RegId::from_idx(0)));
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
    let grid_master = DieId::from_idx(grid_master.unwrap());
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
    if let Some((_, tk)) = rd.tile_kinds.get("FE_FE_FT") {
        if tk.sites.is_empty() {
            disabled.insert(DisabledPart::Sdfec);
        }
    }
    if let Some((_, tk)) = rd.tile_kinds.get("DFE_DFE_TILEA_FT") {
        if tk.sites.is_empty() {
            disabled.insert(DisabledPart::Dfe);
        }
    }
    if let Some((_, tk)) = rd.tile_kinds.get("VCU_VCU_FT") {
        if tk.sites.is_empty() {
            disabled.insert(DisabledPart::Vcu);
        }
    }
    for &crd in rd.tiles_by_kind_name("BLI_BLI_FT") {
        let tile = &rd.tiles[&crd];
        if tile.sites.iter().next().is_none() {
            disabled.insert(DisabledPart::HbmLeft);
        }
    }

    (grids, grid_master, disabled)
}

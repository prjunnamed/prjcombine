use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_interconnect::grid::{ColId, EdgeIoCoord, RowId, TileIobId};
use prjcombine_re_xilinx_rawdump::{Coord, Part, TkSiteSlot};
use prjcombine_spartan6::grid::{
    Column, ColumnIoKind, ColumnKind, DisabledPart, Grid, Gts, Mcb, McbIo, RegId, Row, SharedCfgPin,
};
use unnamed_entity::{EntityId, EntityVec};

use prjcombine_re_xilinx_rd2db_grid::{
    extract_int, find_column, find_columns, find_row, find_rows, find_tiles, IntGrid,
};

fn make_columns(rd: &Part, int: &IntGrid) -> EntityVec<ColId, Column> {
    let mut res: EntityVec<ColId, Option<ColumnKind>> = int.cols.map_values(|_| None);
    for c in find_columns(rd, &["CLEXL", "CLEXL_DUMMY"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::CleXL);
    }
    for c in find_columns(rd, &["CLEXM", "CLEXM_DUMMY"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::CleXM);
    }
    for c in find_columns(rd, &["BRAMSITE2", "BRAMSITE2_DUMMY"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["MACCSITE2"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["RIOI", "LIOI"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["CLKC"]) {
        res[int.lookup_column(c - 3)] = Some(ColumnKind::CleClk);
    }
    for c in find_columns(rd, &["GTPDUAL_DSP_FEEDTHRU"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::DspPlus);
    }
    res.map(|col, kind| Column {
        kind: kind.unwrap(),
        bio: {
            let co = Coord {
                x: int.cols[col] as u16 + 1,
                y: 2,
            };
            let ci = Coord {
                x: int.cols[col] as u16 + 1,
                y: 3,
            };
            let has_o = rd.tile_kinds.key(rd.tiles[&co].kind).ends_with("IOI_OUTER");
            let has_i = rd.tile_kinds.key(rd.tiles[&ci].kind).ends_with("IOI_INNER");
            match (has_o, has_i) {
                (false, false) => ColumnIoKind::None,
                (false, true) => ColumnIoKind::Inner,
                (true, false) => ColumnIoKind::Outer,
                (true, true) => ColumnIoKind::Both,
            }
        },
        tio: {
            let co = Coord {
                x: int.cols[col] as u16 + 1,
                y: rd.height - 3,
            };
            let ci = Coord {
                x: int.cols[col] as u16 + 1,
                y: rd.height - 4,
            };
            let has_o = rd.tile_kinds.key(rd.tiles[&co].kind).ends_with("IOI_OUTER");
            let has_i = rd.tile_kinds.key(rd.tiles[&ci].kind).ends_with("IOI_INNER");
            match (has_o, has_i) {
                (false, false) => ColumnIoKind::None,
                (false, true) => ColumnIoKind::Inner,
                (true, false) => ColumnIoKind::Outer,
                (true, true) => ColumnIoKind::Both,
            }
        },
    })
}

fn make_rows(rd: &Part, int: &IntGrid) -> EntityVec<RowId, Row> {
    int.rows.map_values(|&y| {
        let c_l = Coord { x: 3, y: y as u16 };
        let c_r = Coord {
            x: rd.width - 4,
            y: y as u16,
        };
        Row {
            lio: matches!(
                &rd.tile_kinds.key(rd.tiles[&c_l].kind)[..],
                "LIOI" | "LIOI_BRK"
            ),
            rio: matches!(
                &rd.tile_kinds.key(rd.tiles[&c_r].kind)[..],
                "RIOI" | "RIOI_BRK"
            ),
        }
    })
}

fn get_cols_clk_fold(rd: &Part, int: &IntGrid) -> Option<(ColId, ColId)> {
    let v: Vec<_> = find_columns(rd, &["DSP_HCLK_GCLK_FOLD"])
        .into_iter()
        .map(|x| int.lookup_column(x - 2))
        .collect();
    match v[..] {
        [] => None,
        [l, r] => Some((l, r)),
        _ => unreachable!(),
    }
}

fn get_cols_reg_buf(rd: &Part, int: &IntGrid) -> (ColId, ColId) {
    let l = if let Some(c) = find_column(rd, &["REGH_BRAM_FEEDTHRU_L_GCLK", "REGH_DSP_L"]) {
        int.lookup_column(c - 2)
    } else if let Some(c) = find_column(rd, &["REGH_CLEXM_INT_GCLKL"]) {
        int.lookup_column(c)
    } else {
        unreachable!()
    };
    let r = if let Some(c) = find_column(rd, &["REGH_BRAM_FEEDTHRU_R_GCLK", "REGH_DSP_R"]) {
        int.lookup_column(c - 2)
    } else if let Some(c) = find_column(rd, &["REGH_CLEXL_INT_CLK"]) {
        int.lookup_column(c)
    } else {
        unreachable!()
    };
    (l, r)
}

fn get_rows_midbuf(rd: &Part, int: &IntGrid) -> (RowId, RowId) {
    let b = int.lookup_row(find_row(rd, &["REG_V_MIDBUF_BOT"]).unwrap());
    let t = int.lookup_row(find_row(rd, &["REG_V_MIDBUF_TOP"]).unwrap());
    (b, t)
}

fn get_rows_hclkbuf(rd: &Part, int: &IntGrid) -> (RowId, RowId) {
    let b = int.lookup_row(find_row(rd, &["REG_V_HCLKBUF_BOT"]).unwrap());
    let t = int.lookup_row(find_row(rd, &["REG_V_HCLKBUF_TOP"]).unwrap());
    (b, t)
}

fn get_rows_bank_split(rd: &Part, int: &IntGrid) -> Option<(RowId, RowId)> {
    if let Some(x) = find_row(rd, &["MCB_CAP_INT_BRK"]) {
        let l = int.lookup_row(x);
        let r = int.lookup_row(x) - 1;
        Some((l, r))
    } else {
        None
    }
}

fn get_row_mcb_split(rd: &Part, int: &IntGrid) -> Option<RowId> {
    find_row(rd, &["MCB_CAP_INT_BRK"]).map(|x| int.lookup_row(x))
}

fn get_rows_pci_ce_split(rd: &Part, int: &IntGrid) -> (RowId, RowId) {
    let b = int.lookup_row_inter(find_row(rd, &["HCLK_IOIL_BOT_SPLIT"]).unwrap());
    let t = int.lookup_row_inter(find_row(rd, &["HCLK_IOIL_TOP_SPLIT"]).unwrap());
    (b, t)
}

fn get_gts(rd: &Part, int: &IntGrid) -> Gts {
    let vt: Vec<_> = find_columns(rd, &["GTPDUAL_TOP", "GTPDUAL_TOP_UNUSED"])
        .into_iter()
        .map(|x| int.lookup_column(x - 2))
        .collect();
    let vb: Vec<_> = find_columns(rd, &["GTPDUAL_BOT", "GTPDUAL_BOT_UNUSED"])
        .into_iter()
        .map(|x| int.lookup_column(x - 2))
        .collect();
    match (&vt[..], &vb[..]) {
        (&[], &[]) => Gts::None,
        (&[l], &[]) => Gts::Single(l),
        (&[l, r], &[]) => Gts::Double(l, r),
        (&[l, r], &[_, _]) => Gts::Quad(l, r),
        _ => unreachable!(),
    }
}

fn get_mcbs(rd: &Part, int: &IntGrid) -> Vec<Mcb> {
    let mut res = Vec::new();
    #[allow(non_snake_case)]
    let P = |row, bel| McbIo {
        row,
        iob: TileIobId::from_idx(bel),
    };
    for r in find_rows(rd, &["MCB_L", "MCB_DUMMY"]) {
        let row_mcb = int.lookup_row(r - 6);
        res.push(Mcb {
            row_mcb,
            row_mui: [
                row_mcb - 13,
                row_mcb - 16,
                row_mcb - 19,
                row_mcb - 22,
                row_mcb - 25,
                row_mcb - 28,
                row_mcb - 31,
                row_mcb - 34,
            ],
            iop_dq: [
                row_mcb - 20,
                row_mcb - 17,
                row_mcb - 10,
                row_mcb - 11,
                row_mcb - 23,
                row_mcb - 26,
                row_mcb - 32,
                row_mcb - 35,
            ],
            iop_dqs: [row_mcb - 14, row_mcb - 29],
            io_dm: [P(row_mcb - 9, 0), P(row_mcb - 9, 1)],
            iop_clk: row_mcb - 3,
            io_addr: [
                P(row_mcb - 2, 1),
                P(row_mcb - 2, 0),
                P(row_mcb + 12, 0),
                P(row_mcb - 4, 1),
                P(row_mcb + 14, 0),
                P(row_mcb - 5, 1),
                P(row_mcb - 5, 0),
                P(row_mcb + 12, 1),
                P(row_mcb + 15, 1),
                P(row_mcb + 15, 0),
                P(row_mcb + 14, 1),
                P(row_mcb + 18, 0),
                P(row_mcb + 16, 0),
                P(row_mcb + 20, 1),
                P(row_mcb + 20, 0),
            ],
            io_ba: [P(row_mcb - 1, 1), P(row_mcb - 1, 0), P(row_mcb + 13, 0)],
            io_ras: P(row_mcb - 6, 1),
            io_cas: P(row_mcb - 6, 0),
            io_we: P(row_mcb + 13, 1),
            io_odt: P(row_mcb - 4, 0),
            io_cke: P(row_mcb + 16, 1),
            io_reset: P(row_mcb + 18, 1),
        });
    }
    for r in find_rows(rd, &["MCB_L_BOT"]) {
        let row_mcb = int.lookup_row(r - 6);
        res.push(Mcb {
            row_mcb,
            row_mui: [
                row_mcb - 21,
                row_mcb - 24,
                row_mcb - 27,
                row_mcb - 30,
                row_mcb - 34,
                row_mcb - 37,
                row_mcb - 40,
                row_mcb - 43,
            ],
            iop_dq: [
                row_mcb - 28,
                row_mcb - 25,
                row_mcb - 18,
                row_mcb - 19,
                row_mcb - 31,
                row_mcb - 35,
                row_mcb - 41,
                row_mcb - 44,
            ],
            iop_dqs: [row_mcb - 22, row_mcb - 38],
            io_dm: [P(row_mcb - 17, 0), P(row_mcb - 17, 1)],
            iop_clk: row_mcb - 8,
            io_addr: [
                P(row_mcb - 7, 1),
                P(row_mcb - 7, 0),
                P(row_mcb - 4, 0),
                P(row_mcb - 10, 1),
                P(row_mcb - 1, 0),
                P(row_mcb - 13, 1),
                P(row_mcb - 13, 0),
                P(row_mcb - 4, 1),
                P(row_mcb + 12, 1),
                P(row_mcb + 12, 0),
                P(row_mcb - 1, 1),
                P(row_mcb + 14, 0),
                P(row_mcb + 13, 0),
                P(row_mcb + 15, 1),
                P(row_mcb + 15, 0),
            ],
            io_ba: [P(row_mcb - 5, 1), P(row_mcb - 5, 0), P(row_mcb - 2, 0)],
            io_ras: P(row_mcb - 14, 1),
            io_cas: P(row_mcb - 14, 0),
            io_we: P(row_mcb - 2, 1),
            io_odt: P(row_mcb - 10, 0),
            io_cke: P(row_mcb + 13, 1),
            io_reset: P(row_mcb + 14, 1),
        });
    }
    res.sort_by_key(|x| x.row_mcb);
    res
}

fn has_encrypt(rd: &Part) -> bool {
    for pins in rd.packages.values() {
        for pin in pins {
            if pin.func == "VBATT" {
                return true;
            }
        }
    }
    false
}

fn set_cfg(grid: &mut Grid, cfg: SharedCfgPin, coord: EdgeIoCoord) {
    let old = grid.cfg_io.insert(cfg, coord);
    assert!(old.is_none() || old == Some(coord));
}

fn handle_spec_io(rd: &Part, grid: &mut Grid, int: &IntGrid) {
    let mut io_lookup = HashMap::new();
    for (&crd, tile) in &rd.tiles {
        let tkn = rd.tile_kinds.key(tile.kind);
        let tk = &rd.tile_kinds[tile.kind];
        for (k, v) in &tile.sites {
            if let &TkSiteSlot::Indexed(sn, idx) = tk.sites.key(k) {
                if rd.slot_kinds[sn] == "IOB" {
                    let crd = if tkn.starts_with('T') {
                        EdgeIoCoord::T(
                            int.lookup_column(crd.x.into()),
                            TileIobId::from_idx(3 - (idx as usize)),
                        )
                    } else if tkn.starts_with('B') {
                        EdgeIoCoord::B(
                            int.lookup_column(crd.x.into()),
                            TileIobId::from_idx([1, 0, 2, 3][idx as usize]),
                        )
                    } else if tkn.starts_with('L') {
                        EdgeIoCoord::L(
                            int.lookup_row(crd.y.into()),
                            TileIobId::from_idx(idx as usize ^ 1),
                        )
                    } else if tkn.starts_with('R') {
                        EdgeIoCoord::R(
                            int.lookup_row(crd.y.into()),
                            TileIobId::from_idx(idx as usize ^ 1),
                        )
                    } else {
                        unreachable!();
                    };
                    io_lookup.insert(v.clone(), crd);
                }
            }
        }
    }
    for pins in rd.packages.values() {
        for pin in pins {
            if let Some(ref pad) = pin.pad {
                if !pad.starts_with("PAD") {
                    continue;
                }
                let coord = io_lookup[pad];
                let mut f = pin.func.strip_prefix("IO_L").unwrap();
                f = &f[f.find('_').unwrap() + 1..];
                if f.starts_with("GCLK") {
                    // ignore
                    f = &f[f.find('_').unwrap() + 1..];
                }
                if f.starts_with("IRDY") || f.starts_with("TRDY") {
                    // ignore
                    f = &f[f.find('_').unwrap() + 1..];
                }
                for (p, c) in [
                    ("M0_CMPMISO_", SharedCfgPin::M0),
                    ("M1_", SharedCfgPin::M1),
                    ("CCLK_", SharedCfgPin::Cclk),
                    ("CSO_B_", SharedCfgPin::CsoB),
                    ("INIT_B_", SharedCfgPin::InitB),
                    ("RDWR_B_", SharedCfgPin::RdWrB),
                    ("AWAKE_", SharedCfgPin::Awake),
                    ("FCS_B_", SharedCfgPin::FcsB),
                    ("FOE_B_", SharedCfgPin::FoeB),
                    ("FWE_B_", SharedCfgPin::FweB),
                    ("LDC_", SharedCfgPin::Ldc),
                    ("HDC_", SharedCfgPin::Hdc),
                    ("DOUT_BUSY_", SharedCfgPin::Dout),
                    ("D0_DIN_MISO_MISO1_", SharedCfgPin::Data(0)),
                    ("D1_MISO2_", SharedCfgPin::Data(1)),
                    ("D2_MISO3_", SharedCfgPin::Data(2)),
                    ("MOSI_CSI_B_MISO0_", SharedCfgPin::Mosi),
                    ("CMPCLK_", SharedCfgPin::CmpClk),
                    ("CMPMOSI_", SharedCfgPin::CmpMosi),
                    ("USERCCLK_", SharedCfgPin::UserCclk),
                    ("HSWAPEN_", SharedCfgPin::HswapEn),
                ] {
                    if let Some(nf) = f.strip_prefix(p) {
                        f = nf;
                        set_cfg(grid, c, coord);
                    }
                }
                if f.starts_with('A') {
                    let pos = f.find('_').unwrap();
                    let a = f[1..pos].parse().unwrap();
                    set_cfg(grid, SharedCfgPin::Addr(a), coord);
                    f = &f[pos + 1..];
                }
                if f.starts_with('D') {
                    let pos = f.find('_').unwrap();
                    let a = f[1..pos].parse().unwrap();
                    set_cfg(grid, SharedCfgPin::Data(a), coord);
                    f = &f[pos + 1..];
                }
                if f.starts_with("SCP") {
                    let pos = f.find('_').unwrap();
                    let a = f[3..pos].parse().unwrap();
                    set_cfg(grid, SharedCfgPin::Scp(a), coord);
                    f = &f[pos + 1..];
                }
                if let Some(nf) = f.strip_prefix("VREF_") {
                    f = nf;
                }
                if f.starts_with('M') {
                    let (col, mi) = match &f[0..2] {
                        "M1" => (grid.columns.last_id().unwrap(), 0),
                        "M3" => (grid.columns.first_id().unwrap(), 0),
                        "M4" => (grid.columns.first_id().unwrap(), 1),
                        "M5" => (grid.columns.last_id().unwrap(), 1),
                        _ => unreachable!(),
                    };
                    let (io_col, io_row, iob) = match coord {
                        EdgeIoCoord::L(row, iob) => (grid.col_lio(), row, iob),
                        EdgeIoCoord::R(row, iob) => (grid.col_rio(), row, iob),
                        _ => unreachable!(),
                    };
                    assert_eq!(io_col, col);
                    let mcb = &grid.mcbs[mi];
                    let epos = f.find('_').unwrap();
                    let mf = &f[2..epos];
                    match mf {
                        "RASN" => {
                            assert_eq!(io_row, mcb.io_ras.row);
                            assert_eq!(iob, mcb.io_ras.iob);
                        }
                        "CASN" => {
                            assert_eq!(io_row, mcb.io_cas.row);
                            assert_eq!(iob, mcb.io_cas.iob);
                        }
                        "WE" => {
                            assert_eq!(io_row, mcb.io_we.row);
                            assert_eq!(iob, mcb.io_we.iob);
                        }
                        "ODT" => {
                            assert_eq!(io_row, mcb.io_odt.row);
                            assert_eq!(iob, mcb.io_odt.iob);
                        }
                        "CKE" => {
                            assert_eq!(io_row, mcb.io_cke.row);
                            assert_eq!(iob, mcb.io_cke.iob);
                        }
                        "RESET" => {
                            assert_eq!(io_row, mcb.io_reset.row);
                            assert_eq!(iob, mcb.io_reset.iob);
                        }
                        "LDM" => {
                            assert_eq!(io_row, mcb.io_dm[0].row);
                            assert_eq!(iob, mcb.io_dm[0].iob);
                        }
                        "UDM" => {
                            assert_eq!(io_row, mcb.io_dm[1].row);
                            assert_eq!(iob, mcb.io_dm[1].iob);
                        }
                        "LDQS" => {
                            assert_eq!(io_row, mcb.iop_dqs[0]);
                            assert_eq!(iob.to_idx(), 1);
                        }
                        "LDQSN" => {
                            assert_eq!(io_row, mcb.iop_dqs[0]);
                            assert_eq!(iob.to_idx(), 0);
                        }
                        "UDQS" => {
                            assert_eq!(io_row, mcb.iop_dqs[1]);
                            assert_eq!(iob.to_idx(), 1);
                        }
                        "UDQSN" => {
                            assert_eq!(io_row, mcb.iop_dqs[1]);
                            assert_eq!(iob.to_idx(), 0);
                        }
                        "CLK" => {
                            assert_eq!(io_row, mcb.iop_clk);
                            assert_eq!(iob.to_idx(), 1);
                        }
                        "CLKN" => {
                            assert_eq!(io_row, mcb.iop_clk);
                            assert_eq!(iob.to_idx(), 0);
                        }
                        _ => {
                            if let Some(i) = mf.strip_prefix('A') {
                                let i: usize = i.parse().unwrap();
                                assert_eq!(io_row, mcb.io_addr[i].row);
                                assert_eq!(iob, mcb.io_addr[i].iob);
                            } else if let Some(i) = mf.strip_prefix("BA") {
                                let i: usize = i.parse().unwrap();
                                assert_eq!(io_row, mcb.io_ba[i].row);
                                assert_eq!(iob, mcb.io_ba[i].iob);
                            } else if let Some(i) = mf.strip_prefix("DQ") {
                                let i: usize = i.parse().unwrap();
                                assert_eq!(io_row, mcb.iop_dq[i / 2]);
                                assert_eq!(iob.to_idx(), (i % 2) ^ 1);
                            } else {
                                println!("MCB {mf}");
                            }
                        }
                    }
                    f = &f[epos + 1..];
                }
                if !matches!(f, "0" | "1" | "2" | "3" | "4" | "5") {
                    println!("FUNC {f}");
                }
            }
        }
    }
}

pub fn make_grid(rd: &Part) -> (Grid, BTreeSet<DisabledPart>) {
    let int = extract_int(
        rd,
        &[
            "INT",
            "INT_BRK",
            "INT_BRAM",
            "INT_BRAM_BRK",
            "IOI_INT",
            "LIOI_INT",
        ],
        &[],
    );
    let mut disabled = BTreeSet::new();
    if !find_tiles(rd, &["GTPDUAL_TOP_UNUSED"]).is_empty() {
        disabled.insert(DisabledPart::Gtp);
    }
    if !find_tiles(rd, &["MCB_DUMMY"]).is_empty() {
        disabled.insert(DisabledPart::Mcb);
    }
    for c in find_columns(rd, &["CLEXL_DUMMY", "CLEXM_DUMMY"]) {
        let c = int.lookup_column(c - 1);
        disabled.insert(DisabledPart::ClbColumn(c));
    }
    for (c, r) in find_tiles(rd, &["BRAMSITE2_DUMMY"]) {
        let c = int.lookup_column(c - 2);
        let r = RegId::from_idx(int.lookup_row(r).to_idx() / 16);
        disabled.insert(DisabledPart::BramRegion(c, r));
    }
    for (c, r) in find_tiles(rd, &["MACCSITE2_DUMMY"]) {
        let c = int.lookup_column(c - 2);
        let r = RegId::from_idx(int.lookup_row(r).to_idx() / 16);
        disabled.insert(DisabledPart::DspRegion(c, r));
    }
    let columns = make_columns(rd, &int);
    let rows = make_rows(rd, &int);
    let col_clk = columns
        .iter()
        .find_map(|(k, &v)| {
            if v.kind == ColumnKind::CleClk {
                Some(k)
            } else {
                None
            }
        })
        .unwrap();
    let mut grid = Grid {
        columns,
        col_clk,
        cols_clk_fold: get_cols_clk_fold(rd, &int),
        cols_reg_buf: get_cols_reg_buf(rd, &int),
        rows,
        rows_midbuf: get_rows_midbuf(rd, &int),
        rows_hclkbuf: get_rows_hclkbuf(rd, &int),
        rows_bank_split: get_rows_bank_split(rd, &int),
        rows_pci_ce_split: get_rows_pci_ce_split(rd, &int),
        row_mcb_split: get_row_mcb_split(rd, &int),
        gts: get_gts(rd, &int),
        mcbs: get_mcbs(rd, &int),
        cfg_io: BTreeMap::new(),
        has_encrypt: has_encrypt(rd),
    };
    handle_spec_io(rd, &mut grid, &int);
    (grid, disabled)
}

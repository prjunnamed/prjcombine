use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, RowId};
use prjcombine_re_xilinx_rawdump::{Coord, Part, TkSiteSlot};
use prjcombine_virtex2::{
    chip::{Chip, ChipKind, Column, ColumnIoKind, ColumnKind, Dcms, RowIoKind, SharedCfgPad},
    defs,
};

use prjcombine_re_xilinx_rd2db_grid::{
    IntGrid, extract_int, find_column, find_columns, find_row, find_rows, split_num,
};

fn get_kind(rd: &Part) -> ChipKind {
    match &rd.family[..] {
        "virtex2" => ChipKind::Virtex2,
        "virtex2p" => {
            if find_columns(rd, &["MK_B_IOIS"]).is_empty() {
                ChipKind::Virtex2P
            } else {
                ChipKind::Virtex2PX
            }
        }
        "spartan3" => ChipKind::Spartan3,
        "fpgacore" => ChipKind::FpgaCore,
        "spartan3e" => ChipKind::Spartan3E,
        "spartan3a" => ChipKind::Spartan3A,
        "spartan3adsp" => ChipKind::Spartan3ADsp,
        _ => panic!("unknown family {}", rd.family),
    }
}

fn make_columns(rd: &Part, int: &IntGrid, kind: ChipKind) -> EntityVec<ColId, Column> {
    let mut res = EntityVec::new();
    res.push(Column {
        kind: ColumnKind::Io,
        io: ColumnIoKind::None,
    });
    for _ in 0..(int.cols.len() - 2) {
        res.push(Column {
            kind: ColumnKind::Clb,
            io: ColumnIoKind::None,
        });
    }
    res.push(Column {
        kind: ColumnKind::Io,
        io: ColumnIoKind::None,
    });
    let bram_cont = match kind {
        ChipKind::Spartan3E | ChipKind::Spartan3A => 4,
        ChipKind::Spartan3ADsp => 3,
        _ => 0,
    };
    for rc in find_columns(rd, &["BRAM0", "BRAM0_SMALL"]) {
        let c = int.lookup_column(rc);
        res[c].kind = ColumnKind::Bram;
        if bram_cont != 0 {
            for d in 1..bram_cont {
                res[c + d].kind = ColumnKind::BramCont(d as u8);
            }
        }
    }
    for rc in find_columns(rd, &["MACC0_SMALL"]) {
        let c = int.lookup_column(rc);
        res[c].kind = ColumnKind::Dsp;
    }
    res
}

fn get_cols_io(rd: &Part, int: &IntGrid, kind: ChipKind, cols: &mut EntityVec<ColId, Column>) {
    let mut col = cols.first_id().unwrap() + 1;
    let col_r = cols.last_id().unwrap();
    while col != col_r {
        match kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                let c0 = Coord {
                    x: int.cols[col] as u16,
                    y: 0,
                };
                let c1 = Coord {
                    x: (int.cols[col] + 1) as u16,
                    y: 0,
                };
                let cu0 = Coord {
                    x: int.cols[col] as u16,
                    y: 1,
                };
                let cu1 = Coord {
                    x: (int.cols[col] + 1) as u16,
                    y: 1,
                };
                let tk0 = &rd.tile_kinds.key(rd.tiles[&c0].kind)[..];
                let tk1 = &rd.tile_kinds.key(rd.tiles[&c1].kind)[..];
                let tku0 = &rd.tile_kinds.key(rd.tiles[&cu0].kind)[..];
                let tku1 = &rd.tile_kinds.key(rd.tiles[&cu1].kind)[..];
                match (tk0, tk1) {
                    ("BTERM012" | "BCLKTERM012" | "ML_BCLKTERM012", "BTERM323") => {
                        for i in 0..2 {
                            cols[col].io = ColumnIoKind::DoubleW(i as u8);
                            col += 1;
                        }
                    }
                    ("BTERM010", "BTERM123" | "BCLKTERM123" | "ML_BCLKTERM123") => {
                        if tku1 == "MK_B_IOIS" {
                            for i in 0..2 {
                                cols[col].io = ColumnIoKind::DoubleEClk(i as u8);
                                col += 1;
                            }
                        } else {
                            for i in 0..2 {
                                cols[col].io = ColumnIoKind::DoubleE(i as u8);
                                col += 1;
                            }
                        }
                    }
                    ("BTERM123", _) => {
                        if tku0 == "ML_TBS_IOIS" {
                            cols[col].io = ColumnIoKind::SingleWAlt;
                        } else {
                            cols[col].io = ColumnIoKind::SingleW;
                        }
                        col += 1;
                    }
                    ("BTERM012", _) => {
                        if tku0 == "ML_TBS_IOIS" {
                            cols[col].io = ColumnIoKind::SingleEAlt;
                        } else {
                            cols[col].io = ColumnIoKind::SingleE;
                        }
                        col += 1;
                    }
                    ("BBTERM", _) => {
                        col += 1;
                    }
                    ("BGIGABIT_IOI_TERM" | "BGIGABIT10_IOI_TERM", _) => {
                        col += 3;
                    }
                    _ => panic!("unknown tk {tk0} {tk1}"),
                }
            }
            ChipKind::Spartan3 => {
                if cols[col].kind == ColumnKind::Bram {
                    col += 1;
                } else {
                    for i in 0..2 {
                        cols[col].io = ColumnIoKind::Double(i as u8);
                        col += 1;
                    }
                }
            }
            ChipKind::FpgaCore => {
                cols[col].io = ColumnIoKind::Single;
                col += 1;
            }
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                for i in 0..2 {
                    cols[col].io = ColumnIoKind::Double(i as u8);
                    col += 1;
                }
            }
            ChipKind::Spartan3E => {
                let c = Coord {
                    x: int.cols[col] as u16,
                    y: 0,
                };
                let tk = &rd.tile_kinds.key(rd.tiles[&c].kind)[..];
                match tk {
                    "BTERM4" | "BTERM4_BRAM2" | "BTERM4CLK" => {
                        for i in 0..4 {
                            cols[col].io = ColumnIoKind::Quad(i as u8);
                            col += 1;
                        }
                    }
                    "BTERM3" => {
                        for i in 0..3 {
                            cols[col].io = ColumnIoKind::Triple(i as u8);
                            col += 1;
                        }
                    }
                    "BTERM2" => {
                        for i in 0..2 {
                            cols[col].io = ColumnIoKind::Double(i as u8);
                            col += 1;
                        }
                    }
                    "BTERM1" => {
                        cols[col].io = ColumnIoKind::Single;
                        col += 1;
                    }
                    _ => panic!("unknown tk {tk}"),
                }
            }
        }
    }
}

fn get_col_clk(rd: &Part, int: &IntGrid) -> ColId {
    int.lookup_column(find_column(rd, &["CLKC", "CLKC_50A", "CLKC_LL"]).unwrap() + 1)
}

fn get_cols_clkv(rd: &Part, int: &IntGrid) -> Option<(ColId, ColId)> {
    let cols: Vec<_> = find_columns(rd, &["GCLKV"]).into_iter().collect();
    if cols.is_empty() {
        None
    } else {
        assert_eq!(cols.len(), 2);
        let l = int.lookup_column(cols[0] + 1);
        let r = int.lookup_column(cols[1] + 1);
        Some((l, r))
    }
}

fn get_gt_bank(rd: &Part, c: Coord) -> u32 {
    for s in rd.tiles[&c].sites.values() {
        if let Some(b) = s.strip_prefix("RXNPAD") {
            return b.parse::<u32>().unwrap();
        }
    }
    unreachable!();
}

fn get_cols_gt(rd: &Part, int: &IntGrid) -> BTreeMap<ColId, (u32, u32)> {
    let mut res = BTreeMap::new();
    for rc in find_columns(rd, &["BGIGABIT_INT0"]) {
        let c = int.lookup_column(rc);
        let bb = get_gt_bank(
            rd,
            Coord {
                x: (rc + 1) as u16,
                y: 2,
            },
        );
        let bt = get_gt_bank(
            rd,
            Coord {
                x: (rc + 1) as u16,
                y: rd.height - 6,
            },
        );
        res.insert(c, (bb, bt));
    }
    for rc in find_columns(rd, &["BGIGABIT10_INT0"]) {
        let c = int.lookup_column(rc);
        let bb = get_gt_bank(
            rd,
            Coord {
                x: (rc + 1) as u16,
                y: 2,
            },
        );
        let bt = get_gt_bank(
            rd,
            Coord {
                x: (rc + 1) as u16,
                y: rd.height - 11,
            },
        );
        res.insert(c, (bb, bt));
    }
    res
}

fn get_rows(rd: &Part, int: &IntGrid, kind: ChipKind) -> EntityVec<RowId, RowIoKind> {
    let mut res = EntityVec::new();
    res.push(RowIoKind::None);
    while res.len() < int.rows.len() - 1 {
        match kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                if res.len() < int.rows.len() / 2 {
                    for i in 0..2 {
                        res.push(RowIoKind::DoubleS(i));
                    }
                } else {
                    for i in 0..2 {
                        res.push(RowIoKind::DoubleN(i));
                    }
                }
            }
            ChipKind::Spartan3 | ChipKind::FpgaCore => {
                res.push(RowIoKind::Single);
            }
            ChipKind::Spartan3E => {
                let c = Coord {
                    x: 0,
                    y: int.rows[res.next_id()] as u16,
                };
                let tk = &rd.tile_kinds.key(rd.tiles[&c].kind)[..];
                match tk {
                    "LTERM4" | "LTERM4B" | "LTERM4CLK" => {
                        for i in 0..4 {
                            res.push(RowIoKind::Quad(i));
                        }
                    }
                    "LTERM3" => {
                        for i in 0..3 {
                            res.push(RowIoKind::Triple(i));
                        }
                    }
                    "LTERM2" => {
                        for i in 0..2 {
                            res.push(RowIoKind::Double(i));
                        }
                    }
                    "LTERM1" => {
                        res.push(RowIoKind::Single);
                    }
                    _ => panic!("unknown tk {tk}"),
                }
            }
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                for i in 0..4 {
                    res.push(RowIoKind::Quad(i));
                }
            }
        }
    }
    res.push(RowIoKind::None);
    assert_eq!(res.len(), int.rows.len());
    res
}

fn get_rows_ram(rd: &Part, int: &IntGrid, kind: ChipKind) -> Option<(RowId, RowId)> {
    if kind == ChipKind::Spartan3E {
        let b = int.lookup_row(find_row(rd, &["COB_TERM_B"]).unwrap());
        let t = int.lookup_row(find_row(rd, &["COB_TERM_T"]).unwrap());
        Some((b, t))
    } else {
        None
    }
}

fn get_rows_hclk(rd: &Part, int: &IntGrid) -> Vec<(RowId, RowId, RowId)> {
    let rows_hclk: Vec<_> = find_rows(rd, &["GCLKH"])
        .into_iter()
        .map(|r| int.lookup_row(r - 1) + 1)
        .collect();
    let mut rows_brk = BTreeSet::new();
    for r in find_rows(rd, &["BRKH", "CLKH", "CLKH_LL"]) {
        rows_brk.insert(int.lookup_row(r - 1) + 1);
    }
    for r in find_rows(rd, &["CENTER_SMALL_BRK"]) {
        rows_brk.insert(int.lookup_row(r) + 1);
    }
    let mut rows_brk_d = rows_brk.clone();
    rows_brk.insert(int.rows.next_id());
    rows_brk_d.insert(int.rows.first_id().unwrap());
    assert_eq!(rows_hclk.len(), rows_brk.len());
    assert_eq!(rows_hclk.len(), rows_brk_d.len());
    rows_hclk
        .into_iter()
        .zip(rows_brk_d)
        .zip(rows_brk)
        .map(|((a, b), c)| (a, b, c))
        .collect()
}

fn get_row_pci(rd: &Part, int: &IntGrid, kind: ChipKind) -> Option<RowId> {
    match kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
            Some(int.lookup_row(find_row(rd, &["REG_L"]).unwrap() + 1))
        }
        _ => None,
    }
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(ColId, RowId)> {
    let mut res = Vec::new();
    for tt in ["LLPPC_X0Y0_INT", "LLPPC_X1Y0_INT"] {
        if let Some((_, tk)) = rd.tile_kinds.get(tt) {
            assert_eq!(tk.tiles.len(), 1);
            let tile = &tk.tiles[0];
            let x = int.lookup_column(tile.x as i32);
            let y = int.lookup_row((tile.y - 1) as i32);
            res.push((x, y));
        }
    }
    res
}

fn get_dcms(rd: &Part, kind: ChipKind) -> Option<Dcms> {
    match kind {
        ChipKind::Spartan3E => {
            if !find_columns(rd, &["DCM_H_BL_CENTER"]).is_empty() {
                Some(Dcms::Eight)
            } else if !find_columns(rd, &["DCM_BL_CENTER"]).is_empty() {
                Some(Dcms::Four)
            } else {
                Some(Dcms::Two)
            }
        }
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
            if !find_columns(rd, &["DCM_BGAP"]).is_empty() {
                Some(Dcms::Eight)
            } else if !find_columns(rd, &["DCM_BL_CENTER"]).is_empty() {
                Some(Dcms::Four)
            } else {
                Some(Dcms::Two)
            }
        }
        _ => None,
    }
}

fn get_has_ll(rd: &Part) -> bool {
    !find_columns(rd, &["CLKV_LL"]).is_empty()
}

fn get_has_small_int(rd: &Part) -> bool {
    !find_columns(rd, &["CENTER_SMALL"]).is_empty()
}

fn handle_spec_io(rd: &Part, chip: &mut Chip, int: &IntGrid) {
    if chip.kind == ChipKind::FpgaCore {
        return;
    }
    let mut io_lookup = HashMap::new();
    for (&crd, tile) in &rd.tiles {
        let tk = &rd.tile_kinds[tile.kind];
        for (k, v) in &tile.sites {
            if let &TkSiteSlot::Indexed(sn, idx) = tk.sites.key(k)
                && rd.slot_kinds[sn] == "IOB"
            {
                let col = int.lookup_column(crd.x.into());
                let row = int.lookup_row(crd.y.into());
                let io = chip.get_io_crd(
                    CellCoord::new(DieId::from_idx(0), col, row)
                        .bel(defs::bslots::IOI[idx as usize]),
                );
                io_lookup.insert(v.clone(), io);
            }
        }
    }
    for pins in rd.packages.values() {
        let mut vrp = BTreeMap::new();
        let mut vrn = BTreeMap::new();
        let mut alt_vrp = BTreeMap::new();
        let mut alt_vrn = BTreeMap::new();
        for pin in pins {
            if let Some(ref pad) = pin.pad {
                if !pad.starts_with("PAD") && !pad.starts_with("IPAD") {
                    continue;
                }
                let coord = io_lookup[pad];
                for f in pin.func.split('/').skip(1) {
                    if f.starts_with("VRP_") {
                        vrp.insert(pin.vref_bank.unwrap(), coord);
                    } else if f.starts_with("VRN_") {
                        vrn.insert(pin.vref_bank.unwrap(), coord);
                    } else if f.starts_with("ALT_VRP_") {
                        alt_vrp.insert(pin.vref_bank.unwrap(), coord);
                    } else if f.starts_with("ALT_VRN_") {
                        alt_vrn.insert(pin.vref_bank.unwrap(), coord);
                    } else if f.starts_with("GCLK")
                        || f.starts_with("LHCLK")
                        || f.starts_with("RHCLK")
                        || f.starts_with("IRDY")
                        || f.starts_with("TRDY")
                        || f.starts_with("VREF_")
                    {
                        // ignore
                    } else {
                        let cfg = match f {
                            "No_Pair" | "DIN" | "BUSY" | "MOSI" | "MISO" => continue,
                            "CS_B" => SharedCfgPad::CsiB,
                            "INIT_B" => SharedCfgPad::InitB,
                            "RDWR_B" => SharedCfgPad::RdWrB,
                            "DOUT" => SharedCfgPad::Dout,
                            // Spartan 3E, Spartan 3A only
                            "M0" => SharedCfgPad::M0,
                            "M1" => SharedCfgPad::M1,
                            "M2" => SharedCfgPad::M2,
                            "CSI_B" => SharedCfgPad::CsiB,
                            "CSO_B" => SharedCfgPad::CsoB,
                            "CCLK" => SharedCfgPad::Cclk,
                            "HSWAP" | "PUDC_B" => SharedCfgPad::HswapEn,
                            "LDC0" => SharedCfgPad::Ldc0,
                            "LDC1" => SharedCfgPad::Ldc1,
                            "LDC2" => SharedCfgPad::Ldc2,
                            "HDC" => SharedCfgPad::Hdc,
                            "AWAKE" => SharedCfgPad::Awake,
                            _ => {
                                if let Some((s, n)) = split_num(f) {
                                    match s {
                                        "VS" => continue,
                                        "D" => SharedCfgPad::Data(n as u8),
                                        "A" => SharedCfgPad::Addr(n as u8),
                                        _ => {
                                            println!(
                                                "UNK FUNC {f} {func} {coord:?}",
                                                func = pin.func
                                            );
                                            continue;
                                        }
                                    }
                                } else {
                                    println!("UNK FUNC {f} {func} {coord:?}", func = pin.func);
                                    continue;
                                }
                            }
                        };
                        let old = chip.cfg_io.insert(cfg, coord);
                        assert!(old.is_none() || old == Some(coord));
                    }
                }
            }
        }
        assert_eq!(vrp.len(), vrn.len());
        assert_eq!(alt_vrp.len(), alt_vrn.len());
        for (k, p) in vrp {
            let n = vrn[&k];
            let old = chip.dci_io.insert(k, (p, n));
            assert!(old.is_none() || old == Some((p, n)));
        }
        for (k, p) in alt_vrp {
            let n = alt_vrn[&k];
            let old = chip.dci_io_alt.insert(k, (p, n));
            assert!(old.is_none() || old == Some((p, n)));
        }
    }
}

pub fn make_grid(rd: &Part) -> Chip {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(
        rd,
        &[
            "CENTER",
            "CENTER_SMALL",
            "CENTER_SMALL_BRK",
            "BRAM0",
            "BRAM0_SMALL",
            "MACC0_SMALL",
            "TIOIB",
            "TIOIS",
            "LL",
            "LR",
            "UL",
            "UR",
        ],
        &[],
    );
    let kind = get_kind(rd);
    let mut columns = make_columns(rd, &int, kind);
    get_cols_io(rd, &int, kind, &mut columns);
    let mut grid = Chip {
        kind,
        columns,
        col_clk: get_col_clk(rd, &int),
        cols_clkv: get_cols_clkv(rd, &int),
        cols_gt: get_cols_gt(rd, &int),
        rows: get_rows(rd, &int, kind),
        rows_ram: get_rows_ram(rd, &int, kind),
        rows_hclk: get_rows_hclk(rd, &int),
        row_pci: get_row_pci(rd, &int, kind),
        holes_ppc: get_holes_ppc(rd, &int),
        dcms: get_dcms(rd, kind),
        has_ll: get_has_ll(rd),
        has_small_int: get_has_small_int(rd),
        cfg_io: BTreeMap::new(),
        dci_io: BTreeMap::new(),
        dci_io_alt: BTreeMap::new(),
    };
    handle_spec_io(rd, &mut grid, &int);
    grid
}

use std::collections::{BTreeMap, BTreeSet, HashSet, HashMap};

use prjcombine_xilinx_rawdump::{Part, Coord, PkgPin};
use prjcombine_xilinx_geom::{self as geom, BondPin, CfgPin, Bond, GtPin, ColId, RowId, int, int::Dir};
use prjcombine_xilinx_geom::virtex2::{self, GridKind, Column, ColumnKind, ColumnIoKind, RowIoKind, Dcms};
use prjcombine_entity::EntityVec;

use crate::grid::{extract_int, find_columns, find_column, find_rows, find_row, IntGrid, PreDevice, make_device};
use crate::intb::IntBuilder;
use crate::verify::Verifier;

fn get_kind(rd: &Part) -> GridKind {
    match &rd.family[..] {
        "virtex2" => GridKind::Virtex2,
        "virtex2p" => if find_columns(rd, &["MK_B_IOIS"]).is_empty() {
            GridKind::Virtex2P
        } else {
            GridKind::Virtex2PX
        },
        "spartan3" => GridKind::Spartan3,
        "spartan3e" => GridKind::Spartan3E,
        "spartan3a" => GridKind::Spartan3A,
        "spartan3adsp" => GridKind::Spartan3ADsp,
        _ => panic!("unknown family {}", rd.family),
    }
}

fn make_columns(rd: &Part, int: &IntGrid, kind: GridKind) -> EntityVec<ColId, Column> {
    let mut res = EntityVec::new();
    res.push(Column{kind: ColumnKind::Io, io: ColumnIoKind::None});
    for _ in 0..(int.cols.len() - 2) {
        res.push(Column{kind: ColumnKind::Clb, io: ColumnIoKind::None});
    }
    res.push(Column{kind: ColumnKind::Io, io: ColumnIoKind::None});
    let bram_cont = match kind {
        GridKind::Spartan3E | GridKind::Spartan3A => 4,
        GridKind::Spartan3ADsp => 3,
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

fn get_cols_io(rd: &Part, int: &IntGrid, kind: GridKind, cols: &mut EntityVec<ColId, Column>) {
    let mut col = cols.first_id().unwrap() + 1;
    let col_r = cols.last_id().unwrap();
    while col != col_r {
        match kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                let c0 = Coord {
                    x: int.cols[col] as u16,
                    y: 0,
                };
                let c1 = Coord {
                    x: (int.cols[col] + 1) as u16,
                    y: 0,
                };
                let tk0 = &rd.tiles[&c0].kind[..];
                let tk1 = &rd.tiles[&c1].kind[..];
                match (tk0, tk1) {
                    ("BTERM012" | "BCLKTERM012" | "ML_BCLKTERM012", "BTERM323") | ("BTERM010", "BTERM123" | "BCLKTERM123" | "ML_BCLKTERM123") => {
                        for i in 0..2 {
                            cols[col].io = ColumnIoKind::Double(i as u8);
                            col += 1;
                        }
                    }
                    ("BTERM123" | "BTERM012", _) => {
                        cols[col].io = ColumnIoKind::Single;
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
            GridKind::Spartan3 => {
                if cols[col].kind == ColumnKind::Bram {
                    col += 1;
                } else {
                    for i in 0..2 {
                        cols[col].io = ColumnIoKind::Double(i as u8);
                        col += 1;
                    }
                }
            }
            GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                for i in 0..2 {
                    cols[col].io = ColumnIoKind::Double(i as u8);
                    col += 1;
                }
            }
            GridKind::Spartan3E => {
                let c = Coord {
                    x: int.cols[col] as u16,
                    y: 0,
                };
                let tk = &rd.tiles[&c].kind[..];
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
    for s in &rd.tiles[&c].sites {
        let s = s.as_deref().unwrap();
        if s.starts_with("RXNPAD") {
            return s[6..].parse::<u32>().unwrap();
        }
    }
    unreachable!();
}

fn get_cols_gt(rd: &Part, int: &IntGrid) -> BTreeMap<ColId, (u32, u32)> {
    let mut res = BTreeMap::new();
    for rc in find_columns(rd, &["BGIGABIT_INT0"]) {
        let c = int.lookup_column(rc);
        let bb = get_gt_bank(rd, Coord {
            x: (rc + 1) as u16,
            y: 2,
        });
        let bt = get_gt_bank(rd, Coord {
            x: (rc + 1) as u16,
            y: rd.height - 6,
        });
        res.insert(c, (bb, bt));
    }
    for rc in find_columns(rd, &["BGIGABIT10_INT0"]) {
        let c = int.lookup_column(rc);
        let bb = get_gt_bank(rd, Coord {
            x: (rc + 1) as u16,
            y: 2,
        });
        let bt = get_gt_bank(rd, Coord {
            x: (rc + 1) as u16,
            y: rd.height - 11,
        });
        res.insert(c, (bb, bt));
    }
    res
}

fn get_rows(rd: &Part, int: &IntGrid, kind: GridKind) -> EntityVec<RowId, RowIoKind> {
    let mut res = EntityVec::new();
    res.push(RowIoKind::None);
    while res.len() < int.rows.len() - 1 {
        match kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                for i in 0..2 {
                    res.push(RowIoKind::Double(i));
                }
            }
            GridKind::Spartan3 => {
                res.push(RowIoKind::Single);
            }
            GridKind::Spartan3E => {
                let c = Coord {
                    x: 0,
                    y: int.rows[res.next_id()] as u16,
                };
                let tk = &rd.tiles[&c].kind[..];
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
            GridKind::Spartan3A | GridKind::Spartan3ADsp => {
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

fn get_rows_ram(rd: &Part, int: &IntGrid, kind: GridKind) -> Option<(RowId, RowId)> {
    if kind == GridKind::Spartan3E {
        let b = int.lookup_row(find_row(rd, &["COB_TERM_B"]).unwrap());
        let t = int.lookup_row(find_row(rd, &["COB_TERM_T"]).unwrap());
        Some((b, t))
    } else {
        None
    }
}

fn get_rows_hclk(rd: &Part, int: &IntGrid) -> Vec<(RowId, RowId)> {
    let rows_hclk: Vec<_> = find_rows(rd, &["GCLKH"])
        .into_iter()
        .map(|r| int.lookup_row(r - 1) + 1)
        .collect();
    let mut rows_brk = HashSet::new();
    for r in find_rows(rd, &["BRKH", "CLKH", "CLKH_LL"]) {
        rows_brk.insert(int.lookup_row(r - 1) + 1);
    }
    for r in find_rows(rd, &["CENTER_SMALL_BRK"]) {
        rows_brk.insert(int.lookup_row(r) + 1);
    }
    rows_brk.insert(int.rows.next_id());
    let rows_brk: Vec<_> = rows_brk.into_iter().collect();
    assert_eq!(rows_hclk.len(), rows_brk.len());
    rows_hclk.into_iter().zip(rows_brk.into_iter()).collect()
}

fn get_row_pci(rd: &Part, int: &IntGrid, kind: GridKind) -> Option<RowId> {
    match kind {
        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
            Some(int.lookup_row(find_row(rd, &["REG_L"]).unwrap() + 1))
        }
        _ => None,
    }
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(ColId, RowId)> {
    let mut res = Vec::new();
    for tt in ["LLPPC_X0Y0_INT", "LLPPC_X1Y0_INT"] {
        if let Some(tk) = rd.tile_kinds.get(tt) {
            assert_eq!(tk.tiles.len(), 1);
            let tile = &tk.tiles[0];
            let x = int.lookup_column(tile.x as i32);
            let y = int.lookup_row((tile.y - 1) as i32);
            res.push((x, y));
        }
    }
    res
}

fn get_dcms(rd: &Part, kind: GridKind) -> Option<Dcms> {
    match kind {
        GridKind::Spartan3E => {
            if !find_columns(rd, &["DCM_H_BL_CENTER"]).is_empty() {
                Some(Dcms::Eight)
            } else if !find_columns(rd, &["DCM_BL_CENTER"]).is_empty() {
                Some(Dcms::Four)
            } else {
                Some(Dcms::Two)
            }
        }
        GridKind::Spartan3A | GridKind::Spartan3ADsp => {
            if !find_columns(rd, &["DCM_BGAP"]).is_empty() {
                Some(Dcms::Eight)
            } else if !find_columns(rd, &["DCM_BL_CENTER"]).is_empty() {
                Some(Dcms::Four)
            } else {
                Some(Dcms::Two)
            }
        }
        _ => None
    }
}

fn get_has_ll(rd: &Part) -> bool {
    !find_columns(rd, &["CLKV_LL"]).is_empty()
}

fn get_has_small_int(rd: &Part) -> bool {
    !find_columns(rd, &["CENTER_SMALL"]).is_empty()
}

fn handle_spec_io(rd: &Part, grid: &mut virtex2::Grid) {
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    let mut novref = BTreeSet::new();
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
                let mut is_vref = false;
                for f in pin.func.split('/').skip(1) {
                    if f.starts_with("VREF_") {
                        is_vref = true;
                    } else if f.starts_with("VRP_") {
                        vrp.insert(pin.vref_bank.unwrap(), coord);
                    } else if f.starts_with("VRN_") {
                        vrn.insert(pin.vref_bank.unwrap(), coord);
                    } else if f.starts_with("ALT_VRP_") {
                        alt_vrp.insert(pin.vref_bank.unwrap(), coord);
                    } else if f.starts_with("ALT_VRN_") {
                        alt_vrn.insert(pin.vref_bank.unwrap(), coord);
                    } else if f.starts_with("GCLK") || f.starts_with("LHCLK") || f.starts_with("RHCLK") {
                        // ignore
                    } else if f.starts_with("IRDY") || f.starts_with("TRDY") {
                        // ignore
                    } else {
                        let cfg = match &f[..] {
                            "No_Pair" | "DIN" | "BUSY" | "MOSI" | "MISO" => continue,
                            "CS_B" => CfgPin::CsiB,
                            "INIT_B" => CfgPin::InitB,
                            "RDWR_B" => CfgPin::RdWrB,
                            "DOUT" => CfgPin::Dout,
                            // Spartan 3E, Spartan 3A only
                            "M0" => CfgPin::M0,
                            "M1" => CfgPin::M1,
                            "M2" => CfgPin::M2,
                            "CSI_B" => CfgPin::CsiB,
                            "CSO_B" => CfgPin::CsoB,
                            "CCLK" => CfgPin::Cclk,
                            "HSWAP" | "PUDC_B" => CfgPin::HswapEn,
                            "LDC0" => CfgPin::Ldc(0),
                            "LDC1" => CfgPin::Ldc(1),
                            "LDC2" => CfgPin::Ldc(2),
                            "HDC" => CfgPin::Hdc,
                            "AWAKE" => CfgPin::Awake,
                            _ => if let Some((s, n)) = split_num(f) {
                                match s {
                                    "VS" => continue,
                                    "D" => CfgPin::Data(n as u8),
                                    "A" => CfgPin::Addr(n as u8),
                                    _ => {
                                        println!("UNK FUNC {f} {func} {coord:?}", func=pin.func);
                                        continue;
                                    }
                                }
                            } else {
                                println!("UNK FUNC {f} {func} {coord:?}", func=pin.func);
                                continue;
                            }
                        };
                        let old = grid.cfg_io.insert(cfg, coord);
                        assert!(old.is_none() || old == Some(coord));
                    }
                }
                if is_vref {
                    grid.vref.insert(coord);
                } else {
                    novref.insert(coord);
                }
            }
        }
        assert_eq!(vrp.len(), vrn.len());
        assert_eq!(alt_vrp.len(), alt_vrn.len());
        for (k, p) in vrp {
            let n = vrn[&k];
            let old = grid.dci_io.insert(k, (p, n));
            assert!(old.is_none() || old == Some((p, n)));
        }
        for (k, p) in alt_vrp {
            let n = alt_vrn[&k];
            let old = grid.dci_io_alt.insert(k, (p, n));
            assert!(old.is_none() || old == Some((p, n)));
        }
    }
    for c in novref {
        assert!(!grid.vref.contains(&c));
    }
}

fn make_int_db_v2(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("virtex2", rd);
    builder.node_type("CENTER", "CLB", "NODE.CLB");
    builder.node_type("LR_IOIS", "IOI", "NODE.IOI.LR");
    builder.node_type("TB_IOIS", "IOI", "NODE.IOI.TB");
    builder.node_type("ML_TB_IOIS", "IOI", "NODE.IOI.TB");
    builder.node_type("ML_TBS_IOIS", "IOI", "NODE.IOI.TB");
    builder.node_type("GIGABIT_IOI", "IOI", "NODE.IOI.TB");
    builder.node_type("GIGABIT10_IOI", "IOI", "NODE.IOI.TB");
    builder.node_type("MK_B_IOIS", "IOI.CLK_B", "NODE.IOI.CLK_B");
    builder.node_type("MK_T_IOIS", "IOI.CLK_T", "NODE.IOI.CLK_T");
    builder.node_type("BRAM0", "BRAM", "NODE.BRAM");
    builder.node_type("BRAM1", "BRAM", "NODE.BRAM");
    builder.node_type("BRAM2", "BRAM", "NODE.BRAM");
    builder.node_type("BRAM3", "BRAM", "NODE.BRAM");
    builder.node_type("BRAM_IOIS", "DCM.V2", "NODE.BRAM_IOIS");
    builder.node_type("ML_BRAM_IOIS", "DCM.V2P", "NODE.ML_BRAM_IOIS");
    builder.node_type("LL", "CNR", "NODE.CNR");
    builder.node_type("LR", "CNR", "NODE.CNR");
    builder.node_type("UL", "CNR", "NODE.CNR");
    builder.node_type("UR", "CNR", "NODE.CNR");
    builder.node_type("BGIGABIT_INT0", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT_INT1", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT_INT2", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT_INT3", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT_INT4", "GT.CLKPAD", "NODE.GT.CLKPAD");
    builder.node_type("TGIGABIT_INT0", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT_INT1", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT_INT2", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT_INT3", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT_INT4", "GT.CLKPAD", "NODE.GT.CLKPAD");
    builder.node_type("BGIGABIT10_INT0", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT10_INT1", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT10_INT2", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT10_INT3", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT10_INT4", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT10_INT5", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT10_INT6", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT10_INT7", "PPC", "NODE.GT");
    builder.node_type("BGIGABIT10_INT8", "GT.CLKPAD", "NODE.GT.CLKPAD");
    builder.node_type("TGIGABIT10_INT0", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT10_INT1", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT10_INT2", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT10_INT3", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT10_INT4", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT10_INT5", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT10_INT6", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT10_INT7", "PPC", "NODE.GT");
    builder.node_type("TGIGABIT10_INT8", "GT.CLKPAD", "NODE.GT.CLKPAD");
    builder.node_type("LPPC_X0Y0_INT", "PPC", "NODE.PPC.L");
    builder.node_type("LPPC_X1Y0_INT", "PPC", "NODE.PPC.L");
    builder.node_type("LLPPC_X0Y0_INT", "PPC", "NODE.PPC.L");
    builder.node_type("LLPPC_X1Y0_INT", "PPC", "NODE.PPC.L");
    builder.node_type("ULPPC_X0Y0_INT", "PPC", "NODE.PPC.L");
    builder.node_type("ULPPC_X1Y0_INT", "PPC", "NODE.PPC.L");
    builder.node_type("RPPC_X0Y0_INT", "PPC", "NODE.PPC.R");
    builder.node_type("RPPC_X1Y0_INT", "PPC", "NODE.PPC.R");
    builder.node_type("BPPC_X0Y0_INT", "PPC", "NODE.PPC.B");
    builder.node_type("BPPC_X1Y0_INT", "PPC", "NODE.PPC.B");
    builder.node_type("TPPC_X0Y0_INT", "PPC", "NODE.PPC.T");
    builder.node_type("TPPC_X1Y0_INT", "PPC", "NODE.PPC.T");

    builder.wire("PULLUP", int::WireKind::TiePullup, &[
        "VCC_PINWIRE",
        "IOIS_VCC_WIRE",
        "BRAM_VCC_WIRE",
        "BRAM_IOIS_VCC_WIRE",
        "CNR_VCC_WIRE",
        "GIGABIT_INT_VCC_WIRE",
    ]);
    for i in 0..8 {
        builder.wire(format!("GCLK{i}"), int::WireKind::ClkOut(i), &[
            format!("GCLK{i}"),
        ]);
    }
    for i in 0..8 {
        builder.wire(format!("DLL_CLKPAD{i}"), int::WireKind::ClkOut(8 + i), &[
            format!("BRAM_IOIS_DLL_CLKPAD{i}"),
            format!("GIGABIT_INT_DLL_CLKPAD{i}"),
        ]);
    }

    for (i, da1, da2, db) in [
        (0, Dir::S, None, None),
        (1, Dir::W, Some(Dir::S), None),
        (2, Dir::E, None, Some(Dir::S)),
        (3, Dir::S, Some(Dir::E), None),
        (4, Dir::S, None, None),
        (5, Dir::S, Some(Dir::W), None),
        (6, Dir::W, None, None),
        (7, Dir::E, Some(Dir::S), None),
        (8, Dir::E, Some(Dir::N), None),
        (9, Dir::W, None, None),
        (10, Dir::N, Some(Dir::W), None),
        (11, Dir::N, None, None),
        (12, Dir::N, Some(Dir::E), None),
        (13, Dir::E, None, Some(Dir::N)),
        (14, Dir::W, Some(Dir::N), None),
        (15, Dir::N, None, None),
    ] {
        let omux = builder.mux_out(format!("OMUX{i}"), &[
            format!("OMUX{i}"),
            format!("LPPC_INT_OMUX{i}"),
        ]);
        let omux_da1 = builder.branch(omux, da1, format!("OMUX{i}.{da1}"), &[
            format!("OMUX_{da1}{i}"),
            format!("LPPC_INT_OMUX_{da1}{i}"),
        ]);
        if let Some(da2) = da2 {
            builder.branch(omux_da1, da2, format!("OMUX{i}.{da1}{da2}"), &[
                format!("OMUX_{da1}{da2}{i}"),
                format!("LPPC_INT_OMUX_{da1}{da2}{i}"),
            ]);
        }
        if let Some(db) = db {
            builder.branch(omux, db, format!("OMUX{i}.{db}"), &[
                format!("OMUX_{db}{i}"),
                format!("LPPC_INT_OMUX_{db}{i}"),
            ]);
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let beg = builder.mux_out(format!("DBL.{dir}{i}.0"), &[
                format!("{dir}2BEG{i}"),
                format!("LPPC_INT_{dir}2BEG{i}"),
            ]);
            let mid = builder.branch(beg, dir, format!("DBL.{dir}{i}.1"), &[
                format!("{dir}2MID{i}"),
                format!("LPPC_INT_{dir}2MID{i}"),
            ]);
            let end = builder.branch(mid, dir, format!("DBL.{dir}{i}.2"), &[
                format!("{dir}2END{i}"),
                format!("LPPC_INT_{dir}2END{i}"),
            ]);
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(end, Dir::S, format!("DBL.{dir}{i}.3"), &[
                    format!("{dir}2END_S{i}"),
                    format!("LPPC_INT_{dir}2END_S{i}"),
                ]);
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(end, Dir::N, format!("DBL.{dir}{i}.3"), &[
                    format!("{dir}2END_N{i}"),
                    format!("LPPC_INT_{dir}2END_N{i}"),
                ]);
            }
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let mut last = builder.mux_out(format!("HEX.{dir}{i}.0"), &[
                format!("{dir}6BEG{i}"),
                format!("LR_IOIS_{dir}6BEG{i}"),
                format!("TB_IOIS_{dir}6BEG{i}"),
                format!("LPPC_INT_{dir}6BEG{i}"),
            ]);
            for (j, seg) in [
                (1, "A"),
                (2, "B"),
                (3, "MID"),
                (4, "C"),
                (5, "D"),
                (6, "END"),
            ] {
                last = builder.branch(last, dir, format!("HEX.{dir}{i}.{j}"), &[
                    format!("{dir}6{seg}{i}"),
                    format!("LR_IOIS_{dir}6{seg}{i}"),
                    format!("TB_IOIS_{dir}6{seg}{i}"),
                    format!("LPPC_INT_{dir}6{seg}{i}"),
                ]);
            }
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(last, Dir::S, format!("HEX.{dir}{i}.7"), &[
                    format!("{dir}6END_S{i}"),
                    format!("LR_IOIS_{dir}6END_S{i}"),
                    format!("TB_IOIS_{dir}6END_S{i}"),
                    format!("LPPC_INT_{dir}6END_S{i}"),
                ]);
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(last, Dir::N, format!("HEX.{dir}{i}.7"), &[
                    format!("{dir}6END_N{i}"),
                    format!("LR_IOIS_{dir}6END_N{i}"),
                    format!("TB_IOIS_{dir}6END_N{i}"),
                    format!("LPPC_INT_{dir}6END_N{i}"),
                ]);
            }
        }
    }

    let lh: Vec<_> = (0..24).map(|i| builder.wire(format!("LH.{i}"), int::WireKind::MultiBranch(Dir::W), &[
        format!("LH{i}"),
        format!("LPPC_INT_LH{i}"),
    ])).collect();
    for i in 0..24 {
        builder.conn_branch(lh[i], Dir::E, lh[(i + 1) % 24]);
    }

    let lv: Vec<_> = (0..24).map(|i| builder.wire(format!("LV.{i}"), int::WireKind::MultiBranch(Dir::S), &[
        format!("LV{i}"),
    ])).collect();
    for i in 0..24 {
        builder.conn_branch(lv[i], Dir::N, lv[(i + 23) % 24]);
    }

    for i in 0..4 {
        builder.mux_out(format!("IMUX.CLK{i}"), &[
            format!("CLK{i}"),
            format!("IOIS_CK{j}_B{k}", j = [2, 1, 2, 1][i], k = [1, 1, 3, 3][i]),
            format!("BRAM_CLK{i}"),
            ["BRAM_IOIS_CLKFB", "BRAM_IOIS_CLKIN", "BRAM_IOIS_PSCLK", ""][i].to_string(),
            format!("CNR_CLK{i}"),
            format!("LRPPC_INT_CLK{i}"),
            format!("BPPC_INT_CLK{i}"),
            format!("TPPC_INT_CLK{i}"),
            format!("GIGABIT_INT_CLK{i}"),
        ]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.SR{i}"), &[
            format!("SR{i}"),
            format!("IOIS_SR_B{j}", j = [1, 2, 0, 3][i]),
            format!("BRAM_SR{i}"),
            format!("BRAM_IOIS_SR{i}"),
            format!("CNR_SR{i}"),
            format!("LRPPC_INT_SR{i}"),
            format!("BPPC_INT_SR{i}"),
            format!("TPPC_INT_SR{i}"),
            format!("GIGABIT_INT_SR{i}"),
        ]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.CE{i}"), &[
            format!("CE_B{i}"),
            format!("OCE_B{j}", j = [1, 0, 3, 2][i]),
            format!("BRAM_CE_B{i}"),
            // only 2, 3 actually exist
            format!("BRAM_IOIS_CE_B{i}"),
            format!("CNR_CE_B{i}"),
            format!("LRPPC_INT_CE_B{i}"),
            format!("BPPC_INT_CE_B{i}"),
            format!("TPPC_INT_CE_B{i}"),
            format!("GIGABIT_INT_CE_B{i}"),
        ]);
    }
    for i in 0..2 {
        builder.mux_out(format!("IMUX.TI{i}"), &[
            format!("TI{i}"),
            format!("IOIS_CK{j}_B0", j = [2, 1][i]),
            format!("BRAM_TI{i}"),
            format!("BRAM_IOIS_TI{i}"),
            format!("CNR_TI{i}"),
            format!("LRPPC_INT_TI{i}"),
            format!("BPPC_INT_TI{i}"),
            format!("TPPC_INT_TI{i}"),
            format!("GIGABIT_INT_TI{i}"),
        ]);
    }
    for i in 0..2 {
        builder.mux_out(format!("IMUX.TS{i}"), &[
            format!("TS{i}"),
            format!("IOIS_CK{j}_B2", j = [2, 1][i]),
            format!("BRAM_TS{i}"),
            format!("CNR_TS{i}"),
            format!("LRPPC_INT_TS{i}"),
            format!("BPPC_INT_TS{i}"),
            format!("TPPC_INT_TS{i}"),
            format!("GIGABIT_INT_TS{i}"),
        ]);
    }
    // CLB inputs
    for i in 0..4 {
        for j in 1..5 {
            builder.mux_out(format!("IMUX.S{i}.F{j}"), &[format!("F{j}_B{i}")]);
        }
        for j in 1..5 {
            builder.mux_out(format!("IMUX.S{i}.G{j}"), &[format!("G{j}_B{i}")]);
        }
        builder.mux_out(format!("IMUX.S{i}.BX"), &[format!("BX{i}")]);
        builder.mux_out(format!("IMUX.S{i}.BY"), &[format!("BY{i}")]);
    }
    // non-CLB inputs
    for i in 0..4 {
        for j in 0..2 {
            let ri = 3 - i;
            builder.mux_out(format!("IMUX.G{i}.FAN{j}"), &[
                match (i, j) {
                    (0, 0) => "IOIS_FAN_BX0",
                    (0, 1) => "IOIS_FAN_BX2",
                    (1, 0) => "IOIS_FAN_BY0",
                    (1, 1) => "IOIS_FAN_BY2",
                    (2, 0) => "IOIS_FAN_BY1",
                    (2, 1) => "IOIS_FAN_BY3",
                    (3, 0) => "IOIS_FAN_BX1",
                    (3, 1) => "IOIS_FAN_BX3",
                    _ => unreachable!(),
                }.to_string(),
                format!("CNR_FAN{ri}{j}"),
                format!("BRAM_FAN{ri}{j}"),
                format!("LRPPC_INT_FAN{ri}{j}"),
                format!("BPPC_INT_FAN{ri}{j}"),
                format!("TPPC_INT_FAN{ri}{j}"),
                format!("GIGABIT_INT_FAN{ri}{j}"),
            ]);
        }
        for j in 0..8 {
            builder.mux_out(format!("IMUX.G{i}.DATA{j}"), &[
                match (i, j) {
                    (_, 5) => format!("IOIS_REV_B{i}"),
                    (_, 6) => format!("O2_B{i}"),
                    (_, 7) => format!("O1_B{i}"),
                    _ => "".to_string(),
                },
                format!("DATA_IN{k}", k = i * 8 + j), // CNR
                match (i, j) {
                    (0, 2) => "BRAM_DIPB".to_string(),
                    (0, 3) => "BRAM_DIPA".to_string(),
                    (2, 2) => "BRAM_MULTINB16".to_string(),
                    (2, 3) => "BRAM_MULTINB17".to_string(),
                    (3, 2) => "BRAM_MULTINA16".to_string(),
                    (3, 3) => "BRAM_MULTINA17".to_string(),
                    (_, 4) => format!("BRAM_DIB{i}"),
                    (_, 5) => format!("BRAM_DIB{k}", k = 16 + i),
                    (_, 6) => format!("BRAM_DIA{i}"),
                    (_, 7) => format!("BRAM_DIA{k}", k = 16 + i),
                    _ => "".to_string(),
                },
                match (i, j) {
                    (0, 0) => "BRAM_IOIS_DSSEN".to_string(),
                    (0, 1) => "BRAM_IOIS_CTLSEL0".to_string(),
                    (0, 2) => "BRAM_IOIS_CTLSEL1".to_string(),
                    (0, 3) => "BRAM_IOIS_CTLSEL2".to_string(),
                    (1, 0) => "BRAM_IOIS_PSEN".to_string(),
                    (1, 1) => "BRAM_IOIS_CTLOSC2".to_string(),
                    (1, 2) => "BRAM_IOIS_CTLOSC1".to_string(),
                    (1, 3) => "BRAM_IOIS_CTLGO".to_string(),
                    (2, 0) => "BRAM_IOIS_PSINCDEC".to_string(),
                    (2, 1) => "BRAM_IOIS_CTLMODE".to_string(),
                    (2, 2) => "BRAM_IOIS_FREEZEDLL".to_string(),
                    (2, 3) => "BRAM_IOIS_FREEZEDFS".to_string(),
                    (3, 0) => "BRAM_IOIS_RST".to_string(),
                    (3, 1) => "BRAM_IOIS_STSADRS0".to_string(),
                    (3, 2) => "BRAM_IOIS_STSADRS1".to_string(),
                    (3, 3) => "BRAM_IOIS_STSADRS2".to_string(),
                    (3, 4) => "BRAM_IOIS_STSADRS3".to_string(),
                    (3, 5) if rd.family == "virtex2p" => "BRAM_IOIS_STSADRS4".to_string(),
                    _ => format!("BRAM_IOIS_DATA{k}", k = i * 8 + j),
                },
                format!("LRPPC_INT_DATA_IN{k}", k = j * 4 + i),
                format!("BPPC_INT_DATA_IN{k}", k = j * 4 + i),
                format!("TPPC_INT_DATA_IN{k}", k = j * 4 + i),
                format!("GIGABIT_INT_DATA_IN{k}", k = j * 4 + i),
            ]);
        }
    }
    // IOI special inputs
    for i in 0..4 {
        builder.mux_out(format!("IMUX.IOI.TS1{i}"), &[format!("TS1_B{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.IOI.TS2{i}"), &[format!("TS2_B{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.IOI.ICE{i}"), &[format!("ICE_B{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.IOI.TCE{i}"), &[format!("TCE_B{i}")]);
    }
    // BRAM special inputs
    let bram_s = builder.make_naming("BRAM.S");
    let bram_n = builder.make_naming("BRAM.N");
    for ab in ["A", "B"] {
        for i in 0..4 {
            let root = builder.mux_out(format!("IMUX.BRAM_ADDR{ab}{i}"), &[
                format!("BRAM_ADDR{ab}_B{i}"),
            ]);
            for dir in [Dir::S, Dir::N] {
                let mut last = root;
                for j in 1..5 {
                    if dir == Dir::N {
                        builder.name_wire(bram_s, last, format!("BRAMSITE_NADDRIN_{ab}_S{k}", k = (i ^ 3) + (j - 1) * 4));
                    }
                    if j == 4 {
                        last = builder.branch(last, dir, format!("IMUX.BRAM_ADDR{ab}{i}.{dir}4"), &[
                            format!("BRAM_ADDR{ab}_{dir}END{i}"),
                        ]);
                    } else {
                        last = builder.branch(last, dir, format!("IMUX.BRAM_ADDR{ab}{i}.{dir}{j}"), &[""]);
                    }
                    if dir == Dir::N {
                        builder.name_wire(bram_n, last, format!("BRAMSITE_NADDRIN_{ab}{k}", k = (i ^ 3) + (j - 1) * 4));
                    }
                }
            }
        }
    }

    // logic out stuff
    for i in 0..8 {
        let w = builder.logic_out(format!("OUT.FAN{i}"), &[
            // In CLBs, used for combinatorial outputs.
            ["X0", "X1", "X2", "X3", "Y0", "Y1", "Y2", "Y3"][i],
            // In IOIS, used for combinatorial inputs.  4-7 are unused.
            ["I0", "I1", "I2", "I3", "", "", "", ""][i],
            // In BRAM, used for low data outputs.
            [
                "BRAM_DOA2",
                "BRAM_DOA3",
                "BRAM_DOA0",
                "BRAM_DOA1",
                "BRAM_DOB1",
                "BRAM_DOB0",
                "BRAM_DOB3",
                "BRAM_DOB2",
            ][i],
            &format!("DOUT_FAN{i}"),
            &format!("LRPPC_INT_PPC1{i}"),
            &format!("BPPC_INT_PPC1{i}"),
            &format!("TPPC_INT_PPC1{i}"),
            &format!("GIGABIT_INT_PPC1{i}"),
        ]);
        if i == 0 {
            if let Some((_, n)) = builder.db.namings.get_mut("NODE.IOI.CLK_T") {
                n.insert(w, "IOIS_BREFCLK_SE".to_string());
            }
        }
        if i == 2 {
            if let Some((_, n)) = builder.db.namings.get_mut("NODE.IOI.CLK_B") {
                n.insert(w, "IOIS_BREFCLK_SE".to_string());
            }
        }
    }

    // We call secondary outputs by their OMUX index.
    for i in 2..24 {
        builder.logic_out(format!("OUT.SEC{i}"), &[
            [
                "", "", "", "", "", "", "", "", "YB0", "YB1", "YB3", "YB2", "XB1", "XB2",
                "XB3", "YQ0", "YQ1", "XB0", "YQ2", "YQ3", "XQ0", "XQ1", "XQ2", "XQ3",
            ][i],
            [
                "", "", "", "", "", "", "", "", "", "I_Q21", "I_Q23", "", "TS_FDBK1",
                "TS_FDBK2", "TS_FDBK3", "I_Q20", "", "TS_FDBK0", "I_Q22", "", "I_Q10",
                "I_Q11", "I_Q12", "I_Q13",
            ][i],
            [
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "BRAM_DOPA",
                "BRAM_DOPB",
                "",
                "BRAM_MOUT32",
                "BRAM_MOUT7",
                "BRAM_MOUT6",
                "BRAM_MOUT5",
                "BRAM_MOUT4",
                "BRAM_MOUT3",
                "BRAM_MOUT2",
                "BRAM_MOUT1",
                "BRAM_MOUT0",
            ][i],
            [
                "",
                "",
                "BRAM_IOIS_CLKFX180",
                "BRAM_IOIS_CLKFX",
                "BRAM_IOIS_CLKDV",
                "BRAM_IOIS_CLK2X180",
                "BRAM_IOIS_CLK2X",
                "BRAM_IOIS_CLK270",
                "BRAM_IOIS_CLK180",
                "BRAM_IOIS_CLK90",
                "BRAM_IOIS_CLK0",
                "BRAM_IOIS_CONCUR",
                "BRAM_IOIS_PSDONE",
                "BRAM_IOIS_LOCKED",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
            ][i],
            &if (8..16).contains(&i) {
                format!("LRPPC_INT_PPC2{k}", k = 15 - i)
            } else {
                format!("")
            },
            &if (8..16).contains(&i) {
                format!("BPPC_INT_PPC2{k}", k = 15 - i)
            } else {
                format!("")
            },
            &if (8..16).contains(&i) {
                format!("TPPC_INT_PPC2{k}", k = 15 - i)
            } else {
                format!("")
            },
            &if (8..16).contains(&i) {
                format!("GIGABIT_INT_PPC2{k}", k = 15 - i)
            } else {
                format!("")
            },
        ]);
    }

    // Same for tertiary.
    for i in 8..18 {
        for j in 0..2 {
            builder.logic_out(format!("OUT.HALF{i}.{j}"), &[
                    format!("DOUT{k}", k = (17 - i) * 2 + j),
                    match (i, j) {
                        (8, 0) => "BRAM_DOA16",
                        (9, 0) => "BRAM_DOA17",
                        (10, 0) => "BRAM_DOA19",
                        (11, 0) => "BRAM_DOA18",
                        (8, 1) => "BRAM_DOB16",
                        (9, 1) => "BRAM_DOB17",
                        (10, 1) => "BRAM_DOB19",
                        (11, 1) => "BRAM_DOB18",
                        (14, 0) => "BRAM_IOIS_STATUS0",
                        (15, 0) => "BRAM_IOIS_STATUS1",
                        (16, 0) => "BRAM_IOIS_STATUS2",
                        (17, 0) => "BRAM_IOIS_STATUS3",
                        (14, 1) => "BRAM_IOIS_STATUS4",
                        (15, 1) => "BRAM_IOIS_STATUS5",
                        (16, 1) => "BRAM_IOIS_STATUS6",
                        (17, 1) => "BRAM_IOIS_STATUS7",
                        _ => "",
                    }.to_string(),
                ],
            );
        }
    }

    for i in 0..16 {
        builder.logic_out(format!("OUT.TEST{i}"), &[
            format!("LRPPC_INT_TEST{i}"),
            format!("BPPC_INT_TEST{i}"),
            format!("TPPC_INT_TEST{i}"),
            format!("GIGABIT_INT_TEST{i}"),
        ]);
    }

    builder.logic_out("OUT.TBUS", &["TBUS"]);
    let out_pci0 = builder.logic_out("OUT.PCI0", &[""]);
    let out_pci1 = builder.logic_out("OUT.PCI1", &[""]);
    builder.extra_name("LTERM_PCI_OUT_D0", out_pci0);
    builder.extra_name("LTERM_PCI_OUT_D1", out_pci1);
    builder.extra_name("LTERM_PCI_OUT_U0", out_pci0);
    builder.extra_name("LTERM_PCI_OUT_U1", out_pci1);
    builder.extra_name("RTERM_PCI_OUT_D0", out_pci0);
    builder.extra_name("RTERM_PCI_OUT_D1", out_pci1);
    builder.extra_name("RTERM_PCI_OUT_U0", out_pci0);
    builder.extra_name("RTERM_PCI_OUT_U1", out_pci1);

    builder.extract_nodes();

    for (tkn, n) in [
        ("LTERM321", "TERM.W.U"),
        ("LTERM010", "TERM.W.U"),
        ("LTERM323", "TERM.W.D"),
        ("LTERM210", "TERM.W.D"),
        ("LTERM323_PCI", "TERM.W.U"),
        ("LTERM210_PCI", "TERM.W.U"),
        ("CNR_LTERM", "TERM.W"),
    ] {
        builder.extract_term("W", Dir::W, tkn, n);
    }
    for (tkn, n) in [
        ("RTERM321", "TERM.E.U"),
        ("RTERM010", "TERM.E.U"),
        ("RTERM323", "TERM.E.D"),
        ("RTERM210", "TERM.E.D"),
        ("RTERM323_PCI", "TERM.E.U"),
        ("RTERM210_PCI", "TERM.E.U"),
        ("CNR_RTERM", "TERM.E"),
    ] {
        builder.extract_term("E", Dir::E, tkn, n);
    }
    for tkn in [
        "BTERM010",
        "BTERM123",
        "BTERM012",
        "BTERM323",
        "BTERM123_TBS",
        "BTERM012_TBS",
        "BCLKTERM123",
        "BCLKTERM012",
        "ML_BCLKTERM123",
        "ML_BCLKTERM012",
        "ML_BCLKTERM123_MK",
        "BBTERM",
        "BGIGABIT_IOI_TERM",
        "BGIGABIT10_IOI_TERM",
        "BGIGABIT_INT_TERM",
        "BGIGABIT10_INT_TERM",
    ] {
        builder.extract_term("S", Dir::S, tkn, "TERM.S");
    }
    builder.extract_term("S", Dir::S, "CNR_BTERM", "TERM.S.CNR");
    builder.extract_term("S", Dir::S, "ML_CNR_BTERM", "TERM.S.CNR");
    for tkn in [
        "TTERM321",
        "TTERM010",
        "TTERM323",
        "TTERM210",
        "TTERM321_TBS",
        "TTERM210_TBS",
        "TCLKTERM321",
        "TCLKTERM210",
        "ML_TTERM010",
        "ML_TCLKTERM210",
        "ML_TCLKTERM210_MK",
        "BTTERM",
        "TGIGABIT_IOI_TERM",
        "TGIGABIT10_IOI_TERM",
        "TGIGABIT_INT_TERM",
        "TGIGABIT10_INT_TERM",
    ] {
        builder.extract_term("N", Dir::N, tkn, "TERM.N");
    }
    builder.extract_term("N", Dir::N, "CNR_TTERM", "TERM.N.CNR");

    if let Some(tk) = rd.tile_kinds.get("PTERMB") {
        for &xy_b in &tk.tiles {
            let xy_t = Coord {
                x: xy_b.x,
                y: xy_b.y + 14,
            };
            let int_s_xy = builder.walk_to_int(xy_b, Dir::S).unwrap();
            let int_n_xy = builder.walk_to_int(xy_t, Dir::N).unwrap();
            builder.extract_pass_tile("PPC.S", Dir::S, int_n_xy, Some((xy_t, "TERM.PPC.S", Some("TERM.PPC.S.FAR"))), Some((xy_b, "TERM.PPC.N.OUT", "TERM.PPC.N")), int_s_xy, &[]);
            builder.extract_pass_tile("PPC.N", Dir::N, int_s_xy, Some((xy_b, "TERM.PPC.N", Some("TERM.PPC.N.FAR"))), Some((xy_t, "TERM.PPC.S.OUT", "TERM.PPC.S")), int_n_xy, &[]);
        }
    }
    for tkn in ["PTERMR", "PTERMBR", "PTERMTR"] {
        if let Some(tk) = rd.tile_kinds.get(tkn) {
            for &xy_r in &tk.tiles {
                let int_w_xy = builder.walk_to_int(xy_r, Dir::W).unwrap();
                let int_e_xy = builder.walk_to_int(xy_r, Dir::E).unwrap();
                builder.extract_pass_tile("PPC.W", Dir::W, int_e_xy, Some((xy_r, "TERM.PPC.W", Some("TERM.PPC.W.FAR"))), Some((int_w_xy, "TERM.PPC.E.OUT", "TERM.PPC.E")), int_w_xy, &[]);
                builder.extract_pass_tile("PPC.E", Dir::E, int_w_xy, Some((int_w_xy, "TERM.PPC.E", Some("TERM.PPC.E.FAR"))), Some((xy_r, "TERM.PPC.W.OUT", "TERM.PPC.W")), int_e_xy, &[]);
            }
        }
    }

    for (tkn, name, naming) in [
        ("BGIGABIT_INT0", "GT.0", "NODE.GT"),
        ("BGIGABIT_INT1", "GT.123", "NODE.GT"),
        ("BGIGABIT_INT2", "GT.123", "NODE.GT"),
        ("BGIGABIT_INT3", "GT.123", "NODE.GT"),
        ("BGIGABIT_INT4", "GT.CLKPAD", "NODE.GT.CLKPAD"),
        ("TGIGABIT_INT0", "GT.0", "NODE.GT"),
        ("TGIGABIT_INT1", "GT.123", "NODE.GT"),
        ("TGIGABIT_INT2", "GT.123", "NODE.GT"),
        ("TGIGABIT_INT3", "GT.123", "NODE.GT"),
        ("TGIGABIT_INT4", "GT.CLKPAD", "NODE.GT.CLKPAD"),
        ("BGIGABIT10_INT0", "GT.0", "NODE.GT"),
        ("BGIGABIT10_INT1", "GT.123", "NODE.GT"),
        ("BGIGABIT10_INT2", "GT.123", "NODE.GT"),
        ("BGIGABIT10_INT3", "GT.123", "NODE.GT"),
        ("BGIGABIT10_INT4", "GT.0", "NODE.GT"),
        ("BGIGABIT10_INT5", "GT.123", "NODE.GT"),
        ("BGIGABIT10_INT6", "GT.123", "NODE.GT"),
        ("BGIGABIT10_INT7", "GT.123", "NODE.GT"),
        ("BGIGABIT10_INT8", "GT.CLKPAD", "NODE.GT.CLKPAD"),
        ("TGIGABIT10_INT0", "GT.0", "NODE.GT"),
        ("TGIGABIT10_INT1", "GT.123", "NODE.GT"),
        ("TGIGABIT10_INT2", "GT.123", "NODE.GT"),
        ("TGIGABIT10_INT3", "GT.123", "NODE.GT"),
        ("TGIGABIT10_INT4", "GT.0", "NODE.GT"),
        ("TGIGABIT10_INT5", "GT.123", "NODE.GT"),
        ("TGIGABIT10_INT6", "GT.123", "NODE.GT"),
        ("TGIGABIT10_INT7", "GT.123", "NODE.GT"),
        ("TGIGABIT10_INT8", "GT.CLKPAD", "NODE.GT.CLKPAD"),
        ("LPPC_X0Y0_INT", "PPC", "NODE.PPC.L"),
        ("LPPC_X1Y0_INT", "PPC", "NODE.PPC.L"),
        ("LLPPC_X0Y0_INT", "PPC", "NODE.PPC.L"),
        ("LLPPC_X1Y0_INT", "PPC", "NODE.PPC.L"),
        ("ULPPC_X0Y0_INT", "PPC", "NODE.PPC.L"),
        ("ULPPC_X1Y0_INT", "PPC", "NODE.PPC.L"),
        ("RPPC_X0Y0_INT", "PPC", "NODE.PPC.R"),
        ("RPPC_X1Y0_INT", "PPC", "NODE.PPC.R"),
        ("BPPC_X0Y0_INT", "PPC", "NODE.PPC.B"),
        ("BPPC_X1Y0_INT", "PPC", "NODE.PPC.B"),
        ("TPPC_X0Y0_INT", "PPC", "NODE.PPC.T"),
        ("TPPC_X1Y0_INT", "PPC", "NODE.PPC.T"),
    ] {
        builder.extract_intf(name, Dir::E, tkn, naming, None, None, None);
    }

    // - extract bels + namings
    //   - RLL
    //   - SLICE ×4
    //   - TBUF ×2
    //   - TBUS
    //   - IOI ×4
    //   - BRAM
    //   - MULT
    //   - DCM.V2
    //   - DCM.V2P
    //   - corner stuff
    //     - DCI ×8 [?]
    //     - PMV
    //     - BSCAN
    //     - STARTUP
    //     - ICAP
    //     - CAPTURE
    //     - JTAGPPC
    //   - PCILOGIC
    //   - BUFGMUX.[BT] ×8
    //   - PPC
    //   - GT.[BT]
    //   - GT10.[BT]
    builder.build()
}

fn make_int_db_s3(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("spartan3", rd);
    builder.node_type("CENTER", "CLB", "NODE.CLB");
    builder.node_type("CENTER_SMALL", "CLB", "NODE.CLB");
    builder.node_type("CENTER_SMALL_BRK", "CLB", "NODE.CLB.BRK");
    if rd.family.starts_with("spartan3a") {
        builder.node_type("LIOIS", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("LIOIS_BRK", "IOI.S3A.LR", "NODE.IOI.S3A.LR.BRK");
        builder.node_type("LIOIS_PCI", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("LIOIS_CLK_PCI", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("LIOIS_CLK_PCI_BRK", "IOI.S3A.LR", "NODE.IOI.S3A.LR.BRK");
        builder.node_type("LIBUFS", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("LIBUFS_PCI", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("LIBUFS_CLK_PCI", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("RIOIS", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("RIOIS_PCI", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("RIOIS_CLK_PCI", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("RIBUFS", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("RIBUFS_BRK", "IOI.S3A.LR", "NODE.IOI.S3A.LR.BRK");
        builder.node_type("RIBUFS_PCI", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("RIBUFS_CLK_PCI", "IOI.S3A.LR", "NODE.IOI.S3A.LR");
        builder.node_type("RIBUFS_CLK_PCI_BRK", "IOI.S3A.LR", "NODE.IOI.S3A.LR.BRK");
        builder.node_type("BIOIS", "IOI.S3A.TB", "NODE.IOI.S3A.TB");
        builder.node_type("BIOIB", "IOI.S3A.TB", "NODE.IOI.S3A.TB");
        builder.node_type("TIOIS", "IOI.S3A.TB", "NODE.IOI.S3A.TB");
        builder.node_type("TIOIB", "IOI.S3A.TB", "NODE.IOI.S3A.TB");
    } else if rd.family == "spartan3e" {
        builder.node_type("LIOIS", "IOI.S3E", "NODE.IOI");
        builder.node_type("LIOIS_BRK", "IOI.S3E", "NODE.IOI.BRK");
        builder.node_type("LIOIS_PCI", "IOI.S3E", "NODE.IOI");
        builder.node_type("LIOIS_CLK_PCI", "IOI.S3E", "NODE.IOI");
        builder.node_type("LIBUFS", "IOI.S3E", "NODE.IOI");
        builder.node_type("LIBUFS_PCI", "IOI.S3E", "NODE.IOI");
        builder.node_type("LIBUFS_CLK_PCI", "IOI.S3E", "NODE.IOI");
        builder.node_type("RIOIS", "IOI.S3E", "NODE.IOI");
        builder.node_type("RIOIS_PCI", "IOI.S3E", "NODE.IOI");
        builder.node_type("RIOIS_CLK_PCI", "IOI.S3E", "NODE.IOI");
        builder.node_type("RIBUFS", "IOI.S3E", "NODE.IOI");
        builder.node_type("RIBUFS_BRK", "IOI.S3E", "NODE.IOI.BRK");
        builder.node_type("RIBUFS_PCI", "IOI.S3E", "NODE.IOI");
        builder.node_type("RIBUFS_CLK_PCI", "IOI.S3E", "NODE.IOI");
        builder.node_type("BIOIS", "IOI.S3E", "NODE.IOI");
        builder.node_type("BIBUFS", "IOI.S3E", "NODE.IOI");
        builder.node_type("TIOIS", "IOI.S3E", "NODE.IOI");
        builder.node_type("TIBUFS", "IOI.S3E", "NODE.IOI");
    } else {
        // NOTE: could be unified by pulling extra muxes from CLB
        builder.node_type("LIOIS", "IOI.S3", "NODE.IOI");
        builder.node_type("RIOIS", "IOI.S3", "NODE.IOI");
        builder.node_type("BIOIS", "IOI.S3", "NODE.IOI");
        builder.node_type("TIOIS", "IOI.S3", "NODE.IOI");
    }
    // NOTE:
    // - S3/S3E/S3A could be unified by pulling some extra muxes from CLB
    // - S3A/S3ADSP adds VCC input to B[XY] and splits B[XY] to two nodes
    if rd.family == "spartan3adsp" {
        builder.node_type("BRAM0_SMALL", "BRAM.S3ADSP", "NODE.BRAM.S3ADSP");
        builder.node_type("BRAM0_SMALL_BOT", "BRAM.S3ADSP", "NODE.BRAM.S3ADSP");
        builder.node_type("BRAM1_SMALL", "BRAM.S3ADSP", "NODE.BRAM.S3ADSP");
        builder.node_type("BRAM2_SMALL", "BRAM.S3ADSP", "NODE.BRAM.S3ADSP");
        builder.node_type("BRAM3_SMALL", "BRAM.S3ADSP", "NODE.BRAM.S3ADSP");
        builder.node_type("BRAM3_SMALL_TOP", "BRAM.S3ADSP", "NODE.BRAM.S3ADSP");
        builder.node_type("BRAM3_SMALL_BRK", "BRAM.S3ADSP", "NODE.BRAM.S3ADSP.BRK");
        builder.node_type("MACC0_SMALL", "BRAM.S3ADSP", "NODE.MACC");
        builder.node_type("MACC0_SMALL_BOT", "BRAM.S3ADSP", "NODE.MACC");
        builder.node_type("MACC1_SMALL", "BRAM.S3ADSP", "NODE.MACC");
        builder.node_type("MACC2_SMALL", "BRAM.S3ADSP", "NODE.MACC");
        builder.node_type("MACC3_SMALL", "BRAM.S3ADSP", "NODE.MACC");
        builder.node_type("MACC3_SMALL_TOP", "BRAM.S3ADSP", "NODE.MACC");
        builder.node_type("MACC3_SMALL_BRK", "BRAM.S3ADSP", "NODE.MACC.BRK");
    } else if rd.family == "spartan3a" {
        builder.node_type("BRAM0_SMALL", "BRAM.S3A", "NODE.BRAM");
        builder.node_type("BRAM0_SMALL_BOT", "BRAM.S3A", "NODE.BRAM");
        builder.node_type("BRAM1_SMALL", "BRAM.S3A", "NODE.BRAM");
        builder.node_type("BRAM2_SMALL", "BRAM.S3A", "NODE.BRAM");
        builder.node_type("BRAM3_SMALL", "BRAM.S3A", "NODE.BRAM");
        builder.node_type("BRAM3_SMALL_TOP", "BRAM.S3A", "NODE.BRAM");
        builder.node_type("BRAM3_SMALL_BRK", "BRAM.S3A", "NODE.BRAM.BRK");
    } else if rd.family == "spartan3e" {
        builder.node_type("BRAM0_SMALL", "BRAM.S3E", "NODE.BRAM");
        builder.node_type("BRAM1_SMALL", "BRAM.S3E", "NODE.BRAM");
        builder.node_type("BRAM2_SMALL", "BRAM.S3E", "NODE.BRAM");
        builder.node_type("BRAM3_SMALL", "BRAM.S3E", "NODE.BRAM");
        builder.node_type("BRAM3_SMALL_BRK", "BRAM.S3E", "NODE.BRAM.BRK");
    } else {
        builder.node_type("BRAM0", "BRAM.S3", "NODE.BRAM");
        builder.node_type("BRAM1", "BRAM.S3", "NODE.BRAM");
        builder.node_type("BRAM2", "BRAM.S3", "NODE.BRAM");
        builder.node_type("BRAM3", "BRAM.S3", "NODE.BRAM");
        builder.node_type("BRAM0_SMALL", "BRAM.S3", "NODE.BRAM");
        builder.node_type("BRAM1_SMALL", "BRAM.S3", "NODE.BRAM");
        builder.node_type("BRAM2_SMALL", "BRAM.S3", "NODE.BRAM");
        builder.node_type("BRAM3_SMALL", "BRAM.S3", "NODE.BRAM");
    }
    builder.node_type("BRAM_IOIS", "DCM", "NODE.DCM.S3");
    builder.node_type("BRAM_IOIS_NODCM", "DCM.S3.DUMMY", "NODE.DCM.S3.DUMMY");
    builder.node_type("DCMAUX_BL_CENTER", "DCM.S3E.DUMMY", "NODE.DCM.S3E.DUMMY");
    builder.node_type("DCMAUX_TL_CENTER", "DCM.S3E.DUMMY", "NODE.DCM.S3E.DUMMY");
    builder.node_type("DCM_BL_CENTER", "DCM", "NODE.DCM.S3E");
    builder.node_type("DCM_TL_CENTER", "DCM", "NODE.DCM.S3E");
    builder.node_type("DCM_BR_CENTER", "DCM", "NODE.DCM.S3E");
    builder.node_type("DCM_TR_CENTER", "DCM", "NODE.DCM.S3E");
    builder.node_type("DCM_H_BL_CENTER", "DCM", "NODE.DCM.S3E.H");
    builder.node_type("DCM_H_TL_CENTER", "DCM", "NODE.DCM.S3E.H");
    builder.node_type("DCM_H_BR_CENTER", "DCM", "NODE.DCM.S3E.H");
    builder.node_type("DCM_H_TR_CENTER", "DCM", "NODE.DCM.S3E.H");
    builder.node_type("DCM_BGAP", "DCM", "NODE.DCM.S3E.H");
    builder.node_type("DCM_SPLY", "DCM", "NODE.DCM.S3E.H");
    builder.node_type("LL", "CLB", "NODE.CNR");
    builder.node_type("LR", "CLB", "NODE.CNR");
    builder.node_type("UL", "CLB", "NODE.CNR");
    builder.node_type("UR", "CLB", "NODE.CNR");

    builder.wire("PULLUP", int::WireKind::TiePullup, &[
        "VCC_PINWIRE",
        "IOIS_VCC_WIRE",
        "BRAM_VCC_WIRE",
        "MACC_VCC_WIRE",
        "BRAM_IOIS_VCC_WIRE",
        "DCM_VCC_WIRE",
        "CNR_VCC_WIRE",
    ]);

    for i in 0..8 {
        builder.wire(format!("GCLK{i}"), int::WireKind::ClkOut(i), &[
            format!("GCLK{i}"),
            format!("GCLK{i}_BRK"),
        ]);
    }
    for i in 0..4 {
        builder.wire(format!("DLL_CLKPAD{i}"), int::WireKind::ClkOut(8 + i), &[
            format!("BRAM_IOIS_DLL_CLKPAD{i}"),
            format!("DCM_DLL_CLKPAD{i}"),
            format!("DCM_H_DLL_CLKPAD{i}"),
        ]);
    }

    for (i, da1, da2, db) in [
        (0, Dir::S, None, None),
        (1, Dir::W, Some(Dir::S), None),
        (2, Dir::E, None, Some(Dir::S)),
        (3, Dir::S, Some(Dir::E), None),
        (4, Dir::S, None, None),
        (5, Dir::S, Some(Dir::W), None),
        (6, Dir::W, None, None),
        (7, Dir::E, Some(Dir::S), None),
        (8, Dir::E, Some(Dir::N), None),
        (9, Dir::W, None, Some(Dir::N)),
        (10, Dir::N, Some(Dir::W), None),
        (11, Dir::N, None, None),
        (12, Dir::N, Some(Dir::E), None),
        (13, Dir::E, None, None),
        (14, Dir::W, Some(Dir::N), None),
        (15, Dir::N, None, None),
    ] {
        let omux = builder.mux_out(format!("OMUX{i}"), &[
            format!("OMUX{i}"),
        ]);
        let omux_da1 = builder.branch(omux, da1, format!("OMUX{i}.{da1}"), &[
            format!("OMUX_{da1}{i}"),
        ]);
        if let Some(da2) = da2 {
            builder.branch(omux_da1, da2, format!("OMUX{i}.{da1}{da2}"), &[
                format!("OMUX_{da1}{da2}{i}"),
            ]);
        }
        if let Some(db) = db {
            builder.branch(omux, db, format!("OMUX{i}.{db}"), &[
                format!("{db}{da1}_{db}"),
            ]);
        }
    }

    for dir in Dir::DIRS {
        for i in 0..8 {
            let beg = builder.mux_out(format!("DBL.{dir}{i}.0"), &[
                format!("{dir}2BEG{i}"),
            ]);
            let mid = builder.branch(beg, dir, format!("DBL.{dir}{i}.1"), &[
                format!("{dir}2MID{i}"),
            ]);
            let end = builder.branch(mid, dir, format!("DBL.{dir}{i}.2"), &[
                format!("{dir}2END{i}"),
            ]);
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(end, Dir::S, format!("DBL.{dir}{i}.3"), &[
                    format!("{dir}2END_S{i}"),
                ]);
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 6 {
                builder.branch(end, Dir::N, format!("DBL.{dir}{i}.3"), &[
                    format!("{dir}2END_N{i}"),
                ]);
            }
        }
    }

    for dir in Dir::DIRS {
        for i in 0..8 {
            let mut last = builder.mux_out(format!("HEX.{dir}{i}.0"), &[
                format!("{dir}6BEG{i}"),
            ]);
            for (j, seg) in [
                (1, "A"),
                (2, "B"),
                (3, "MID"),
                (4, "C"),
                (5, "D"),
                (6, "END"),
            ] {
                last = builder.branch(last, dir, format!("HEX.{dir}{i}.{j}"), &[
                    format!("{dir}6{seg}{i}"),
                ]);
            }
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(last, Dir::S, format!("HEX.{dir}{i}.7"), &[
                    format!("{dir}6END_S{i}"),
                ]);
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 6 {
                builder.branch(last, Dir::N, format!("HEX.{dir}{i}.7"), &[
                    format!("{dir}6END_N{i}"),
                ]);
            }
        }
    }

    let lh: Vec<_> = (0..24).map(|i| builder.wire(format!("LH.{i}"), int::WireKind::MultiBranch(Dir::W), &[
        format!("LH{i}"),
    ])).collect();
    for i in 0..24 {
        builder.conn_branch(lh[i], Dir::E, lh[(i + 1) % 24]);
    }

    let lv: Vec<_> = (0..24).map(|i| builder.wire(format!("LV.{i}"), int::WireKind::MultiBranch(Dir::S), &[
        format!("LV{i}"),
    ])).collect();
    for i in 0..24 {
        builder.conn_branch(lv[i], Dir::N, lv[(i + 23) % 24]);
    }

    // The set/reset inputs.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.SR{i}"),
            &[
                format!("SR{i}"),
                format!("IOIS_SR{i}"),
                format!("CNR_SR{i}"),
                format!("BRAM_SR{i}"),
                format!("MACC_SR{i}"),
            ],
        );
    }

    // The clock inputs.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[
                format!("CLK{i}"),
                format!("CNR_CLK{i}"),
                format!("BRAM_CLK{i}"),
                format!("MACC_CLK{i}"),
                // these have a different mux
                ["", "BRAM_IOIS_PSCLK", "BRAM_IOIS_CLKIN", "BRAM_IOIS_CLKFB"][i].to_string(),
                ["", "DCM_PSCLK", "DCM_CLKIN", "DCM_CLKFB"][i].to_string(),
            ],
        );
    }

    for i in 0..8 {
        builder.mux_out(
            format!("IMUX.IOCLK{i}"),
            &[format!("IOIS_CLK{i}")],
        );
    }

    // The clock enables.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CE{i}"),
            &[
                format!("CE_B{i}"),
                format!("IOIS_CE_B{i}"),
                format!("CNR_CE_B{i}"),
                format!("BRAM_CE_B{i}"),
                format!("MACC_CE_B{i}"),
            ],
        );
    }

    for xy in ['X', 'Y'] {
        for i in 0..4 {
            let w = builder.mux_out(
                format!("IMUX.FAN.B{xy}{i}"),
                &[
                    format!("B{xy}{i}"),
                    format!("IOIS_FAN_B{xy}{i}"),
                    format!("CNR_B{xy}{i}"),
                    if rd.family == "spartan3adsp" {
                        format!("BRAM_B{xy}_B{i}")
                    } else {
                        format!("BRAM_FAN_B{xy}{i}")
                    },
                    format!("MACC_B{xy}_B{i}"),
                    format!("BRAM_IOIS_FAN_B{xy}{i}"),
                    format!("DCM_FAN_B{xy}{i}"),
                ],
            );
            let mut wires = vec![];
            if rd.family == "spartan3adsp" {
                wires.extend([format!("BRAM_FAN_B{xy}{i}"), format!("MACC_FAN_B{xy}{i}")]);
            }
            builder.buf(
                w,
                format!("IMUX.FAN.B{xy}{i}.BOUNCE"),
                &wires,
            );
        }
    }

    for i in 0..32 {
        builder.mux_out(
            format!("IMUX.DATA{i}"),
            &[
                format!("{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                format!("IOIS_{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                format!(
                    "TBIOIS_{}{}_B{}",
                    ["F", "G"][i >> 4],
                    (i >> 2 & 3) + 1,
                    i & 3
                ),
                format!(
                    "LRIOIS_{}{}_B{}",
                    ["F", "G"][i >> 4],
                    (i >> 2 & 3) + 1,
                    i & 3
                ),
                format!("CNR_DATA_IN{i}"),
                [
                    "BRAM_DIA_B18",
                    "BRAM_MULTINA_B15",
                    "BRAM_MULTINB_B17",
                    "BRAM_DIA_B1",
                    "BRAM_ADDRB_B0",
                    "BRAM_DIB_B19",
                    "BRAM_DIB_B0",
                    "BRAM_ADDRA_B3",
                    "BRAM_DIA_B19",
                    "BRAM_DIPB_B",
                    "BRAM_MULTINA_B17",
                    "BRAM_DIA_B0",
                    "BRAM_ADDRB_B1",
                    "BRAM_DIB_B18",
                    "BRAM_DIB_B1",
                    "BRAM_ADDRA_B2",
                    "BRAM_DIA_B2",
                    "BRAM_MULTINA_B14",
                    "BRAM_MULTINB_B16",
                    "BRAM_DIA_B17",
                    "BRAM_ADDRA_B0",
                    "BRAM_DIB_B3",
                    "BRAM_DIB_B16",
                    "BRAM_ADDRB_B3",
                    "BRAM_DIA_B3",
                    "BRAM_DIPA_B",
                    "BRAM_MULTINA_B16",
                    "BRAM_DIA_B16",
                    "BRAM_ADDRA_B1",
                    "BRAM_DIB_B2",
                    "BRAM_DIB_B17",
                    "BRAM_ADDRB_B2",
                ][i].to_string(),
                // 3A DSP version
                [
                    "",
                    "BRAM_MULTINA_B1",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "BRAM_MULTINA_B3",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "BRAM_MULTINA_B0",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "BRAM_MULTINA_B2",
                    "",
                    "",
                    "",
                    "",
                    "",
                ][i].to_string(),
                [
                    "MACC_DIA_B18",
                    "MACC_MULTINA_B1",
                    "MACC_MULTINB_B17",
                    "MACC_DIA_B1",
                    "MACC_ADDRB_B0",
                    "MACC_DIB_B19",
                    "MACC_DIB_B0",
                    "MACC_ADDRA_B3",
                    "MACC_DIA_B19",
                    "MACC_DIPB_B",
                    "MACC_MULTINA_B3",
                    "MACC_DIA_B0",
                    "MACC_ADDRB_B1",
                    "MACC_DIB_B18",
                    "MACC_DIB_B1",
                    "MACC_ADDRA_B2",
                    "MACC_DIA_B2",
                    "MACC_MULTINA_B0",
                    "MACC_MULTINB_B16",
                    "MACC_DIA_B17",
                    "MACC_ADDRA_B0",
                    "MACC_DIB_B3",
                    "MACC_DIB_B16",
                    "MACC_ADDRB_B3",
                    "MACC_DIA_B3",
                    "MACC_DIPA_B",
                    "MACC_MULTINA_B2",
                    "MACC_DIA_B16",
                    "MACC_ADDRA_B1",
                    "MACC_DIB_B2",
                    "MACC_DIB_B17",
                    "MACC_ADDRB_B2",
                ][i].to_string(),
                format!(
                    "BRAM_IOIS_{}{}_B{}",
                    ["F", "G"][i >> 4],
                    (i >> 2 & 3) + 1,
                    i & 3
                ),
                format!("DCM_{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                [
                    "",
                    "",
                    "DCM_CTLSEL0_STUB",
                    "DCM_CTLSEL1_STUB",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "DCM_DSSEN_STUB",
                    "DCM_PSEN_STUB",
                    "DCM_PSINCDEC_STUB",
                    "DCM_RST_STUB",
                    "DCM_STSADRS1_STUB",
                    "DCM_STSADRS2_STUB",
                    "DCM_STSADRS3_STUB",
                    "DCM_STSADRS4_STUB",
                    "DCM_CTLMODE_STUB",
                    "DCM_FREEZEDLL_STUB",
                    "DCM_FREEZEDFS_STUB",
                    "DCM_STSADRS0_STUB",
                    "DCM_CTLSEL2_STUB",
                    "DCM_CTLOSC2_STUB",
                    "DCM_CTLOSC1_STUB",
                    "DCM_CTLG0_STUB",
                ][i].to_string(),
            ],
        );
    }

    for i in 0..8 {
        builder.logic_out(
            format!("OUT.FAN{i}"),
            &[
                // In CLBs, used for combinatorial outputs.
                ["X0", "X1", "X2", "X3", "Y0", "Y1", "Y2", "Y3"][i],
                [
                    "IOIS_X0", "IOIS_X1", "IOIS_X2", "IOIS_X3", "IOIS_Y0", "IOIS_Y1",
                    "IOIS_Y2", "IOIS_Y3",
                ][i],
                // In BRAM, used for low data outputs.
                [
                    "BRAM_DOA0",
                    "BRAM_DOA1",
                    "BRAM_DOA2",
                    "BRAM_DOA3",
                    "BRAM_DOB0",
                    "BRAM_DOB1",
                    "BRAM_DOB2",
                    "BRAM_DOB3",
                ][i],
                [
                    "MACC_DOA0",
                    "MACC_DOA1",
                    "MACC_DOA2",
                    "MACC_DOA3",
                    "MACC_DOB0",
                    "MACC_DOB1",
                    "MACC_DOB2",
                    "MACC_DOB3",
                ][i],
                [
                    "BRAM_IOIS_CLK270",
                    "BRAM_IOIS_CLK180",
                    "BRAM_IOIS_CLK90",
                    "BRAM_IOIS_CLK0",
                    "BRAM_IOIS_CLKFX180",
                    "BRAM_IOIS_CLKFX",
                    "BRAM_IOIS_CLK2X180",
                    "BRAM_IOIS_CLK2X",
                ][i],
                [
                    "DCM_CLK270",
                    "DCM_CLK180",
                    "DCM_CLK90",
                    "DCM_CLK0",
                    "DCM_CLKFX180",
                    "DCM_CLKFX",
                    "DCM_CLK2X180",
                    "DCM_CLK2X",
                ][i],
                &format!("CNR_D_O_FAN_B{i}")[..],
            ],
        );
    }

    for i in 0..16 {
        builder.logic_out(
            format!("OUT.SEC{i}"),
            &[
                [
                    "XB0", "XB1", "XB2", "XB3", "YB0", "YB1", "YB2", "YB3", "XQ0", "XQ1",
                    "XQ2", "XQ3", "YQ0", "YQ1", "YQ2", "YQ3",
                ][i],
                [
                    "", "", "", "", "", "", "", "", "IOIS_XQ0", "IOIS_XQ1", "IOIS_XQ2",
                    "IOIS_XQ3", "IOIS_YQ0", "IOIS_YQ1", "IOIS_YQ2", "IOIS_YQ3",
                ][i],
                // sigh. this does not appear to actually be true.
                [
                    "",
                    "",
                    "",
                    "",
                    "BRAM_DOPA",
                    "BRAM_DOPB",
                    "",
                    "BRAM_MOUT32",
                    "BRAM_MOUT7",
                    "BRAM_MOUT6",
                    "BRAM_MOUT5",
                    "BRAM_MOUT4",
                    "BRAM_MOUT3",
                    "BRAM_MOUT2",
                    "BRAM_MOUT1",
                    "BRAM_MOUT0",
                ][i],
                [
                    "",
                    "",
                    "",
                    "",
                    "MACC_DOPA",
                    "MACC_DOPB",
                    "",
                    "MACC_MOUT32",
                    "MACC_MOUT7",
                    "MACC_MOUT6",
                    "MACC_MOUT5",
                    "MACC_MOUT4",
                    "MACC_MOUT3",
                    "MACC_MOUT2",
                    "MACC_MOUT1",
                    "MACC_MOUT0",
                ][i],
                [
                    "BRAM_IOIS_PSDONE",
                    "BRAM_IOIS_CONCUR",
                    "BRAM_IOIS_LOCKED",
                    "BRAM_IOIS_CLKDV",
                    "BRAM_IOIS_STATUS4",
                    "BRAM_IOIS_STATUS5",
                    "BRAM_IOIS_STATUS6",
                    "BRAM_IOIS_STATUS7",
                    "BRAM_IOIS_STATUS0",
                    "BRAM_IOIS_STATUS1",
                    "BRAM_IOIS_STATUS2",
                    "BRAM_IOIS_STATUS3",
                    "BRAM_IOIS_PTE2OMUX0",
                    "BRAM_IOIS_PTE2OMUX1",
                    "BRAM_IOIS_PTE2OMUX2",
                    "BRAM_IOIS_PTE2OMUX3",
                ][i],
                [
                    "DCM_PSDONE",
                    "DCM_CONCUR",
                    "DCM_LOCKED",
                    "DCM_CLKDV",
                    "DCM_STATUS4",
                    "DCM_STATUS5",
                    "DCM_STATUS6",
                    "DCM_STATUS7",
                    "DCM_STATUS0",
                    "DCM_STATUS1",
                    "DCM_STATUS2",
                    "DCM_STATUS3",
                    "DCM_PTE2OMUX0",
                    "DCM_PTE2OMUX1",
                    "DCM_PTE2OMUX2",
                    "DCM_PTE2OMUX3",
                ][i],
                [
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "DCM_PTE2OMUX0_STUB",
                    "DCM_PTE2OMUX1_STUB",
                    "DCM_PTE2OMUX2_STUB",
                    "DCM_PTE2OMUX3_STUB",
                ][i],
                &format!("CNR_D_OUT_B{i}")[..],
            ],
        );
    }
    builder.stub_out("STUB_IOIS_X3");
    builder.stub_out("STUB_IOIS_Y3");
    builder.stub_out("STUB_IOIS_XQ3");
    builder.stub_out("STUB_IOIS_YQ3");

    for i in 0..4 {
        for j in 0..2 {
            builder.logic_out(
                format!("OUT.HALF{i}.{j}"),
                &[
                    [
                        "BRAM_DOA16",
                        "BRAM_DOA17",
                        "BRAM_DOA19",
                        "BRAM_DOA18",
                        "BRAM_DOB16",
                        "BRAM_DOB17",
                        "BRAM_DOB19",
                        "BRAM_DOB18",
                    ][i + j * 4],
                    [
                        "MACC_DOA16",
                        "MACC_DOA17",
                        "MACC_DOA19",
                        "MACC_DOA18",
                        "MACC_DOB16",
                        "MACC_DOB17",
                        "MACC_DOB19",
                        "MACC_DOB18",
                    ][i + j * 4],
                ],
            );
        }
    }

    builder.extract_nodes();

    for tkn in [
        "LTERM",
        "LTERM1",
        "LTERM2",
        "LTERM3",
        "LTERM4",
        "LTERM4B",
        "LTERM4CLK",
        "LTERMCLK",
        "LTERMCLKA",
        "CNR_LBTERM",
        "CNR_LTTERM",
    ] {
        builder.extract_term("W", Dir::W, tkn, "TERM.W");
    }
    for tkn in [
        "RTERM",
        "RTERM1",
        "RTERM2",
        "RTERM3",
        "RTERM4",
        "RTERM4B",
        "RTERM4CLK",
        "RTERM4CLKB",
        "RTERMCLKA",
        "RTERMCLKB",
        "CNR_RBTERM",
        "CNR_RTTERM",
    ] {
        builder.extract_term("E", Dir::E, tkn, "TERM.E");
    }
    for tkn in [
        "BTERM",
        "BTERM1",
        "BTERM1_MACC",
        "BTERM2",
        "BTERM2CLK",
        "BTERM3",
        "BTERM4",
        "BTERM4CLK",
        "BTERM4_BRAM2",
        "BTERMCLK",
        "BTERMCLKA",
        "BTERMCLKB",
        "BCLKTERM2",
        "BCLKTERM3",
        "BBTERM",
    ] {
        builder.extract_term("S", Dir::S, tkn, "TERM.S");
    }
    for tkn in [
        "TTERM",
        "TTERM1",
        "TTERM1_MACC",
        "TTERM2",
        "TTERM2CLK",
        "TTERM3",
        "TTERM4",
        "TTERM4CLK",
        "TTERM4_BRAM2",
        "TTERMCLK",
        "TTERMCLKA",
        "TCLKTERM2",
        "TCLKTERM3",
        "BTTERM",
    ] {
        builder.extract_term("N", Dir::N, tkn, "TERM.N");
    }
    builder.extract_term("S", Dir::S, "CNR_BTERM", "TERM.S.CNR");
    builder.extract_term("N", Dir::N, "CNR_TTERM", "TERM.N.CNR");

    if rd.family == "spartan3e" {
        let cob_term_t_y = rd.tile_kinds["COB_TERM_T"].tiles[0].y;
        for &xy_b in &rd.tile_kinds["COB_TERM_B"].tiles {
            let xy_t = Coord {
                x: xy_b.x,
                y: cob_term_t_y,
            };
            let int_s_xy = builder.walk_to_int(xy_b, Dir::S).unwrap();
            let int_n_xy = builder.walk_to_int(xy_t, Dir::N).unwrap();
            builder.extract_pass_tile("BRAM.S", Dir::S, int_n_xy, Some((xy_t, "TERM.BRAM.S", None)), None, int_s_xy, &lv);
            builder.extract_pass_tile("BRAM.N", Dir::N, int_s_xy, Some((xy_b, "TERM.BRAM.N", None)), None, int_n_xy, &lv);
        }
        for tkn in [
            "CLKL_IOIS",
            "CLKR_IOIS",
        ] {
            builder.extract_pass_simple("CLKLR.S3E", Dir::S, tkn, &[]);
        }
    }
    if rd.family == "spartan3" {
        builder.extract_pass_simple("BRKH.S3", Dir::S, "BRKH", &[]);
    }
    for tkn in [
        "CLKH_LL",
        "CLKH_DCM_LL",
        "CLKLH_DCM_LL",
        "CLKRH_DCM_LL",
    ] {
        builder.extract_pass_buf("LLV", Dir::S, tkn, "LLV");
    }
    let llv_clklr_kind = if rd.family == "spartan3a" {
        "LLV"
    } else {
        "LLV.CLKLR.S3E"
    };
    builder.extract_pass_buf(llv_clklr_kind, Dir::S, "CLKL_IOIS_LL", "LLV.CLKL");
    builder.extract_pass_buf(llv_clklr_kind, Dir::S, "CLKR_IOIS_LL", "LLV.CLKR");
    for tkn in [
        "CLKV_LL",
        "CLKT_LL",
    ] {
        builder.extract_pass_buf("LLH", Dir::W, tkn, "LLH");
    }
    if let Some(tk) = rd.tile_kinds.get("CLKB_LL") {
        for &xy in &tk.tiles {
            let fix_xy = Coord {
                x: xy.x,
                y: xy.y + 1,
            };
            let int_fwd_xy = builder.walk_to_int(fix_xy, Dir::W).unwrap();
            let int_bwd_xy = builder.walk_to_int(fix_xy, Dir::E).unwrap();
            builder.extract_pass_tile("LLH.W", Dir::W, int_bwd_xy, Some((xy, "LLH.E", Some("LLH.W"))), None, int_fwd_xy, &[]);
            builder.extract_pass_tile("LLH.E", Dir::E, int_fwd_xy, Some((xy, "LLH.W", Some("LLH.E"))), None, int_bwd_xy, &[]);
        }
    }
    if rd.family == "spartan3adsp" {
        for tkn in [
            "EMPTY_TIOI",
            "EMPTY_BIOI",
        ] {
            builder.extract_pass_simple("DSPHOLE", Dir::W, tkn, &lh);
        }
        if let Some(tk) = rd.tile_kinds.get("DCM_BGAP") {
            for &xy in &tk.tiles {
                let mut int_w_xy = xy;
                let mut int_e_xy = xy;
                int_e_xy.x += 5;
                builder.extract_pass_tile("DSPHOLE.W", Dir::W, int_e_xy, None, None, int_w_xy, &lh);
                builder.extract_pass_tile("DSPHOLE.E", Dir::E, int_w_xy, None, None, int_e_xy, &lh);
                int_w_xy.x -= 1;
                for _ in 0..3 {
                    int_w_xy.y -= 1;
                    int_e_xy.y -= 1;
                    builder.extract_pass_tile("HDCM.W", Dir::W, int_e_xy, None, None, int_w_xy, &lh);
                    builder.extract_pass_tile("HDCM.E", Dir::E, int_w_xy, None, None, int_e_xy, &lh);
                }
            }
        }
        if let Some(tk) = rd.tile_kinds.get("DCM_SPLY") {
            for &xy in &tk.tiles {
                let mut int_w_xy = xy;
                let mut int_e_xy = xy;
                int_e_xy.x += 5;
                builder.extract_pass_tile("DSPHOLE.W", Dir::W, int_e_xy, None, None, int_w_xy, &lh);
                builder.extract_pass_tile("DSPHOLE.E", Dir::E, int_w_xy, None, None, int_e_xy, &lh);
                int_w_xy.x -= 1;
                for _ in 0..3 {
                    int_w_xy.y += 1;
                    int_e_xy.y += 1;
                    builder.extract_pass_tile("HDCM.W", Dir::W, int_e_xy, None, None, int_w_xy, &lh);
                    builder.extract_pass_tile("HDCM.E", Dir::E, int_w_xy, None, None, int_e_xy, &lh);
                }
            }
        }
        builder.extract_pass_buf("LLH.DCM.S3ADSP", Dir::W, "CLKV_DCM_LL", "LLH");
    } else {
        builder.extract_pass_buf("LLH", Dir::W, "CLKV_DCM_LL", "LLH");
    }

    // XXX
    // - extract bels + namings
    //   - RLL
    //   - SLICE ×4
    //   - IOI.S3 ×3
    //   - IOI.S3A ×3
    //   - BRAM.S3
    //   - BRAM.S3A
    //   - BRAM.S3ADSP
    //   - MULT.S3
    //   - MULT.S3E
    //   - DSP
    //   - DCM.S3
    //   - DCM.S3E
    //   - corners
    //     - DCI ×8 [?]
    //     - DCIRESET ×8 [?]
    //     - CARRYOUT
    //     - PMV
    //     - BSCAN.S3
    //     - BSCAN.S3A
    //     - STARTUP.S3
    //     - STARTUP.S3E
    //     - STARTUP.S3A
    //     - ICAP.S3
    //     - ICAP.S3A
    //     - CAPTURE
    //     - SPI_ACCESS
    //     - DNA_PORT
    //   - BUFGMUX.[BT] ×4
    //   - BUFGMUX.[LR] ×8 [?]
    //   - PCILOGICSE
    builder.build()
}

fn make_grid(rd: &Part) -> virtex2::Grid {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &[
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
    ], &[]);
    let kind = get_kind(rd);
    let mut columns = make_columns(rd, &int, kind);
    get_cols_io(rd, &int, kind, &mut columns);
    let mut grid = virtex2::Grid {
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
        vref: BTreeSet::new(),
        cfg_io: BTreeMap::new(),
        dci_io: BTreeMap::new(),
        dci_io_alt: BTreeMap::new(),
    };
    handle_spec_io(rd, &mut grid);
    grid
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(grid: &virtex2::Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, (io.coord, io.bank)))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if pad.starts_with("PAD") || pad.starts_with("IPAD") || pad.starts_with("CLK") {
                let (coord, bank) = io_lookup[pad];
                assert_eq!(pin.vref_bank, Some(bank));
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByCoord(coord)
            } else if let Some((n, b)) = split_num(pad) {
                let pk = match n {
                    "RXPPAD" => GtPin::RxP,
                    "RXNPAD" => GtPin::RxN,
                    "TXPPAD" => GtPin::TxP,
                    "TXNPAD" => GtPin::TxN,
                    _ => panic!("FUNNY PAD {}", pad),
                };
                BondPin::GtByBank(b, pk, 0)
            } else {
                panic!("FUNNY PAD {}", pad);
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "RSVD" => BondPin::Rsvd, // virtex2: likely DXP/DXN
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VCCO" => BondPin::VccO(0),
                "VBATT" => BondPin::VccBatt,
                "TCK" => BondPin::Cfg(CfgPin::Tck),
                "TDI" => BondPin::Cfg(CfgPin::Tdi),
                "TDO" => BondPin::Cfg(CfgPin::Tdo),
                "TMS" => BondPin::Cfg(CfgPin::Tms),
                "CCLK" => BondPin::Cfg(CfgPin::Cclk),
                "DONE" => BondPin::Cfg(CfgPin::Done),
                "PROG_B" => BondPin::Cfg(CfgPin::ProgB),
                "M0" => BondPin::Cfg(CfgPin::M0),
                "M1" => BondPin::Cfg(CfgPin::M1),
                "M2" => BondPin::Cfg(CfgPin::M2),
                "HSWAP_EN" => BondPin::Cfg(CfgPin::HswapEn),
                "PWRDWN_B" => BondPin::Cfg(CfgPin::PwrdwnB),
                "SUSPEND" => BondPin::Cfg(CfgPin::Suspend),
                "DXN" => BondPin::Dxn,
                "DXP" => BondPin::Dxp,
                _ => if let Some((n, b)) = split_num(&pin.func) {
                    match n {
                        "VCCO_" => BondPin::VccO(b),
                        "GNDA" => BondPin::GtByBank(b, GtPin::GndA, 0),
                        "VTRXPAD" => BondPin::GtByBank(b, GtPin::VtRx, 0),
                        "VTTXPAD" => BondPin::GtByBank(b, GtPin::VtTx, 0),
                        "AVCCAUXRX" => BondPin::GtByBank(b, GtPin::AVccAuxRx, 0),
                        "AVCCAUXTX" => BondPin::GtByBank(b, GtPin::AVccAuxTx, 0),
                        _ => {
                            println!("UNK FUNC {}", pin.func);
                            continue;
                        }
                    }
                } else {
                    println!("UNK FUNC {}", pin.func);
                    continue;
                }
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks,
    }
}

pub fn ingest(rd: &Part) -> (PreDevice, Option<int::IntDb>) {
    let grid = make_grid(rd);
    let int_db = if rd.family.starts_with("virtex2") {
        make_int_db_v2(rd)
    } else {
        make_int_db_s3(rd)
    };
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(&grid, pins),
        ));
    }
    let eint = grid.expand_grid(&int_db);
    let mut vrf = Verifier::new(rd, &eint);
    vrf.finish();
    (make_device(rd, geom::Grid::Virtex2(grid), bonds, BTreeSet::new()), Some(int_db))
}

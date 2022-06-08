use std::collections::{BTreeMap, BTreeSet, HashSet, HashMap};

use prjcombine_xilinx_rawdump::{Part, Coord, PkgPin};
use prjcombine_xilinx_geom::{self as geom, BondPin, CfgPin, Bond, GtPin};
use prjcombine_xilinx_geom::virtex2::{self, GridKind, ColumnKind, ColumnIoKind, RowIoKind, Dcms};

use itertools::Itertools;

use crate::grid::{extract_int, find_columns, find_column, find_rows, find_row, IntGrid, PreDevice, make_device};

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

fn make_columns(rd: &Part, int: &IntGrid, kind: GridKind) -> Vec<ColumnKind> {
    let mut res = Vec::new();
    res.push(ColumnKind::Io);
    for _ in 0..(int.cols.len() - 2) {
        res.push(ColumnKind::Clb);
    }
    res.push(ColumnKind::Io);
    let bram_cont = match kind {
        GridKind::Spartan3E | GridKind::Spartan3A => 4,
        GridKind::Spartan3ADsp => 3,
        _ => 0,
    };
    for rc in find_columns(rd, &["BRAM0", "BRAM0_SMALL"]) {
        let c = int.lookup_column(rc);
        res[c as usize] = ColumnKind::Bram;
        if bram_cont != 0 {
            for d in 1..bram_cont {
                res[(c + d) as usize] = ColumnKind::BramCont(d as u8);
            }
        }
    }
    for rc in find_columns(rd, &["MACC0_SMALL"]) {
        let c = int.lookup_column(rc);
        res[c as usize] = ColumnKind::Dsp;
    }
    res
}

fn get_cols_io(rd: &Part, int: &IntGrid, kind: GridKind, cols: &[ColumnKind]) -> Vec<ColumnIoKind> {
    let mut res = Vec::new();
    res.push(ColumnIoKind::None);
    while res.len() < int.cols.len() - 1 {
        match kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                let c0 = Coord {
                    x: int.cols[res.len()] as u16,
                    y: 0,
                };
                let c1 = Coord {
                    x: (int.cols[res.len()] + 1) as u16,
                    y: 0,
                };
                let tk0 = &rd.tiles[&c0].kind[..];
                let tk1 = &rd.tiles[&c1].kind[..];
                match (tk0, tk1) {
                    ("BTERM012" | "BCLKTERM012" | "ML_BCLKTERM012", "BTERM323") | ("BTERM010", "BTERM123" | "BCLKTERM123" | "ML_BCLKTERM123") => {
                        for i in 0..2 {
                            res.push(ColumnIoKind::Double(i));
                        }
                    }
                    ("BTERM123" | "BTERM012", _) => {
                        res.push(ColumnIoKind::Single);
                    }
                    ("BBTERM", _) => {
                        res.push(ColumnIoKind::None);
                    }
                    ("BGIGABIT_IOI_TERM" | "BGIGABIT10_IOI_TERM", _) => {
                        for _ in 0..3 {
                            res.push(ColumnIoKind::None);
                        }
                    }
                    _ => panic!("unknown tk {tk0} {tk1}"),
                }
            }
            GridKind::Spartan3 => {
                if cols[res.len()] == ColumnKind::Bram {
                    res.push(ColumnIoKind::None);
                } else {
                    for i in 0..2 {
                        res.push(ColumnIoKind::Double(i));
                    }
                }
            }
            GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                for i in 0..2 {
                    res.push(ColumnIoKind::Double(i));
                }
            }
            GridKind::Spartan3E => {
                let c = Coord {
                    x: int.cols[res.len()] as u16,
                    y: 0,
                };
                let tk = &rd.tiles[&c].kind[..];
                match tk {
                    "BTERM4" | "BTERM4_BRAM2" | "BTERM4CLK" => {
                        for i in 0..4 {
                            res.push(ColumnIoKind::Quad(i));
                        }
                    }
                    "BTERM3" => {
                        for i in 0..3 {
                            res.push(ColumnIoKind::Triple(i));
                        }
                    }
                    "BTERM2" => {
                        for i in 0..2 {
                            res.push(ColumnIoKind::Double(i));
                        }
                    }
                    "BTERM1" => {
                        res.push(ColumnIoKind::Single);
                    }
                    _ => panic!("unknown tk {tk}"),
                }
            }
        }
    }
    res.push(ColumnIoKind::None);
    assert_eq!(res.len(), int.cols.len());
    res
}

fn get_col_clk(rd: &Part, int: &IntGrid) -> u32 {
    int.lookup_column(find_column(rd, &["CLKC", "CLKC_50A", "CLKC_LL"]).unwrap() + 1)
}

fn get_cols_clkv(rd: &Part, int: &IntGrid) -> Option<(u32, u32)> {
    let cols: Vec<_> = find_columns(rd, &["GCLKV"]).into_iter().sorted().collect();
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

fn get_cols_gt(rd: &Part, int: &IntGrid) -> BTreeMap<u32, (u32, u32)> {
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

fn get_rows_io(rd: &Part, int: &IntGrid, kind: GridKind) -> Vec<RowIoKind> {
    let mut res = Vec::new();
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
                    y: int.rows[res.len()] as u16,
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

fn get_rows_ram(rd: &Part, int: &IntGrid, kind: GridKind) -> Option<(u32, u32)> {
    if kind == GridKind::Spartan3E {
        let b = int.lookup_row(find_row(rd, &["COB_TERM_B"]).unwrap());
        let t = int.lookup_row(find_row(rd, &["COB_TERM_T"]).unwrap());
        Some((b, t))
    } else {
        None
    }
}

fn get_rows_hclk(rd: &Part, int: &IntGrid) -> Vec<(u32, u32)> {
    let rows_hclk: Vec<_> = find_rows(rd, &["GCLKH"])
        .into_iter()
        .map(|r| int.lookup_row(r - 1) + 1)
        .sorted()
        .collect();
    let mut rows_brk = HashSet::new();
    for r in find_rows(rd, &["BRKH", "CLKH", "CLKH_LL"]) {
        rows_brk.insert(int.lookup_row(r - 1) + 1);
    }
    for r in find_rows(rd, &["CENTER_SMALL_BRK"]) {
        rows_brk.insert(int.lookup_row(r) + 1);
    }
    rows_brk.insert(int.rows.len() as u32);
    let rows_brk: Vec<_> = rows_brk.into_iter().sorted().collect();
    assert_eq!(rows_hclk.len(), rows_brk.len());
    rows_hclk.into_iter().zip(rows_brk.into_iter()).collect()
}

fn get_row_pci(rd: &Part, int: &IntGrid, kind: GridKind) -> Option<u32> {
    match kind {
        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
            Some(int.lookup_row(find_row(rd, &["REG_L"]).unwrap() + 1))
        }
        _ => None,
    }
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(u32, u32)> {
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
    let columns = make_columns(rd, &int, kind);
    let cols_io = get_cols_io(rd, &int, kind, &columns);
    let mut grid = virtex2::Grid {
        kind,
        columns,
        cols_io,
        col_clk: get_col_clk(rd, &int),
        cols_clkv: get_cols_clkv(rd, &int),
        cols_gt: get_cols_gt(rd, &int),
        rows: int.rows.len() as u32,
        rows_io: get_rows_io(rd, &int, kind),
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

pub fn ingest(rd: &Part) -> PreDevice {
    let grid = make_grid(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(&grid, pins),
        ));
    }
    make_device(rd, geom::Grid::Virtex2(grid), bonds, BTreeSet::new())
}

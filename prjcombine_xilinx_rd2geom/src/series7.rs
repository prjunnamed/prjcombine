use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use prjcombine_xilinx_rawdump::{Part, PkgPin, Coord};
use prjcombine_xilinx_geom::{self as geom, CfgPin, Bond, BondPin, GtPin, GtRegionPin, SysMonPin, ExtraDie, PsPin};
use prjcombine_xilinx_geom::series7::{self, GridKind, ColumnKind, IoColumn, IoKind, HoleKind, Hole, GtColumn, GtKind};

use itertools::Itertools;

use crate::grid::{extract_int, find_column, find_columns, find_rows, find_row, find_tiles, IntGrid, ExtraCol, PreDevice, make_device_multi};

fn get_kind(rd: &Part) -> GridKind {
    if find_columns(rd, &["GTX_COMMON", "GTH_COMMON"]).is_empty() {
        GridKind::Artix
    } else if !find_columns(rd, &["MONITOR_BOT_FUJI2", "MONITOR_BOT_PELE1"]).is_empty() {
        GridKind::Kintex
    } else {
        GridKind::Virtex
    }
}

fn make_columns(rd: &Part, int: &IntGrid) -> Vec<ColumnKind> {
    let mut res: Vec<Option<ColumnKind>> = Vec::new();
    for _ in 0..int.cols.len() {
        res.push(None);
    }
    res[0] = Some(ColumnKind::Gt);
    res[int.cols.len() - 1] = Some(ColumnKind::Gt);
    for c in find_columns(rd, &["CLBLL_L"]) {
        res[int.lookup_column(c + 1) as usize] = Some(ColumnKind::ClbLL);
    }
    for c in find_columns(rd, &["CLBLM_L"]) {
        res[int.lookup_column(c + 1) as usize] = Some(ColumnKind::ClbLM);
    }
    for c in find_columns(rd, &["CLBLL_R"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::ClbLL);
    }
    for c in find_columns(rd, &["CLBLM_R"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::ClbLM);
    }
    for c in find_columns(rd, &["BRAM_L"]) {
        res[int.lookup_column(c + 2) as usize] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["BRAM_R"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP_L"]) {
        res[int.lookup_column(c + 2) as usize] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["DSP_R"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["RIOI", "RIOI3"]) {
        res[int.lookup_column_inter(c) as usize - 1] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["LIOI", "LIOI3"]) {
        res[int.lookup_column_inter(c) as usize] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["CMT_FIFO_R"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Cmt);
    }
    for c in find_columns(rd, &["CMT_FIFO_L"]) {
        res[int.lookup_column(c + 2) as usize] = Some(ColumnKind::Cmt);
    }
    for c in find_columns(rd, &["VFRAME"]) {
        res[int.lookup_column(c + 2) as usize] = Some(ColumnKind::Cfg);
    }
    for c in find_columns(rd, &["CLK_HROW_BOT_R"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Clk);
    }
    for c in find_columns(rd, &["CFG_CENTER_BOT"]) {
        for d in [-10, -9, -6, -5, -2, -1] {
            res[int.lookup_column(c + d) as usize] = Some(ColumnKind::ClbLL);
        }
    }
    for c in find_columns(rd, &["INT_INTERFACE_PSS_L"]) {
        for (d, kind) in [
            (-46, ColumnKind::Io),
            (-45, ColumnKind::Cmt),
            (-39, ColumnKind::ClbLM),
            (-38, ColumnKind::ClbLM),
            (-35, ColumnKind::ClbLM),
            (-34, ColumnKind::ClbLM),
            (-29, ColumnKind::Bram),
            (-28, ColumnKind::ClbLM),
            (-25, ColumnKind::ClbLM),
            (-24, ColumnKind::Dsp),
            (-19, ColumnKind::ClbLM),
            (-18, ColumnKind::ClbLM),
            (-15, ColumnKind::ClbLM),
            (-14, ColumnKind::ClbLM),
            (-9, ColumnKind::Dsp),
            (-8, ColumnKind::ClbLM),
            (-5, ColumnKind::ClbLM),
            (-4, ColumnKind::Bram),
            (1, ColumnKind::ClbLL),
        ] {
            res[int.lookup_column(c + d) as usize] = Some(kind);
        }
    }
    res.into_iter().map(|x| x.unwrap()).collect()
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<u32> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["VBRK"]) {
        res.insert(int.lookup_column_inter(c));
    }
    for c in find_columns(rd, &["INT_INTERFACE_PSS_L"]) {
        res.insert(int.lookup_column_inter(c - 41));
        res.insert(int.lookup_column_inter(c - 32));
        res.insert(int.lookup_column_inter(c - 21));
        res.insert(int.lookup_column_inter(c - 12));
        res.insert(int.lookup_column_inter(c - 1));
    }
    res
}

fn get_holes(rd: &Part, int: &IntGrid, row_start: u32, row_end: u32) -> Vec<Hole> {
    let mut res = Vec::new();
    for (x, y) in find_tiles(rd, &["PCIE_BOT"]).into_iter().sorted() {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 10);
        assert_eq!(row % 50, 0);
        if row < row_start || row >= row_end {
            continue;
        }
        res.push(Hole {
            kind: HoleKind::Pcie2Right,
            col,
            row: row - row_start,
        });
    }
    for (x, y) in find_tiles(rd, &["PCIE_BOT_LEFT"]).into_iter().sorted() {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 10);
        assert_eq!(row % 50, 0);
        if row < row_start || row >= row_end {
            continue;
        }
        res.push(Hole {
            kind: HoleKind::Pcie2Left,
            col,
            row: row - row_start,
        });
    }
    for (x, y) in find_tiles(rd, &["PCIE3_BOT_RIGHT"]).into_iter().sorted() {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 7);
        assert_eq!(row % 50, 25);
        if row < row_start || row >= row_end {
            continue;
        }
        res.push(Hole {
            kind: HoleKind::Pcie3,
            col,
            row: row - row_start,
        });
    }
    for (x, y) in find_tiles(rd, &["GTP_CHANNEL_0_MID_LEFT"]).into_iter().sorted() {
        let col = int.lookup_column(x - 14);
        let row = int.lookup_row(y - 5);
        assert_eq!(row % 50, 0);
        if row < row_start || row >= row_end {
            continue;
        }
        res.push(Hole {
            kind: HoleKind::GtpLeft,
            col,
            row: row - row_start,
        });
    }
    for (x, y) in find_tiles(rd, &["GTP_CHANNEL_0_MID_RIGHT"]).into_iter().sorted() {
        let col = int.lookup_column(x + 19);
        let row = int.lookup_row(y - 5);
        assert_eq!(row % 50, 0);
        if row < row_start || row >= row_end {
            continue;
        }
        res.push(Hole {
            kind: HoleKind::GtpRight,
            col,
            row: row - row_start,
        });
    }
    res
}

fn get_cols_io(rd: &Part, int: &IntGrid, row_start: u32, row_end: u32) -> [Option<IoColumn>; 2] {
    let mut res = [None, None];
    if let Some(x) = find_column(rd, &["LIOI", "LIOI3"]) {
        let col = int.lookup_column_inter(x);
        let mut rows = Vec::new();
        for i in (row_start / 50)..(row_end / 50) {
            let c = Coord {
                x: x as u16,
                y: int.rows[i as usize * 50 + 1] as u16,
            };
            let kind = match &rd.tiles[&c].kind[..] {
                "LIOI" => Some(IoKind::Hpio),
                "LIOI3" => Some(IoKind::Hrio),
                "PCIE_NULL" | "NULL" => None,
                _ => unreachable!(),
            };
            rows.push(kind);
        }
        res[0] = Some(IoColumn { col, rows });
    }
    if let Some(x) = find_column(rd, &["RIOI", "RIOI3"]) {
        let col = int.lookup_column_inter(x) - 1;
        let mut rows = Vec::new();
        for i in (row_start / 50)..(row_end / 50) {
            let c = Coord {
                x: x as u16,
                y: int.rows[i as usize * 50 + 1] as u16,
            };
            let kind = match &rd.tiles[&c].kind[..] {
                "RIOI" => Some(IoKind::Hpio),
                "RIOI3" => Some(IoKind::Hrio),
                "NULL" => None,
                _ => unreachable!(),
            };
            rows.push(kind);
        }
        res[1] = Some(IoColumn { col, rows });
    }
    res
}

fn get_cols_gt(rd: &Part, int: &IntGrid, columns: &[ColumnKind], row_start: u32, row_end: u32) -> [Option<GtColumn>; 2] {
    let mut res = [None, None];
    if columns[0] == ColumnKind::Gt {
        let mut rows = Vec::new();
        for i in (row_start / 50)..(row_end / 50) {
            let c = Coord {
                x: 0,
                y: int.rows[i as usize * 50 + 5] as u16,
            };
            let kind = match &rd.tiles[&c].kind[..] {
                "GTH_CHANNEL_0" => Some(GtKind::Gth),
                "GTX_CHANNEL_0" => Some(GtKind::Gtx),
                _ => unreachable!(),
            };
            rows.push(kind);
        }
        res[0] = Some(GtColumn { col: 0, rows });
    }
    let col;
    if columns[columns.len() - 1] == ColumnKind::Gt {
        col = columns.len() as u32 - 1;
    } else {
        col = columns.len() as u32 - 7;
    }
    let x = int.cols[col as usize] + 4;
    let mut rows = Vec::new();
    for i in (row_start / 50)..(row_end / 50) {
        let c = Coord {
            x: x as u16,
            y: int.rows[i as usize * 50 + 5] as u16,
        };
        let kind = match &rd.tiles[&c].kind[..] {
            "GTH_CHANNEL_0" => Some(GtKind::Gth),
            "GTX_CHANNEL_0" => Some(GtKind::Gtx),
            "GTP_CHANNEL_0" => Some(GtKind::Gtp),
            _ => None,
        };
        rows.push(kind);
    }
    if rows.iter().any(|&x| x.is_some()) {
        res[1] = Some(GtColumn { col, rows });
    }
    res
}

fn make_grids(rd: &Part) -> (Vec<series7::Grid>, usize, Vec<ExtraDie>) {
    let int = extract_int(rd, &["INT_L", "INT_R", "INT_L_SLV", "INT_L_SLV_FLY", "INT_R_SLV", "INT_R_SLV_FLY"], &[
        ExtraCol { tts: &["CFG_CENTER_BOT"], dx: &[-10, -9, -6, -5, -2, -1] },
        ExtraCol { tts: &["INT_INTERFACE_PSS_L"], dx: &[
            -46, -45,
            -39, -38,
            -35, -34,
            -29, -28,
            -25, -24,
            -19, -18,
            -15, -14,
            -9, -8,
            -5, -4
        ] },
    ]);
    let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["T_TERM_INT_SLV"]).into_iter().map(|r| int.lookup_row_inter(r)).collect();
    rows_slr_split.remove(&0);
    rows_slr_split.insert(int.rows.len() as u32);
    let kind = get_kind(rd);
    let columns = make_columns(rd, &int);
    let cols_vbrk = get_cols_vbrk(rd, &int);
    let col_cfg = int.lookup_column(find_column(rd, &["CFG_CENTER_BOT"]).unwrap() + 3);
    let col_clk = int.lookup_column(find_column(rd, &["CLK_HROW_BOT_R"]).unwrap() - 2);
    let has_no_tbuturn = !find_rows(rd, &["T_TERM_INT_NOUTURN"]).is_empty();
    let rows_cfg: Vec<_> = find_rows(rd, &["CFG_CENTER_BOT"]).into_iter().map(|r| int.lookup_row(r - 10) + 50).sorted().collect();
    let rows_clk: Vec<_> = find_rows(rd, &["CLK_BUFG_BOT_R"]).into_iter().map(|r| int.lookup_row(r) + 4).sorted().collect();
    let has_ps = !find_columns(rd, &["INT_INTERFACE_PSS_L"]).is_empty();
    let has_slr = !find_columns(rd, &["INT_L_SLV"]).is_empty();
    for &x in &rows_cfg {
        assert_eq!(x % 50, 0);
    }
    for &x in &rows_clk {
        assert_eq!(x % 50, 0);
    }
    let mut grids = Vec::new();
    let mut row_start = 0;
    for (i, row_end) in rows_slr_split.into_iter().enumerate() {
        assert_eq!(row_end % 50, 0);
        grids.push(series7::Grid {
            kind,
            columns: columns.clone(),
            cols_vbrk: cols_vbrk.clone(),
            col_cfg,
            col_clk,
            cols_io: get_cols_io(rd, &int, row_start, row_end),
            cols_gt: get_cols_gt(rd, &int, &columns, row_start, row_end),
            rows: (row_end - row_start) / 50,
            row_cfg: (rows_cfg[i] - row_start) / 50,
            row_clk: (rows_clk[i] - row_start) / 50,
            holes: get_holes(rd, &int, row_start, row_end),
            has_ps,
            has_slr,
            has_no_tbuturn,
        });
        row_start = row_end;
    }
    let row_cfg_master = int.lookup_row(find_row(rd, &["CFG_CENTER_MID"]).unwrap() - 31) + 50;
    let grid_master = rows_cfg.iter().position(|&x| x == row_cfg_master).unwrap();
    let mut extras = Vec::new();
    if find_row(rd, &["GTZ_BOT"]).is_some() {
        extras.push(ExtraDie::GtzBottom);
    }
    if find_row(rd, &["GTZ_TOP"]).is_some() {
        extras.push(ExtraDie::GtzTop);
    }
    (grids, grid_master, extras)
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(rd: &Part, pkg: &str, grids: &[series7::Grid], grid_master: usize, extras: &[ExtraDie], pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let is_7k70t = rd.part.contains("7k70t");
    let io_lookup: HashMap<_, _> = series7::get_io(grids, grid_master)
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = series7::get_gt(grids, grid_master, extras, is_7k70t)
        .into_iter()
        .flat_map(|gt| gt.get_pads().into_iter().map(move |(name, func, pin, idx)| (name, (func, gt.bank, pin, idx))))
        .collect();
    let gtz_lookup: HashMap<_, _> = series7::get_gtz_pads(extras)
        .into_iter()
        .map(|(name, func, bank, pin, bel)| (name, (func, bank, pin, bel)))
        .collect();
    let sm_lookup: HashMap<_, _> = series7::get_sysmon_pads(grids, extras, is_7k70t)
        .into_iter()
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    let ps_lookup: HashMap<_, _> = series7::get_ps_pads(grids)
        .into_iter()
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    let has_14 = io_lookup.values().any(|io| io.bank == 14);
    let has_15 = io_lookup.values().any(|io| io.bank == 15);
    let has_35 = io_lookup.values().any(|io| io.bank == 35);
    let is_spartan = rd.part.contains("7s");
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func = match io.row % 50 {
                    0 => format!("IO_25"),
                    49 => format!("IO_0"),
                    n => format!("IO_L{}{}_T{}", (50 - n) / 2, ['P', 'N'][n as usize % 2], 3 - (n - 1) / 12),
                };
                if matches!(pkg, "fbg484" | "fbv484") && rd.part.contains("7k") && io.bank == 16 && matches!(io.row % 50, 2 | 14 | 37) {
                    exp_func = format!("IO_{}_T{}", (50 - io.row % 50) / 2, 3 - (io.row % 50 - 1) / 12);
                }
                if io.bank == 35 && matches!(io.row % 50, 21 | 22) {
                    if let Some(sm) = io.sm_pair(has_15, has_35) {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.row as usize % 2]).unwrap();
                    }
                }
                if io.is_srcc() {
                    exp_func += "_SRCC";
                }
                if io.is_mrcc() {
                    exp_func += "_MRCC";
                }
                if io.is_dqs() {
                    exp_func += "_DQS";
                }
                match io.get_cfg(has_14) {
                    Some(CfgPin::Data(d)) => {
                        if d >= 16 && !is_spartan {
                            write!(exp_func, "_A{:02}", d - 16).unwrap();
                        }
                        write!(exp_func, "_D{d:02}").unwrap();
                        if d == 0 {
                            exp_func += "_MOSI";
                        }
                        if d == 1 {
                            exp_func += "_DIN";
                        }
                    }
                    Some(CfgPin::Addr(a)) => {
                        if !is_spartan {
                            write!(exp_func, "_A{a}").unwrap();
                        }
                    }
                    Some(CfgPin::Rs(a)) => {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(CfgPin::HswapEn) => exp_func += "_PUDC_B",
                    Some(CfgPin::UserCclk) => exp_func += "_EMCCLK",
                    Some(CfgPin::RdWrB) => exp_func += "_RDWR_B",
                    Some(CfgPin::CsiB) => exp_func += "_CSI_B",
                    Some(CfgPin::Dout) => exp_func += "_DOUT_CSO_B",
                    Some(CfgPin::FweB) => if !is_spartan { exp_func += "_FWE_B" },
                    Some(CfgPin::FoeB) => if !is_spartan { exp_func += "_FOE_B" },
                    Some(CfgPin::FcsB) => exp_func += "_FCS_B",
                    Some(CfgPin::AdvB) => if !is_spartan { exp_func += "_ADV_B" },
                    None => (),
                    _ => unreachable!(),
                }
                if !(io.bank == 35 && matches!(io.row % 50, 21 | 22)) {
                    if let Some(sm) = io.sm_pair(has_15, has_35) {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.row as usize % 2]).unwrap();
                    }
                }
                if io.is_vref() {
                    exp_func += "_VREF";
                }
                if io.is_vrp() {
                    exp_func += "_VRP";
                }
                if io.is_vrn() {
                    exp_func += "_VRN";
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pkg} {pad} {io:?} got {f} exp {exp_func}", f=pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::IoByBank(io.bank, io.row % 50)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gtz_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
            } else if let Some(&(bank, spin)) = sm_lookup.get(pad) {
                let exp_func = match spin {
                    SysMonPin::VP => "VP_0",
                    SysMonPin::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::SysMonByBank(bank, spin)
            } else if let Some(&(bank, spin)) = ps_lookup.get(pad) {
                let exp_func = match spin {
                    PsPin::Clk => format!("PS_CLK_{bank}"),
                    PsPin::PorB => format!("PS_POR_B_{bank}"),
                    PsPin::SrstB => format!("PS_SRST_B_{bank}"),
                    PsPin::Mio(x) => format!("PS_MIO{x}_{bank}"),
                    PsPin::DdrDm(x) => format!("PS_DDR_DM{x}_{bank}"),
                    PsPin::DdrDq(x) => format!("PS_DDR_DQ{x}_{bank}"),
                    PsPin::DdrDqsP(x) => format!("PS_DDR_DQS_P{x}_{bank}"),
                    PsPin::DdrDqsN(x) => format!("PS_DDR_DQS_N{x}_{bank}"),
                    PsPin::DdrA(x) => format!("PS_DDR_A{x}_{bank}"),
                    PsPin::DdrBa(x) => format!("PS_DDR_BA{x}_{bank}"),
                    PsPin::DdrVrP => format!("PS_DDR_VRP_{bank}"),
                    PsPin::DdrVrN => format!("PS_DDR_VRN_{bank}"),
                    PsPin::DdrCkP(0) => format!("PS_DDR_CKP_{bank}"),
                    PsPin::DdrCkN(0) => format!("PS_DDR_CKN_{bank}"),
                    PsPin::DdrCke(0) => format!("PS_DDR_CKE_{bank}"),
                    PsPin::DdrOdt(0) => format!("PS_DDR_ODT_{bank}"),
                    PsPin::DdrDrstB => format!("PS_DDR_DRST_B_{bank}"),
                    PsPin::DdrCsB(0) => format!("PS_DDR_CS_B_{bank}"),
                    PsPin::DdrRasB => format!("PS_DDR_RAS_B_{bank}"),
                    PsPin::DdrCasB => format!("PS_DDR_CAS_B_{bank}"),
                    PsPin::DdrWeB => format!("PS_DDR_WE_B_{bank}"),
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::IoPs(bank, spin)
            } else {
                println!("unk iopad {pad} {f}", f=pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VCCBRAM" => BondPin::VccBram,
                "VCCBATT_0" => BondPin::VccBatt,
                "TCK_0" => BondPin::Cfg(CfgPin::Tck),
                "TDI_0" => BondPin::Cfg(CfgPin::Tdi),
                "TDO_0" => BondPin::Cfg(CfgPin::Tdo),
                "TMS_0" => BondPin::Cfg(CfgPin::Tms),
                "CCLK_0" => BondPin::Cfg(CfgPin::Cclk),
                "RSVDGND" if !has_14 => BondPin::Cfg(CfgPin::Cclk),
                "RSVDVCC3" if !has_14 => BondPin::Cfg(CfgPin::M0),
                "RSVDVCC2" if !has_14 => BondPin::Cfg(CfgPin::M1),
                "RSVDVCC1" if !has_14 => BondPin::Cfg(CfgPin::M2),
                "RSVDGND" => BondPin::RsvdGnd, // used for disabled transceiver RX pins on 7a12t
                "DONE_0" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM_B_0" => BondPin::Cfg(CfgPin::ProgB),
                "INIT_B_0" => BondPin::Cfg(CfgPin::InitB),
                "M0_0" => BondPin::Cfg(CfgPin::M0),
                "M1_0" => BondPin::Cfg(CfgPin::M1),
                "M2_0" => BondPin::Cfg(CfgPin::M2),
                "CFGBVS_0" => BondPin::Cfg(CfgPin::CfgBvs),
                "DXN_0" => BondPin::Dxn,
                "DXP_0" => BondPin::Dxp,
                "GNDADC_0" | "GNDADC" => BondPin::SysMonByBank(grid_master as u32, SysMonPin::AVss),
                "VCCADC_0" | "VCCADC" => BondPin::SysMonByBank(grid_master as u32, SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMonByBank(grid_master as u32, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMonByBank(grid_master as u32, SysMonPin::VRefN),
                "MGTAVTT" => BondPin::GtByRegion(10, GtRegionPin::AVtt),
                "MGTAVCC" => BondPin::GtByRegion(10, GtRegionPin::AVcc),
                "MGTVCCAUX" => BondPin::GtByRegion(10, GtRegionPin::VccAux),
                "VCCO_MIO0_500" => BondPin::VccO(500),
                "VCCO_MIO1_501" => BondPin::VccO(501),
                "VCCO_DDR_502" => BondPin::VccO(502),
                "VCCPINT" => BondPin::VccPsInt,
                "VCCPAUX" => BondPin::VccPsAux,
                "VCCPLL" => BondPin::VccPsPll,
                "PS_MIO_VREF_501" => BondPin::IoVref(501, 0),
                "PS_DDR_VREF0_502" => BondPin::IoVref(502, 0),
                "PS_DDR_VREF1_502" => BondPin::IoVref(502, 1),
                _ => if let Some((n, b)) = split_num(&pin.func) {
                    match n {
                        "VCCO_" => BondPin::VccO(b),
                        "VCCAUX_IO_G" => BondPin::VccAuxIo(b),
                        "MGTAVTTRCAL_" => BondPin::GtByBank(b, GtPin::AVttRCal, 0),
                        "MGTRREF_" => BondPin::GtByBank(b, GtPin::RRef, 0),
                        "MGTAVTT_G" => BondPin::GtByRegion(b, GtRegionPin::AVtt),
                        "MGTAVCC_G" => BondPin::GtByRegion(b, GtRegionPin::AVcc),
                        "MGTVCCAUX_G" => BondPin::GtByRegion(b, GtRegionPin::VccAux),
                        "MGTZAGND_" => BondPin::GtByBank(b, GtPin::GtzAGnd, 0),
                        "MGTZAVCC_" => BondPin::GtByBank(b, GtPin::GtzAVcc, 0),
                        "MGTZVCCH_" => BondPin::GtByBank(b, GtPin::GtzVccH, 0),
                        "MGTZVCCL_" => BondPin::GtByBank(b, GtPin::GtzVccL, 0),
                        "MGTZ_OBS_CLK_P_" => BondPin::GtByBank(b, GtPin::GtzObsClkP, 0),
                        "MGTZ_OBS_CLK_N_" => BondPin::GtByBank(b, GtPin::GtzObsClkN, 0),
                        "MGTZ_SENSE_AVCC_" => BondPin::GtByBank(b, GtPin::GtzSenseAVcc, 0),
                        "MGTZ_SENSE_AGND_" => BondPin::GtByBank(b, GtPin::GtzSenseAGnd, 0),
                        "MGTZ_SENSE_GNDL_" => BondPin::GtByBank(b, GtPin::GtzSenseGndL, 0),
                        "MGTZ_SENSE_GND_" => BondPin::GtByBank(b, GtPin::GtzSenseGnd, 0),
                        "MGTZ_SENSE_VCC_" => BondPin::GtByBank(b, GtPin::GtzSenseVcc, 0),
                        "MGTZ_SENSE_VCCL_" => BondPin::GtByBank(b, GtPin::GtzSenseVccL, 0),
                        "MGTZ_SENSE_VCCH_" => BondPin::GtByBank(b, GtPin::GtzSenseVccH, 0),
                        "MGTZ_THERM_IN_" => BondPin::GtByBank(b, GtPin::GtzThermIn, 0),
                        "MGTZ_THERM_OUT_" => BondPin::GtByBank(b, GtPin::GtzThermOut, 0),
                        _ => {
                            println!("UNK FUNC {} {} {:?}", pkg, pin.func, pin);
                            continue;
                        }
                    }
                } else {
                    println!("UNK FUNC {} {} {:?}", pkg, pin.func, pin);
                    continue;
                }
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
    let (grids, grid_master, extras) = make_grids(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(rd, pkg, &grids, grid_master, &extras, pins),
        ));
    }
    let grids = grids.into_iter().map(|x| geom::Grid::Series7(x)).collect();
    make_device_multi(rd, grids, grid_master, extras, bonds, BTreeSet::new())
}

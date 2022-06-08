use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use prjcombine_xilinx_rawdump::{Part, PkgPin};
use prjcombine_xilinx_geom::{self as geom, CfgPin, Bond, BondPin, GtPin, GtRegionPin, SysMonPin};
use prjcombine_xilinx_geom::virtex5::{self, ColumnKind, HardColumn};

use itertools::Itertools;

use crate::grid::{extract_int, find_column, find_columns, find_rows, find_row, IntGrid, PreDevice, make_device};

fn make_columns(rd: &Part, int: &IntGrid) -> Vec<ColumnKind> {
    let mut res: Vec<Option<ColumnKind>> = Vec::new();
    for _ in 0..int.cols.len() {
        res.push(None);
    }
    for c in find_columns(rd, &["CLBLL"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::ClbLL);
    }
    for c in find_columns(rd, &["CLBLM"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::ClbLM);
    }
    for c in find_columns(rd, &["BRAM", "PCIE_BRAM"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["IOI"]) {
        res[int.lookup_column_inter(c) as usize - 1] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["GT3"]) {
        res[int.lookup_column(c - 3) as usize] = Some(ColumnKind::Gtp);
    }
    for c in find_columns(rd, &["GTX"]) {
        res[int.lookup_column(c - 3) as usize] = Some(ColumnKind::Gtx);
    }
    for c in find_columns(rd, &["GTX_LEFT"]) {
        res[int.lookup_column(c + 2) as usize] = Some(ColumnKind::Gtx);
    }
    res.into_iter().map(|x| x.unwrap()).collect()
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<u32> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["CFG_VBRK"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_mgt_buf(rd: &Part, int: &IntGrid) -> BTreeSet<u32> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["HCLK_BRAM_MGT, HCLK_BRAM_MGT_LEFT"]) {
        res.insert(int.lookup_column(c - 2));
    }
    res
}

fn get_col_hard(rd: &Part, int: &IntGrid) -> Option<HardColumn> {
    let col = int.lookup_column(find_column(rd, &["EMAC", "PCIE_B"])? - 2);
    let rows_emac = find_rows(rd, &["EMAC"]).into_iter().map(|r| int.lookup_row(r)).sorted().collect();
    let rows_pcie = find_rows(rd, &["PCIE_B"]).into_iter().map(|r| int.lookup_row(r) - 10).sorted().collect();
    Some(HardColumn {
        col,
        rows_emac,
        rows_pcie,
    })
}

fn get_cols_io(columns: &[ColumnKind]) -> [Option<u32>; 3] {
    let v: Vec<_> = columns.iter().copied().enumerate().filter(|&(_, x)| x == ColumnKind::Io).map(|(i, _)| i as u32).collect();
    if v.len() == 2 {
        [Some(v[0]), Some(v[1]), None]
    } else {
        [Some(v[0]), Some(v[1]), Some(v[2])]
    }
}

fn get_row_cfg(rd: &Part, int: &IntGrid) -> u32 {
    int.lookup_row_inter(find_row(rd, &["CFG_CENTER"]).unwrap()) / 20
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(u32, u32)> {
    let mut res = Vec::new();
    if let Some(tk) = rd.tile_kinds.get("PPC_B") {
        for tile in &tk.tiles {
            let x = int.lookup_column((tile.x - 11) as i32);
            let y = int.lookup_row((tile.y - 10) as i32);
            assert_eq!(y % 20, 0);
            res.push((x, y));
        }
    }
    res
}

fn make_grid(rd: &Part) -> virtex5::Grid {
    let int = extract_int(rd, &["INT"], &[]);
    let columns = make_columns(rd, &int);
    let cols_io = get_cols_io(&columns);
    let row_cfg = get_row_cfg(rd, &int);
    virtex5::Grid {
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_mgt_buf: get_cols_mgt_buf(rd, &int),
        col_hard: get_col_hard(rd, &int),
        cols_io,
        rows: (int.rows.len() / 20) as u32,
        row_cfg,
        holes_ppc: get_holes_ppc(rd, &int),
    }
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(grid: &virtex5::Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = grid
        .get_gt()
        .into_iter()
        .flat_map(|gt| gt.get_pads(grid).into_iter().map(move |(name, func, pin, idx)| (name, (func, gt.bank, pin, idx))))
        .collect();
    let sm_lookup: HashMap<_, _> = grid
        .get_sysmon_pads()
        .into_iter()
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func = format!("IO_L{}{}", io.bbel / 2, ['N', 'P'][io.bbel as usize % 2]);
                if io.is_cc() {
                    exp_func += "_CC";
                }
                if io.is_gc() {
                    exp_func += "_GC";
                }
                if io.is_vref() {
                    exp_func += "_VREF";
                }
                if io.is_vr() {
                    match io.bel {
                        0 => exp_func += "_VRP",
                        1 => exp_func += "_VRN",
                        _ => unreachable!(),
                    }
                }
                match io.get_cfg() {
                    Some(CfgPin::Data(d)) => {
                        if d >= 16 {
                            write!(exp_func, "_A{}", d - 16).unwrap();
                        }
                        write!(exp_func, "_D{d}").unwrap();
                        if d < 3 {
                            write!(exp_func, "_FS{d}").unwrap();
                        }
                    }
                    Some(CfgPin::Addr(a)) => {
                        write!(exp_func, "_A{a}").unwrap();
                    }
                    Some(CfgPin::Rs(a)) => {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(CfgPin::CsoB) => exp_func += "_CSO_B",
                    Some(CfgPin::FweB) => exp_func += "_FWE_B",
                    Some(CfgPin::FoeB) => exp_func += "_FOE_B_MOSI",
                    Some(CfgPin::FcsB) => exp_func += "_FCS_B",
                    None => (),
                    _ => unreachable!(),
                }
                if let Some(sm) = io.sm_pair() {
                    write!(exp_func, "_SM{}{}", sm, ['N', 'P'][io.bbel as usize % 2]).unwrap();
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pad} {io:?} got {f} exp {exp_func}", f=pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::IoByBank(io.bank, io.bbel)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
            } else if let Some(&spin) = sm_lookup.get(pad) {
                let exp_func = match spin {
                    SysMonPin::VP => "VP_0",
                    SysMonPin::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::SysMonByBank(0, spin)
            } else {
                println!("unk iopad {pad} {f}", f=pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "RSVD" => BondPin::Rsvd, // ??? on TXT devices
                "RSVD_0" => BondPin::Rsvd, // actually VFS, R_FUSE
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VBATT_0" => BondPin::VccBatt,
                "TCK_0" => BondPin::Cfg(CfgPin::Tck),
                "TDI_0" => BondPin::Cfg(CfgPin::Tdi),
                "TDO_0" => BondPin::Cfg(CfgPin::Tdo),
                "TMS_0" => BondPin::Cfg(CfgPin::Tms),
                "CCLK_0" => BondPin::Cfg(CfgPin::Cclk),
                "DONE_0" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM_B_0" => BondPin::Cfg(CfgPin::ProgB),
                "INIT_B_0" => BondPin::Cfg(CfgPin::InitB),
                "RDWR_B_0" => BondPin::Cfg(CfgPin::RdWrB),
                "CS_B_0" => BondPin::Cfg(CfgPin::CsiB),
                "D_IN_0" => BondPin::Cfg(CfgPin::Din),
                "D_OUT_BUSY_0" => BondPin::Cfg(CfgPin::Dout),
                "M0_0" => BondPin::Cfg(CfgPin::M0),
                "M1_0" => BondPin::Cfg(CfgPin::M1),
                "M2_0" => BondPin::Cfg(CfgPin::M2),
                "HSWAPEN_0" => BondPin::Cfg(CfgPin::HswapEn),
                "DXN_0" => BondPin::Dxn,
                "DXP_0" => BondPin::Dxp,
                "AVSS_0" => BondPin::SysMonByBank(0, SysMonPin::AVss),
                "AVDD_0" => BondPin::SysMonByBank(0, SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMonByBank(0, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMonByBank(0, SysMonPin::VRefN),
                "MGTAVTTRXC" => BondPin::GtByRegion(1, GtRegionPin::AVttRxC),
                "MGTAVTTRXC_L" => BondPin::GtByRegion(0, GtRegionPin::AVttRxC),
                "MGTAVTTRXC_R" => BondPin::GtByRegion(1, GtRegionPin::AVttRxC),
                _ => if let Some((n, b)) = split_num(&pin.func) {
                    match n {
                        "VCCO_" => BondPin::VccO(b),
                        "MGTAVCC_" => BondPin::GtByBank(b, GtPin::AVcc, 0),
                        "MGTAVCCPLL_" => BondPin::GtByBank(b, GtPin::AVccPll, 0),
                        "MGTAVTTRX_" => BondPin::GtByBank(b, GtPin::VtRx, 0),
                        "MGTAVTTTX_" => BondPin::GtByBank(b, GtPin::VtTx, 0),
                        "MGTRREF_" => BondPin::GtByBank(b, GtPin::RRef, 0),
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
        io_banks: Default::default(),
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
    make_device(rd, geom::Grid::Virtex5(grid), bonds, BTreeSet::new())
}

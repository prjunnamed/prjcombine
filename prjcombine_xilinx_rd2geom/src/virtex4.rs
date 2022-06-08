use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use prjcombine_xilinx_rawdump::{Part, PkgPin};
use prjcombine_xilinx_geom::{self as geom, CfgPin, Bond, BondPin, GtPin, SysMonPin};
use prjcombine_xilinx_geom::virtex4::{self, ColumnKind};

use crate::grid::{extract_int, find_columns, find_rows, find_row, IntGrid, PreDevice, make_device};

fn make_columns(rd: &Part, int: &IntGrid) -> Vec<ColumnKind> {
    let mut res: Vec<Option<ColumnKind>> = Vec::new();
    for _ in 0..int.cols.len() {
        res.push(None);
    }
    for c in find_columns(rd, &["CLB"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::Clb);
    }
    for c in find_columns(rd, &["BRAM"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["IOIS_LC", "IOIS_LC_L"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["MGT_AR"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::Gt);
    }
    for c in find_columns(rd, &["MGT_AL"]) {
        res[int.lookup_column(c + 1) as usize] = Some(ColumnKind::Gt);
    }
    res.into_iter().map(|x| x.unwrap()).collect()
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<u32> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["CFG_VBRK_FRAME"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_io(columns: &[ColumnKind]) -> [u32; 3] {
    let v: Vec<_> = columns.iter().copied().enumerate().filter(|&(_, x)| x == ColumnKind::Io).map(|(i, _)| i as u32).collect();
    v.try_into().unwrap()
}

fn get_row_cfg(rd: &Part, int: &IntGrid) -> u32 {
    int.lookup_row_inter(find_row(rd, &["CFG_CENTER"]).unwrap()) / 16
}

fn get_rows_cfg_io(rd: &Part, int: &IntGrid, row_cfg: u32) -> u32 {
    let d2i = int.lookup_row_inter(find_row(rd, &["HCLK_DCMIOB"]).unwrap());
    let i2d = int.lookup_row_inter(find_row(rd, &["HCLK_IOBDCM"]).unwrap());
    assert_eq!(i2d - row_cfg * 16, row_cfg * 16 - d2i);
    (i2d - row_cfg * 16 - 8) / 16
}

fn get_ccm(rd: &Part) -> u32 {
    (find_rows(rd, &["CCM"]).len() / 2) as u32
}

fn get_has_sysmons(rd: &Part) -> (bool, bool) {
    let sysmons = find_rows(rd, &["SYS_MON"]);
    (sysmons.contains(&1), sysmons.contains(&((rd.height - 9) as i32)))
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(u32, u32)> {
    let mut res = Vec::new();
    if let Some(tk) = rd.tile_kinds.get("PB") {
        for tile in &tk.tiles {
            let x = int.lookup_column((tile.x - 1) as i32);
            let y = int.lookup_row((tile.y - 4) as i32);
            assert_eq!(y % 16, 12);
            res.push((x, y));
        }
    }
    res
}

fn get_has_bram_fx(rd: &Part) -> bool {
    !find_columns(rd, &["HCLK_BRAM_FX"]).is_empty()
}

fn make_grid(rd: &Part) -> virtex4::Grid {
    let int = extract_int(rd, &["INT", "INT_SO"], &[]);
    let columns = make_columns(rd, &int);
    let cols_io = get_cols_io(&columns);
    let (has_bot_sysmon, has_top_sysmon) = get_has_sysmons(rd);
    let row_cfg = get_row_cfg(rd, &int);
    virtex4::Grid {
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_io,
        rows: (int.rows.len() / 16) as u32,
        has_bot_sysmon,
        has_top_sysmon,
        rows_cfg_io: get_rows_cfg_io(rd, &int, row_cfg),
        ccm: get_ccm(rd),
        row_cfg,
        holes_ppc: get_holes_ppc(rd, &int),
        has_bram_fx: get_has_bram_fx(rd),
    }
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(grid: &virtex4::Grid, pins: &[PkgPin]) -> Bond {
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
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func = format!("IO_L{}{}", io.bbel / 2, ['N', 'P'][io.bbel as usize % 2]);
                match io.get_cfg() {
                    Some(CfgPin::Data(d)) => write!(exp_func, "_D{d}").unwrap(),
                    None => (),
                    _ => unreachable!(),
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
                if io.is_cc() {
                    exp_func += "_CC";
                }
                if let Some((bank, sm)) = io.sm_pair(grid) {
                    write!(exp_func, "_{}{}", ["SM", "ADC"][bank as usize], sm).unwrap();
                }
                if io.is_lc() {
                    exp_func += "_LC";
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
            } else if let Some(&(bank, spin)) = sm_lookup.get(pad) {
                let exp_func = match (bank, spin) {
                    (0, SysMonPin::VP) => "VP_SM",
                    (0, SysMonPin::VN) => "VN_SM",
                    (1, SysMonPin::VP) => "VP_ADC",
                    (1, SysMonPin::VN) => "VN_ADC",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::SysMonByBank(bank, spin)
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
                "VBATT_0" => BondPin::VccBatt,
                "TCK_0" => BondPin::Cfg(CfgPin::Tck),
                "TDI_0" => BondPin::Cfg(CfgPin::Tdi),
                "TDO_0" => BondPin::Cfg(CfgPin::Tdo),
                "TMS_0" => BondPin::Cfg(CfgPin::Tms),
                "CCLK_0" => BondPin::Cfg(CfgPin::Cclk),
                "DONE_0" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM_B_0" => BondPin::Cfg(CfgPin::ProgB),
                "PWRDWN_B_0" => BondPin::Cfg(CfgPin::PwrdwnB),
                "INIT_0" => BondPin::Cfg(CfgPin::InitB),
                "RDWR_B_0" => BondPin::Cfg(CfgPin::RdWrB),
                "CS_B_0" => BondPin::Cfg(CfgPin::CsiB),
                "D_IN_0" => BondPin::Cfg(CfgPin::Din),
                "DOUT_BUSY_0" => BondPin::Cfg(CfgPin::Dout),
                "M0_0" => BondPin::Cfg(CfgPin::M0),
                "M1_0" => BondPin::Cfg(CfgPin::M1),
                "M2_0" => BondPin::Cfg(CfgPin::M2),
                "HSWAPEN_0" => BondPin::Cfg(CfgPin::HswapEn),
                "TDN_0" => BondPin::Dxn,
                "TDP_0" => BondPin::Dxp,
                "AVSS_SM" => BondPin::SysMonByBank(0, SysMonPin::AVss),
                "AVSS_ADC" => BondPin::SysMonByBank(1, SysMonPin::AVss),
                "AVDD_SM" => BondPin::SysMonByBank(0, SysMonPin::AVdd),
                "AVDD_ADC" => BondPin::SysMonByBank(1, SysMonPin::AVdd),
                "VREFP_SM" => BondPin::SysMonByBank(0, SysMonPin::VRefP),
                "VREFP_ADC" => BondPin::SysMonByBank(1, SysMonPin::VRefP),
                "VREFN_SM" => BondPin::SysMonByBank(0, SysMonPin::VRefN),
                "VREFN_ADC" => BondPin::SysMonByBank(1, SysMonPin::VRefN),
                _ => if let Some((n, b)) = split_num(&pin.func) {
                    match n {
                        "VCCO_" => BondPin::VccO(b),
                        "GNDA_" => BondPin::GtByBank(b, GtPin::GndA, 0),
                        "VTRXA_" => BondPin::GtByBank(b, GtPin::VtRx, 1),
                        "VTRXB_" => BondPin::GtByBank(b, GtPin::VtRx, 0),
                        "VTTXA_" => BondPin::GtByBank(b, GtPin::VtTx, 1),
                        "VTTXB_" => BondPin::GtByBank(b, GtPin::VtTx, 0),
                        "AVCCAUXRXA_" => BondPin::GtByBank(b, GtPin::AVccAuxRx, 1),
                        "AVCCAUXRXB_" => BondPin::GtByBank(b, GtPin::AVccAuxRx, 0),
                        "AVCCAUXTX_" => BondPin::GtByBank(b, GtPin::AVccAuxTx, 0),
                        "AVCCAUXMGT_" => BondPin::GtByBank(b, GtPin::AVccAuxMgt, 0),
                        "RTERM_" => BondPin::GtByBank(b, GtPin::RTerm, 0),
                        "MGTVREF_" => BondPin::GtByBank(b, GtPin::MgtVRef, 0),
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
    make_device(rd, geom::Grid::Virtex4(grid), bonds, BTreeSet::new())
}

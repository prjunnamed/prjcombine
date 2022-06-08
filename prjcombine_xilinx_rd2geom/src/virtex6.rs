use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use prjcombine_xilinx_rawdump::{Part, PkgPin};
use prjcombine_xilinx_geom::{self as geom, CfgPin, Bond, BondPin, GtPin, GtRegionPin, SysMonPin, DisabledPart};
use prjcombine_xilinx_geom::virtex6::{self, ColumnKind, HardColumn};

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
    for c in find_columns(rd, &["BRAM"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["RIOI"]) {
        res[int.lookup_column_inter(c) as usize - 1] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["LIOI"]) {
        res[int.lookup_column_inter(c) as usize] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["CMT_TOP"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Cmt);
    }
    for c in find_columns(rd, &["GTX"]) {
        res[int.lookup_column(c - 3) as usize] = Some(ColumnKind::Gt);
    }
    for c in find_columns(rd, &["GTX_LEFT"]) {
        res[int.lookup_column(c + 2) as usize] = Some(ColumnKind::Gt);
    }
    res.into_iter().map(|x| x.unwrap()).collect()
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<u32> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["VBRK"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_mgt_buf(rd: &Part, int: &IntGrid) -> BTreeSet<u32> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["HCLK_CLBLM_MGT", "HCLK_CLBLM_MGT_LEFT"]) {
        res.insert(int.lookup_column(c - 1));
    }
    res
}

fn get_col_hard(rd: &Part, int: &IntGrid) -> Option<HardColumn> {
    let col = int.lookup_column(find_column(rd, &["EMAC"])? - 2);
    let rows_emac = find_rows(rd, &["EMAC", "EMAC_DUMMY"]).into_iter().map(|r| int.lookup_row(r)).sorted().collect();
    let rows_pcie = find_rows(rd, &["PCIE", "PCIE_DUMMY"]).into_iter().map(|r| int.lookup_row(r) - 10).sorted().collect();
    Some(HardColumn {
        col,
        rows_emac,
        rows_pcie,
    })
}

fn get_cols_io(rd: &Part, int: &IntGrid) -> [Option<u32>; 4] {
    let mut res = [None; 4];
    let lc: Vec<_> = find_columns(rd, &["LIOI"]).into_iter().map(|x| int.lookup_column_inter(x)).sorted().collect();
    match &lc[..] {
        &[il] => {
            res[1] = Some(il);
        }
        &[ol, il] => {
            res[0] = Some(ol);
            res[1] = Some(il);
        }
        _ => unreachable!(),
    }
    let rc: Vec<_> = find_columns(rd, &["RIOI"]).into_iter().map(|x| int.lookup_column_inter(x) - 1).sorted().collect();
    match &rc[..] {
        &[ir] => {
            res[2] = Some(ir);
        }
        &[ir, or] => {
            res[2] = Some(ir);
            res[3] = Some(or);
        }
        _ => unreachable!(),
    }
    res
}

fn get_cols_qbuf(rd: &Part, int: &IntGrid) -> (u32, u32) {
    (int.lookup_column(find_column(rd, &["HCLK_QBUF_L"]).unwrap()), int.lookup_column(find_column(rd, &["HCLK_QBUF_R"]).unwrap()))
}

fn get_col_cfg(rd: &Part, int: &IntGrid) -> u32 {
    int.lookup_column(find_column(rd, &["CFG_CENTER_0"]).unwrap() + 2)
}

fn get_row_cfg(rd: &Part, int: &IntGrid) -> u32 {
    int.lookup_row(find_row(rd, &["CFG_CENTER_2"]).unwrap() - 10) / 40
}

fn get_row_gth_start(rd: &Part, int: &IntGrid) -> u32 {
    if let Some(r) = find_rows(rd, &["GTH_BOT"]).into_iter().min() {
        int.lookup_row(r - 10) / 40
    } else {
        int.rows.len() as u32 / 40
    }
}

fn make_grid(rd: &Part) -> (virtex6::Grid, BTreeSet<DisabledPart>) {
    let mut disabled = BTreeSet::new();
    let int = extract_int(rd, &["INT"], &[]);
    let columns = make_columns(rd, &int);
    if rd.part.contains("vcx") {
        disabled.insert(DisabledPart::Virtex6SysMon);
    }
    for r in find_rows(rd, &["EMAC_DUMMY"]) {
        disabled.insert(DisabledPart::Virtex6Emac(int.lookup_row(r)));
    }
    for r in find_rows(rd, &["GTX_DUMMY"]) {
        disabled.insert(DisabledPart::Virtex6GtxRow(int.lookup_row(r) / 40));
    }
    let grid = virtex6::Grid {
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_mgt_buf: get_cols_mgt_buf(rd, &int),
        col_cfg: get_col_cfg(rd, &int),
        cols_qbuf: get_cols_qbuf(rd, &int),
        col_hard: get_col_hard(rd, &int),
        cols_io: get_cols_io(&rd, &int),
        rows: (int.rows.len() / 40) as u32,
        row_cfg: get_row_cfg(rd, &int),
        row_gth_start: get_row_gth_start(rd, &int),
    };
    (grid, disabled)
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(rd: &Part, grid: &virtex6::Grid, disabled: &BTreeSet<DisabledPart>, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let is_vcx = rd.part.contains("vcx");
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = grid
        .get_gt(disabled)
        .into_iter()
        .flat_map(|gt| gt.get_pads(grid).into_iter().map(move |(name, func, pin, idx)| (name, (func, gt.bank, pin, idx))))
        .collect();
    let sm_lookup: HashMap<_, _> = grid
        .get_sysmon_pads(disabled)
        .into_iter()
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func = format!("IO_L{}{}", io.bbel / 2, ['P', 'N'][io.bbel as usize % 2]);
                if io.is_srcc() {
                    exp_func += "_SRCC";
                }
                if io.is_mrcc() {
                    exp_func += "_MRCC";
                }
                if io.is_gc() {
                    exp_func += "_GC";
                }
                if io.is_vref() {
                    exp_func += "_VREF";
                }
                if io.is_vr() {
                    match io.row % 2 {
                        0 => exp_func += "_VRP",
                        1 => exp_func += "_VRN",
                        _ => unreachable!(),
                    }
                }
                match io.get_cfg() {
                    Some(CfgPin::Data(d)) => {
                        if d >= 16 {
                            write!(exp_func, "_A{:02}", d - 16).unwrap();
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
                if !is_vcx {
                    if let Some(sm) = io.sm_pair(grid) {
                        write!(exp_func, "_SM{}{}", sm, ['P', 'N'][io.bbel as usize % 2]).unwrap();
                    }
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
                "RSVD" => BondPin::Rsvd, // GTH-related
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
                "INIT_B_0" => BondPin::Cfg(CfgPin::InitB),
                "RDWR_B_0" => BondPin::Cfg(CfgPin::RdWrB),
                "CSI_B_0" => BondPin::Cfg(CfgPin::CsiB),
                "DIN_0" => BondPin::Cfg(CfgPin::Din),
                "DOUT_BUSY_0" => BondPin::Cfg(CfgPin::Dout),
                "M0_0" => BondPin::Cfg(CfgPin::M0),
                "M1_0" => BondPin::Cfg(CfgPin::M1),
                "M2_0" => BondPin::Cfg(CfgPin::M2),
                "HSWAPEN_0" => BondPin::Cfg(CfgPin::HswapEn),
                "DXN_0" => BondPin::Dxn,
                "DXP_0" => BondPin::Dxp,
                "VFS_0" => BondPin::Vfs,
                "AVSS_0" => BondPin::SysMonByBank(0, SysMonPin::AVss),
                "AVDD_0" => BondPin::SysMonByBank(0, SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMonByBank(0, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMonByBank(0, SysMonPin::VRefN),
                "MGTAVTT" => BondPin::GtByRegion(3, GtRegionPin::AVtt),
                "MGTAVCC" => BondPin::GtByRegion(3, GtRegionPin::AVcc),
                "MGTAVTT_S" => BondPin::GtByRegion(2, GtRegionPin::AVtt),
                "MGTAVCC_S" => BondPin::GtByRegion(2, GtRegionPin::AVcc),
                "MGTAVTT_N" => BondPin::GtByRegion(3, GtRegionPin::AVtt),
                "MGTAVCC_N" => BondPin::GtByRegion(3, GtRegionPin::AVcc),
                "MGTAVTT_L" => BondPin::GtByRegion(0, GtRegionPin::AVtt),
                "MGTAVCC_L" => BondPin::GtByRegion(0, GtRegionPin::AVcc),
                "MGTAVTT_R" => BondPin::GtByRegion(2, GtRegionPin::AVtt),
                "MGTAVCC_R" => BondPin::GtByRegion(2, GtRegionPin::AVcc),
                "MGTAVTT_LS" => BondPin::GtByRegion(0, GtRegionPin::AVtt),
                "MGTAVCC_LS" => BondPin::GtByRegion(0, GtRegionPin::AVcc),
                "MGTAVTT_LN" => BondPin::GtByRegion(1, GtRegionPin::AVtt),
                "MGTAVCC_LN" => BondPin::GtByRegion(1, GtRegionPin::AVcc),
                "MGTAVTT_RS" => BondPin::GtByRegion(2, GtRegionPin::AVtt),
                "MGTAVCC_RS" => BondPin::GtByRegion(2, GtRegionPin::AVcc),
                "MGTAVTT_RN" => BondPin::GtByRegion(3, GtRegionPin::AVtt),
                "MGTAVCC_RN" => BondPin::GtByRegion(3, GtRegionPin::AVcc),
                "MGTHAVTT_L" => BondPin::GtByRegion(1, GtRegionPin::GthAVtt),
                "MGTHAVCC_L" => BondPin::GtByRegion(1, GtRegionPin::GthAVcc),
                "MGTHAVCCRX_L" => BondPin::GtByRegion(1, GtRegionPin::GthAVccRx),
                "MGTHAVCCPLL_L" => BondPin::GtByRegion(1, GtRegionPin::GthAVccPll),
                "MGTHAGND_L" => BondPin::GtByRegion(1, GtRegionPin::GthAGnd),
                "MGTHAVTT_R" => BondPin::GtByRegion(3, GtRegionPin::GthAVtt),
                "MGTHAVCC_R" => BondPin::GtByRegion(3, GtRegionPin::GthAVcc),
                "MGTHAVCCRX_R" => BondPin::GtByRegion(3, GtRegionPin::GthAVccRx),
                "MGTHAVCCPLL_R" => BondPin::GtByRegion(3, GtRegionPin::GthAVccPll),
                "MGTHAGND_R" => BondPin::GtByRegion(3, GtRegionPin::GthAGnd),
                "MGTHAVTT" => BondPin::GtByRegion(3, GtRegionPin::GthAVtt),
                "MGTHAVCC" => BondPin::GtByRegion(3, GtRegionPin::GthAVcc),
                "MGTHAVCCRX" => BondPin::GtByRegion(3, GtRegionPin::GthAVccRx),
                "MGTHAVCCPLL" => BondPin::GtByRegion(3, GtRegionPin::GthAVccPll),
                "MGTHAGND" => BondPin::GtByRegion(3, GtRegionPin::GthAGnd),
                _ => if let Some((n, b)) = split_num(&pin.func) {
                    match n {
                        "VCCO_" => BondPin::VccO(b),
                        "MGTAVTTRCAL_" => BondPin::GtByBank(b, GtPin::AVttRCal, 0),
                        "MGTRREF_" => BondPin::GtByBank(b, GtPin::RRef, 0),
                        "MGTRBIAS_" => BondPin::GtByBank(b, GtPin::RBias, 0),
                        _ => {
                            println!("UNK FUNC {} {:?}", pin.func, pin);
                            continue;
                        }
                    }
                } else {
                    println!("UNK FUNC {} {:?}", pin.func, pin);
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
    let (grid, disabled) = make_grid(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(rd, &grid, &disabled, pins),
        ));
    }
    make_device(rd, geom::Grid::Virtex6(grid), bonds, disabled)
}

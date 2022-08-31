use prjcombine_rawdump::PkgPin;
use prjcombine_virtex5::{
    Bond, BondPin, CfgPin, Grid, GtPin, GtRegion, GtRegionPin, SharedCfgPin, SysMonPin,
};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use prjcombine_rdgrid::split_num;

pub fn make_bond(grid: &Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = grid
        .get_gt()
        .into_iter()
        .flat_map(|gt| {
            gt.get_pads(grid)
                .into_iter()
                .map(move |(name, func, pin)| (name, (func, gt.bank, pin)))
        })
        .collect();
    let sm_lookup: HashMap<_, _> = grid.get_sysmon_pads().into_iter().collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func =
                    format!("IO_L{}{}", io.bbel / 2, ['N', 'P'][io.bbel as usize % 2]);
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
                    Some(SharedCfgPin::Data(d)) => {
                        if d >= 16 {
                            write!(exp_func, "_A{}", d - 16).unwrap();
                        }
                        write!(exp_func, "_D{d}").unwrap();
                        if d < 3 {
                            write!(exp_func, "_FS{d}").unwrap();
                        }
                    }
                    Some(SharedCfgPin::Addr(a)) => {
                        write!(exp_func, "_A{a}").unwrap();
                    }
                    Some(SharedCfgPin::Rs(a)) => {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(SharedCfgPin::CsoB) => exp_func += "_CSO_B",
                    Some(SharedCfgPin::FweB) => exp_func += "_FWE_B",
                    Some(SharedCfgPin::FoeB) => exp_func += "_FOE_B_MOSI",
                    Some(SharedCfgPin::FcsB) => exp_func += "_FCS_B",
                    None => (),
                }
                if let Some(sm) = io.sm_pair() {
                    write!(exp_func, "_SM{}{}", sm, ['N', 'P'][io.bbel as usize % 2]).unwrap();
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pad} {io:?} got {f} exp {exp_func}", f = pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::Io(io.bank, io.bbel)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gt(bank, gpin)
            } else if let Some(&spin) = sm_lookup.get(pad) {
                let exp_func = match spin {
                    SysMonPin::VP => "VP_0",
                    SysMonPin::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::SysMon(spin)
            } else {
                println!("unk iopad {pad} {f}", f = pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "RSVD" => BondPin::Rsvd,   // ??? on TXT devices
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
                "AVSS_0" => BondPin::SysMon(SysMonPin::AVss),
                "AVDD_0" => BondPin::SysMon(SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMon(SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMon(SysMonPin::VRefN),
                "MGTAVTTRXC" => BondPin::GtRegion(GtRegion::All, GtRegionPin::AVttRxC),
                "MGTAVTTRXC_L" => BondPin::GtRegion(GtRegion::L, GtRegionPin::AVttRxC),
                "MGTAVTTRXC_R" => BondPin::GtRegion(GtRegion::R, GtRegionPin::AVttRxC),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPin::VccO(b),
                            "MGTAVCC_" => BondPin::Gt(b, GtPin::AVcc),
                            "MGTAVCCPLL_" => BondPin::Gt(b, GtPin::AVccPll),
                            "MGTAVTTRX_" => BondPin::Gt(b, GtPin::VtRx),
                            "MGTAVTTTX_" => BondPin::Gt(b, GtPin::VtTx),
                            "MGTRREF_" => BondPin::Gt(b, GtPin::RRef),
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
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond { pins: bond_pins }
}

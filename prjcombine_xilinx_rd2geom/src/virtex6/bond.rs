use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::pkg::{Bond, BondPin, CfgPin, GtPin, GtRegionPin, SysMonPin};
use prjcombine_xilinx_geom::virtex6::{Grid, SharedCfgPin};
use prjcombine_xilinx_geom::DisabledPart;
use prjcombine_xilinx_rawdump::{Part, PkgPin};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;

use crate::util::split_num;

pub fn make_bond(
    rd: &Part,
    grid: &Grid,
    disabled: &BTreeSet<DisabledPart>,
    pins: &[PkgPin],
) -> Bond {
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
        .flat_map(|gt| {
            gt.get_pads(grid)
                .into_iter()
                .map(move |(name, func, pin, idx)| (name, (func, gt.bank, pin, idx)))
        })
        .collect();
    let sm_lookup: HashMap<_, _> = grid.get_sysmon_pads(disabled).into_iter().collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func =
                    format!("IO_L{}{}", io.bbel / 2, ['P', 'N'][io.bbel as usize % 2]);
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
                    match io.row.to_idx() % 2 {
                        0 => exp_func += "_VRP",
                        1 => exp_func += "_VRN",
                        _ => unreachable!(),
                    }
                }
                match io.get_cfg() {
                    Some(SharedCfgPin::Data(d)) => {
                        if d >= 16 {
                            write!(exp_func, "_A{:02}", d - 16).unwrap();
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
                if !is_vcx {
                    if let Some(sm) = io.sm_pair(grid) {
                        write!(exp_func, "_SM{}{}", sm, ['P', 'N'][io.bbel as usize % 2]).unwrap();
                    }
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pad} {io:?} got {f} exp {exp_func}", f = pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::IoByBank(io.bank, io.bbel)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
            } else if let Some(&spin) = sm_lookup.get(pad) {
                let exp_func = match spin {
                    SysMonPin::VP => "VP_0",
                    SysMonPin::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::SysMonByBank(0, spin)
            } else {
                println!("unk iopad {pad} {f}", f = pin.func);
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
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
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
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks: Default::default(),
    }
}

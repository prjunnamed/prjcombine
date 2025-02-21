use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_virtex4::bond::{
    Bond, BondPin, CfgPin, GtPin, GtRegion, GtRegionPin, SharedCfgPin, SysMonPin,
};
use prjcombine_virtex4::expanded::{IoCoord, IoDiffKind, IoVrKind};
use prjcombine_virtex4::grid::{DisabledPart, GtKind};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use prjcombine_re_xilinx_rd2db_grid::split_num;

pub fn make_bond(endev: &ExpandedNamedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = endev
        .edev
        .io
        .iter()
        .copied()
        .map(|io| (endev.get_io_name(io), io))
        .collect();
    let mut gt_lookup: HashMap<&str, (String, u32, GtPin)> = HashMap::new();
    for gt in endev.get_gts() {
        let bank = gt.bank;
        for (i, (pp, pn)) in gt.pads_clk.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (
                    if gt.kind == GtKind::Gth {
                        format!("MGTREFCLKP_{bank}")
                    } else {
                        format!("MGTREFCLK{i}P_{bank}")
                    },
                    bank,
                    GtPin::ClkP(i as u8),
                ),
            );
            gt_lookup.insert(
                pn,
                (
                    if gt.kind == GtKind::Gth {
                        format!("MGTREFCLKN_{bank}")
                    } else {
                        format!("MGTREFCLK{i}N_{bank}")
                    },
                    bank,
                    GtPin::ClkN(i as u8),
                ),
            );
        }
        for (i, (pp, pn)) in gt.pads_rx.iter().enumerate() {
            gt_lookup.insert(pp, (format!("MGTRXP{i}_{bank}"), bank, GtPin::RxP(i as u8)));
            gt_lookup.insert(pn, (format!("MGTRXN{i}_{bank}"), bank, GtPin::RxN(i as u8)));
        }
        for (i, (pp, pn)) in gt.pads_tx.iter().enumerate() {
            gt_lookup.insert(pp, (format!("MGTTXP{i}_{bank}"), bank, GtPin::TxP(i as u8)));
            gt_lookup.insert(pn, (format!("MGTTXN{i}_{bank}"), bank, GtPin::TxN(i as u8)));
        }
    }
    let mut sm_lookup: HashMap<&str, (u32, SysMonPin)> = HashMap::new();
    let mut vaux_lookup: HashMap<IoCoord, (usize, char)> = HashMap::new();
    for sysmon in &endev.get_sysmons() {
        sm_lookup.insert(sysmon.pad_vp, (sysmon.bank, SysMonPin::VP));
        sm_lookup.insert(sysmon.pad_vn, (sysmon.bank, SysMonPin::VN));
        for (i, vaux) in sysmon.vaux.iter().enumerate() {
            if let &Some((vauxp, vauxn)) = vaux {
                vaux_lookup.insert(vauxp, (i, 'P'));
                vaux_lookup.insert(vauxn, (i, 'N'));
            }
        }
    }
    let cfg_lookup: HashMap<_, _> = endev.edev.cfg_io.iter().map(|(&k, &v)| (v, k)).collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&**pad) {
                let io_info = endev.edev.get_io_info(io);
                let mut exp_func = match io_info.diff {
                    IoDiffKind::None => format!("IO_{}", io_info.pkgid),
                    IoDiffKind::P(_) => format!("IO_L{}P", io_info.pkgid),
                    IoDiffKind::N(_) => format!("IO_L{}N", io_info.pkgid),
                };
                if io_info.is_srcc {
                    exp_func += "_SRCC";
                }
                if io_info.is_mrcc {
                    exp_func += "_MRCC";
                }
                if io_info.is_gc {
                    exp_func += "_GC";
                }
                if io_info.is_vref {
                    exp_func += "_VREF";
                }
                match io_info.vr {
                    IoVrKind::VrP => exp_func += "_VRP",
                    IoVrKind::VrN => exp_func += "_VRN",
                    IoVrKind::None => (),
                }
                match cfg_lookup.get(&io).copied() {
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
                    Some(_) => unreachable!(),
                    None => (),
                }
                if !endev.edev.disabled.contains(&DisabledPart::SysMon) {
                    if let Some(&(i, pn)) = vaux_lookup.get(&io) {
                        write!(exp_func, "_SM{i}{pn}").unwrap();
                    }
                }
                write!(exp_func, "_{}", io_info.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pad} {io:?} got {f} exp {exp_func}", f = pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io_info.bank));
                assert_eq!(pin.vcco_bank, Some(io_info.bank));
                BondPin::Io(io_info.bank, io_info.biob)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(&**pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gt(bank, gpin)
            } else if let Some(&(bank, spin)) = sm_lookup.get(&**pad) {
                let exp_func = match spin {
                    SysMonPin::VP => "VP_0",
                    SysMonPin::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::SysMon(bank, spin)
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
                "AVSS_0" => BondPin::SysMon(0, SysMonPin::AVss),
                "AVDD_0" => BondPin::SysMon(0, SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMon(0, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMon(0, SysMonPin::VRefN),
                "MGTAVTT" => BondPin::GtRegion(GtRegion::All, GtRegionPin::AVtt),
                "MGTAVCC" => BondPin::GtRegion(GtRegion::All, GtRegionPin::AVcc),
                "MGTAVTT_S" => BondPin::GtRegion(GtRegion::S, GtRegionPin::AVtt),
                "MGTAVCC_S" => BondPin::GtRegion(GtRegion::S, GtRegionPin::AVcc),
                "MGTAVTT_N" => BondPin::GtRegion(GtRegion::N, GtRegionPin::AVtt),
                "MGTAVCC_N" => BondPin::GtRegion(GtRegion::N, GtRegionPin::AVcc),
                "MGTAVTT_L" => BondPin::GtRegion(GtRegion::L, GtRegionPin::AVtt),
                "MGTAVCC_L" => BondPin::GtRegion(GtRegion::L, GtRegionPin::AVcc),
                "MGTAVTT_R" => BondPin::GtRegion(GtRegion::R, GtRegionPin::AVtt),
                "MGTAVCC_R" => BondPin::GtRegion(GtRegion::R, GtRegionPin::AVcc),
                "MGTAVTT_LS" => BondPin::GtRegion(GtRegion::LS, GtRegionPin::AVtt),
                "MGTAVCC_LS" => BondPin::GtRegion(GtRegion::LS, GtRegionPin::AVcc),
                "MGTAVTT_LN" => BondPin::GtRegion(GtRegion::LN, GtRegionPin::AVtt),
                "MGTAVCC_LN" => BondPin::GtRegion(GtRegion::LN, GtRegionPin::AVcc),
                "MGTAVTT_RS" => BondPin::GtRegion(GtRegion::RS, GtRegionPin::AVtt),
                "MGTAVCC_RS" => BondPin::GtRegion(GtRegion::RS, GtRegionPin::AVcc),
                "MGTAVTT_RN" => BondPin::GtRegion(GtRegion::RN, GtRegionPin::AVtt),
                "MGTAVCC_RN" => BondPin::GtRegion(GtRegion::RN, GtRegionPin::AVcc),
                "MGTHAVTT_L" => BondPin::GtRegion(GtRegion::LH, GtRegionPin::AVtt),
                "MGTHAVCC_L" => BondPin::GtRegion(GtRegion::LH, GtRegionPin::AVcc),
                "MGTHAVCCRX_L" => BondPin::GtRegion(GtRegion::LH, GtRegionPin::AVccRx),
                "MGTHAVCCPLL_L" => BondPin::GtRegion(GtRegion::LH, GtRegionPin::AVccPll),
                "MGTHAGND_L" => BondPin::GtRegion(GtRegion::LH, GtRegionPin::AGnd),
                "MGTHAVTT_R" => BondPin::GtRegion(GtRegion::RH, GtRegionPin::AVtt),
                "MGTHAVCC_R" => BondPin::GtRegion(GtRegion::RH, GtRegionPin::AVcc),
                "MGTHAVCCRX_R" => BondPin::GtRegion(GtRegion::RH, GtRegionPin::AVccRx),
                "MGTHAVCCPLL_R" => BondPin::GtRegion(GtRegion::RH, GtRegionPin::AVccPll),
                "MGTHAGND_R" => BondPin::GtRegion(GtRegion::RH, GtRegionPin::AGnd),
                "MGTHAVTT" => BondPin::GtRegion(GtRegion::H, GtRegionPin::AVtt),
                "MGTHAVCC" => BondPin::GtRegion(GtRegion::H, GtRegionPin::AVcc),
                "MGTHAVCCRX" => BondPin::GtRegion(GtRegion::H, GtRegionPin::AVccRx),
                "MGTHAVCCPLL" => BondPin::GtRegion(GtRegion::H, GtRegionPin::AVccPll),
                "MGTHAGND" => BondPin::GtRegion(GtRegion::H, GtRegionPin::AGnd),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPin::VccO(b),
                            "MGTAVTTRCAL_" => BondPin::Gt(b, GtPin::AVttRCal),
                            "MGTRREF_" => BondPin::Gt(b, GtPin::RRef),
                            "MGTRBIAS_" => BondPin::Gt(b, GtPin::RBias),
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
    Bond { pins: bond_pins }
}

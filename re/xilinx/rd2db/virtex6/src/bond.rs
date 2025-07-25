use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_virtex4::bond::{
    Bond, BondPad, CfgPad, GtPad, GtRegion, GtRegionPad, SharedCfgPad, SysMonPad,
};
use prjcombine_virtex4::chip::{DisabledPart, GtKind};
use prjcombine_virtex4::expanded::{IoCoord, IoDiffKind, IoVrKind};
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
    let mut gt_lookup: HashMap<&str, (String, u32, GtPad)> = HashMap::new();
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
                    GtPad::ClkP(i as u8),
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
                    GtPad::ClkN(i as u8),
                ),
            );
        }
        for (i, (pp, pn)) in gt.pads_rx.iter().enumerate() {
            gt_lookup.insert(pp, (format!("MGTRXP{i}_{bank}"), bank, GtPad::RxP(i as u8)));
            gt_lookup.insert(pn, (format!("MGTRXN{i}_{bank}"), bank, GtPad::RxN(i as u8)));
        }
        for (i, (pp, pn)) in gt.pads_tx.iter().enumerate() {
            gt_lookup.insert(pp, (format!("MGTTXP{i}_{bank}"), bank, GtPad::TxP(i as u8)));
            gt_lookup.insert(pn, (format!("MGTTXN{i}_{bank}"), bank, GtPad::TxN(i as u8)));
        }
    }
    let mut sm_lookup: HashMap<&str, (u32, SysMonPad)> = HashMap::new();
    let mut vaux_lookup: HashMap<IoCoord, (usize, char)> = HashMap::new();
    for sysmon in &endev.get_sysmons() {
        sm_lookup.insert(sysmon.pad_vp, (sysmon.bank, SysMonPad::VP));
        sm_lookup.insert(sysmon.pad_vn, (sysmon.bank, SysMonPad::VN));
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
                    Some(SharedCfgPad::Data(d)) => {
                        if d >= 16 {
                            write!(exp_func, "_A{:02}", d - 16).unwrap();
                        }
                        write!(exp_func, "_D{d}").unwrap();
                        if d < 3 {
                            write!(exp_func, "_FS{d}").unwrap();
                        }
                    }
                    Some(SharedCfgPad::Addr(a)) => {
                        write!(exp_func, "_A{a}").unwrap();
                    }
                    Some(SharedCfgPad::Rs(a)) => {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(SharedCfgPad::CsoB) => exp_func += "_CSO_B",
                    Some(SharedCfgPad::FweB) => exp_func += "_FWE_B",
                    Some(SharedCfgPad::FoeB) => exp_func += "_FOE_B_MOSI",
                    Some(SharedCfgPad::FcsB) => exp_func += "_FCS_B",
                    Some(_) => unreachable!(),
                    None => (),
                }
                if !endev.edev.disabled.contains(&DisabledPart::SysMon)
                    && let Some(&(i, pn)) = vaux_lookup.get(&io)
                {
                    write!(exp_func, "_SM{i}{pn}").unwrap();
                }
                write!(exp_func, "_{}", io_info.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pad} {io:?} got {f} exp {exp_func}", f = pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io_info.bank));
                assert_eq!(pin.vcco_bank, Some(io_info.bank));
                BondPad::Io(io_info.bank, io_info.biob)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(&**pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPad::Gt(bank, gpin)
            } else if let Some(&(bank, spin)) = sm_lookup.get(&**pad) {
                let exp_func = match spin {
                    SysMonPad::VP => "VP_0",
                    SysMonPad::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPad::SysMon(bank, spin)
            } else {
                println!("unk iopad {pad} {f}", f = pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPad::Nc,
                "RSVD" => BondPad::Rsvd, // GTH-related
                "GND" => BondPad::Gnd,
                "VCCINT" => BondPad::VccInt,
                "VCCAUX" => BondPad::VccAux,
                "VBATT_0" => BondPad::VccBatt,
                "TCK_0" => BondPad::Cfg(CfgPad::Tck),
                "TDI_0" => BondPad::Cfg(CfgPad::Tdi),
                "TDO_0" => BondPad::Cfg(CfgPad::Tdo),
                "TMS_0" => BondPad::Cfg(CfgPad::Tms),
                "CCLK_0" => BondPad::Cfg(CfgPad::Cclk),
                "DONE_0" => BondPad::Cfg(CfgPad::Done),
                "PROGRAM_B_0" => BondPad::Cfg(CfgPad::ProgB),
                "INIT_B_0" => BondPad::Cfg(CfgPad::InitB),
                "RDWR_B_0" => BondPad::Cfg(CfgPad::RdWrB),
                "CSI_B_0" => BondPad::Cfg(CfgPad::CsiB),
                "DIN_0" => BondPad::Cfg(CfgPad::Din),
                "DOUT_BUSY_0" => BondPad::Cfg(CfgPad::Dout),
                "M0_0" => BondPad::Cfg(CfgPad::M0),
                "M1_0" => BondPad::Cfg(CfgPad::M1),
                "M2_0" => BondPad::Cfg(CfgPad::M2),
                "HSWAPEN_0" => BondPad::Cfg(CfgPad::HswapEn),
                "DXN_0" => BondPad::Dxn,
                "DXP_0" => BondPad::Dxp,
                "VFS_0" => BondPad::Vfs,
                "AVSS_0" => BondPad::SysMon(0, SysMonPad::AVss),
                "AVDD_0" => BondPad::SysMon(0, SysMonPad::AVdd),
                "VREFP_0" => BondPad::SysMon(0, SysMonPad::VRefP),
                "VREFN_0" => BondPad::SysMon(0, SysMonPad::VRefN),
                "MGTAVTT" => BondPad::GtRegion(GtRegion::All, GtRegionPad::AVtt),
                "MGTAVCC" => BondPad::GtRegion(GtRegion::All, GtRegionPad::AVcc),
                "MGTAVTT_S" => BondPad::GtRegion(GtRegion::S, GtRegionPad::AVtt),
                "MGTAVCC_S" => BondPad::GtRegion(GtRegion::S, GtRegionPad::AVcc),
                "MGTAVTT_N" => BondPad::GtRegion(GtRegion::N, GtRegionPad::AVtt),
                "MGTAVCC_N" => BondPad::GtRegion(GtRegion::N, GtRegionPad::AVcc),
                "MGTAVTT_L" => BondPad::GtRegion(GtRegion::L, GtRegionPad::AVtt),
                "MGTAVCC_L" => BondPad::GtRegion(GtRegion::L, GtRegionPad::AVcc),
                "MGTAVTT_R" => BondPad::GtRegion(GtRegion::R, GtRegionPad::AVtt),
                "MGTAVCC_R" => BondPad::GtRegion(GtRegion::R, GtRegionPad::AVcc),
                "MGTAVTT_LS" => BondPad::GtRegion(GtRegion::LS, GtRegionPad::AVtt),
                "MGTAVCC_LS" => BondPad::GtRegion(GtRegion::LS, GtRegionPad::AVcc),
                "MGTAVTT_LN" => BondPad::GtRegion(GtRegion::LN, GtRegionPad::AVtt),
                "MGTAVCC_LN" => BondPad::GtRegion(GtRegion::LN, GtRegionPad::AVcc),
                "MGTAVTT_RS" => BondPad::GtRegion(GtRegion::RS, GtRegionPad::AVtt),
                "MGTAVCC_RS" => BondPad::GtRegion(GtRegion::RS, GtRegionPad::AVcc),
                "MGTAVTT_RN" => BondPad::GtRegion(GtRegion::RN, GtRegionPad::AVtt),
                "MGTAVCC_RN" => BondPad::GtRegion(GtRegion::RN, GtRegionPad::AVcc),
                "MGTHAVTT_L" => BondPad::GtRegion(GtRegion::LH, GtRegionPad::AVtt),
                "MGTHAVCC_L" => BondPad::GtRegion(GtRegion::LH, GtRegionPad::AVcc),
                "MGTHAVCCRX_L" => BondPad::GtRegion(GtRegion::LH, GtRegionPad::AVccRx),
                "MGTHAVCCPLL_L" => BondPad::GtRegion(GtRegion::LH, GtRegionPad::AVccPll),
                "MGTHAGND_L" => BondPad::GtRegion(GtRegion::LH, GtRegionPad::AGnd),
                "MGTHAVTT_R" => BondPad::GtRegion(GtRegion::RH, GtRegionPad::AVtt),
                "MGTHAVCC_R" => BondPad::GtRegion(GtRegion::RH, GtRegionPad::AVcc),
                "MGTHAVCCRX_R" => BondPad::GtRegion(GtRegion::RH, GtRegionPad::AVccRx),
                "MGTHAVCCPLL_R" => BondPad::GtRegion(GtRegion::RH, GtRegionPad::AVccPll),
                "MGTHAGND_R" => BondPad::GtRegion(GtRegion::RH, GtRegionPad::AGnd),
                "MGTHAVTT" => BondPad::GtRegion(GtRegion::H, GtRegionPad::AVtt),
                "MGTHAVCC" => BondPad::GtRegion(GtRegion::H, GtRegionPad::AVcc),
                "MGTHAVCCRX" => BondPad::GtRegion(GtRegion::H, GtRegionPad::AVccRx),
                "MGTHAVCCPLL" => BondPad::GtRegion(GtRegion::H, GtRegionPad::AVccPll),
                "MGTHAGND" => BondPad::GtRegion(GtRegion::H, GtRegionPad::AGnd),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPad::VccO(b),
                            "MGTAVTTRCAL_" => BondPad::Gt(b, GtPad::AVttRCal),
                            "MGTRREF_" => BondPad::Gt(b, GtPad::RRef),
                            "MGTRBIAS_" => BondPad::Gt(b, GtPad::RBias),
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

use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_virtex4::bond::{
    Bond, BondPin, CfgPin, GtPin, GtRegion, GtRegionPin, SharedCfgPin, SysMonPin,
};
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
    let mut gt_lookup: HashMap<&str, (String, u32, GtPin)> = HashMap::new();
    for gt in endev.get_gts() {
        let bank = gt.bank;
        for (i, (pp, pn)) in gt.pads_clk.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (format!("MGTREFCLKP_{bank}"), bank, GtPin::ClkP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("MGTREFCLKN_{bank}"), bank, GtPin::ClkN(i as u8)),
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
                    exp_func += "_CC";
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
                    Some(_) => unreachable!(),
                    None => (),
                }
                if let Some(&(i, pn)) = vaux_lookup.get(&io) {
                    write!(exp_func, "_SM{i}{pn}").unwrap();
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
                "AVSS_0" => BondPin::SysMon(0, SysMonPin::AVss),
                "AVDD_0" => BondPin::SysMon(0, SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMon(0, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMon(0, SysMonPin::VRefN),
                "MGTAVTTRXC" => BondPin::GtRegion(GtRegion::All, GtRegionPin::AVttRxC),
                "MGTAVTTRXC_L" => BondPin::GtRegion(GtRegion::L, GtRegionPin::AVttRxC),
                "MGTAVTTRXC_R" => BondPin::GtRegion(GtRegion::R, GtRegionPin::AVttRxC),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPin::VccO(b),
                            "MGTAVCC_" => BondPin::Gt(b, GtPin::AVcc),
                            "MGTAVCCPLL_" => BondPin::Gt(b, GtPin::AVccPll),
                            "MGTAVTTRX_" => BondPin::Gt(b, GtPin::VtRx(0)),
                            "MGTAVTTTX_" => BondPin::Gt(b, GtPin::VtTx(0)),
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

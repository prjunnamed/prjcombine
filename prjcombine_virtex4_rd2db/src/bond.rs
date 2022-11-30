use prjcombine_virtex4::bond::{Bond, BondPin, CfgPin, GtPin, SharedCfgPin, SysMonPin};
use prjcombine_virtex4::expanded::{ExpandedDevice, IoCoord, IoDiffKind, IoVrKind};

use prjcombine_rawdump::PkgPin;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use prjcombine_rdgrid::split_num;

pub fn make_bond(edev: &ExpandedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = edev.io.iter().map(|io| (&*io.name, io)).collect();
    let mut gt_lookup: HashMap<String, (String, u32, GtPin)> = HashMap::new();
    for gt in &edev.gt {
        let bank = gt.bank;
        for (i, (pp, pn)) in gt.pads_clk.iter().enumerate() {
            gt_lookup.insert(
                pp.clone(),
                (format!("MGTCLK_P_{bank}"), bank, GtPin::ClkP(i as u8)),
            );
            gt_lookup.insert(
                pn.clone(),
                (format!("MGTCLK_N_{bank}"), bank, GtPin::ClkN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_rx.iter().enumerate() {
            let ab = ['B', 'A'][i];
            gt_lookup.insert(
                pp.clone(),
                (format!("RXPPAD{ab}_{bank}"), bank, GtPin::RxP(i as u8)),
            );
            gt_lookup.insert(
                pn.clone(),
                (format!("RXNPAD{ab}_{bank}"), bank, GtPin::RxN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_tx.iter().enumerate() {
            let ab = ['B', 'A'][i];
            gt_lookup.insert(
                pp.clone(),
                (format!("TXPPAD{ab}_{bank}"), bank, GtPin::TxP(i as u8)),
            );
            gt_lookup.insert(
                pn.clone(),
                (format!("TXNPAD{ab}_{bank}"), bank, GtPin::TxN(i as u8)),
            );
        }
    }
    let mut sm_lookup: HashMap<String, (u32, SysMonPin)> = HashMap::new();
    let mut vaux_lookup: HashMap<IoCoord, (u32, usize, char)> = HashMap::new();
    for sysmon in &edev.sysmon {
        sm_lookup.insert(sysmon.pad_vp.clone(), (sysmon.bank, SysMonPin::VP));
        sm_lookup.insert(sysmon.pad_vn.clone(), (sysmon.bank, SysMonPin::VN));
        for (i, vaux) in sysmon.vaux.iter().enumerate() {
            if let &Some((vauxp, vauxn)) = vaux {
                vaux_lookup.insert(vauxp, (sysmon.bank, i, 'P'));
                vaux_lookup.insert(vauxn, (sysmon.bank, i, 'N'));
            }
        }
    }
    let cfg_lookup: HashMap<_, _> = edev.cfg_io.iter().map(|(&k, &v)| (v, k)).collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&**pad) {
                let mut exp_func = match io.diff {
                    IoDiffKind::None => format!("IO_{}", io.pkgid),
                    IoDiffKind::P(_) => format!("IO_L{}P", io.pkgid),
                    IoDiffKind::N(_) => format!("IO_L{}N", io.pkgid),
                };
                match cfg_lookup.get(&io.crd).copied() {
                    Some(SharedCfgPin::Data(d)) => write!(exp_func, "_D{d}").unwrap(),
                    Some(_) => unreachable!(),
                    None => (),
                }
                if io.is_gc {
                    exp_func += "_GC";
                }
                if io.is_vref {
                    exp_func += "_VREF";
                }
                match io.vr {
                    IoVrKind::VrP => exp_func += "_VRP",
                    IoVrKind::VrN => exp_func += "_VRN",
                    IoVrKind::None => (),
                }
                if io.is_srcc {
                    exp_func += "_CC";
                }
                if let Some(&(bank, i, _)) = vaux_lookup.get(&io.crd) {
                    write!(exp_func, "_{}{}", ["SM", "ADC"][bank as usize], i).unwrap();
                }
                if io.is_lc {
                    exp_func += "_LC";
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pad} {io:?} got {f} exp {exp_func}", f = pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::Io(io.bank, io.biob)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gt(bank, gpin)
            } else if let Some(&(bank, spin)) = sm_lookup.get(pad) {
                let exp_func = match (bank, spin) {
                    (0, SysMonPin::VP) => "VP_SM",
                    (0, SysMonPin::VN) => "VN_SM",
                    (1, SysMonPin::VP) => "VP_ADC",
                    (1, SysMonPin::VN) => "VN_ADC",
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
                "AVSS_SM" => BondPin::SysMon(0, SysMonPin::AVss),
                "AVSS_ADC" => BondPin::SysMon(1, SysMonPin::AVss),
                "AVDD_SM" => BondPin::SysMon(0, SysMonPin::AVdd),
                "AVDD_ADC" => BondPin::SysMon(1, SysMonPin::AVdd),
                "VREFP_SM" => BondPin::SysMon(0, SysMonPin::VRefP),
                "VREFP_ADC" => BondPin::SysMon(1, SysMonPin::VRefP),
                "VREFN_SM" => BondPin::SysMon(0, SysMonPin::VRefN),
                "VREFN_ADC" => BondPin::SysMon(1, SysMonPin::VRefN),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPin::VccO(b),
                            "GNDA_" => BondPin::Gt(b, GtPin::GndA),
                            "VTRXA_" => BondPin::Gt(b, GtPin::VtRx(1)),
                            "VTRXB_" => BondPin::Gt(b, GtPin::VtRx(0)),
                            "VTTXA_" => BondPin::Gt(b, GtPin::VtTx(1)),
                            "VTTXB_" => BondPin::Gt(b, GtPin::VtTx(0)),
                            "AVCCAUXRXA_" => BondPin::Gt(b, GtPin::AVccAuxRx(1)),
                            "AVCCAUXRXB_" => BondPin::Gt(b, GtPin::AVccAuxRx(0)),
                            "AVCCAUXTX_" => BondPin::Gt(b, GtPin::AVccAuxTx),
                            "AVCCAUXMGT_" => BondPin::Gt(b, GtPin::AVccAuxMgt),
                            "RTERM_" => BondPin::Gt(b, GtPin::RTerm),
                            "MGTVREF_" => BondPin::Gt(b, GtPin::MgtVRef),
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

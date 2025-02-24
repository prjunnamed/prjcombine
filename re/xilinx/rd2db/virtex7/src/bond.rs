use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::{Part, PkgPin};
use prjcombine_virtex4::bond::{
    Bond, BondPin, CfgPin, GtPin, GtRegion, GtRegionPin, GtzPin, PsPin, SharedCfgPin, SysMonPin,
};
use prjcombine_virtex4::chip::GtKind;
use prjcombine_virtex4::expanded::{IoCoord, IoDiffKind, IoVrKind};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use prjcombine_re_xilinx_rd2db_grid::split_num;

pub fn make_bond(rd: &Part, pkg: &str, endev: &ExpandedNamedDevice, pins: &[PkgPin]) -> Bond {
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
        let t = match gt.kind {
            GtKind::Gtp => "GTP",
            GtKind::Gtx => "GTX",
            GtKind::Gth => "GTH",
        };
        for (i, (pp, pn)) in gt.pads_clk.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (format!("MGTREFCLK{i}P_{bank}"), bank, GtPin::ClkP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("MGTREFCLK{i}N_{bank}"), bank, GtPin::ClkN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_rx.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (format!("M{t}RXP{i}_{bank}"), bank, GtPin::RxP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("M{t}RXN{i}_{bank}"), bank, GtPin::RxN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_tx.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (format!("M{t}TXP{i}_{bank}"), bank, GtPin::TxP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("M{t}TXN{i}_{bank}"), bank, GtPin::TxN(i as u8)),
            );
        }
    }
    let mut gtz_lookup: HashMap<String, (String, u32, GtzPin)> = HashMap::new();
    for (dir, egt) in &endev.edev.gtz {
        let ngt = &endev.gtz[dir];
        let bank = egt.bank;
        for (i, (pp, pn)) in ngt.pads_clk.iter().enumerate() {
            gtz_lookup.insert(
                pp.clone(),
                (
                    format!("MGTZREFCLK{i}P_{bank}"),
                    bank,
                    GtzPin::ClkP(i as u8),
                ),
            );
            gtz_lookup.insert(
                pn.clone(),
                (
                    format!("MGTZREFCLK{i}N_{bank}"),
                    bank,
                    GtzPin::ClkN(i as u8),
                ),
            );
        }
        for (i, (pp, pn)) in ngt.pads_rx.iter().enumerate() {
            gtz_lookup.insert(
                pp.clone(),
                (format!("MGTZRXP{i}_{bank}"), bank, GtzPin::RxP(i as u8)),
            );
            gtz_lookup.insert(
                pn.clone(),
                (format!("MGTZRXN{i}_{bank}"), bank, GtzPin::RxN(i as u8)),
            );
        }
        for (i, (pp, pn)) in ngt.pads_tx.iter().enumerate() {
            gtz_lookup.insert(
                pp.clone(),
                (format!("MGTZTXP{i}_{bank}"), bank, GtzPin::TxP(i as u8)),
            );
            gtz_lookup.insert(
                pn.clone(),
                (format!("MGTZTXN{i}_{bank}"), bank, GtzPin::TxN(i as u8)),
            );
        }
    }
    let mut sm_lookup: HashMap<&str, (u32, SysMonPin)> = HashMap::new();
    let mut vaux_lookup: HashMap<IoCoord, (usize, char)> = HashMap::new();
    for sysmon in &endev.get_sysmons() {
        if sysmon.die == endev.edev.interposer.unwrap().primary {
            sm_lookup.insert(sysmon.pad_vp, (sysmon.bank, SysMonPin::VP));
            sm_lookup.insert(sysmon.pad_vn, (sysmon.bank, SysMonPin::VN));
            for (i, vaux) in sysmon.vaux.iter().enumerate() {
                if let &Some((vauxp, vauxn)) = vaux {
                    vaux_lookup.insert(vauxp, (i, 'P'));
                    vaux_lookup.insert(vauxn, (i, 'N'));
                }
            }
        }
    }
    let cfg_lookup: HashMap<_, _> = endev.edev.cfg_io.iter().map(|(&k, &v)| (v, k)).collect();
    let ps_lookup: HashMap<_, _> = endev
        .edev
        .get_ps_pins()
        .into_iter()
        .map(|k| (endev.get_ps_pin_name(k), k))
        .collect();
    let has_14 = io_lookup
        .values()
        .any(|io| endev.edev.get_io_info(*io).bank == 14);
    let is_spartan = rd.part.contains("7s");
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&**pad) {
                let io_info = endev.edev.get_io_info(io);
                let mut exp_func = match io_info.diff {
                    IoDiffKind::None => format!("IO_{}", io_info.pkgid),
                    IoDiffKind::P(_) => format!("IO_L{}P", io_info.pkgid),
                    IoDiffKind::N(_) => format!("IO_L{}N", io_info.pkgid),
                };
                if matches!(pkg, "fbg484" | "fbv484" | "fbv485")
                    && rd.part.contains("7k")
                    && io_info.bank == 16
                    && matches!(io_info.biob, 2 | 14 | 37)
                {
                    exp_func = format!("IO_{}", io_info.pkgid);
                }
                if let Some(byte) = io_info.byte {
                    write!(exp_func, "_T{byte}").unwrap();
                }
                if io_info.bank == 35 && matches!(io_info.biob, 21 | 22) {
                    if let Some(&(i, pn)) = vaux_lookup.get(&io) {
                        write!(exp_func, "_AD{i}{pn}").unwrap();
                    }
                }
                if io_info.is_srcc {
                    exp_func += "_SRCC";
                }
                if io_info.is_mrcc {
                    exp_func += "_MRCC";
                }
                if io_info.is_dqs {
                    exp_func += "_DQS";
                }
                match cfg_lookup.get(&io).copied() {
                    Some(SharedCfgPin::Data(d)) => {
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
                    Some(SharedCfgPin::Addr(a)) => {
                        if !is_spartan {
                            write!(exp_func, "_A{a}").unwrap();
                        }
                    }
                    Some(SharedCfgPin::Rs(a)) => {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(SharedCfgPin::PudcB) => exp_func += "_PUDC_B",
                    Some(SharedCfgPin::EmCclk) => exp_func += "_EMCCLK",
                    Some(SharedCfgPin::RdWrB) => exp_func += "_RDWR_B",
                    Some(SharedCfgPin::CsiB) => exp_func += "_CSI_B",
                    Some(SharedCfgPin::CsoB) => exp_func += "_DOUT_CSO_B",
                    Some(SharedCfgPin::FweB) => {
                        if !is_spartan {
                            exp_func += "_FWE_B"
                        }
                    }
                    Some(SharedCfgPin::FoeB) => {
                        if !is_spartan {
                            exp_func += "_FOE_B"
                        }
                    }
                    Some(SharedCfgPin::FcsB) => exp_func += "_FCS_B",
                    Some(SharedCfgPin::AdvB) => {
                        if !is_spartan {
                            exp_func += "_ADV_B"
                        }
                    }
                    None => (),
                }
                if !(io_info.bank == 35 && matches!(io_info.biob, 21 | 22)) {
                    if let Some(&(i, pn)) = vaux_lookup.get(&io) {
                        write!(exp_func, "_AD{i}{pn}").unwrap();
                    }
                }
                if io_info.is_vref {
                    exp_func += "_VREF";
                }
                match io_info.vr {
                    IoVrKind::VrP => exp_func += "_VRP",
                    IoVrKind::VrN => exp_func += "_VRN",
                    IoVrKind::None => (),
                }
                write!(exp_func, "_{}", io_info.bank).unwrap();
                if exp_func != pin.func {
                    println!(
                        "pad {pkg} {pad} {io:?} got {f} exp {exp_func}",
                        f = pin.func
                    );
                }
                assert_eq!(pin.vref_bank, Some(io_info.bank));
                assert_eq!(pin.vcco_bank, Some(io_info.bank));
                BondPin::Io(io_info.bank, io_info.biob)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(&**pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gt(bank, gpin)
            } else if let Some(&(ref exp_func, bank, gpin)) = gtz_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gtz(bank, gpin)
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
            } else if let Some(&spin) = ps_lookup.get(&**pad) {
                let bank = endev.edev.get_ps_bank(spin);
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
                    PsPin::DdrCkP => format!("PS_DDR_CKP_{bank}"),
                    PsPin::DdrCkN => format!("PS_DDR_CKN_{bank}"),
                    PsPin::DdrCke => format!("PS_DDR_CKE_{bank}"),
                    PsPin::DdrOdt => format!("PS_DDR_ODT_{bank}"),
                    PsPin::DdrDrstB => format!("PS_DDR_DRST_B_{bank}"),
                    PsPin::DdrCsB => format!("PS_DDR_CS_B_{bank}"),
                    PsPin::DdrRasB => format!("PS_DDR_RAS_B_{bank}"),
                    PsPin::DdrCasB => format!("PS_DDR_CAS_B_{bank}"),
                    PsPin::DdrWeB => format!("PS_DDR_WE_B_{bank}"),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::PsIo(bank, spin)
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
                "GNDADC_0" | "GNDADC" => BondPin::SysMon(0, SysMonPin::AVss),
                "VCCADC_0" | "VCCADC" => BondPin::SysMon(0, SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMon(0, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMon(0, SysMonPin::VRefN),
                "MGTAVTT" => BondPin::GtRegion(GtRegion::All, GtRegionPin::AVtt),
                "MGTAVCC" => BondPin::GtRegion(GtRegion::All, GtRegionPin::AVcc),
                "MGTVCCAUX" => BondPin::GtRegion(GtRegion::All, GtRegionPin::VccAux),
                "VCCO_MIO0_500" => BondPin::VccO(500),
                "VCCO_MIO1_501" => BondPin::VccO(501),
                "VCCO_DDR_502" => BondPin::VccO(502),
                "VCCPINT" => BondPin::VccPsInt,
                "VCCPAUX" => BondPin::VccPsAux,
                "VCCPLL" => BondPin::VccPsPll,
                "PS_MIO_VREF_501" => BondPin::PsVref(501, 0),
                "PS_DDR_VREF0_502" => BondPin::PsVref(502, 0),
                "PS_DDR_VREF1_502" => BondPin::PsVref(502, 1),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPin::VccO(b),
                            "VCCAUX_IO_G" => BondPin::VccAuxIo(b),
                            "MGTAVTTRCAL_" => BondPin::Gt(b, GtPin::AVttRCal),
                            "MGTRREF_" => BondPin::Gt(b, GtPin::RRef),
                            "MGTAVTT_G" => BondPin::GtRegion(GtRegion::Num(b), GtRegionPin::AVtt),
                            "MGTAVCC_G" => BondPin::GtRegion(GtRegion::Num(b), GtRegionPin::AVcc),
                            "MGTVCCAUX_G" => {
                                BondPin::GtRegion(GtRegion::Num(b), GtRegionPin::VccAux)
                            }
                            "MGTZAGND_" => BondPin::Gtz(b, GtzPin::AGnd),
                            "MGTZAVCC_" => BondPin::Gtz(b, GtzPin::AVcc),
                            "MGTZVCCH_" => BondPin::Gtz(b, GtzPin::VccH),
                            "MGTZVCCL_" => BondPin::Gtz(b, GtzPin::VccL),
                            "MGTZ_OBS_CLK_P_" => BondPin::Gtz(b, GtzPin::ObsClkP),
                            "MGTZ_OBS_CLK_N_" => BondPin::Gtz(b, GtzPin::ObsClkN),
                            "MGTZ_SENSE_AVCC_" => BondPin::Gtz(b, GtzPin::SenseAVcc),
                            "MGTZ_SENSE_AGND_" => BondPin::Gtz(b, GtzPin::SenseAGnd),
                            "MGTZ_SENSE_GNDL_" => BondPin::Gtz(b, GtzPin::SenseGndL),
                            "MGTZ_SENSE_GND_" => BondPin::Gtz(b, GtzPin::SenseGnd),
                            "MGTZ_SENSE_VCC_" => BondPin::Gtz(b, GtzPin::SenseVcc),
                            "MGTZ_SENSE_VCCL_" => BondPin::Gtz(b, GtzPin::SenseVccL),
                            "MGTZ_SENSE_VCCH_" => BondPin::Gtz(b, GtzPin::SenseVccH),
                            "MGTZ_THERM_IN_" => BondPin::Gtz(b, GtzPin::ThermIn),
                            "MGTZ_THERM_OUT_" => BondPin::Gtz(b, GtzPin::ThermOut),
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
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond { pins: bond_pins }
}

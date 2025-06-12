use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::{Part, PkgPin};
use prjcombine_virtex4::bond::{
    Bond, BondPad, CfgPad, GtPad, GtRegion, GtRegionPad, GtzPad, PsPad, SharedCfgPad, SysMonPad,
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
    let mut gt_lookup: HashMap<&str, (String, u32, GtPad)> = HashMap::new();
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
                (format!("MGTREFCLK{i}P_{bank}"), bank, GtPad::ClkP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("MGTREFCLK{i}N_{bank}"), bank, GtPad::ClkN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_rx.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (format!("M{t}RXP{i}_{bank}"), bank, GtPad::RxP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("M{t}RXN{i}_{bank}"), bank, GtPad::RxN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_tx.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (format!("M{t}TXP{i}_{bank}"), bank, GtPad::TxP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("M{t}TXN{i}_{bank}"), bank, GtPad::TxN(i as u8)),
            );
        }
    }
    let mut gtz_lookup: HashMap<String, (String, u32, GtzPad)> = HashMap::new();
    for (dir, egt) in &endev.edev.gtz {
        let ngt = &endev.gtz[dir];
        let bank = egt.bank;
        for (i, (pp, pn)) in ngt.pads_clk.iter().enumerate() {
            gtz_lookup.insert(
                pp.clone(),
                (
                    format!("MGTZREFCLK{i}P_{bank}"),
                    bank,
                    GtzPad::ClkP(i as u8),
                ),
            );
            gtz_lookup.insert(
                pn.clone(),
                (
                    format!("MGTZREFCLK{i}N_{bank}"),
                    bank,
                    GtzPad::ClkN(i as u8),
                ),
            );
        }
        for (i, (pp, pn)) in ngt.pads_rx.iter().enumerate() {
            gtz_lookup.insert(
                pp.clone(),
                (format!("MGTZRXP{i}_{bank}"), bank, GtzPad::RxP(i as u8)),
            );
            gtz_lookup.insert(
                pn.clone(),
                (format!("MGTZRXN{i}_{bank}"), bank, GtzPad::RxN(i as u8)),
            );
        }
        for (i, (pp, pn)) in ngt.pads_tx.iter().enumerate() {
            gtz_lookup.insert(
                pp.clone(),
                (format!("MGTZTXP{i}_{bank}"), bank, GtzPad::TxP(i as u8)),
            );
            gtz_lookup.insert(
                pn.clone(),
                (format!("MGTZTXN{i}_{bank}"), bank, GtzPad::TxN(i as u8)),
            );
        }
    }
    let mut sm_lookup: HashMap<&str, (u32, SysMonPad)> = HashMap::new();
    let mut vaux_lookup: HashMap<IoCoord, (usize, char)> = HashMap::new();
    for sysmon in &endev.get_sysmons() {
        if sysmon.cell.die == endev.edev.interposer.unwrap().primary {
            sm_lookup.insert(sysmon.pad_vp, (sysmon.bank, SysMonPad::VP));
            sm_lookup.insert(sysmon.pad_vn, (sysmon.bank, SysMonPad::VN));
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
                    Some(SharedCfgPad::Data(d)) => {
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
                    Some(SharedCfgPad::Addr(a)) => {
                        if !is_spartan {
                            write!(exp_func, "_A{a}").unwrap();
                        }
                    }
                    Some(SharedCfgPad::Rs(a)) => {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(SharedCfgPad::PudcB) => exp_func += "_PUDC_B",
                    Some(SharedCfgPad::EmCclk) => exp_func += "_EMCCLK",
                    Some(SharedCfgPad::RdWrB) => exp_func += "_RDWR_B",
                    Some(SharedCfgPad::CsiB) => exp_func += "_CSI_B",
                    Some(SharedCfgPad::CsoB) => exp_func += "_DOUT_CSO_B",
                    Some(SharedCfgPad::FweB) => {
                        if !is_spartan {
                            exp_func += "_FWE_B"
                        }
                    }
                    Some(SharedCfgPad::FoeB) => {
                        if !is_spartan {
                            exp_func += "_FOE_B"
                        }
                    }
                    Some(SharedCfgPad::FcsB) => exp_func += "_FCS_B",
                    Some(SharedCfgPad::AdvB) => {
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
                BondPad::Io(io_info.bank, io_info.biob)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(&**pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPad::Gt(bank, gpin)
            } else if let Some(&(ref exp_func, bank, gpin)) = gtz_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPad::Gtz(bank, gpin)
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
            } else if let Some(&spin) = ps_lookup.get(&**pad) {
                let bank = endev.edev.get_ps_bank(spin);
                let exp_func = match spin {
                    PsPad::Clk => format!("PS_CLK_{bank}"),
                    PsPad::PorB => format!("PS_POR_B_{bank}"),
                    PsPad::SrstB => format!("PS_SRST_B_{bank}"),
                    PsPad::Mio(x) => format!("PS_MIO{x}_{bank}"),
                    PsPad::DdrDm(x) => format!("PS_DDR_DM{x}_{bank}"),
                    PsPad::DdrDq(x) => format!("PS_DDR_DQ{x}_{bank}"),
                    PsPad::DdrDqsP(x) => format!("PS_DDR_DQS_P{x}_{bank}"),
                    PsPad::DdrDqsN(x) => format!("PS_DDR_DQS_N{x}_{bank}"),
                    PsPad::DdrA(x) => format!("PS_DDR_A{x}_{bank}"),
                    PsPad::DdrBa(x) => format!("PS_DDR_BA{x}_{bank}"),
                    PsPad::DdrVrP => format!("PS_DDR_VRP_{bank}"),
                    PsPad::DdrVrN => format!("PS_DDR_VRN_{bank}"),
                    PsPad::DdrCkP => format!("PS_DDR_CKP_{bank}"),
                    PsPad::DdrCkN => format!("PS_DDR_CKN_{bank}"),
                    PsPad::DdrCke => format!("PS_DDR_CKE_{bank}"),
                    PsPad::DdrOdt => format!("PS_DDR_ODT_{bank}"),
                    PsPad::DdrDrstB => format!("PS_DDR_DRST_B_{bank}"),
                    PsPad::DdrCsB => format!("PS_DDR_CS_B_{bank}"),
                    PsPad::DdrRasB => format!("PS_DDR_RAS_B_{bank}"),
                    PsPad::DdrCasB => format!("PS_DDR_CAS_B_{bank}"),
                    PsPad::DdrWeB => format!("PS_DDR_WE_B_{bank}"),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPad::PsIo(bank, spin)
            } else {
                println!("unk iopad {pad} {f}", f = pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPad::Nc,
                "GND" => BondPad::Gnd,
                "VCCINT" => BondPad::VccInt,
                "VCCAUX" => BondPad::VccAux,
                "VCCBRAM" => BondPad::VccBram,
                "VCCBATT_0" => BondPad::VccBatt,
                "TCK_0" => BondPad::Cfg(CfgPad::Tck),
                "TDI_0" => BondPad::Cfg(CfgPad::Tdi),
                "TDO_0" => BondPad::Cfg(CfgPad::Tdo),
                "TMS_0" => BondPad::Cfg(CfgPad::Tms),
                "CCLK_0" => BondPad::Cfg(CfgPad::Cclk),
                "RSVDGND" if !has_14 => BondPad::Cfg(CfgPad::Cclk),
                "RSVDVCC3" if !has_14 => BondPad::Cfg(CfgPad::M0),
                "RSVDVCC2" if !has_14 => BondPad::Cfg(CfgPad::M1),
                "RSVDVCC1" if !has_14 => BondPad::Cfg(CfgPad::M2),
                "RSVDGND" => BondPad::RsvdGnd, // used for disabled transceiver RX pins on 7a12t
                "DONE_0" => BondPad::Cfg(CfgPad::Done),
                "PROGRAM_B_0" => BondPad::Cfg(CfgPad::ProgB),
                "INIT_B_0" => BondPad::Cfg(CfgPad::InitB),
                "M0_0" => BondPad::Cfg(CfgPad::M0),
                "M1_0" => BondPad::Cfg(CfgPad::M1),
                "M2_0" => BondPad::Cfg(CfgPad::M2),
                "CFGBVS_0" => BondPad::Cfg(CfgPad::CfgBvs),
                "DXN_0" => BondPad::Dxn,
                "DXP_0" => BondPad::Dxp,
                "GNDADC_0" | "GNDADC" => BondPad::SysMon(0, SysMonPad::AVss),
                "VCCADC_0" | "VCCADC" => BondPad::SysMon(0, SysMonPad::AVdd),
                "VREFP_0" => BondPad::SysMon(0, SysMonPad::VRefP),
                "VREFN_0" => BondPad::SysMon(0, SysMonPad::VRefN),
                "MGTAVTT" => BondPad::GtRegion(GtRegion::All, GtRegionPad::AVtt),
                "MGTAVCC" => BondPad::GtRegion(GtRegion::All, GtRegionPad::AVcc),
                "MGTVCCAUX" => BondPad::GtRegion(GtRegion::All, GtRegionPad::VccAux),
                "VCCO_MIO0_500" => BondPad::VccO(500),
                "VCCO_MIO1_501" => BondPad::VccO(501),
                "VCCO_DDR_502" => BondPad::VccO(502),
                "VCCPINT" => BondPad::VccPsInt,
                "VCCPAUX" => BondPad::VccPsAux,
                "VCCPLL" => BondPad::VccPsPll,
                "PS_MIO_VREF_501" => BondPad::PsVref(501, 0),
                "PS_DDR_VREF0_502" => BondPad::PsVref(502, 0),
                "PS_DDR_VREF1_502" => BondPad::PsVref(502, 1),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPad::VccO(b),
                            "VCCAUX_IO_G" => BondPad::VccAuxIo(b),
                            "MGTAVTTRCAL_" => BondPad::Gt(b, GtPad::AVttRCal),
                            "MGTRREF_" => BondPad::Gt(b, GtPad::RRef),
                            "MGTAVTT_G" => BondPad::GtRegion(GtRegion::Num(b), GtRegionPad::AVtt),
                            "MGTAVCC_G" => BondPad::GtRegion(GtRegion::Num(b), GtRegionPad::AVcc),
                            "MGTVCCAUX_G" => {
                                BondPad::GtRegion(GtRegion::Num(b), GtRegionPad::VccAux)
                            }
                            "MGTZAGND_" => BondPad::Gtz(b, GtzPad::AGnd),
                            "MGTZAVCC_" => BondPad::Gtz(b, GtzPad::AVcc),
                            "MGTZVCCH_" => BondPad::Gtz(b, GtzPad::VccH),
                            "MGTZVCCL_" => BondPad::Gtz(b, GtzPad::VccL),
                            "MGTZ_OBS_CLK_P_" => BondPad::Gtz(b, GtzPad::ObsClkP),
                            "MGTZ_OBS_CLK_N_" => BondPad::Gtz(b, GtzPad::ObsClkN),
                            "MGTZ_SENSE_AVCC_" => BondPad::Gtz(b, GtzPad::SenseAVcc),
                            "MGTZ_SENSE_AGND_" => BondPad::Gtz(b, GtzPad::SenseAGnd),
                            "MGTZ_SENSE_GNDL_" => BondPad::Gtz(b, GtzPad::SenseGndL),
                            "MGTZ_SENSE_GND_" => BondPad::Gtz(b, GtzPad::SenseGnd),
                            "MGTZ_SENSE_VCC_" => BondPad::Gtz(b, GtzPad::SenseVcc),
                            "MGTZ_SENSE_VCCL_" => BondPad::Gtz(b, GtzPad::SenseVccL),
                            "MGTZ_SENSE_VCCH_" => BondPad::Gtz(b, GtzPad::SenseVccH),
                            "MGTZ_THERM_IN_" => BondPad::Gtz(b, GtzPad::ThermIn),
                            "MGTZ_THERM_OUT_" => BondPad::Gtz(b, GtzPad::ThermOut),
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

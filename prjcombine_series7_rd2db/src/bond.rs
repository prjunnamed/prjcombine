use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::DieId;
use prjcombine_rawdump::{Part, PkgPin};
use prjcombine_series7::io::{get_gt, get_gtz_pads, get_io, get_ps_pads, get_sysmon_pads};
use prjcombine_series7::{
    Bond, BondPin, CfgPin, ExtraDie, Grid, GtPin, GtRegionPin, GtzPin, PsPin, SharedCfgPin,
    SysMonPin,
};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use prjcombine_rdgrid::split_num;

pub fn make_bond(
    rd: &Part,
    pkg: &str,
    grids: &EntityVec<DieId, Grid>,
    grid_master: DieId,
    extras: &[ExtraDie],
    pins: &[PkgPin],
) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let is_7k70t = rd.part.contains("7k70t");
    let io_lookup: HashMap<_, _> = get_io(grids, grid_master)
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = get_gt(grids, grid_master, extras, is_7k70t)
        .into_iter()
        .flat_map(|gt| {
            gt.get_pads()
                .into_iter()
                .map(move |(name, func, pin)| (name, (func, gt.bank, pin)))
        })
        .collect();
    let gtz_lookup: HashMap<_, _> = get_gtz_pads(extras)
        .into_iter()
        .map(|(name, func, bank, pin)| (name, (func, bank, pin)))
        .collect();
    let sm_lookup: HashMap<_, _> = get_sysmon_pads(grids, extras, is_7k70t)
        .into_iter()
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    let ps_lookup: HashMap<_, _> = get_ps_pads(grids)
        .into_iter()
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    let has_14 = io_lookup.values().any(|io| io.bank == 14);
    let has_15 = io_lookup.values().any(|io| io.bank == 15);
    let has_35 = io_lookup.values().any(|io| io.bank == 35);
    let is_spartan = rd.part.contains("7s");
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func = match io.row.to_idx() % 50 {
                    0 => "IO_25".to_string(),
                    49 => "IO_0".to_string(),
                    n => format!(
                        "IO_L{}{}_T{}",
                        (50 - n) / 2,
                        ['P', 'N'][n % 2],
                        3 - (n - 1) / 12
                    ),
                };
                if matches!(pkg, "fbg484" | "fbv484")
                    && rd.part.contains("7k")
                    && io.bank == 16
                    && matches!(io.row.to_idx() % 50, 2 | 14 | 37)
                {
                    exp_func = format!(
                        "IO_{}_T{}",
                        (50 - io.row.to_idx() % 50) / 2,
                        3 - (io.row.to_idx() % 50 - 1) / 12
                    );
                }
                if io.bank == 35 && matches!(io.row.to_idx() % 50, 21 | 22) {
                    if let Some(sm) = io.sm_pair(has_15, has_35) {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.row.to_idx() % 2]).unwrap();
                    }
                }
                if io.is_srcc() {
                    exp_func += "_SRCC";
                }
                if io.is_mrcc() {
                    exp_func += "_MRCC";
                }
                if io.is_dqs() {
                    exp_func += "_DQS";
                }
                match io.get_cfg(has_14) {
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
                    Some(SharedCfgPin::Dout) => exp_func += "_DOUT_CSO_B",
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
                if !(io.bank == 35 && matches!(io.row.to_idx() % 50, 21 | 22)) {
                    if let Some(sm) = io.sm_pair(has_15, has_35) {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.row.to_idx() % 2]).unwrap();
                    }
                }
                if io.is_vref() {
                    exp_func += "_VREF";
                }
                if io.is_vrp() {
                    exp_func += "_VRP";
                }
                if io.is_vrn() {
                    exp_func += "_VRN";
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!(
                        "pad {pkg} {pad} {io:?} got {f} exp {exp_func}",
                        f = pin.func
                    );
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::Io(io.bank, (io.row.to_idx() % 50) as u32)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gt(bank, gpin)
            } else if let Some(&(ref exp_func, bank, gpin)) = gtz_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gtz(bank, gpin)
            } else if let Some(&(die, spin)) = sm_lookup.get(pad) {
                let exp_func = match spin {
                    SysMonPin::VP => "VP_0",
                    SysMonPin::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::SysMon(die, spin)
            } else if let Some(&(bank, spin)) = ps_lookup.get(pad) {
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
                    PsPin::DdrCkP(0) => format!("PS_DDR_CKP_{bank}"),
                    PsPin::DdrCkN(0) => format!("PS_DDR_CKN_{bank}"),
                    PsPin::DdrCke(0) => format!("PS_DDR_CKE_{bank}"),
                    PsPin::DdrOdt(0) => format!("PS_DDR_ODT_{bank}"),
                    PsPin::DdrDrstB => format!("PS_DDR_DRST_B_{bank}"),
                    PsPin::DdrCsB(0) => format!("PS_DDR_CS_B_{bank}"),
                    PsPin::DdrRasB => format!("PS_DDR_RAS_B_{bank}"),
                    PsPin::DdrCasB => format!("PS_DDR_CAS_B_{bank}"),
                    PsPin::DdrWeB => format!("PS_DDR_WE_B_{bank}"),
                    _ => unreachable!(),
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
                "GNDADC_0" | "GNDADC" => BondPin::SysMon(grid_master, SysMonPin::AVss),
                "VCCADC_0" | "VCCADC" => BondPin::SysMon(grid_master, SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMon(grid_master, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMon(grid_master, SysMonPin::VRefN),
                "MGTAVTT" => BondPin::GtRegion(10, GtRegionPin::AVtt),
                "MGTAVCC" => BondPin::GtRegion(10, GtRegionPin::AVcc),
                "MGTVCCAUX" => BondPin::GtRegion(10, GtRegionPin::VccAux),
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
                            "MGTAVTT_G" => BondPin::GtRegion(b, GtRegionPin::AVtt),
                            "MGTAVCC_G" => BondPin::GtRegion(b, GtRegionPin::AVcc),
                            "MGTVCCAUX_G" => BondPin::GtRegion(b, GtRegionPin::VccAux),
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

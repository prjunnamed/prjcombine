use prjcombine_rawdump::{Part, PkgPin};
use prjcombine_ultrascale::bond::{
    Bond, BondPin, CfgPin, GtPin, GtRegion, GtRegionPin, HbmPin, PsPin, RfAdcPin, RfDacPin,
    SharedCfgPin, SysMonPin,
};
use prjcombine_ultrascale::expanded::{IoCoord, IoDiffKind, IoKind};
use prjcombine_ultrascale::grid::{DisabledPart, Grid, GridKind, IoRowKind};
use prjcombine_ultrascale_naming::ExpandedNamedDevice;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use unnamed_entity::EntityId;

use prjcombine_rdgrid::split_num;

fn lookup_nonpad_pin(rd: &Part, pin: &PkgPin, pgrid: &Grid) -> Option<BondPin> {
    match &pin.func[..] {
        "NC" => return Some(BondPin::Nc),
        "GND" => return Some(BondPin::Gnd),
        "VCCINT" => return Some(BondPin::VccInt),
        "VCCAUX" => return Some(BondPin::VccAux),
        "VCCAUX_HPIO" => return Some(BondPin::VccAuxHpio),
        "VCCAUX_HDIO" => return Some(BondPin::VccAuxHdio),
        "VCCBRAM" => return Some(BondPin::VccBram),
        "VCCINT_IO" => return Some(BondPin::VccIntIo),
        "VCCAUX_IO" => return Some(BondPin::VccAuxIo),
        "VBATT" => return Some(BondPin::VccBatt),
        "D00_MOSI_0" if !pgrid.has_csec => return Some(BondPin::Cfg(CfgPin::Data(0))),
        "D00_MOSI_DOUT_0" if pgrid.has_csec => return Some(BondPin::Cfg(CfgPin::Data(0))),
        "D01_DIN_0" => return Some(BondPin::Cfg(CfgPin::Data(1))),
        "D02_0" if !pgrid.has_csec => return Some(BondPin::Cfg(CfgPin::Data(2))),
        "D02_CS_B_0" if pgrid.has_csec => return Some(BondPin::Cfg(CfgPin::Data(2))),
        "D03_0" if !pgrid.has_csec => return Some(BondPin::Cfg(CfgPin::Data(3))),
        "D03_READY_0" if pgrid.has_csec => return Some(BondPin::Cfg(CfgPin::Data(3))),
        "RDWR_FCS_B_0" => return Some(BondPin::Cfg(CfgPin::RdWrB)),
        "TCK_0" => return Some(BondPin::Cfg(CfgPin::Tck)),
        "TDI_0" => return Some(BondPin::Cfg(CfgPin::Tdi)),
        "TDO_0" => return Some(BondPin::Cfg(CfgPin::Tdo)),
        "TMS_0" => return Some(BondPin::Cfg(CfgPin::Tms)),
        "CCLK_0" => return Some(BondPin::Cfg(CfgPin::Cclk)),
        "PUDC_B_0" | "PUDC_B" => return Some(BondPin::Cfg(CfgPin::HswapEn)),
        "POR_OVERRIDE" => return Some(BondPin::Cfg(CfgPin::PorOverride)),
        "DONE_0" => return Some(BondPin::Cfg(CfgPin::Done)),
        "PROGRAM_B_0" => return Some(BondPin::Cfg(CfgPin::ProgB)),
        "INIT_B_0" => return Some(BondPin::Cfg(CfgPin::InitB)),
        "M0_0" => return Some(BondPin::Cfg(CfgPin::M0)),
        "M1_0" => return Some(BondPin::Cfg(CfgPin::M1)),
        "M2_0" => return Some(BondPin::Cfg(CfgPin::M2)),
        "CFGBVS_0" => return Some(BondPin::Cfg(CfgPin::CfgBvs)),
        "DXN" => return Some(BondPin::Dxn),
        "DXP" => return Some(BondPin::Dxp),
        "GNDADC" => return Some(BondPin::SysMonGnd),
        "VCCADC" => return Some(BondPin::SysMonVcc),
        "VREFP" => return Some(BondPin::SysMonVRefP),
        "VREFN" => return Some(BondPin::SysMonVRefN),
        "GND_PSADC" => return Some(BondPin::PsSysMonGnd),
        "VCC_PSADC" => return Some(BondPin::PsSysMonVcc),
        "GND_SENSE" => return Some(BondPin::GndSense),
        "VCCINT_SENSE" => return Some(BondPin::VccIntSense),
        "VCCO_PSIO0_500" => return Some(BondPin::VccO(500)),
        "VCCO_PSIO1_501" => return Some(BondPin::VccO(501)),
        "VCCO_PSIO2_502" => return Some(BondPin::VccO(502)),
        "VCCO_PSIO3_503" => return Some(BondPin::VccO(503)),
        "VCCO_PSDDR_504" => return Some(BondPin::VccO(504)),
        "VCC_PSAUX" => return Some(BondPin::VccPsAux),
        "VCC_PSINTLP" => return Some(BondPin::VccPsIntLp),
        "VCC_PSINTFP" => return Some(BondPin::VccPsIntFp),
        "VCC_PSINTFP_DDR" => return Some(BondPin::VccPsIntFpDdr),
        "VCC_PSPLL" => return Some(BondPin::VccPsPll),
        "VCC_PSDDR_PLL" => return Some(BondPin::VccPsDdrPll),
        "VCC_PSBATT" => return Some(BondPin::VccPsBatt),
        "VCCINT_VCU" => return Some(BondPin::VccIntVcu),
        "PS_MGTRAVCC" => return Some(BondPin::Gt(505, GtPin::AVcc)),
        "PS_MGTRAVTT" => return Some(BondPin::Gt(505, GtPin::AVtt)),
        "VCCSDFEC" => return Some(BondPin::VccSdfec),
        "VCCINT_AMS" => return Some(BondPin::VccIntAms),
        "DAC_GND" => return Some(BondPin::RfDacGnd),
        "DAC_SUB_GND" => return Some(BondPin::RfDacSubGnd),
        "DAC_AVCC" => return Some(BondPin::RfDacAVcc),
        "DAC_AVCCAUX" => return Some(BondPin::RfDacAVccAux),
        "DAC_AVTT" => return Some(BondPin::RfDacAVtt),
        "ADC_GND" => return Some(BondPin::RfAdcGnd),
        "ADC_SUB_GND" => return Some(BondPin::RfAdcSubGnd),
        "ADC_AVCC" => return Some(BondPin::RfAdcAVcc),
        "ADC_AVCCAUX" => return Some(BondPin::RfAdcAVccAux),
        "RSVD" => {
            if let Some(bank) = pin.vcco_bank {
                return Some(BondPin::Hbm(bank, HbmPin::Rsvd));
            } else {
                // disabled DACs
                if rd.part.contains("zu25dr") {
                    return Some(BondPin::Rsvd);
                }
            }
        }
        "RSVDGND" => {
            if let Some(bank) = pin.vcco_bank {
                if bank == 0 {
                    return Some(BondPin::Cfg(CfgPin::CfgBvs));
                } else {
                    return Some(BondPin::Hbm(bank, HbmPin::RsvdGnd));
                }
            } else {
                for p in [
                    "zu2cg", "zu2eg", "zu3cg", "zu3eg", "zu3tcg", "zu3teg", "zu4cg", "zu4eg",
                    "zu5cg", "zu5eg", "zu7cg", "zu7eg",
                ] {
                    if rd.part.contains(p) {
                        return Some(BondPin::VccIntVcu);
                    }
                }
                // disabled DACs
                if rd.part.contains("zu25dr") {
                    return Some(BondPin::RsvdGnd);
                }
                // disabled GT VCCINT
                if rd.part.contains("ku19p") {
                    return Some(BondPin::RsvdGnd);
                }
            }
        }
        _ => (),
    }
    if let Some(b) = pin.func.strip_prefix("VCCO_") {
        return Some(BondPin::VccO(b.parse().ok()?));
    }
    if let Some(b) = pin.func.strip_prefix("VREF_") {
        return Some(BondPin::IoVref(b.parse().ok()?));
    }
    if let Some(b) = pin.func.strip_prefix("VCC_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::Vcc));
    }
    if let Some(b) = pin.func.strip_prefix("VCCAUX_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::VccAux));
    }
    if let Some(b) = pin.func.strip_prefix("VCC_IO_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::VccIo));
    }
    if let Some(b) = pin.func.strip_prefix("VCM01_") {
        return Some(BondPin::RfAdc(b.parse().ok()?, RfAdcPin::VCm(0)));
    }
    if let Some(b) = pin.func.strip_prefix("VCM23_") {
        return Some(BondPin::RfAdc(b.parse().ok()?, RfAdcPin::VCm(2)));
    }
    if let Some(b) = pin.func.strip_prefix("ADC_REXT_") {
        return Some(BondPin::RfAdc(b.parse().ok()?, RfAdcPin::RExt));
    }
    if let Some(b) = pin.func.strip_prefix("DAC_REXT_") {
        return Some(BondPin::RfDac(b.parse().ok()?, RfDacPin::RExt));
    }
    for (suf, region) in [
        ("", GtRegion::All),
        ("_L", GtRegion::L),
        ("_R", GtRegion::R),
        ("_LS", GtRegion::LS),
        ("_RS", GtRegion::RS),
        ("_LLC", GtRegion::LLC),
        ("_RLC", GtRegion::RLC),
        ("_LC", GtRegion::LC),
        ("_RC", GtRegion::RC),
        ("_LUC", GtRegion::LUC),
        ("_RUC", GtRegion::RUC),
        ("_LN", GtRegion::LN),
        ("_RN", GtRegion::RN),
    ] {
        if let Some(f) = pin.func.strip_suffix(suf) {
            match f {
                "MGTAVTT" => return Some(BondPin::GtRegion(region, GtRegionPin::AVtt)),
                "MGTAVCC" => return Some(BondPin::GtRegion(region, GtRegionPin::AVcc)),
                "MGTVCCAUX" => return Some(BondPin::GtRegion(region, GtRegionPin::VccAux)),
                "MGTRREF" => return Some(BondPin::Gt(pin.vcco_bank.unwrap(), GtPin::RRef)),
                "MGTAVTTRCAL" => return Some(BondPin::Gt(pin.vcco_bank.unwrap(), GtPin::AVttRCal)),
                "VCCINT_GT" => return Some(BondPin::GtRegion(region, GtRegionPin::VccInt)),
                _ => (),
            }
        }
    }
    None
}

pub fn make_bond(rd: &Part, pkg: &str, endev: &ExpandedNamedDevice, pins: &[PkgPin]) -> Bond {
    let pgrid = endev.edev.grids[endev.edev.interposer.primary];
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = endev
        .edev
        .io
        .iter()
        .map(|&io| (endev.get_io_name(io), io))
        .collect();
    let mut gt_common_lookup: HashMap<_, _> = HashMap::new();
    let mut gt_channel_lookup: HashMap<_, _> = HashMap::new();
    for gt in endev.get_gts() {
        gt_common_lookup.insert(gt.name_common, gt.crd);
        for (i, &name) in gt.name_channel.iter().enumerate() {
            gt_channel_lookup.insert(name, (gt.crd, i));
        }
    }
    let is_zynq = endev.edev.grids[endev.edev.interposer.primary].ps.is_some()
        && !endev.edev.disabled.contains(&DisabledPart::Ps);
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&**pad) {
                let io_info = endev.edev.get_io_info(io);
                if pin.vcco_bank.unwrap() != io_info.bank
                    && (pin.vcco_bank != Some(64) || !matches!(io_info.bank, 84 | 94))
                {
                    println!(
                        "wrong bank pad {pkg} {pad} {io:?} got {f} exp {b}",
                        f = pin.func,
                        b = io_info.bank
                    );
                }
                let old = io_banks.insert(io_info.bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                let mut exp_func = "IO".to_string();
                match io {
                    IoCoord::Hdio(crd) => {
                        write!(
                            exp_func,
                            "_L{}{}",
                            1 + crd.iob.to_idx() / 2,
                            ['P', 'N'][crd.iob.to_idx() % 2]
                        )
                        .unwrap();
                    }
                    IoCoord::HdioLc(crd) => {
                        write!(
                            exp_func,
                            "_L{}{}",
                            1 + crd.iob.to_idx() / 2,
                            ['P', 'N'][crd.iob.to_idx() % 2]
                        )
                        .unwrap();
                    }
                    IoCoord::Hpio(crd) => {
                        let group = crd.iob.to_idx() / 13;
                        if crd.iob.to_idx() % 13 != 12 {
                            write!(
                                exp_func,
                                "_L{}{}",
                                1 + group * 6 + crd.iob.to_idx() % 13 / 2,
                                ['P', 'N'][crd.iob.to_idx() % 13 % 2]
                            )
                            .unwrap();
                        }
                        write!(
                            exp_func,
                            "_T{}{}_N{}",
                            group,
                            if crd.iob.to_idx() % 13 < 6 { 'L' } else { 'U' },
                            crd.iob.to_idx() % 13
                        )
                        .unwrap();
                    }
                }
                if io_info.is_gc {
                    if io_info.kind == IoKind::Hdio {
                        exp_func += "_HDGC";
                    } else {
                        exp_func += "_GC";
                    }
                }
                if io_info.is_dbc {
                    exp_func += "_DBC";
                }
                if io_info.is_qbc {
                    exp_func += "_QBC";
                }
                if io_info.is_vrp {
                    exp_func += "_VRP";
                }
                if let Some(sm) = io_info.sm_pair {
                    let pn = match io_info.diff {
                        IoDiffKind::P(_) => 'P',
                        IoDiffKind::N(_) => 'N',
                        _ => unreachable!(),
                    };
                    write!(exp_func, "_AD{sm}{pn}").unwrap();
                }
                match endev.edev.cfg_io[endev.edev.interposer.primary]
                    .get_by_right(&io)
                    .copied()
                {
                    Some(SharedCfgPin::Data(d)) => {
                        if !is_zynq {
                            if d >= 16 && !pgrid.has_csec {
                                write!(exp_func, "_A{:02}", d - 16).unwrap();
                            }
                            write!(exp_func, "_D{d:02}").unwrap();
                            if (4..12).contains(&d) && pgrid.has_csec {
                                write!(exp_func, "_OSPID{:02}", d - 4).unwrap();
                            }
                        }
                    }
                    Some(SharedCfgPin::Addr(a)) => {
                        if !is_zynq {
                            write!(exp_func, "_A{a}").unwrap();
                        }
                    }
                    Some(SharedCfgPin::Rs(a)) => {
                        if !is_zynq {
                            write!(exp_func, "_RS{a}").unwrap();
                        }
                    }
                    Some(SharedCfgPin::EmCclk) => {
                        if !is_zynq {
                            exp_func += "_EMCCLK"
                        }
                    }
                    Some(SharedCfgPin::Dout) => {
                        if !is_zynq {
                            exp_func += "_DOUT_CSO_B"
                        }
                    }
                    Some(SharedCfgPin::FweB) => {
                        if !is_zynq {
                            exp_func += "_FWE_FCS2_B"
                        }
                    }
                    Some(SharedCfgPin::FoeB) => {
                        if !is_zynq {
                            exp_func += "_FOE_B"
                        }
                    }
                    Some(SharedCfgPin::CsiB) => {
                        if !is_zynq {
                            if pgrid.has_csec {
                                exp_func += "_CSI_B"
                            } else {
                                exp_func += "_CSI_ADV_B"
                            }
                        }
                    }
                    Some(SharedCfgPin::Busy) => {
                        if !is_zynq {
                            exp_func += "_BUSY"
                        }
                    }
                    Some(SharedCfgPin::Fcs1B) => {
                        if !is_zynq {
                            exp_func += "_FCS1_B"
                        }
                    }
                    Some(SharedCfgPin::OspiDs) => {
                        if !is_zynq {
                            exp_func += "_OSPI_DS"
                        }
                    }
                    Some(SharedCfgPin::OspiEccFail) => {
                        if !is_zynq {
                            exp_func += "_OSPI_ECC_FAIL"
                        }
                    }
                    Some(SharedCfgPin::OspiRstB) => {
                        if !is_zynq {
                            exp_func += "_OSPI_RST_B"
                        }
                    }
                    Some(SharedCfgPin::PerstN0) => {
                        if pgrid.has_csec {
                            exp_func += "_PERSTN0_B"
                        } else {
                            exp_func += "_PERSTN0"
                        }
                    }
                    Some(SharedCfgPin::PerstN1) => exp_func += "_PERSTN1",
                    Some(SharedCfgPin::SmbAlert) => exp_func += "_SMBALERT",
                    Some(SharedCfgPin::I2cSclk) => exp_func += "_I2C_SCLK",
                    Some(SharedCfgPin::I2cSda) => {
                        exp_func += if endev.edev.kind == GridKind::Ultrascale || pgrid.has_csec {
                            "_I2C_SDA"
                        } else {
                            "_PERSTN1_I2C_SDA"
                        }
                    }
                    None => (),
                    Some(x) => println!("ummm {x:?}?"),
                }
                write!(exp_func, "_{}", io_banks[&io_info.bank]).unwrap();
                if exp_func != pin.func {
                    println!(
                        "pad {pkg} {pad} {io:?} got {f} exp {exp_func}",
                        f = pin.func
                    );
                }
                match io {
                    IoCoord::Hpio(crd) => BondPin::Hpio(io_info.bank, crd.iob),
                    IoCoord::Hdio(crd) => BondPin::Hdio(io_info.bank, crd.iob),
                    IoCoord::HdioLc(crd) => BondPin::HdioLc(io_info.bank, crd.iob),
                }
            } else if let Some(&gt) = gt_common_lookup.get(&**pad) {
                let gt_info = endev.edev.get_gt_info(gt);
                let (f, bank) = pin.func.rsplit_once('_').unwrap();
                let bank: u32 = bank.parse().unwrap();
                if bank != gt_info.bank {
                    println!(
                        "gt pad bank mismatch {pkg} {p} {pad} {f} {gt:?}",
                        f = pin.func,
                        p = rd.part
                    );
                }
                match gt_info.kind {
                    IoRowKind::HsAdc | IoRowKind::RfAdc => match f {
                        "ADC_VIN0_P" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInP(0)),
                        "ADC_VIN0_N" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInN(0)),
                        "ADC_VIN1_P" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInP(1)),
                        "ADC_VIN1_N" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInN(1)),
                        "ADC_VIN2_P" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInP(2)),
                        "ADC_VIN2_N" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInN(2)),
                        "ADC_VIN3_P" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInP(3)),
                        "ADC_VIN3_N" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInN(3)),
                        "ADC_VIN_I01_P" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInPairP(0)),
                        "ADC_VIN_I01_N" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInPairN(0)),
                        "ADC_VIN_I23_P" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInPairP(2)),
                        "ADC_VIN_I23_N" => BondPin::RfAdc(gt_info.bank, RfAdcPin::VInPairN(2)),
                        "ADC_CLK_P" => BondPin::RfAdc(gt_info.bank, RfAdcPin::ClkP),
                        "ADC_CLK_N" => BondPin::RfAdc(gt_info.bank, RfAdcPin::ClkN),
                        "ADC_PLL_TEST_OUT_P" => BondPin::RfAdc(gt_info.bank, RfAdcPin::PllTestOutP),
                        "ADC_PLL_TEST_OUT_N" => BondPin::RfAdc(gt_info.bank, RfAdcPin::PllTestOutN),
                        _ => {
                            println!(
                                "weird hsadc iopad {pkg} {p} {pad} {f} {gt:?}",
                                f = pin.func,
                                p = rd.part
                            );
                            continue;
                        }
                    },
                    IoRowKind::HsDac | IoRowKind::RfDac => match f {
                        "DAC_VOUT0_P" => BondPin::RfDac(gt_info.bank, RfDacPin::VOutP(0)),
                        "DAC_VOUT0_N" => BondPin::RfDac(gt_info.bank, RfDacPin::VOutN(0)),
                        "DAC_VOUT1_P" => BondPin::RfDac(gt_info.bank, RfDacPin::VOutP(1)),
                        "DAC_VOUT1_N" => BondPin::RfDac(gt_info.bank, RfDacPin::VOutN(1)),
                        "DAC_VOUT2_P" => BondPin::RfDac(gt_info.bank, RfDacPin::VOutP(2)),
                        "DAC_VOUT2_N" => BondPin::RfDac(gt_info.bank, RfDacPin::VOutN(2)),
                        "DAC_VOUT3_P" => BondPin::RfDac(gt_info.bank, RfDacPin::VOutP(3)),
                        "DAC_VOUT3_N" => BondPin::RfDac(gt_info.bank, RfDacPin::VOutN(3)),
                        "DAC_CLK_P" => BondPin::RfDac(gt_info.bank, RfDacPin::ClkP),
                        "DAC_CLK_N" => BondPin::RfDac(gt_info.bank, RfDacPin::ClkN),
                        "SYSREF_P" => BondPin::RfDac(gt_info.bank, RfDacPin::SysRefP),
                        "SYSREF_N" => BondPin::RfDac(gt_info.bank, RfDacPin::SysRefN),
                        _ => {
                            println!(
                                "weird hsdac iopad {pkg} {p} {pad} {f} {gt:?}",
                                f = pin.func,
                                p = rd.part
                            );
                            continue;
                        }
                    },
                    IoRowKind::Gtm => match f {
                        "MGTREFCLKP" => BondPin::Gt(gt_info.bank, GtPin::ClkP(0)),
                        "MGTREFCLKN" => BondPin::Gt(gt_info.bank, GtPin::ClkN(0)),
                        _ => {
                            println!(
                                "weird gtm clk iopad {pkg} {p} {pad} {f} {gt:?}",
                                f = pin.func,
                                p = rd.part
                            );
                            continue;
                        }
                    },
                    IoRowKind::Gth | IoRowKind::Gty | IoRowKind::Gtf => match f {
                        "MGTREFCLK0P" => BondPin::Gt(gt_info.bank, GtPin::ClkP(0)),
                        "MGTREFCLK0N" => BondPin::Gt(gt_info.bank, GtPin::ClkN(0)),
                        "MGTREFCLK1P" => BondPin::Gt(gt_info.bank, GtPin::ClkP(1)),
                        "MGTREFCLK1N" => BondPin::Gt(gt_info.bank, GtPin::ClkN(1)),
                        _ => {
                            println!(
                                "weird gt common iopad {pkg} {p} {pad} {f} {gt:?}",
                                f = pin.func,
                                p = rd.part
                            );
                            continue;
                        }
                    },
                    _ => unreachable!(),
                }
            } else if let Some(&(gt, ch)) = gt_channel_lookup.get(&**pad) {
                let gt_info = endev.edev.get_gt_info(gt);
                let (f, bank) = pin.func.rsplit_once('_').unwrap();
                let bank: u32 = bank.parse().unwrap();
                if bank != gt_info.bank {
                    println!(
                        "gt pad bank mismatch {pkg} {p} {pad} {f} {gt:?}",
                        f = pin.func,
                        p = rd.part
                    );
                }
                if gt_info.kind == IoRowKind::Gtm {
                    match f {
                        "MGTMRXP0" => BondPin::Gt(gt_info.bank, GtPin::RxP(0)),
                        "MGTMRXN0" => BondPin::Gt(gt_info.bank, GtPin::RxN(0)),
                        "MGTMTXP0" => BondPin::Gt(gt_info.bank, GtPin::TxP(0)),
                        "MGTMTXN0" => BondPin::Gt(gt_info.bank, GtPin::TxN(0)),
                        "MGTMRXP1" => BondPin::Gt(gt_info.bank, GtPin::RxP(1)),
                        "MGTMRXN1" => BondPin::Gt(gt_info.bank, GtPin::RxN(1)),
                        "MGTMTXP1" => BondPin::Gt(gt_info.bank, GtPin::TxP(1)),
                        "MGTMTXN1" => BondPin::Gt(gt_info.bank, GtPin::TxN(1)),
                        _ => {
                            println!(
                                "weird gtm iopad {pkg} {p} {pad} {f} {gt:?}",
                                f = pin.func,
                                p = rd.part
                            );
                            continue;
                        }
                    }
                } else if let Some(f) = f.strip_suffix(&format!("{ch}")) {
                    match f {
                        "MGTHRXP" | "MGTYRXP" | "MGTFRXP" => {
                            BondPin::Gt(gt_info.bank, GtPin::RxP(ch as u8))
                        }
                        "MGTHRXN" | "MGTYRXN" | "MGTFRXN" => {
                            BondPin::Gt(gt_info.bank, GtPin::RxN(ch as u8))
                        }
                        "MGTHTXP" | "MGTYTXP" | "MGTFTXP" => {
                            BondPin::Gt(gt_info.bank, GtPin::TxP(ch as u8))
                        }
                        "MGTHTXN" | "MGTYTXN" | "MGTFTXN" => {
                            BondPin::Gt(gt_info.bank, GtPin::TxN(ch as u8))
                        }
                        _ => {
                            println!(
                                "weird gt iopad {pkg} {p} {pad} {f} {gt:?}",
                                f = pin.func,
                                p = rd.part
                            );
                            continue;
                        }
                    }
                } else {
                    println!(
                        "weird gt iopad {pkg} {p} {pad} {f} {gt:?}",
                        f = pin.func,
                        p = rd.part
                    );
                    continue;
                }
            } else if pad.starts_with("SYSMON") {
                let exp_site = match endev.edev.kind {
                    GridKind::Ultrascale => {
                        format!("SYSMONE1_X0Y{}", endev.edev.interposer.primary.to_idx())
                    }
                    GridKind::UltrascalePlus => {
                        format!("SYSMONE4_X0Y{}", endev.edev.interposer.primary.to_idx())
                    }
                };
                if exp_site != *pad {
                    println!(
                        "weird sysmon iopad {p} {pad} {f}",
                        f = pin.func,
                        p = rd.part
                    );
                }
                match &pin.func[..] {
                    "VP" => BondPin::SysMon(endev.edev.interposer.primary, SysMonPin::VP),
                    "VN" => BondPin::SysMon(endev.edev.interposer.primary, SysMonPin::VN),
                    _ => {
                        println!(
                            "weird sysmon iopad {p} {pad} {f}",
                            f = pin.func,
                            p = rd.part
                        );
                        continue;
                    }
                }
            } else if pad == "PS8_X0Y0" {
                let pos = pin.func.rfind('_').unwrap();
                let bank: u32 = pin.func[pos + 1..].parse().unwrap();
                if bank == 505 {
                    let gtpin = match &pin.func[..pos] {
                        "PS_MGTRREF" => GtPin::RRef,
                        "PS_MGTREFCLK0P" => GtPin::ClkP(0),
                        "PS_MGTREFCLK0N" => GtPin::ClkN(0),
                        "PS_MGTREFCLK1P" => GtPin::ClkP(1),
                        "PS_MGTREFCLK1N" => GtPin::ClkN(1),
                        "PS_MGTREFCLK2P" => GtPin::ClkP(2),
                        "PS_MGTREFCLK2N" => GtPin::ClkN(2),
                        "PS_MGTREFCLK3P" => GtPin::ClkP(3),
                        "PS_MGTREFCLK3N" => GtPin::ClkN(3),
                        x => {
                            if let Some((n, b)) = split_num(x) {
                                match n {
                                    "PS_MGTRTXP" => GtPin::TxP(b as u8),
                                    "PS_MGTRTXN" => GtPin::TxN(b as u8),
                                    "PS_MGTRRXP" => GtPin::RxP(b as u8),
                                    "PS_MGTRRXN" => GtPin::RxN(b as u8),
                                    _ => {
                                        println!(
                                            "weird ps8 iopad {p} {pad} {f}",
                                            f = pin.func,
                                            p = rd.part
                                        );
                                        continue;
                                    }
                                }
                            } else {
                                println!(
                                    "weird ps8 iopad {p} {pad} {f}",
                                    f = pin.func,
                                    p = rd.part
                                );
                                continue;
                            }
                        }
                    };
                    BondPin::Gt(bank, gtpin)
                } else {
                    let pspin = match &pin.func[..pos] {
                        "PS_DONE" => PsPin::Done,
                        "PS_PROG_B" => PsPin::ProgB,
                        "PS_INIT_B" => PsPin::InitB,
                        "PS_ERROR_OUT" => PsPin::ErrorOut,
                        "PS_ERROR_STATUS" => PsPin::ErrorStatus,
                        "PS_PADI" => PsPin::PadI,
                        "PS_PADO" => PsPin::PadO,
                        "PS_POR_B" => PsPin::PorB,
                        "PS_SRST_B" => PsPin::SrstB,
                        "PS_REF_CLK" => PsPin::Clk,
                        "PS_JTAG_TDO" => PsPin::JtagTdo,
                        "PS_JTAG_TDI" => PsPin::JtagTdi,
                        "PS_JTAG_TCK" => PsPin::JtagTck,
                        "PS_JTAG_TMS" => PsPin::JtagTms,
                        "PS_DDR_ACT_N" => PsPin::DdrActN,
                        "PS_DDR_ALERT_N" => PsPin::DdrAlertN,
                        "PS_DDR_PARITY" => PsPin::DdrParity,
                        "PS_DDR_RAM_RST_N" => PsPin::DdrDrstB,
                        "PS_DDR_ZQ" => PsPin::DdrZq,
                        x => {
                            if let Some((n, b)) = split_num(x) {
                                match n {
                                    "PS_MIO" => PsPin::Mio(b),
                                    "PS_MODE" => PsPin::Mode(b),
                                    "PS_DDR_DQ" => PsPin::DdrDq(b),
                                    "PS_DDR_DM" => PsPin::DdrDm(b),
                                    "PS_DDR_DQS_P" => PsPin::DdrDqsP(b),
                                    "PS_DDR_DQS_N" => PsPin::DdrDqsN(b),
                                    "PS_DDR_A" => PsPin::DdrA(b),
                                    "PS_DDR_BA" => PsPin::DdrBa(b),
                                    "PS_DDR_BG" => PsPin::DdrBg(b),
                                    "PS_DDR_CKE" => PsPin::DdrCke(b),
                                    "PS_DDR_ODT" => PsPin::DdrOdt(b),
                                    "PS_DDR_CS_N" => PsPin::DdrCsB(b),
                                    "PS_DDR_CK" => PsPin::DdrCkP(b),
                                    "PS_DDR_CK_N" => PsPin::DdrCkN(b),
                                    _ => {
                                        println!(
                                            "weird ps8 iopad {p} {pad} {f}",
                                            f = pin.func,
                                            p = rd.part
                                        );
                                        continue;
                                    }
                                }
                            } else {
                                println!(
                                    "weird ps8 iopad {p} {pad} {f}",
                                    f = pin.func,
                                    p = rd.part
                                );
                                continue;
                            }
                        }
                    };
                    BondPin::IoPs(bank, pspin)
                }
            } else {
                println!("unk iopad {pad} {f}", f = pin.func);
                continue;
            }
        } else if let Some(p) = lookup_nonpad_pin(rd, pin, pgrid) {
            p
        } else {
            println!("UNK FUNC {} {} {:?}", pkg, pin.func, pin);
            continue;
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond { pins: bond_pins }
}

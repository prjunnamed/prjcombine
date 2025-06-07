use prjcombine_re_xilinx_naming_ultrascale::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::{Part, PkgPin};
use prjcombine_ultrascale::bond::{
    Bond, BondPad, CfgPad, GtPad, GtRegion, GtRegionPad, HbmPad, PsPad, RfAdcPad, RfDacPad,
    SharedCfgPad, SysMonPad,
};
use prjcombine_ultrascale::chip::{Chip, ChipKind, DisabledPart, IoRowKind};
use prjcombine_ultrascale::expanded::{IoCoord, IoDiffKind, IoKind};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use unnamed_entity::EntityId;

use prjcombine_re_xilinx_rd2db_grid::split_num;

fn lookup_nonpad_pin(rd: &Part, pin: &PkgPin, pgrid: &Chip) -> Option<BondPad> {
    match &pin.func[..] {
        "NC" => return Some(BondPad::Nc),
        "GND" => return Some(BondPad::Gnd),
        "VCCINT" => return Some(BondPad::VccInt),
        "VCCAUX" => return Some(BondPad::VccAux),
        "VCCAUX_HPIO" => return Some(BondPad::VccAuxHpio),
        "VCCAUX_HDIO" => return Some(BondPad::VccAuxHdio),
        "VCCBRAM" => return Some(BondPad::VccBram),
        "VCCINT_IO" => return Some(BondPad::VccIntIo),
        "VCCAUX_IO" => return Some(BondPad::VccAuxIo),
        "VBATT" => return Some(BondPad::VccBatt),
        "D00_MOSI_0" if !pgrid.has_csec => return Some(BondPad::Cfg(CfgPad::Data(0))),
        "D00_MOSI_DOUT_0" if pgrid.has_csec => return Some(BondPad::Cfg(CfgPad::Data(0))),
        "D01_DIN_0" => return Some(BondPad::Cfg(CfgPad::Data(1))),
        "D02_0" if !pgrid.has_csec => return Some(BondPad::Cfg(CfgPad::Data(2))),
        "D02_CS_B_0" if pgrid.has_csec => return Some(BondPad::Cfg(CfgPad::Data(2))),
        "D03_0" if !pgrid.has_csec => return Some(BondPad::Cfg(CfgPad::Data(3))),
        "D03_READY_0" if pgrid.has_csec => return Some(BondPad::Cfg(CfgPad::Data(3))),
        "RDWR_FCS_B_0" => return Some(BondPad::Cfg(CfgPad::RdWrB)),
        "TCK_0" => return Some(BondPad::Cfg(CfgPad::Tck)),
        "TDI_0" => return Some(BondPad::Cfg(CfgPad::Tdi)),
        "TDO_0" => return Some(BondPad::Cfg(CfgPad::Tdo)),
        "TMS_0" => return Some(BondPad::Cfg(CfgPad::Tms)),
        "CCLK_0" => return Some(BondPad::Cfg(CfgPad::Cclk)),
        "PUDC_B_0" | "PUDC_B" => return Some(BondPad::Cfg(CfgPad::HswapEn)),
        "POR_OVERRIDE" => return Some(BondPad::Cfg(CfgPad::PorOverride)),
        "DONE_0" => return Some(BondPad::Cfg(CfgPad::Done)),
        "PROGRAM_B_0" => return Some(BondPad::Cfg(CfgPad::ProgB)),
        "INIT_B_0" => return Some(BondPad::Cfg(CfgPad::InitB)),
        "M0_0" => return Some(BondPad::Cfg(CfgPad::M0)),
        "M1_0" => return Some(BondPad::Cfg(CfgPad::M1)),
        "M2_0" => return Some(BondPad::Cfg(CfgPad::M2)),
        "CFGBVS_0" => return Some(BondPad::Cfg(CfgPad::CfgBvs)),
        "DXN" => return Some(BondPad::Dxn),
        "DXP" => return Some(BondPad::Dxp),
        "GNDADC" => return Some(BondPad::SysMonGnd),
        "VCCADC" => return Some(BondPad::SysMonVcc),
        "VREFP" => return Some(BondPad::SysMonVRefP),
        "VREFN" => return Some(BondPad::SysMonVRefN),
        "GND_PSADC" => return Some(BondPad::PsSysMonGnd),
        "VCC_PSADC" => return Some(BondPad::PsSysMonVcc),
        "GND_SENSE" => return Some(BondPad::GndSense),
        "VCCINT_SENSE" => return Some(BondPad::VccIntSense),
        "VCCO_PSIO0_500" => return Some(BondPad::VccO(500)),
        "VCCO_PSIO1_501" => return Some(BondPad::VccO(501)),
        "VCCO_PSIO2_502" => return Some(BondPad::VccO(502)),
        "VCCO_PSIO3_503" => return Some(BondPad::VccO(503)),
        "VCCO_PSDDR_504" => return Some(BondPad::VccO(504)),
        "VCC_PSAUX" => return Some(BondPad::VccPsAux),
        "VCC_PSINTLP" => return Some(BondPad::VccPsIntLp),
        "VCC_PSINTFP" => return Some(BondPad::VccPsIntFp),
        "VCC_PSINTFP_DDR" => return Some(BondPad::VccPsIntFpDdr),
        "VCC_PSPLL" => return Some(BondPad::VccPsPll),
        "VCC_PSDDR_PLL" => return Some(BondPad::VccPsDdrPll),
        "VCC_PSBATT" => return Some(BondPad::VccPsBatt),
        "VCCINT_VCU" => return Some(BondPad::VccIntVcu),
        "PS_MGTRAVCC" => return Some(BondPad::Gt(505, GtPad::AVcc)),
        "PS_MGTRAVTT" => return Some(BondPad::Gt(505, GtPad::AVtt)),
        "VCCSDFEC" => return Some(BondPad::VccSdfec),
        "VCCINT_AMS" => return Some(BondPad::VccIntAms),
        "DAC_GND" => return Some(BondPad::RfDacGnd),
        "DAC_SUB_GND" => return Some(BondPad::RfDacSubGnd),
        "DAC_AVCC" => return Some(BondPad::RfDacAVcc),
        "DAC_AVCCAUX" => return Some(BondPad::RfDacAVccAux),
        "DAC_AVTT" => return Some(BondPad::RfDacAVtt),
        "ADC_GND" => return Some(BondPad::RfAdcGnd),
        "ADC_SUB_GND" => return Some(BondPad::RfAdcSubGnd),
        "ADC_AVCC" => return Some(BondPad::RfAdcAVcc),
        "ADC_AVCCAUX" => return Some(BondPad::RfAdcAVccAux),
        "RSVD" => {
            if let Some(bank) = pin.vcco_bank {
                return Some(BondPad::Hbm(bank, HbmPad::Rsvd));
            } else {
                // disabled DACs
                if rd.part.contains("zu25dr") {
                    return Some(BondPad::Rsvd);
                }
            }
        }
        "RSVDGND" => {
            if let Some(bank) = pin.vcco_bank {
                if bank == 0 {
                    return Some(BondPad::Cfg(CfgPad::CfgBvs));
                } else {
                    return Some(BondPad::Hbm(bank, HbmPad::RsvdGnd));
                }
            } else {
                for p in [
                    "zu2cg", "zu2eg", "zu3cg", "zu3eg", "zu3tcg", "zu3teg", "zu4cg", "zu4eg",
                    "zu5cg", "zu5eg", "zu7cg", "zu7eg",
                ] {
                    if rd.part.contains(p) {
                        return Some(BondPad::VccIntVcu);
                    }
                }
                // disabled DACs
                if rd.part.contains("zu25dr") {
                    return Some(BondPad::RsvdGnd);
                }
                // disabled GT VCCINT
                if rd.part.contains("ku19p") {
                    return Some(BondPad::RsvdGnd);
                }
            }
        }
        _ => (),
    }
    if let Some(b) = pin.func.strip_prefix("VCCO_") {
        return Some(BondPad::VccO(b.parse().ok()?));
    }
    if let Some(b) = pin.func.strip_prefix("VREF_") {
        return Some(BondPad::IoVref(b.parse().ok()?));
    }
    if let Some(b) = pin.func.strip_prefix("VCC_HBM_") {
        return Some(BondPad::Hbm(b.parse().ok()?, HbmPad::Vcc));
    }
    if let Some(b) = pin.func.strip_prefix("VCCAUX_HBM_") {
        return Some(BondPad::Hbm(b.parse().ok()?, HbmPad::VccAux));
    }
    if let Some(b) = pin.func.strip_prefix("VCC_IO_HBM_") {
        return Some(BondPad::Hbm(b.parse().ok()?, HbmPad::VccIo));
    }
    if let Some(b) = pin.func.strip_prefix("VCM01_") {
        return Some(BondPad::RfAdc(b.parse().ok()?, RfAdcPad::VCm(0)));
    }
    if let Some(b) = pin.func.strip_prefix("VCM23_") {
        return Some(BondPad::RfAdc(b.parse().ok()?, RfAdcPad::VCm(2)));
    }
    if let Some(b) = pin.func.strip_prefix("ADC_REXT_") {
        return Some(BondPad::RfAdc(b.parse().ok()?, RfAdcPad::RExt));
    }
    if let Some(b) = pin.func.strip_prefix("DAC_REXT_") {
        return Some(BondPad::RfDac(b.parse().ok()?, RfDacPad::RExt));
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
                "MGTAVTT" => return Some(BondPad::GtRegion(region, GtRegionPad::AVtt)),
                "MGTAVCC" => return Some(BondPad::GtRegion(region, GtRegionPad::AVcc)),
                "MGTVCCAUX" => return Some(BondPad::GtRegion(region, GtRegionPad::VccAux)),
                "MGTRREF" => return Some(BondPad::Gt(pin.vcco_bank.unwrap(), GtPad::RRef)),
                "MGTAVTTRCAL" => return Some(BondPad::Gt(pin.vcco_bank.unwrap(), GtPad::AVttRCal)),
                "VCCINT_GT" => return Some(BondPad::GtRegion(region, GtRegionPad::VccInt)),
                _ => (),
            }
        }
    }
    None
}

pub fn make_bond(rd: &Part, pkg: &str, endev: &ExpandedNamedDevice, pins: &[PkgPin]) -> Bond {
    let pgrid = endev.edev.chips[endev.edev.interposer.primary];
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
    let is_zynq = endev.edev.chips[endev.edev.interposer.primary].ps.is_some()
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
                    Some(SharedCfgPad::Data(d)) => {
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
                    Some(SharedCfgPad::Addr(a)) => {
                        if !is_zynq {
                            write!(exp_func, "_A{a}").unwrap();
                        }
                    }
                    Some(SharedCfgPad::Rs(a)) => {
                        if !is_zynq {
                            write!(exp_func, "_RS{a}").unwrap();
                        }
                    }
                    Some(SharedCfgPad::EmCclk) => {
                        if !is_zynq {
                            exp_func += "_EMCCLK"
                        }
                    }
                    Some(SharedCfgPad::Dout) => {
                        if !is_zynq {
                            exp_func += "_DOUT_CSO_B"
                        }
                    }
                    Some(SharedCfgPad::FweB) => {
                        if !is_zynq {
                            exp_func += "_FWE_FCS2_B"
                        }
                    }
                    Some(SharedCfgPad::FoeB) => {
                        if !is_zynq {
                            exp_func += "_FOE_B"
                        }
                    }
                    Some(SharedCfgPad::CsiB) => {
                        if !is_zynq {
                            if pgrid.has_csec {
                                exp_func += "_CSI_B"
                            } else {
                                exp_func += "_CSI_ADV_B"
                            }
                        }
                    }
                    Some(SharedCfgPad::Busy) => {
                        if !is_zynq {
                            exp_func += "_BUSY"
                        }
                    }
                    Some(SharedCfgPad::Fcs1B) => {
                        if !is_zynq {
                            exp_func += "_FCS1_B"
                        }
                    }
                    Some(SharedCfgPad::OspiDs) => {
                        if !is_zynq {
                            exp_func += "_OSPI_DS"
                        }
                    }
                    Some(SharedCfgPad::OspiEccFail) => {
                        if !is_zynq {
                            exp_func += "_OSPI_ECC_FAIL"
                        }
                    }
                    Some(SharedCfgPad::OspiRstB) => {
                        if !is_zynq {
                            exp_func += "_OSPI_RST_B"
                        }
                    }
                    Some(SharedCfgPad::PerstN0) => {
                        if pgrid.has_csec {
                            exp_func += "_PERSTN0_B"
                        } else {
                            exp_func += "_PERSTN0"
                        }
                    }
                    Some(SharedCfgPad::PerstN1) => exp_func += "_PERSTN1",
                    Some(SharedCfgPad::SmbAlert) => exp_func += "_SMBALERT",
                    Some(SharedCfgPad::I2cSclk) => exp_func += "_I2C_SCLK",
                    Some(SharedCfgPad::I2cSda) => {
                        exp_func += if endev.edev.kind == ChipKind::Ultrascale || pgrid.has_csec {
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
                    IoCoord::Hpio(crd) => BondPad::Hpio(io_info.bank, crd.iob),
                    IoCoord::Hdio(crd) => BondPad::Hdio(io_info.bank, crd.iob),
                    IoCoord::HdioLc(crd) => BondPad::HdioLc(io_info.bank, crd.iob),
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
                        "ADC_VIN0_P" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInP(0)),
                        "ADC_VIN0_N" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInN(0)),
                        "ADC_VIN1_P" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInP(1)),
                        "ADC_VIN1_N" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInN(1)),
                        "ADC_VIN2_P" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInP(2)),
                        "ADC_VIN2_N" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInN(2)),
                        "ADC_VIN3_P" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInP(3)),
                        "ADC_VIN3_N" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInN(3)),
                        "ADC_VIN_I01_P" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInPairP(0)),
                        "ADC_VIN_I01_N" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInPairN(0)),
                        "ADC_VIN_I23_P" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInPairP(2)),
                        "ADC_VIN_I23_N" => BondPad::RfAdc(gt_info.bank, RfAdcPad::VInPairN(2)),
                        "ADC_CLK_P" => BondPad::RfAdc(gt_info.bank, RfAdcPad::ClkP),
                        "ADC_CLK_N" => BondPad::RfAdc(gt_info.bank, RfAdcPad::ClkN),
                        "ADC_PLL_TEST_OUT_P" => BondPad::RfAdc(gt_info.bank, RfAdcPad::PllTestOutP),
                        "ADC_PLL_TEST_OUT_N" => BondPad::RfAdc(gt_info.bank, RfAdcPad::PllTestOutN),
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
                        "DAC_VOUT0_P" => BondPad::RfDac(gt_info.bank, RfDacPad::VOutP(0)),
                        "DAC_VOUT0_N" => BondPad::RfDac(gt_info.bank, RfDacPad::VOutN(0)),
                        "DAC_VOUT1_P" => BondPad::RfDac(gt_info.bank, RfDacPad::VOutP(1)),
                        "DAC_VOUT1_N" => BondPad::RfDac(gt_info.bank, RfDacPad::VOutN(1)),
                        "DAC_VOUT2_P" => BondPad::RfDac(gt_info.bank, RfDacPad::VOutP(2)),
                        "DAC_VOUT2_N" => BondPad::RfDac(gt_info.bank, RfDacPad::VOutN(2)),
                        "DAC_VOUT3_P" => BondPad::RfDac(gt_info.bank, RfDacPad::VOutP(3)),
                        "DAC_VOUT3_N" => BondPad::RfDac(gt_info.bank, RfDacPad::VOutN(3)),
                        "DAC_CLK_P" => BondPad::RfDac(gt_info.bank, RfDacPad::ClkP),
                        "DAC_CLK_N" => BondPad::RfDac(gt_info.bank, RfDacPad::ClkN),
                        "SYSREF_P" => BondPad::RfDac(gt_info.bank, RfDacPad::SysRefP),
                        "SYSREF_N" => BondPad::RfDac(gt_info.bank, RfDacPad::SysRefN),
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
                        "MGTREFCLKP" => BondPad::Gt(gt_info.bank, GtPad::ClkP(0)),
                        "MGTREFCLKN" => BondPad::Gt(gt_info.bank, GtPad::ClkN(0)),
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
                        "MGTREFCLK0P" => BondPad::Gt(gt_info.bank, GtPad::ClkP(0)),
                        "MGTREFCLK0N" => BondPad::Gt(gt_info.bank, GtPad::ClkN(0)),
                        "MGTREFCLK1P" => BondPad::Gt(gt_info.bank, GtPad::ClkP(1)),
                        "MGTREFCLK1N" => BondPad::Gt(gt_info.bank, GtPad::ClkN(1)),
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
                        "MGTMRXP0" => BondPad::Gt(gt_info.bank, GtPad::RxP(0)),
                        "MGTMRXN0" => BondPad::Gt(gt_info.bank, GtPad::RxN(0)),
                        "MGTMTXP0" => BondPad::Gt(gt_info.bank, GtPad::TxP(0)),
                        "MGTMTXN0" => BondPad::Gt(gt_info.bank, GtPad::TxN(0)),
                        "MGTMRXP1" => BondPad::Gt(gt_info.bank, GtPad::RxP(1)),
                        "MGTMRXN1" => BondPad::Gt(gt_info.bank, GtPad::RxN(1)),
                        "MGTMTXP1" => BondPad::Gt(gt_info.bank, GtPad::TxP(1)),
                        "MGTMTXN1" => BondPad::Gt(gt_info.bank, GtPad::TxN(1)),
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
                            BondPad::Gt(gt_info.bank, GtPad::RxP(ch as u8))
                        }
                        "MGTHRXN" | "MGTYRXN" | "MGTFRXN" => {
                            BondPad::Gt(gt_info.bank, GtPad::RxN(ch as u8))
                        }
                        "MGTHTXP" | "MGTYTXP" | "MGTFTXP" => {
                            BondPad::Gt(gt_info.bank, GtPad::TxP(ch as u8))
                        }
                        "MGTHTXN" | "MGTYTXN" | "MGTFTXN" => {
                            BondPad::Gt(gt_info.bank, GtPad::TxN(ch as u8))
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
                    ChipKind::Ultrascale => {
                        format!("SYSMONE1_X0Y{}", endev.edev.interposer.primary.to_idx())
                    }
                    ChipKind::UltrascalePlus => {
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
                    "VP" => BondPad::SysMon(endev.edev.interposer.primary, SysMonPad::VP),
                    "VN" => BondPad::SysMon(endev.edev.interposer.primary, SysMonPad::VN),
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
                        "PS_MGTRREF" => GtPad::RRef,
                        "PS_MGTREFCLK0P" => GtPad::ClkP(0),
                        "PS_MGTREFCLK0N" => GtPad::ClkN(0),
                        "PS_MGTREFCLK1P" => GtPad::ClkP(1),
                        "PS_MGTREFCLK1N" => GtPad::ClkN(1),
                        "PS_MGTREFCLK2P" => GtPad::ClkP(2),
                        "PS_MGTREFCLK2N" => GtPad::ClkN(2),
                        "PS_MGTREFCLK3P" => GtPad::ClkP(3),
                        "PS_MGTREFCLK3N" => GtPad::ClkN(3),
                        x => {
                            if let Some((n, b)) = split_num(x) {
                                match n {
                                    "PS_MGTRTXP" => GtPad::TxP(b as u8),
                                    "PS_MGTRTXN" => GtPad::TxN(b as u8),
                                    "PS_MGTRRXP" => GtPad::RxP(b as u8),
                                    "PS_MGTRRXN" => GtPad::RxN(b as u8),
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
                    BondPad::Gt(bank, gtpin)
                } else {
                    let pspin = match &pin.func[..pos] {
                        "PS_DONE" => PsPad::Done,
                        "PS_PROG_B" => PsPad::ProgB,
                        "PS_INIT_B" => PsPad::InitB,
                        "PS_ERROR_OUT" => PsPad::ErrorOut,
                        "PS_ERROR_STATUS" => PsPad::ErrorStatus,
                        "PS_PADI" => PsPad::PadI,
                        "PS_PADO" => PsPad::PadO,
                        "PS_POR_B" => PsPad::PorB,
                        "PS_SRST_B" => PsPad::SrstB,
                        "PS_REF_CLK" => PsPad::Clk,
                        "PS_JTAG_TDO" => PsPad::JtagTdo,
                        "PS_JTAG_TDI" => PsPad::JtagTdi,
                        "PS_JTAG_TCK" => PsPad::JtagTck,
                        "PS_JTAG_TMS" => PsPad::JtagTms,
                        "PS_DDR_ACT_N" => PsPad::DdrActN,
                        "PS_DDR_ALERT_N" => PsPad::DdrAlertN,
                        "PS_DDR_PARITY" => PsPad::DdrParity,
                        "PS_DDR_RAM_RST_N" => PsPad::DdrDrstB,
                        "PS_DDR_ZQ" => PsPad::DdrZq,
                        x => {
                            if let Some((n, b)) = split_num(x) {
                                match n {
                                    "PS_MIO" => PsPad::Mio(b),
                                    "PS_MODE" => PsPad::Mode(b),
                                    "PS_DDR_DQ" => PsPad::DdrDq(b),
                                    "PS_DDR_DM" => PsPad::DdrDm(b),
                                    "PS_DDR_DQS_P" => PsPad::DdrDqsP(b),
                                    "PS_DDR_DQS_N" => PsPad::DdrDqsN(b),
                                    "PS_DDR_A" => PsPad::DdrA(b),
                                    "PS_DDR_BA" => PsPad::DdrBa(b),
                                    "PS_DDR_BG" => PsPad::DdrBg(b),
                                    "PS_DDR_CKE" => PsPad::DdrCke(b),
                                    "PS_DDR_ODT" => PsPad::DdrOdt(b),
                                    "PS_DDR_CS_N" => PsPad::DdrCsB(b),
                                    "PS_DDR_CK" => PsPad::DdrCkP(b),
                                    "PS_DDR_CK_N" => PsPad::DdrCkN(b),
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
                    BondPad::IoPs(bank, pspin)
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

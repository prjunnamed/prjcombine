use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::DieId;
use prjcombine_rawdump::{Part, PkgPin};
use prjcombine_ultrascale::io::{get_gt, get_io, Gt};
use prjcombine_ultrascale::{
    Bond, BondPin, CfgPin, DisabledPart, Grid, GridKind, GtPin, GtRegion, GtRegionPin, HbmPin,
    IoKind, IoRowKind, PsPin, RfAdcPin, RfDacPin, SharedCfgPin, SysMonPin,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;

use prjcombine_rdgrid::split_num;

fn lookup_nonpad_pin(rd: &Part, pin: &PkgPin) -> Option<BondPin> {
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
        "D00_MOSI_0" => return Some(BondPin::Cfg(CfgPin::Data(0))),
        "D01_DIN_0" => return Some(BondPin::Cfg(CfgPin::Data(1))),
        "D02_0" => return Some(BondPin::Cfg(CfgPin::Data(2))),
        "D03_0" => return Some(BondPin::Cfg(CfgPin::Data(3))),
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
                    "zu2cg", "zu2eg", "zu3cg", "zu3eg", "zu4cg", "zu4eg", "zu5cg", "zu5eg",
                    "zu7cg", "zu7eg",
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

fn lookup_gt_pin(
    gt_lookup: &HashMap<(IoRowKind, u32, u32), Gt>,
    pad: &str,
    func: &str,
) -> Option<BondPin> {
    if let Some(p) = pad.strip_prefix("HSADC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py + 1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::HsAdc, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "ADC_VIN0_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInP(0))),
            "ADC_VIN0_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInN(0))),
            "ADC_VIN1_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInP(1))),
            "ADC_VIN1_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInN(1))),
            "ADC_VIN2_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInP(2))),
            "ADC_VIN2_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInN(2))),
            "ADC_VIN3_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInP(3))),
            "ADC_VIN3_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInN(3))),
            "ADC_VIN_I01_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInPairP(0))),
            "ADC_VIN_I01_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInPairN(0))),
            "ADC_VIN_I23_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInPairP(2))),
            "ADC_VIN_I23_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInPairN(2))),
            "ADC_CLK_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::ClkP)),
            "ADC_CLK_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::ClkN)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("RFADC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py + 1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::RfAdc, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "ADC_VIN0_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInP(0))),
            "ADC_VIN0_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInN(0))),
            "ADC_VIN1_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInP(1))),
            "ADC_VIN1_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInN(1))),
            "ADC_VIN2_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInP(2))),
            "ADC_VIN2_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInN(2))),
            "ADC_VIN3_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInP(3))),
            "ADC_VIN3_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInN(3))),
            "ADC_VIN_I01_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInPairP(0))),
            "ADC_VIN_I01_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInPairN(0))),
            "ADC_VIN_I23_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInPairP(2))),
            "ADC_VIN_I23_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::VInPairN(2))),
            "ADC_CLK_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::ClkP)),
            "ADC_CLK_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::ClkN)),
            "ADC_PLL_TEST_OUT_P" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::PllTestOutP)),
            "ADC_PLL_TEST_OUT_N" => Some(BondPin::RfAdc(gt.bank, RfAdcPin::PllTestOutN)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("HSDAC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py + 1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::HsDac, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "DAC_VOUT0_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutP(0))),
            "DAC_VOUT0_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutN(0))),
            "DAC_VOUT1_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutP(1))),
            "DAC_VOUT1_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutN(1))),
            "DAC_VOUT2_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutP(2))),
            "DAC_VOUT2_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutN(2))),
            "DAC_VOUT3_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutP(3))),
            "DAC_VOUT3_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutN(3))),
            "DAC_CLK_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::ClkP)),
            "DAC_CLK_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::ClkN)),
            "SYSREF_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::SysRefP)),
            "SYSREF_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::SysRefN)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("RFDAC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py + 1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::RfDac, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "DAC_VOUT0_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutP(0))),
            "DAC_VOUT0_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutN(0))),
            "DAC_VOUT1_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutP(1))),
            "DAC_VOUT1_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutN(1))),
            "DAC_VOUT2_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutP(2))),
            "DAC_VOUT2_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutN(2))),
            "DAC_VOUT3_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutP(3))),
            "DAC_VOUT3_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::VOutN(3))),
            "DAC_CLK_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::ClkP)),
            "DAC_CLK_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::ClkN)),
            "SYSREF_P" => Some(BondPin::RfDac(gt.bank, RfDacPin::SysRefP)),
            "SYSREF_N" => Some(BondPin::RfDac(gt.bank, RfDacPin::SysRefN)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("GTM_DUAL_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py + 1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::Gtm, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "MGTMRXP0" => Some(BondPin::Gt(gt.bank, GtPin::RxP(0))),
            "MGTMRXN0" => Some(BondPin::Gt(gt.bank, GtPin::RxN(0))),
            "MGTMTXP0" => Some(BondPin::Gt(gt.bank, GtPin::TxP(0))),
            "MGTMTXN0" => Some(BondPin::Gt(gt.bank, GtPin::TxN(0))),
            "MGTMRXP1" => Some(BondPin::Gt(gt.bank, GtPin::RxP(1))),
            "MGTMRXN1" => Some(BondPin::Gt(gt.bank, GtPin::RxN(1))),
            "MGTMTXP1" => Some(BondPin::Gt(gt.bank, GtPin::TxP(1))),
            "MGTMTXN1" => Some(BondPin::Gt(gt.bank, GtPin::TxN(1))),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("GTM_REFCLK_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py + 1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::Gtm, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "MGTREFCLKP" => Some(BondPin::Gt(gt.bank, GtPin::ClkP(0))),
            "MGTREFCLKN" => Some(BondPin::Gt(gt.bank, GtPin::ClkN(0))),
            _ => None,
        }
    } else {
        let p;
        let kind;
        if let Some(x) = pad.strip_prefix("GTHE3_") {
            p = x;
            kind = IoRowKind::Gth;
        } else if let Some(x) = pad.strip_prefix("GTHE4_") {
            p = x;
            kind = IoRowKind::Gth;
        } else if let Some(x) = pad.strip_prefix("GTYE3_") {
            p = x;
            kind = IoRowKind::Gty;
        } else if let Some(x) = pad.strip_prefix("GTYE4_") {
            p = x;
            kind = IoRowKind::Gty;
        } else if let Some(x) = pad.strip_prefix("GTF_") {
            p = x;
            kind = IoRowKind::Gtf;
        } else {
            return None;
        }
        if let Some(p) = p.strip_prefix("COMMON_X") {
            let py = p.find('Y')?;
            let gx: u32 = p[..py].parse().ok()?;
            let gy: u32 = p[py + 1..].parse().ok()?;
            let gt = gt_lookup.get(&(kind, gx, gy))?;
            let suf = format!("_{}", gt.bank);
            let f = func.strip_suffix(&suf)?;
            match f {
                "MGTREFCLK0P" => Some(BondPin::Gt(gt.bank, GtPin::ClkP(0))),
                "MGTREFCLK0N" => Some(BondPin::Gt(gt.bank, GtPin::ClkN(0))),
                "MGTREFCLK1P" => Some(BondPin::Gt(gt.bank, GtPin::ClkP(1))),
                "MGTREFCLK1N" => Some(BondPin::Gt(gt.bank, GtPin::ClkN(1))),
                _ => None,
            }
        } else if let Some(p) = p.strip_prefix("CHANNEL_X") {
            let py = p.find('Y')?;
            let gx: u32 = p[..py].parse().ok()?;
            let y: u32 = p[py + 1..].parse().ok()?;
            let bel = (y % 4) as u8;
            let gy = y / 4;
            let gt = gt_lookup.get(&(kind, gx, gy))?;
            let suf = format!("{}_{}", bel, gt.bank);
            let f = func.strip_suffix(&suf)?;
            match f {
                "MGTHRXP" => Some(BondPin::Gt(gt.bank, GtPin::RxP(bel))),
                "MGTHRXN" => Some(BondPin::Gt(gt.bank, GtPin::RxN(bel))),
                "MGTHTXP" => Some(BondPin::Gt(gt.bank, GtPin::TxP(bel))),
                "MGTHTXN" => Some(BondPin::Gt(gt.bank, GtPin::TxN(bel))),
                "MGTYRXP" => Some(BondPin::Gt(gt.bank, GtPin::RxP(bel))),
                "MGTYRXN" => Some(BondPin::Gt(gt.bank, GtPin::RxN(bel))),
                "MGTYTXP" => Some(BondPin::Gt(gt.bank, GtPin::TxP(bel))),
                "MGTYTXN" => Some(BondPin::Gt(gt.bank, GtPin::TxN(bel))),
                "MGTFRXP" => Some(BondPin::Gt(gt.bank, GtPin::RxP(bel))),
                "MGTFRXN" => Some(BondPin::Gt(gt.bank, GtPin::RxN(bel))),
                "MGTFTXP" => Some(BondPin::Gt(gt.bank, GtPin::TxP(bel))),
                "MGTFTXN" => Some(BondPin::Gt(gt.bank, GtPin::TxN(bel))),
                _ => None,
            }
        } else {
            None
        }
    }
}

pub fn make_bond(
    rd: &Part,
    pkg: &str,
    grids: &EntityVec<DieId, Grid>,
    grid_master: DieId,
    disabled: &BTreeSet<DisabledPart>,
    pins: &[PkgPin],
) -> Bond {
    let kind = grids[grid_master].kind;
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = get_io(grids, grid_master, disabled)
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = get_gt(grids, grid_master, disabled)
        .into_iter()
        .map(|gt| ((gt.kind, gt.gx, gt.gy), gt))
        .collect();
    let is_zynq = grids[grid_master].ps.is_some() && !disabled.contains(&DisabledPart::Ps);
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                if pin.vcco_bank.unwrap() != io.bank
                    && (pin.vcco_bank != Some(64) || !matches!(io.bank, 84 | 94))
                {
                    println!(
                        "wrong bank pad {pkg} {pad} {io:?} got {f} exp {b}",
                        f = pin.func,
                        b = io.bank
                    );
                }
                let old = io_banks.insert(io.bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                let mut exp_func = "IO".to_string();
                if io.kind == IoKind::Hdio {
                    write!(
                        exp_func,
                        "_L{}{}",
                        1 + io.bel / 2,
                        ['P', 'N'][io.bel as usize % 2]
                    )
                    .unwrap();
                } else {
                    let group = io.bel / 13;
                    if io.bel % 13 != 12 {
                        write!(
                            exp_func,
                            "_L{}{}",
                            1 + group * 6 + io.bel % 13 / 2,
                            ['P', 'N'][io.bel as usize % 13 % 2]
                        )
                        .unwrap();
                    }
                    write!(
                        exp_func,
                        "_T{}{}_N{}",
                        group,
                        if io.bel % 13 < 6 { 'L' } else { 'U' },
                        io.bel % 13
                    )
                    .unwrap();
                }
                if io.is_gc() {
                    if io.kind == IoKind::Hdio {
                        exp_func += "_HDGC";
                    } else {
                        exp_func += "_GC";
                    }
                }
                if io.is_dbc() {
                    exp_func += "_DBC";
                }
                if io.is_qbc() {
                    exp_func += "_QBC";
                }
                if io.is_vrp() {
                    exp_func += "_VRP";
                }
                if let Some(sm) = io.sm_pair() {
                    if io.kind == IoKind::Hdio {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.bel as usize % 2]).unwrap();
                    } else {
                        write!(
                            exp_func,
                            "_AD{}{}",
                            sm,
                            ['P', 'N'][io.bel as usize % 13 % 2]
                        )
                        .unwrap();
                    }
                }
                match io.get_cfg() {
                    Some(SharedCfgPin::Data(d)) => {
                        if !is_zynq {
                            if d >= 16 {
                                write!(exp_func, "_A{:02}", d - 16).unwrap();
                            }
                            write!(exp_func, "_D{d:02}").unwrap();
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
                            exp_func += "_CSI_ADV_B"
                        }
                    }
                    Some(SharedCfgPin::PerstN0) => exp_func += "_PERSTN0",
                    Some(SharedCfgPin::PerstN1) => exp_func += "_PERSTN1",
                    Some(SharedCfgPin::SmbAlert) => exp_func += "_SMBALERT",
                    Some(SharedCfgPin::I2cSclk) => exp_func += "_I2C_SCLK",
                    Some(SharedCfgPin::I2cSda) => {
                        exp_func += if kind == GridKind::Ultrascale {
                            "_I2C_SDA"
                        } else {
                            "_PERSTN1_I2C_SDA"
                        }
                    }
                    None => (),
                    _ => unreachable!(),
                }
                write!(exp_func, "_{}", io_banks[&io.bank]).unwrap();
                if exp_func != pin.func {
                    println!(
                        "pad {pkg} {pad} {io:?} got {f} exp {exp_func}",
                        f = pin.func
                    );
                }
                BondPin::Io(io.bank, io.bel)
            } else if pad.starts_with("GT") || pad.starts_with("RF") || pad.starts_with("HS") {
                if let Some(pin) = lookup_gt_pin(&gt_lookup, pad, &pin.func) {
                    pin
                } else {
                    println!(
                        "weird gt iopad {pkg} {p} {pad} {f}",
                        f = pin.func,
                        p = rd.part
                    );
                    continue;
                }
            } else if pad.starts_with("SYSMON") {
                let exp_site = match kind {
                    GridKind::Ultrascale => format!("SYSMONE1_X0Y{}", grid_master.to_idx()),
                    GridKind::UltrascalePlus => format!("SYSMONE4_X0Y{}", grid_master.to_idx()),
                };
                if exp_site != *pad {
                    println!(
                        "weird sysmon iopad {p} {pad} {f}",
                        f = pin.func,
                        p = rd.part
                    );
                }
                match &pin.func[..] {
                    "VP" => BondPin::SysMon(grid_master, SysMonPin::VP),
                    "VN" => BondPin::SysMon(grid_master, SysMonPin::VN),
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
        } else if let Some(p) = lookup_nonpad_pin(rd, pin) {
            p
        } else {
            println!("UNK FUNC {} {} {:?}", pkg, pin.func, pin);
            continue;
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond { pins: bond_pins }
}

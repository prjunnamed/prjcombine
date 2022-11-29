use prjcombine_entity::EntityId;
use prjcombine_int::grid::DieId;
use prjcombine_rawdump::PkgPin;
use prjcombine_virtex6::{
    Bond, BondPin, CfgPin, ExpandedDevice, GtKind, GtPin, GtRegion, GthRegionPin, GtxRegionPin,
    SharedCfgPin, SysMonPin, IoCoord, TileIobId, DisabledPart,
};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use prjcombine_rdgrid::split_num;

pub fn make_bond(edev: &ExpandedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = edev
        .get_io()
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let mut gt_lookup: HashMap<String, (String, u32, GtPin)> = HashMap::new();
    for gt in &edev.gt {
        let bank = gt.bank;
        for (i, (pp, pn)) in gt.pads_clk.iter().enumerate() {
            gt_lookup.insert(
                pp.clone(),
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
                pn.clone(),
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
            gt_lookup.insert(
                pp.clone(),
                (format!("MGTRXP{i}_{bank}"), bank, GtPin::RxP(i as u8)),
            );
            gt_lookup.insert(
                pn.clone(),
                (format!("MGTRXN{i}_{bank}"), bank, GtPin::RxN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_tx.iter().enumerate() {
            gt_lookup.insert(
                pp.clone(),
                (format!("MGTTXP{i}_{bank}"), bank, GtPin::TxP(i as u8)),
            );
            gt_lookup.insert(
                pn.clone(),
                (format!("MGTTXN{i}_{bank}"), bank, GtPin::TxN(i as u8)),
            );
        }
    }
    let mut sm_lookup: HashMap<String, (u32, SysMonPin)> = HashMap::new();
    let mut vaux_lookup: HashMap<IoCoord, (usize, char)> = HashMap::new();
    for sysmon in &edev.sysmon {
        sm_lookup.insert(sysmon.pad_vp.clone(), (sysmon.bank, SysMonPin::VP));
        sm_lookup.insert(sysmon.pad_vn.clone(), (sysmon.bank, SysMonPin::VN));
        for (i, vaux) in sysmon.vaux.iter().enumerate() {
            if let &Some((vauxp, vauxn)) = vaux {
                vaux_lookup.insert(vauxp, (i, 'P'));
                vaux_lookup.insert(vauxn, (i, 'N'));
            }
        }
    }
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let ioc = IoCoord {
                    die: DieId::from_idx(0),
                    col: io.col,
                    row: io.row - (!io.bbel as usize % 2),
                    iob: TileIobId::from_idx(io.bbel as usize % 2),
                };
                let mut exp_func =
                    format!("IO_L{}{}", io.bbel / 2, ['P', 'N'][io.bbel as usize % 2]);
                if io.is_srcc() {
                    exp_func += "_SRCC";
                }
                if io.is_mrcc() {
                    exp_func += "_MRCC";
                }
                if io.is_gc() {
                    exp_func += "_GC";
                }
                if io.is_vref() {
                    exp_func += "_VREF";
                }
                if io.is_vr() {
                    match io.row.to_idx() % 2 {
                        0 => exp_func += "_VRP",
                        1 => exp_func += "_VRN",
                        _ => unreachable!(),
                    }
                }
                match io.get_cfg() {
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
                if !edev.disabled.contains(&DisabledPart::SysMon) {
                    if let Some(&(i, pn)) = vaux_lookup.get(&ioc) {
                        write!(exp_func, "_SM{}{}", i, pn).unwrap();
                    }
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pad} {io:?} got {f} exp {exp_func}", f = pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::Io(io.bank, io.bbel)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gt(bank, gpin)
            } else if let Some(&(bank, spin)) = sm_lookup.get(pad) {
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
                "MGTAVTT" => BondPin::GtxRegion(GtRegion::All, GtxRegionPin::AVtt),
                "MGTAVCC" => BondPin::GtxRegion(GtRegion::All, GtxRegionPin::AVcc),
                "MGTAVTT_S" => BondPin::GtxRegion(GtRegion::S, GtxRegionPin::AVtt),
                "MGTAVCC_S" => BondPin::GtxRegion(GtRegion::S, GtxRegionPin::AVcc),
                "MGTAVTT_N" => BondPin::GtxRegion(GtRegion::N, GtxRegionPin::AVtt),
                "MGTAVCC_N" => BondPin::GtxRegion(GtRegion::N, GtxRegionPin::AVcc),
                "MGTAVTT_L" => BondPin::GtxRegion(GtRegion::L, GtxRegionPin::AVtt),
                "MGTAVCC_L" => BondPin::GtxRegion(GtRegion::L, GtxRegionPin::AVcc),
                "MGTAVTT_R" => BondPin::GtxRegion(GtRegion::R, GtxRegionPin::AVtt),
                "MGTAVCC_R" => BondPin::GtxRegion(GtRegion::R, GtxRegionPin::AVcc),
                "MGTAVTT_LS" => BondPin::GtxRegion(GtRegion::LS, GtxRegionPin::AVtt),
                "MGTAVCC_LS" => BondPin::GtxRegion(GtRegion::LS, GtxRegionPin::AVcc),
                "MGTAVTT_LN" => BondPin::GtxRegion(GtRegion::LN, GtxRegionPin::AVtt),
                "MGTAVCC_LN" => BondPin::GtxRegion(GtRegion::LN, GtxRegionPin::AVcc),
                "MGTAVTT_RS" => BondPin::GtxRegion(GtRegion::RS, GtxRegionPin::AVtt),
                "MGTAVCC_RS" => BondPin::GtxRegion(GtRegion::RS, GtxRegionPin::AVcc),
                "MGTAVTT_RN" => BondPin::GtxRegion(GtRegion::RN, GtxRegionPin::AVtt),
                "MGTAVCC_RN" => BondPin::GtxRegion(GtRegion::RN, GtxRegionPin::AVcc),
                "MGTHAVTT_L" => BondPin::GthRegion(GtRegion::L, GthRegionPin::AVtt),
                "MGTHAVCC_L" => BondPin::GthRegion(GtRegion::L, GthRegionPin::AVcc),
                "MGTHAVCCRX_L" => BondPin::GthRegion(GtRegion::L, GthRegionPin::AVccRx),
                "MGTHAVCCPLL_L" => BondPin::GthRegion(GtRegion::L, GthRegionPin::AVccPll),
                "MGTHAGND_L" => BondPin::GthRegion(GtRegion::L, GthRegionPin::AGnd),
                "MGTHAVTT_R" => BondPin::GthRegion(GtRegion::R, GthRegionPin::AVtt),
                "MGTHAVCC_R" => BondPin::GthRegion(GtRegion::R, GthRegionPin::AVcc),
                "MGTHAVCCRX_R" => BondPin::GthRegion(GtRegion::R, GthRegionPin::AVccRx),
                "MGTHAVCCPLL_R" => BondPin::GthRegion(GtRegion::R, GthRegionPin::AVccPll),
                "MGTHAGND_R" => BondPin::GthRegion(GtRegion::R, GthRegionPin::AGnd),
                "MGTHAVTT" => BondPin::GthRegion(GtRegion::All, GthRegionPin::AVtt),
                "MGTHAVCC" => BondPin::GthRegion(GtRegion::All, GthRegionPin::AVcc),
                "MGTHAVCCRX" => BondPin::GthRegion(GtRegion::All, GthRegionPin::AVccRx),
                "MGTHAVCCPLL" => BondPin::GthRegion(GtRegion::All, GthRegionPin::AVccPll),
                "MGTHAGND" => BondPin::GthRegion(GtRegion::All, GthRegionPin::AGnd),
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

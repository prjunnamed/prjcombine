use prjcombine_virtex4::{Bond, BondPin, CfgPin, Grid, GtPin, SharedCfgPin, SysMonPin};

use prjcombine_rawdump::PkgPin;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use prjcombine_rdgrid::split_num;

pub fn make_bond(grid: &Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = grid
        .get_gt()
        .into_iter()
        .flat_map(|gt| {
            gt.get_pads(grid)
                .into_iter()
                .map(move |(name, func, pin)| (name, (func, gt.bank, pin)))
        })
        .collect();
    let sm_lookup: HashMap<_, _> = grid
        .get_sysmon_pads()
        .into_iter()
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func =
                    format!("IO_L{}{}", io.bbel / 2, ['N', 'P'][io.bbel as usize % 2]);
                #[allow(clippy::single_match)]
                match io.get_cfg() {
                    Some(SharedCfgPin::Data(d)) => write!(exp_func, "_D{d}").unwrap(),
                    None => (),
                }
                if io.is_gc() {
                    exp_func += "_GC";
                }
                if io.is_vref() {
                    exp_func += "_VREF";
                }
                if io.is_vr() {
                    match io.bel {
                        0 => exp_func += "_VRP",
                        1 => exp_func += "_VRN",
                        _ => unreachable!(),
                    }
                }
                if io.is_cc() {
                    exp_func += "_CC";
                }
                if let Some((bank, sm)) = io.sm_pair(grid) {
                    write!(exp_func, "_{}{}", ["SM", "ADC"][bank as usize], sm).unwrap();
                }
                if io.is_lc() {
                    exp_func += "_LC";
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
